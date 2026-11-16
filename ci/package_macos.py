"""Package the already validated native executable using standard macOS bundle metadata."""

import plistlib
import re
import shutil
import subprocess
import tomllib
from pathlib import Path


def main() -> None:
    root = Path(__file__).resolve().parents[1]
    package = tomllib.loads((root / "crates/onetagger/Cargo.toml").read_text())["package"]
    metadata = package["metadata"]["bundle"]
    binary = root / "target/release/onetagger"

    # Match the executable's actual deployment target instead of claiming support for an older OS.
    build_info = subprocess.check_output(["vtool", "-show-build", str(binary)], text=True)
    minimum = re.search(r"^\s*(?:minos|version)\s+(\d+(?:\.\d+)+)\s*$", build_info, re.MULTILINE)
    if minimum is None:
        raise RuntimeError(f"Could not determine the binary's minimum macOS version: {build_info}")

    bundle = root / "target/release/bundle/osx/OneTagger.app"
    if bundle.exists():
        shutil.rmtree(bundle)
    contents = bundle / "Contents"
    (contents / "MacOS").mkdir(parents=True)
    (contents / "Resources").mkdir()
    shutil.copy2(binary, contents / "MacOS/onetagger")
    shutil.copy2(root / "assets/icon.icns", contents / "Resources/icon.icns")
    info = {
        "CFBundleDevelopmentRegion": "en",
        "CFBundleExecutable": "onetagger",
        "CFBundleIconFile": "icon.icns",
        "CFBundleIdentifier": metadata["identifier"],
        "CFBundleInfoDictionaryVersion": "6.0",
        "CFBundleName": metadata["name"],
        "CFBundleDisplayName": metadata["name"],
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": package["version"],
        "CFBundleVersion": package["version"],
        "LSApplicationCategoryType": "public.app-category.music",
        "LSMinimumSystemVersion": minimum.group(1),
        "NSHighResolutionCapable": True,
        "NSHumanReadableCopyright": metadata["copyright"],
    }
    with (contents / "Info.plist").open("wb") as output:
        plistlib.dump(info, output)
    (contents / "PkgInfo").write_bytes(b"APPL????")
    subprocess.run(["plutil", "-lint", str(contents / "Info.plist")], check=True)
    print(f"Packaged {bundle} (macOS {minimum.group(1)} or newer)")


if __name__ == "__main__":
    main()
