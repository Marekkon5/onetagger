"""Version synchronization and offline verification of native release artifacts."""

import argparse
from datetime import date
import hashlib
import json
import os
from pathlib import Path
import re
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CRATES = ("onetagger", "onetagger-cli", "onetagger-shared")
PACKAGES = {
    "linux-x64": {"OneTagger-linux-x64.tar.gz", "OneTagger-linux-x64-cli.tar.gz"},
    "windows-x64": {"OneTagger-windows.exe", "OneTagger-windows-cli.exe", "OneTagger-windows-setup.exe"},
    "macos-intel": {"OneTagger-macos-intel.zip", "OneTagger-macos-intel-cli.zip"},
    "macos-arm64": {"OneTagger-macos-arm64.zip", "OneTagger-macos-arm64-cli.zip"},
}


def version_key(value):
    # Windows VIProductVersion needs numeric, 16-bit components. Preview releases
    # need a separate product/file-version policy before they can be supported.
    if not re.fullmatch(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)", value):
        raise ValueError("Use a stable X.Y.Z version without a v prefix")
    parts = tuple(map(int, value.split(".")))
    if max(parts) > 65535:
        raise ValueError("Version components must fit the Windows installer format")
    return parts


def checked_version(root=ROOT):
    version = (root / "VERSION").read_text(encoding="utf-8").strip()
    version_key(version)
    for crate in CRATES:
        package = tomllib.loads((root / f"crates/{crate}/Cargo.toml").read_text(encoding="utf-8"))["package"]
        if package["version"] != version:
            raise ValueError(f"{crate} version does not match VERSION")
        if crate == "onetagger" and package["metadata"]["bundle"]["version"] != version:
            raise ValueError("macOS bundle metadata does not match VERSION")
    lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
    for crate in CRATES:
        entries = [p for p in lock["package"] if p["name"] == crate and "source" not in p]
        if len(entries) != 1 or entries[0]["version"] != version:
            raise ValueError(f"Cargo.lock version for {crate} does not match VERSION")
    return version


def release_notes(changelog, version):
    version_key(version)
    match = re.search(r"(?m)^## " + re.escape(version) + r" — (\d{4}-\d{2}-\d{2})\s*\n", changelog)
    if match is None:
        raise ValueError(f"Add a dated '## {version} — YYYY-MM-DD' changelog section")
    date.fromisoformat(match[1])
    notes = re.split(r"(?m)^#{1,2} ", changelog[match.end():], maxsplit=1)[0].strip()
    if not notes:
        raise ValueError("Release notes must not be empty")
    return notes


def bump_version(version, root=ROOT, today=None):
    old = checked_version(root)
    if version_key(version) <= version_key(old):
        raise ValueError("The new version must increase")
    changes = {}
    for crate in CRATES:
        path = root / f"crates/{crate}/Cargo.toml"
        content, count = re.subn(r'(?m)^version = "' + re.escape(old) + r'"$',
                                 f'version = "{version}"', path.read_text(encoding="utf-8"))
        if count != (2 if crate == "onetagger" else 1):
            raise ValueError(f"Unexpected version layout in {path}")
        changes[path] = content
    lock_path = root / "Cargo.lock"
    lock = lock_path.read_text(encoding="utf-8")
    for crate in CRATES:
        lock, count = re.subn(r'(\[\[package\]\]\nname = "' + crate + r'"\nversion = ")' +
                             re.escape(old) + '"', lambda m: m[1] + version + '"', lock)
        if count != 1:
            raise ValueError(f"Unexpected lockfile entry for {crate}")
    changes[lock_path] = lock
    changelog_path = root / "CHANGELOG.md"
    changelog = changelog_path.read_text(encoding="utf-8")
    prefix = "## Unreleased (fantuz fork)\n"
    if not changelog.startswith(prefix):
        raise ValueError("Expected the Unreleased changelog section first")
    rest = changelog[len(prefix):]
    boundary = re.search(r"(?m)^#{1,2} ", rest)
    notes = rest[:boundary.start() if boundary else len(rest)].strip()
    if not notes:
        raise ValueError("Add release notes under Unreleased before bumping")
    historical = rest[boundary.start():] if boundary else ""
    changes[changelog_path] = (prefix + f"\n## {version} — {today or date.today()}\n\n" +
                               notes + "\n\n" + historical)
    changes[root / "VERSION"] = version + "\n"
    # Validate the complete edit set before changing any files; dependencies stay locked.
    for path, content in changes.items():
        path.write_text(content, encoding="utf-8")


def sha256(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def provenance(version, source, repository, run_id, platform):
    version_key(version)
    if not re.fullmatch(r"[0-9a-f]{40}", source):
        raise ValueError("Expected a full source commit SHA")
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
        raise ValueError("Invalid repository name")
    if not str(run_id).isdigit() or int(run_id) <= 0 or platform not in PACKAGES:
        raise ValueError("Invalid build run or platform")
    return {"schema": 1, "version": version, "source_commit": source,
            "repository": repository, "run_id": str(run_id), "platform": platform}


def write_manifest(directory, version, source, repository, run_id, platform):
    manifest = provenance(version, source, repository, run_id, platform)
    checksums = f"SHA256SUMS-{platform}.txt"
    expected = PACKAGES[platform] | {checksums}
    if {p.name for p in directory.iterdir()} != expected:
        raise ValueError(f"Unexpected or missing native packages for {platform}")
    if any(p.is_symlink() or not p.is_file() or p.stat().st_size == 0 for p in directory.iterdir()):
        raise ValueError("Only nonempty regular release files are allowed")
    manifest["sha256"] = {name: sha256(directory / name) for name in sorted(expected)}
    (directory / f"release-manifest-{platform}.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def verify_artifacts(directory, version, source, repository, run_id):
    expected_dirs = {"onetagger-" + p for p in PACKAGES}
    if {p.name for p in directory.iterdir()} != expected_dirs:
        raise ValueError("All four native platform artifacts are required")
    assets = []
    for platform, packages in PACKAGES.items():
        folder = directory / ("onetagger-" + platform)
        manifest_name = f"release-manifest-{platform}.json"
        sums_name = f"SHA256SUMS-{platform}.txt"
        expected_files = packages | {sums_name, manifest_name}
        if folder.is_symlink() or not folder.is_dir():
            raise ValueError("Invalid artifact directory")
        if {p.name for p in folder.iterdir()} != expected_files:
            raise ValueError(f"Unexpected or missing files for {platform}")
        if any(p.is_symlink() or not p.is_file() or p.stat().st_size == 0 for p in folder.iterdir()):
            raise ValueError("Only nonempty regular release files are allowed")
        manifest = json.loads((folder / manifest_name).read_text(encoding="utf-8"))
        expected = provenance(version, source, repository, run_id, platform)
        if {k: v for k, v in manifest.items() if k != "sha256"} != expected:
            raise ValueError(f"Artifact provenance mismatch for {platform}")
        digests = manifest.get("sha256", {})
        if set(digests) != packages | {sums_name}:
            raise ValueError("Incomplete artifact digest list")
        for name, digest in digests.items():
            if digest != sha256(folder / name):
                raise ValueError(f"Artifact checksum mismatch: {name}")
        lines = (folder / sums_name).read_text(encoding="utf-8").splitlines()
        if len(lines) != len(packages) or set(lines) != {f"{digests[n]}  {n}" for n in packages}:
            raise ValueError("Native checksum file does not match the release packages")
        assets.extend(folder / name for name in sorted(expected_files))
    return sorted(assets)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("check")
    bump = sub.add_parser("bump")
    bump.add_argument("version")
    manifest = sub.add_parser("manifest")
    manifest.add_argument("platform", choices=PACKAGES)
    manifest.add_argument("directory", type=Path)
    verify = sub.add_parser("verify")
    verify.add_argument("directory", type=Path)
    args = parser.parse_args()
    if args.command == "bump":
        bump_version(args.version)
    elif args.command == "check":
        print("Synchronized release version:", checked_version())
    else:
        common = (checked_version(), os.environ["GITHUB_SHA"],
                  os.environ["GITHUB_REPOSITORY"], os.environ["GITHUB_RUN_ID"])
        if args.command == "manifest":
            write_manifest(args.directory, *common, args.platform)
        else:
            print("Verified", len(verify_artifacts(args.directory, *common)), "release assets")


if __name__ == "__main__":
    main()
