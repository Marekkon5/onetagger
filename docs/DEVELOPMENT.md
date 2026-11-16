# Development and GitHub validation

The application remains a Rust workspace with a Vue/Quasar frontend. Python is used for small portable CI utilities; the tagging engine is not being rewritten.

## Reproducible dependencies

- Rust is pinned by `rust-toolchain.toml` (1.100.0).
- The frontend uses Node.js 24 and pnpm 10.11.0 (`client/package.json`).
- Commit `Cargo.lock` and `client/pnpm-lock.yaml` whenever dependency declarations change.
- Normal builds use `cargo ... --locked` and `pnpm install --frozen-lockfile`. CI never runs `cargo update` or an unlocked frontend installation as part of validation.

Resolve intentional dependency changes separately, review the resulting lockfile diff, then run the complete matrix. GitHub Actions are pinned to reviewed commit SHAs; Dependabot tracks future action and package updates.

Dependabot groups routine minor/patch version updates for Actions and the frontend, and patch updates for Cargo. Cargo minor updates remain separate because a `0.x` minor change can break Rust APIs; major upgrades also remain separate for compatibility review. Security-update PRs may arrive separately. Grouping reduces duplicate builds but does not establish compatibility: review every manifest and lockfile diff, including changes beyond the dependency named in a PR title.

## Build and checks

Build the frontend first because the Rust UI crate embeds `client/dist`:

```sh
pnpm --dir client install --frozen-lockfile
pnpm --dir client run build
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked
cargo build --workspace --release --locked
```

The Build workflow first runs `cargo metadata --locked --format-version 1` in the frontend job to catch inconsistent Rust lockfiles before starting native builds. It then runs the checks above on Ubuntu 24.04 x64, Windows Server 2022 x64, macOS 15 Intel, and macOS 26 ARM64. `Required checks` fails unless the frontend and every platform pass. Configure that check as required in the repository ruleset before merging.

Existing tests that call live Beatport, Beatsource, Bandcamp, and MusicBrainz services are explicitly ignored in normal regression runs. Run them deliberately with `cargo test -p onetagger-platforms --locked -- --ignored --test-threads=1`; external outages and rate limits must not determine ordinary PR validation. New safety and matching tests are deterministic and run without provider accounts.

Clippy runs across the workspace. Existing style warnings are visible but are not promoted wholesale to errors; compilation and Clippy errors still fail the job.

### Compilation caches and timing

Native jobs use a commit-pinned `Swatinem/rust-cache` action to retain compiled dependencies and Cargo downloads. Before saving, it removes workspace build outputs, incremental artifacts, unpacked registry sources and unused dependency artifacts. It does not cache installed Cargo binaries. Application packages and timing reports are uploaded before this post-job cleanup.

Cache keys separate runner images/architectures, the virtual workspace manifest, toolchain and Cargo configuration. The action additionally hashes compiler versions, Cargo/Rust environment settings, member manifests and dependency versions. There is no commit-SHA suffix, so source-only changes can reuse an exact cache without uploading another copy. Dependency changes can fall back to a compatible prior cache; Cargo still checks fingerprints and every build remains locked. The new cache namespace starts cold and does not restore the previous full-target archives.

Successful native jobs save caches on both PRs and `master`. A PR can restore a cache from its base branch, but its own cache is scoped to that PR's merge ref: it cannot seed `master` or another PR. After merging this workflow, successful `master` native jobs populate the shared baseline. Existing caches expire under GitHub's cache eviction policy; this workflow does not delete them or enlarge the repository's cache allowance.

Only native CI jobs set `CARGO_PROFILE_DEV_DEBUG` and `CARGO_PROFILE_TEST_DEBUG` to `line-tables-only`, retaining filename/line information for backtraces without full variable/type debug data. They also disable incremental compilation because incremental files are not retained by the dependency cache. Debug assertions and overflow checks remain enabled in test builds; local development defaults and optimized release profiles are unchanged. Tests and Clippy still run on all four platforms.

Compare cold and warm runs using **Restore Rust dependency cache**, its post-job cleanup/save log (including compressed archive size), and the native job's step durations. The job summary identifies exact hits; a false result can also mean a partial restore, so check the restore log. Test and release compilation HTML reports are available in `cargo-timings-<platform>` artifacts for seven days. They are diagnostic artifacts, not release packages. Runner queue time is separate from compilation time and is not improved by caching. Validate warm-cache gains on GitHub before quoting a speedup.

## Frontend development

Start the backend, then run `pnpm --dir client dev`. Vite proxies `/session`, `/ws`, `/audio`, `/thumb`, and `/spotify` to the loopback backend. Open the printed session link with its port changed to 8080, preserving the fragment. The frontend authenticates before mounting. Production and desktop builds use the same session flow.

## Artifacts and releases

Every successful PR or `master` build uploads application/CLI packages, per-platform SHA-256 checksum files and release manifests. The manifests record `VERSION`, the source commit, repository, build run and every package digest. The **Validate release assets** job downloads all four native artifacts and checks their contents and provenance before **Required checks** can pass. Frontend assets are built once and reused by all native jobs.

### Automatic release after a version PR

1. Add reviewed release notes under `## Unreleased (fantuz fork)` in `CHANGELOG.md`.
2. In a release PR, run `python3 ci/release_tools.py bump 1.7.1` (replace the example with the intended higher stable version). This synchronizes `VERSION`, the application/CLI/shared manifests, macOS bundle metadata and their existing Cargo lockfile entries. It moves the Unreleased notes into a dated release section. No dependency resolution or build runs locally; GitHub performs validation and packaging.
3. Review and merge that version PR. After **Build** passes on the resulting `master` commit, **Release validated version** creates `vX.Y.Z`, attaches the already tested packages and publishes the release as latest. Release notes come from the dated changelog section and include download/checksum links and the source/build references. Packages are reused from that exact Build run without another native compilation.

The initial `VERSION` baseline, ordinary commits without a version change, PR builds and manually dispatched Build runs do not automatically publish anything. Adding this automation does not publish the inherited `1.7.0`. Stable numeric `X.Y.Z` versions are currently supported; prerelease suffixes are rejected because Windows and macOS packaging need a separate preview/file-version policy.

`VERSION` must increase relative to the merge commit's first parent. The successful build must still be the latest Build of the current default-branch commit. If another merge supersedes the version bump while it builds, automatic publication is skipped. Use the manual recovery path after reviewing the newer source and its release notes; no unvalidated or stale packages are substituted.

### Manual draft, publication and recovery

In **Actions → Release validated version → Run workflow**, select `master` and enter the numeric ID from the latest successful **Build** URL (`.../actions/runs/ID`). Leave **publish** unchecked to prepare a draft; check it to publish the verified release. The source must have a valid dated changelog section and synchronized versions, and all artifacts must still be available. A manually dispatched Build on `master` can supply fresh artifacts when the previous ones have expired. Starting the release workflow on another branch or supplying a PR run is rejected.

The workflow creates a draft first and verifies every uploaded asset before publishing. Rerunning with the same source and artifacts resumes missing uploads without replacing files. Existing published releases are left untouched, and a tag pointing elsewhere is rejected. A new release must be newer than the latest published version, including in manual mode. A draft with unexpected or changed files requires manual inspection and cleanup before retrying; the automation will not delete it or overwrite its assets. A draft-only run reserves its version tag, which the automation will not move, so reserve that version for the reviewed source.

No personal access token or additional secret is required: jobs use the repository's `GITHUB_TOKEN`. Only the publishing job receives `contents: write`, after the read-only source checks. Repository/organization policy must allow that permission and creation of `v*` tags; no ruleset bypass is configured. PR/build jobs remain read-only, and downloaded artifact contents are never executed. The old tag-push publisher is replaced by this single release workflow; pushing a tag alone no longer publishes a release.

macOS packages are separate unsigned Intel/ARM64 archives. CI verifies that native binaries do not link unbundled libraries outside the macOS system directories. `.cargo/config.toml` builds liblzma statically. The macOS packager and Windows installer obtain versions from the application manifest; the installer rejects a missing version argument. The Python packaging/release helpers use the standard library, and GitHub runs their regression tests before the native matrix.

Artifact downloads require a GitHub login and expire after 14 days. Published release assets provide permanent downloads. Once the first fork release is published, the README's artifact badge can be replaced with release/download badges.

## Repository references

Downloads, build badges, update checks, and support links point to `fantuz/onetagger`. The original project website is labeled as upstream documentation. Original author credits and donation destinations remain intact. SongRec dependencies and the custom-platform catalog still point to their upstream repositories intentionally.
