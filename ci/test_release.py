"""Offline release regressions: no credentials, network requests or publication."""

import base64
import copy
import json
from pathlib import Path
import tempfile
import unittest
import urllib.error
from urllib.parse import parse_qs, urlsplit

import release
import release_tools as tools

SOURCE = "a" * 40
PARENT = "b" * 40
REPO = "fantuz/onetagger"
RUN = "42"
VERSION = "1.7.1"


def not_found(path):
    raise urllib.error.HTTPError(path, 404, "Not Found", {}, None)


class FakeGitHub:
    def __init__(self):
        self.run = {"id": int(RUN), "head_sha": SOURCE, "head_branch": "master",
                    "head_repository": {"full_name": REPO}, "path": ".github/workflows/build.yml",
                    "event": "push", "status": "completed", "conclusion": "success", "run_attempt": 1}
        self.runs = [self.run]
        self.branch = SOURCE
        self.version = VERSION
        self.previous = "1.7.0"
        self.changelog = "## Unreleased (fantuz fork)\n\n## 1.7.1 — 2026-11-16\n\n- Fix token refresh.\n"
        self.artifacts = [{"id": i + 1, "name": "onetagger-" + platform, "expired": False,
                           "workflow_run": {"id": int(RUN), "head_sha": SOURCE}}
                          for i, platform in enumerate(tools.PACKAGES)]
        self.tag = None
        self.release = None
        self.assets = []
        self.writes = []
        self.fail_upload_after = None
        self.move_after_upload = False

    def __call__(self, path, method="GET", data=None):
        if method != "GET":
            self.writes.append((method, path, copy.deepcopy(data)))
            if path == "git/refs" and method == "POST":
                self.tag = {"type": "commit", "sha": data["sha"]}
                return {"object": self.tag}
            if path == "releases" and method == "POST":
                self.release = {"id": 7, **data}
                return copy.deepcopy(self.release)
            if path == "releases/7" and method == "PATCH":
                self.release.update(data)
                return copy.deepcopy(self.release)
            raise AssertionError((method, path, data))
        if path == "actions/runs/" + RUN:
            return copy.deepcopy(self.run)
        if path.startswith("actions/workflows/build.yml/runs?"):
            return {"workflow_runs": copy.deepcopy(self.runs)}
        if path == "branches/master":
            return {"commit": {"sha": self.branch}}
        if path == "commits/" + SOURCE:
            return {"parents": [{"sha": PARENT}]}
        if path.startswith("contents/"):
            url = urlsplit(path)
            source = parse_qs(url.query)["ref"][0]
            if url.path == "contents/VERSION":
                value = self.version if source == SOURCE else self.previous
            elif url.path == "contents/CHANGELOG.md":
                value = self.changelog
            else:
                raise AssertionError(path)
            if value is None:
                return not_found(path)
            return {"encoding": "base64", "content": base64.b64encode(value.encode()).decode()}
        if path.startswith("git/ref/tags/"):
            return {"object": copy.deepcopy(self.tag)} if self.tag else not_found(path)
        if path.startswith("releases/tags/") or path == "releases/7":
            return copy.deepcopy(self.release) if self.release else not_found(path)
        if path == f"actions/runs/{RUN}/artifacts?per_page=100":
            return {"artifacts": copy.deepcopy(self.artifacts)}
        if path == "releases/7/assets?per_page=100":
            return copy.deepcopy(self.assets)
        raise AssertionError(path)

    def upload(self, tag, path):
        if self.fail_upload_after is not None and len(self.assets) >= self.fail_upload_after:
            raise OSError("Simulated upload interruption")
        if any(a["name"] == path.name for a in self.assets):
            raise AssertionError("An existing asset would be overwritten")
        self.writes.append(("UPLOAD", tag, path.name))
        self.assets.append({"name": path.name, "digest": "sha256:" + tools.sha256(path), "state": "uploaded"})
        if self.move_after_upload:
            self.branch = "c" * 40


def make_artifacts(root):
    for platform, packages in tools.PACKAGES.items():
        folder = root / ("onetagger-" + platform)
        folder.mkdir()
        for name in packages:
            (folder / name).write_bytes(("synthetic package: " + name).encode())
        (folder / f"SHA256SUMS-{platform}.txt").write_text(
            "".join(f"{tools.sha256(folder / name)}  {name}\n" for name in sorted(packages)), encoding="utf-8")
        tools.write_manifest(folder, VERSION, SOURCE, REPO, RUN, platform)


class VersionTests(unittest.TestCase):
    def test_repository_versions_are_synchronized(self):
        tools.checked_version()

    def test_invalid_and_unpackageable_versions(self):
        for value in ["v1.7.1", "1.7", "1.7.1-rc.1", "1.7.1+build", "01.7.1", "1.7.65536", "1.7.1\n"]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                tools.version_key(value)

    def test_release_notes_require_valid_date_and_nonempty_section(self):
        for content in ["# 1.7.1\nOld notes", "## 1.7.1 — 2026-02-30\nBad date",
                        "## 1.7.1 — 2026-11-16\n\n## 1.7.0 — 2026-11-01\nOld notes"]:
            with self.subTest(content=content), self.assertRaises(ValueError):
                tools.release_notes(content, VERSION)

    def test_bump_synchronizes_versions_and_preserves_dependencies(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "VERSION").write_text("1.7.0\n")
            (root / "CHANGELOG.md").write_text("## Unreleased (fantuz fork)\n\n- Fix refresh.\n\n# 1.7.0\nHistorical notes\n")
            for crate in tools.CRATES:
                path = root / f"crates/{crate}/Cargo.toml"
                path.parent.mkdir(parents=True)
                content = f'[package]\nname = "{crate}"\nversion = "1.7.0"\n[dependencies]\nid3 = "1.7.0"\n'
                if crate == "onetagger":
                    content += '[package.metadata.bundle]\nversion = "1.7.0"\n'
                path.write_text(content)
            lock = "\n".join(f'[[package]]\nname = "{crate}"\nversion = "1.7.0"\n' for crate in tools.CRATES)
            foreign = '[[package]]\nname = "id3"\nversion = "1.7.0"\nsource = "registry+https://example.invalid"\n'
            (root / "Cargo.lock").write_text(lock + foreign)
            tools.bump_version(VERSION, root, "2026-11-16")
            self.assertEqual(tools.checked_version(root), VERSION)
            self.assertIn(foreign, (root / "Cargo.lock").read_text())
            self.assertEqual(tools.release_notes((root / "CHANGELOG.md").read_text(), VERSION), "- Fix refresh.")
            self.assertIn("Historical notes", (root / "CHANGELOG.md").read_text())
            with self.assertRaises(ValueError):
                tools.bump_version("1.7.0", root)


class ReleasePlanTests(unittest.TestCase):
    def setUp(self):
        self.api = FakeGitHub()

    def plan(self, automatic=True):
        return release.plan_release(self.api, RUN, REPO, "master", automatic)

    def test_version_bump_selects_exact_native_artifacts(self):
        plan, _ = self.plan()
        self.assertEqual((plan["source"], plan["version"], plan["run_id"]), (SOURCE, VERSION, RUN))
        self.assertEqual(set(plan["artifact_ids"].split(",")), {"1", "2", "3", "4"})
        self.assertFalse(self.api.writes)

    def test_initial_baseline_and_normal_commits_do_not_release(self):
        for previous in [None, VERSION]:
            self.api.previous = previous
            self.assertIsNone(self.plan()[0])

    def test_downgrade_is_rejected(self):
        self.api.previous = "2.0.0"
        with self.assertRaises(ValueError):
            self.plan()

    def test_pr_fork_failed_wrong_workflow_and_unfinished_runs_are_rejected(self):
        mutations = [{"event": "pull_request"}, {"head_repository": {"full_name": "other/onetagger"}},
                     {"conclusion": "failure"}, {"status": "in_progress"}, {"head_branch": "topic"},
                     {"path": ".github/workflows/other.yml"}, {"head_sha": "not-a-sha"}]
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                self.api = FakeGitHub()
                self.api.run.update(mutation)
                with self.assertRaises(ValueError):
                    self.plan()

    def test_manual_build_requires_explicit_release_request(self):
        self.api.run["event"] = "workflow_dispatch"
        self.assertIsNone(self.plan()[0])
        self.assertIsNotNone(self.plan(automatic=False)[0])

    def test_stale_head_skips_automatic_trigger_but_rejects_manual_release(self):
        self.api.branch = "c" * 40
        self.assertIsNone(self.plan()[0])
        with self.assertRaises(release.StaleBuild):
            self.plan(automatic=False)

    def test_newer_failed_or_pending_build_cannot_be_bypassed(self):
        for state in [("completed", "failure"), ("in_progress", None)]:
            newer = {**self.api.run, "id": 43, "status": state[0], "conclusion": state[1]}
            self.api.runs = [self.api.run, newer]
            with self.subTest(state=state), self.assertRaises(release.StaleBuild):
                self.plan(automatic=False)

    def test_missing_expired_duplicate_and_foreign_artifacts_are_rejected(self):
        for case in ["missing", "expired", "duplicate", "foreign"]:
            self.api = FakeGitHub()
            if case == "missing":
                self.api.artifacts.pop()
            elif case == "expired":
                self.api.artifacts[0]["expired"] = True
            elif case == "duplicate":
                self.api.artifacts.append(self.api.artifacts[0])
            else:
                self.api.artifacts[0]["workflow_run"]["head_sha"] = "c" * 40
            with self.subTest(case=case), self.assertRaises(ValueError):
                self.plan()

    def test_conflicting_tag_and_unowned_draft_are_rejected(self):
        self.api.tag = {"type": "commit", "sha": PARENT}
        with self.assertRaises(ValueError):
            self.plan()
        self.api.tag["sha"] = SOURCE
        self.api.release = {"id": 7, "draft": True, "body": "Manually prepared draft"}
        with self.assertRaises(ValueError):
            self.plan()

    def test_published_release_is_not_modified(self):
        self.api.tag = {"type": "commit", "sha": SOURCE}
        self.api.release = {"id": 7, "draft": False, "body": "Published"}
        self.assertIsNone(self.plan()[0])
        self.assertEqual(self.api.writes, [])

    def test_annotated_tags_resolve_and_cycles_fail(self):
        def read(path):
            return {"object": {"type": "tag", "sha": PARENT}} if path.startswith("git/ref/") else {"object": {"type": "commit", "sha": SOURCE}}
        self.assertEqual(release.tag_commit(read, "v1.7.1"), SOURCE)
        with self.assertRaises(ValueError):
            release.tag_commit(lambda _: {"object": {"type": "tag", "sha": PARENT}}, "v1.7.1")


class ArtifactAndPublicationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        make_artifacts(self.root)
        self.api = FakeGitHub()
        self.plan = release.plan_release(self.api, RUN, REPO, "master", True)[0]

    def verify(self):
        return tools.verify_artifacts(self.root, VERSION, SOURCE, REPO, RUN)

    def publish(self, publish=False):
        return release.publish_release(self.api, self.plan, self.root, REPO, "master", publish)

    def test_all_native_packages_and_metadata_are_verified(self):
        self.assertEqual(len(self.verify()), 17)

    def test_tampered_package_fails_before_any_remote_write(self):
        (self.root / "onetagger-linux-x64/OneTagger-linux-x64.tar.gz").write_bytes(b"tampered")
        with self.assertRaises(ValueError):
            self.publish(True)
        self.assertFalse(self.api.writes)

    def test_wrong_source_version_run_or_repository_is_rejected(self):
        manifest = self.root / "onetagger-linux-x64/release-manifest-linux-x64.json"
        original = json.loads(manifest.read_text())
        for field, value in [("source_commit", PARENT), ("version", "1.7.0"), ("run_id", "41"), ("repository", "other/repo")]:
            manifest.write_text(json.dumps({**original, field: value}))
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.verify()

    def test_unexpected_file_missing_package_and_symlink_are_rejected(self):
        folder = self.root / "onetagger-linux-x64"
        extra = folder / "unexpected.py"
        extra.write_text("raise RuntimeError('must never run')")
        with self.assertRaises(ValueError):
            self.verify()
        extra.unlink()
        package = folder / "OneTagger-linux-x64.tar.gz"
        data = package.read_bytes()
        package.unlink()
        with self.assertRaises(ValueError):
            self.verify()
        package.symlink_to(folder / "OneTagger-linux-x64-cli.tar.gz")
        with self.assertRaises(ValueError):
            self.verify()
        package.unlink()
        package.write_bytes(data)
        self.verify()

    def test_checksum_file_contents_must_describe_the_packages(self):
        folder = self.root / "onetagger-linux-x64"
        sums = folder / "SHA256SUMS-linux-x64.txt"
        sums.write_text("0" * 64 + "  OneTagger-linux-x64.tar.gz\n")
        manifest_path = folder / "release-manifest-linux-x64.json"
        manifest = json.loads(manifest_path.read_text())
        manifest["sha256"][sums.name] = tools.sha256(sums)
        manifest_path.write_text(json.dumps(manifest))
        with self.assertRaises(ValueError):
            self.verify()

    def test_default_manual_mode_leaves_complete_draft(self):
        self.assertIn("Prepared draft", self.publish())
        self.assertTrue(self.api.release["draft"])
        self.assertEqual(len(self.api.assets), 17)
        self.assertIn("## Downloads", self.api.release["body"])
        self.assertFalse(any(w[0] == "PATCH" for w in self.api.writes))

    def test_publish_only_after_all_assets_are_verified(self):
        self.publish(True)
        self.assertFalse(self.api.release["draft"])
        self.assertEqual(self.api.writes[-1], ("PATCH", "releases/7", {"draft": False, "make_latest": "true"}))
        self.assertEqual(len(self.api.assets), 17)

    def test_interrupted_upload_resumes_without_replacing_assets(self):
        self.api.fail_upload_after = 2
        with self.assertRaises(OSError):
            self.publish(True)
        self.assertTrue(self.api.release["draft"])
        self.api.fail_upload_after = None
        self.publish(True)
        self.assertEqual(len([w for w in self.api.writes if w[0] == "UPLOAD"]), 17)
        self.assertFalse(self.api.release["draft"])

    def test_changed_draft_asset_is_never_overwritten(self):
        self.publish()
        self.api.assets[0]["digest"] = "sha256:" + "0" * 64
        before = list(self.api.writes)
        with self.assertRaises(ValueError):
            self.publish(True)
        self.assertEqual(self.api.writes, before)
        self.assertTrue(self.api.release["draft"])

    def test_branch_moving_during_upload_prevents_publication(self):
        self.api.move_after_upload = True
        with self.assertRaises(release.StaleBuild):
            self.publish(True)
        self.assertTrue(self.api.release["draft"])
        self.assertFalse(any(w[0] == "PATCH" for w in self.api.writes))


if __name__ == "__main__":
    unittest.main()
