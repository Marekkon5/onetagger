# Auto-tag provider scheduling

AutoTagger searches enabled providers in the order shown by their cards. Drag a
card to change its priority. Re-enabling a card puts it back at its displayed
position in the search order. A track that misses on one provider can immediately
try the next provider while other tracks are still being searched.

With **Multiplatform** off, the first successful tag save ends the search for
that file. A match can save a subset of the requested fields. Enable
**Multiplatform** to visit every selected provider in order, with one writer at
a time for each file.

**Skip tagged** reads the AutoTagger marker when the run first reads each file.
A marker from an earlier run makes the file ineligible. A marker written during
the current run still allows its remaining providers and album pass to run.

Global overwrite and per-tag overwrite permissions control which fields a
provider can replace. Earlier values survive when a populated field is protected.
Later providers can update populated fields when overwrite is allowed.
**Merge genres** unions the file's existing genres with the provider's genres
and needs overwrite permission for Genre to save that union to a populated field.

Green in the status view means a successful save. The exported report lists
each provider outcome and the file's before and after values.

**Manual Tag** searches the chosen providers concurrently. It merges the results
you select in their selection order. Scalar fields use the first present value,
and array fields combine their entries.

All providers share the configured thread limit. Each provider's own `max_threads`
limit also applies. Reusable provider sources are initialized only when needed.
Each provider initializes its own sources independently. The scheduler submits
one provider lookup at a time for each track. Exact duplicate
paths and canonical aliases are collapsed before scheduling. Album tagging keeps
the existing provider-wide batches because its matching relies on folder groups.

Progress counts tracks that have finished their provider sequence, so it remains
monotonic even when providers finish in a different order. Stop discards queued
work and waits for already-running provider calls to return. The existing checks
before extension/tag writing remain in place. This change cannot interrupt a
blocking HTTP request; the provider's existing timeouts still determine that wait.

## Measure provider pace

Open **Provider pace** from the speedometer beside the elapsed time. The estimate
counts completed matching attempts, including misses, over up to the last minute.
It updates as the run proceeds and appears after at least three attempts and
five seconds of observation. Deferred lookups are shown in the throttling count.
One matching attempt can make several HTTP requests, so the displayed pace
measures how many tracks the app is processing with the current settings.

Bandcamp, BPM Supreme, Deezer, Discogs, JunoDownload, MusicBrainz, and Spotify
share a cooldown across their sources when the provider signals a rate limit.
The next provider can take over while the cooldown is active. Requests resume
when the pause expires, including the delay supplied by the provider when
available. A track that exhausts its providers during a cooldown can be retried
after the pause.

Try the same track selection and tag options when comparing two provider orders.
A first provider with a high match rate receives most of the work. Put a provider
with a higher measured pace first if its results suit your collection. With
Multiplatform enabled, the run asks each provider for each eligible track, so the
slowest provider can determine overall throughput.

## GitHub validation

The scheduler tests and benchmarks run on Linux, Windows, Intel macOS and ARM
macOS. **Provider pipeline** handles Linux, Windows and macOS 15 Intel. The
existing **Build → macOS ARM64** job runs the same command on macOS 26 before
the full Rust tests and packaging, sharing one hosted ARM runner allocation.
ARM scheduler coverage therefore uses macOS 26 instead of the former standalone
macOS 15 job. This reduces the number of ARM jobs requesting a runner; free
hosted runner availability can still delay the build.

The command compiles the production scheduler directly, runs its concurrency and
cancellation tests, then compares provider-wide barriers against pipelining with
the same worker and provider limits. `ci/provider_benchmark.py` orchestrates the
Rust harness and publishes a job summary plus CSV, JSON, logs and environment
metadata as 30-day artifacts. Find `provider-benchmark-macos-26` in **Build**;
the other platform reports remain in **Provider pipeline**. Five samples per
mode alternate execution order. An ARM scheduler or benchmark assertion failure
fails the native Build job and blocks **Required checks** and release packages.

Scenarios cover first-provider hits, a slow tail, a concurrency-limited provider,
all misses, provider failures, Multiplatform, and cancellation. The harness asserts
identical ordered provider attempts and outcomes, no speculative requests, and
the global/provider limits. It reports total time, throughput, time to first
completed track, p95 completion time, peak active calls and Stop-to-drain latency.
Wall-clock speedup is informational, not a flaky pass/fail threshold.

These are synthetic provider waits, not measurements against live services.
Attempt counts do not measure HTTP request counts, payload bytes, connection reuse,
or provider rate-limit responses. Pipelining can reduce idle worker time and tail
latency; it does not promise fewer bytes or a speedup when the first provider
already saturates all workers. The provider tests cover shared cooldowns and
following the provider's delay. The frontend tests cover the pace estimate
and distinguish deferred attempts from completed lookups.

The normal **Build** workflow also runs the full Rust regression suite, including
real audio-tag write/read-back, report parity and skip-marker tests on all
supported desktop targets. The browser smoke suite verifies the visible
provider order against the tagging request after toggling a provider. It also
verifies that a deferred lookup appears in the Provider pace panel.
