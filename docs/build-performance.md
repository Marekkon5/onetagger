# Build reuse and checkpoints

The Build workflow produces one frontend artifact. Native packaging, browser
regressions and the reusable localization workflow all consume those same bytes
from that workflow run. Catalog contracts, locale negotiation and message
compilation run before the frontend artifact is uploaded. Localized browser jobs
remain parallel with native builds and ordinary browser tests; their results are
included in the Required checks gate and therefore in release eligibility.

This removes one complete frontend installation/build/upload per commit compared
with the independent Localization workflow. It does not remove language coverage.
The native compilation path remains the main area to measure for wall-clock gains.

| Stage | Reused work | Validation boundary |
| --- | --- | --- |
| Frontend | One artifact for all consumers in the run | Frozen lockfile, typecheck, catalog compilation and tests |
| Rust dependencies | Compiled dependencies and registry data, including after failed tests or packaging | OS/image, toolchain, manifests and relevant environment settings |
| Native workspace | Fresh application compilation in each source run | Workspace crates excluded from the dependency cache |
| Packaging | The GUI/CLI binaries already built in the native job | Platform smoke checks, package metadata and checksums |
| Publishing | Validated native artifacts from the successful master Build | Exact source commit, version, workflow identity and artifact integrity |

Retaining the dependency cache on failure avoids losing completed dependency
compilation when an unrelated regression test or packaging upload fails. It does
not turn a failed run into a successful checkpoint for release publication.
Cancelled jobs may still end before cache saving finishes. GitHub cache scope
rules also mean PR caches are not a general shared cache for later master runs.

The publisher already downloads the validated Build artifacts; it does not compile
the application again. Promoting PR binaries across a squash merge would require
separate provenance and embedded-revision work. Full workspace caching also needs
careful tracking of build-time Git revision and generated frontend inputs. The
current changes keep those application outputs fresh while reusing dependencies.

Use uploaded Cargo timing reports and GitHub step durations to compare cold and
warm runs for each platform. Report frontend work removed separately from measured
end-to-end speed; runner queues and native toolchain cache misses can dominate.
