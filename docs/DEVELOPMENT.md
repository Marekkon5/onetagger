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

## Frontend development

Start the backend, then run `pnpm --dir client dev`. Vite proxies `/session`, `/ws`, `/audio`, `/thumb`, and `/spotify` to the loopback backend. Open the printed session link with its port changed to 8080, preserving the fragment. The frontend authenticates before mounting. Production and desktop builds use the same session flow.

## Artifacts and releases

Every successful PR build uploads native application/CLI packages and per-platform SHA-256 checksum files. Frontend assets are built once and reused by all native jobs. No release is published for a PR or ordinary branch push.

Pushing an intentional `v*` tag runs all checks, verifies that the tag agrees with the application, CLI and shared crate versions, then publishes packages with generated GitHub release notes. Release publication alone receives `contents: write`; build jobs have read-only repository permissions. macOS packages are separate Intel/ARM64 unsigned archives. CI verifies that native binaries do not link unbundled libraries outside the macOS system directories. `.cargo/config.toml` builds liblzma statically to avoid a dependency on Homebrew on the user’s machine.

Before tagging, update the three package versions and the changelog in a reviewed PR. The macOS bundle and Windows installer read their release version from the application manifest. The macOS packaging helper uses Python’s standard library and packages the validated executable without rebuilding it.

Until the first tagged fork release, the README links to workflow artifacts instead of displaying an unavailable release badge. When publishing that release, replace the artifact-download badge with the release/download badges and update the installation instructions and supported-version policy in `SECURITY.md`. Artifact downloads require a GitHub login and expire after 14 days; published release assets are the intended permanent download location.

## Repository references

Downloads, build badges, update checks, and support links point to `fantuz/onetagger`. The original project website is labeled as upstream documentation. Original author credits and donation destinations remain intact. SongRec dependencies and the custom-platform catalog still point to their upstream repositories intentionally.
