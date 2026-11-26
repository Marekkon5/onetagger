"""Publish only complete artifacts from the latest successful default-branch Build."""

import argparse
import base64
import json
import os
from pathlib import Path
import re
import subprocess
import urllib.error
import urllib.request
from urllib.parse import quote, urlencode

from release_tools import PACKAGES, release_notes, sha256, verify_artifacts, version_key


class StaleBuild(ValueError):
    """A newer default-branch commit or Build superseded an automatic trigger."""


class GitHub:
    def __init__(self, repository):
        if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
            raise ValueError("Invalid repository")
        self.repository = repository

    def __call__(self, path, method="GET", data=None):
        request = urllib.request.Request(
            f"https://api.github.com/repos/{self.repository}/{path}",
            data=json.dumps(data).encode() if data is not None else None,
            method=method,
            headers={"Authorization": "Bearer " + os.environ["GH_TOKEN"],
                     "Accept": "application/vnd.github+json", "Content-Type": "application/json",
                     "X-GitHub-Api-Version": "2022-11-28", "User-Agent": "OneTagger-release"})
        with urllib.request.urlopen(request, timeout=30) as response:
            body = response.read()
            return json.loads(body) if body else None

    def upload(self, tag, path):
        # No --clobber: a retry can add missing assets, never replace existing ones.
        subprocess.run(["gh", "release", "upload", tag, str(path), "--repo", self.repository], check=True)


def optional(read, path):
    try:
        return read(path)
    except urllib.error.HTTPError as error:
        if error.code != 404:
            raise
        return None


def source_text(read, path, commit, allow_missing=False):
    row = optional(read, "contents/" + path + "?" + urlencode({"ref": commit}))
    if row is None and allow_missing:
        return None
    if row is None or row.get("encoding") != "base64":
        raise ValueError("Missing source file: " + path)
    return base64.b64decode(row["content"]).decode("utf-8")


def tag_commit(read, tag):
    ref = optional(read, "git/ref/tags/" + quote(tag, safe=""))
    if ref is None:
        return None
    obj = ref["object"]
    seen = set()
    while obj.get("type") == "tag":
        if obj["sha"] in seen or len(seen) >= 8:
            raise ValueError("Invalid annotated tag chain")
        seen.add(obj["sha"])
        obj = read("git/tags/" + obj["sha"])["object"]
    if obj.get("type") != "commit":
        raise ValueError("Release tag must point to a commit")
    return obj["sha"]


def successful_build(read, run_id, repository, branch):
    if not str(run_id).isdigit() or int(run_id) <= 0:
        raise ValueError("Specify a numeric Build run ID")
    run = read("actions/runs/" + str(run_id))
    if (run.get("id") != int(run_id) or
            run.get("path") != ".github/workflows/build.yml" or
            run.get("event") not in ("push", "workflow_dispatch") or
            run.get("head_branch") != branch or
            (run.get("head_repository") or {}).get("full_name") != repository or
            run.get("status") != "completed" or run.get("conclusion") != "success"):
        raise ValueError("Release requires a successful default-branch Build from this repository")
    if not re.fullmatch(r"[0-9a-f]{40}", run.get("head_sha", "")):
        raise ValueError("Invalid source commit")
    return run


def validated_run(read, run_id, repository, branch):
    run = successful_build(read, run_id, repository, branch) if run_id else None
    source = read("branches/" + quote(branch, safe=""))["commit"]["sha"]
    if run is not None and run["head_sha"] != source:
        raise StaleBuild("The default branch moved; use its latest successful Build")
    query = urlencode({"head_sha": source, "branch": branch, "per_page": 100})
    runs = read("actions/workflows/build.yml/runs?" + query)["workflow_runs"]
    trusted = [r for r in runs if r.get("head_sha") == source and r.get("head_branch") == branch
               and (r.get("head_repository") or {}).get("full_name") == repository
               and r.get("event") in ("push", "workflow_dispatch")]
    latest = max(trusted, key=lambda r: (r["id"], r.get("run_attempt", 1)), default=None)
    if latest is None or latest.get("status") != "completed" or latest.get("conclusion") != "success":
        raise StaleBuild("The latest Build for the current default-branch commit must pass")
    if run is None:
        run = successful_build(read, latest["id"], repository, branch)
    if (run["id"] != latest["id"] or run["head_sha"] != source or
            run.get("run_attempt", 1) != latest.get("run_attempt", 1)):
        raise StaleBuild("Use the latest successful Build of the current default-branch commit")
    return run


def plan_release(read, run_id, repository, branch, automatic=False):
    try:
        run = validated_run(read, run_id, repository, branch)
    except StaleBuild as error:
        if automatic:
            return None, str(error) + "; automatic release skipped."
        raise
    run_id = str(run["id"])
    source = run["head_sha"]
    version = source_text(read, "VERSION", source).strip()
    number = version_key(version)
    if automatic and run["event"] != "push":
        return None, "Manual Build runs do not automatically publish releases."

    latest = optional(read, "releases/latest")
    latest_number = None
    if latest is not None:
        latest_tag = latest.get("tag_name", "")
        if not latest_tag.startswith("v"):
            raise ValueError("The latest published release must have a stable version tag")
        latest_number = version_key(latest_tag[1:])
    if automatic:
        parents = read("commits/" + source)["parents"]
        previous = source_text(read, "VERSION", parents[0]["sha"], allow_missing=True) if parents else None
        if previous is None:
            return None, "VERSION baseline added; this does not publish an inherited release."
        previous_number = version_key(previous.strip())
        if number < previous_number:
            raise ValueError("Automatic release versions must increase")
        if latest_number is None and number == previous_number:
            return None, "The first automatic release requires a VERSION bump."
        if number == latest_number:
            return None, f"Version {version} is already published."
    notes = release_notes(source_text(read, "CHANGELOG.md", source), version)
    tag = "v" + version
    target = tag_commit(read, tag)
    if target is not None and target != source:
        raise ValueError("The version tag already points to a different commit; it will not be moved")
    existing = optional(read, "releases/tags/" + tag)
    if existing is not None and not existing["draft"]:
        if target != source:
            raise ValueError("Published release tag is missing or changed")
        return None, "This version is already published; nothing will be overwritten."
    marker = "<!-- onetagger-release:" + source + " -->"
    if existing is not None and (target != source or marker not in (existing.get("body") or "")):
        raise ValueError("Existing draft is not an automation draft for this source")
    if latest_number is not None and number <= latest_number:
        raise ValueError("The release must be newer than the latest published version")
    rows = read(f"actions/runs/{run_id}/artifacts?per_page=100")["artifacts"]
    expected = {"onetagger-" + platform for platform in PACKAGES}
    artifacts = [a for a in rows if a["name"] in expected]
    if (len(artifacts) != len(expected) or {a["name"] for a in artifacts} != expected or
            any(a.get("expired", True) for a in artifacts)):
        raise ValueError("All four non-expired native artifacts are required")
    for artifact in artifacts:
        origin = artifact.get("workflow_run", {})
        if origin.get("id") != int(run_id) or origin.get("head_sha") != source:
            raise ValueError("Artifact does not belong to the validated source run")
    return {"version": version, "tag": tag, "source": source, "run_id": run_id,
            "artifact_ids": ",".join(str(int(a["id"])) for a in sorted(artifacts, key=lambda a: a["name"])),
            "notes": notes, "marker": marker}, "Validated release " + tag


def release_body(plan, repository):
    base = f"https://github.com/{repository}/releases/download/{plan['tag']}/"
    rows = []
    for platform, packages in PACKAGES.items():
        links = " · ".join(f"[{name}]({base}{name})" for name in sorted(packages))
        rows.append(f"| {platform} | {links} | [SHA-256]({base}SHA256SUMS-{platform}.txt) |")
    return (plan["marker"] + f"\n\n{plan['notes']}\n\n"
            "## Downloads\n\n| Platform | Packages | Checksums |\n| --- | --- | --- |\n" +
            "\n".join(rows) + "\n\nWindows and macOS packages are unsigned. "
            "Linux packages require compatible glibc, ALSA and WebKitGTK 4.1 runtimes.\n\n"
            f"Source: [{plan['source']}](https://github.com/{repository}/commit/{plan['source']})\n\n"
            f"Validated build: https://github.com/{repository}/actions/runs/{plan['run_id']}\n")


def publish_release(api, plan, directory, repository, branch, publish=False):
    # Artifact contents are data only. Never execute scripts from a downloaded artifact.
    assets = verify_artifacts(directory, plan["version"], plan["source"], repository, plan["run_id"])
    fresh, _ = plan_release(api, plan["run_id"], repository, branch)
    if fresh is None:
        return "Release is already published."
    if fresh != plan:
        raise ValueError("Release plan changed during artifact validation")
    tag = plan["tag"]
    if tag_commit(api, tag) is None:
        api("git/refs", "POST", {"ref": "refs/tags/" + tag, "sha": plan["source"]})
    if tag_commit(api, tag) != plan["source"]:
        raise ValueError("Release tag changed")
    release = optional(api, "releases/tags/" + tag)
    if release is None:
        release = api("releases", "POST", {"tag_name": tag, "target_commitish": plan["source"],
                      "name": "OneTagger " + plan["version"], "body": release_body(plan, repository),
                      "draft": True, "prerelease": False})
    release_id = release["id"]
    existing = api(f"releases/{release_id}/assets?per_page=100")
    by_name = {a["name"]: a for a in existing}
    if len(by_name) != len(existing) or not set(by_name) <= {p.name for p in assets}:
        raise ValueError("The draft contains unexpected or duplicate assets")
    for path in assets:
        current = api("releases/" + str(release_id))
        if not current["draft"] or plan["marker"] not in (current.get("body") or ""):
            raise ValueError("Release changed while uploading; published releases are not modified")
        if path.name in by_name:
            asset = by_name[path.name]
            if asset.get("digest") != "sha256:" + sha256(path) or asset.get("state") != "uploaded":
                raise ValueError("Existing draft asset differs; refusing to overwrite " + path.name)
        else:
            api.upload(tag, path)
    uploaded = api(f"releases/{release_id}/assets?per_page=100")
    if len(uploaded) != len(assets) or {a["name"] for a in uploaded} != {p.name for p in assets}:
        raise ValueError("The draft does not have all release assets")
    expected = {p.name: "sha256:" + sha256(p) for p in assets}
    if any(a.get("state") != "uploaded" or a.get("digest") != expected[a["name"]] for a in uploaded):
        raise ValueError("Uploaded release asset verification failed")
    # Recheck the current branch, successful build and tag immediately before publication.
    final, _ = plan_release(api, plan["run_id"], repository, branch)
    if final != plan or tag_commit(api, tag) != plan["source"]:
        raise ValueError("Release source changed before publication")
    if publish:
        api("releases/" + str(release_id), "PATCH", {"draft": False, "make_latest": "true"})
    return ("Published " if publish else "Prepared draft ") + tag


def summary(message):
    print(message)
    with Path(os.environ["GITHUB_STEP_SUMMARY"]).open("a", encoding="utf-8") as out:
        out.write(message + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("plan", "publish"))
    parser.add_argument("--assets", type=Path, default=Path("release-artifacts"))
    args = parser.parse_args()
    repo = os.environ["GITHUB_REPOSITORY"]
    branch = os.environ["RELEASE_DEFAULT_BRANCH"]
    if os.environ["GITHUB_REF"] != "refs/heads/" + branch:
        raise ValueError("Run release automation from the default branch only")
    api = GitHub(repo)
    run_id = os.environ.get("BUILD_RUN_ID", "").strip()
    automatic = args.command == "plan" and os.environ.get("AUTOMATIC_RELEASE") == "true"
    if args.command == "publish" and not run_id:
        raise ValueError("Publication requires the Build run selected by the release plan")
    if automatic and not run_id:
        raise ValueError("An automatic release requires its triggering Build run ID")
    plan, reason = plan_release(api, run_id, repo, branch, automatic=automatic)
    if args.command == "plan":
        with Path(os.environ["GITHUB_OUTPUT"]).open("a", encoding="utf-8") as output:
            output.write("release=" + str(plan is not None).lower() + "\n")
            if plan:
                for key in ("version", "source", "run_id", "artifact_ids"):
                    output.write(f"{key}={plan[key]}\n")
        summary(reason)
    elif plan:
        if plan["source"] != os.environ["EXPECTED_SOURCE"] or plan["version"] != os.environ["EXPECTED_VERSION"]:
            raise ValueError("Source or version changed between release jobs")
        summary(publish_release(api, plan, args.assets, repo, branch,
                                publish=os.environ.get("PUBLISH_RELEASE") == "true"))
    else:
        summary(reason)


if __name__ == "__main__":
    main()
