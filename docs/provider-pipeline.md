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
There are no speculative parallel lookups for the same track, added retries, or
additional provider attempts compared with the batch scheduler. Exact duplicate
paths and canonical aliases are collapsed before scheduling. Album tagging keeps
the existing provider-wide batches because its matching relies on folder groups.

Progress counts tracks that have finished their provider sequence, so it remains
monotonic even when providers finish in a different order. Stop discards queued
work and waits for already-running provider calls to return. The existing checks
before extension/tag writing remain in place. This change cannot interrupt a
blocking HTTP request; the provider's existing timeouts still determine that wait.

## GitHub validation

The **Provider pipeline** workflow runs on Linux, Windows, Intel macOS and ARM
macOS. It compiles the production scheduler directly, runs its concurrency and
cancellation tests, then compares provider-wide barriers against pipelining with
the same worker and provider limits. `ci/provider_benchmark.py` orchestrates the
Rust harness and publishes a job summary plus CSV, JSON, logs and environment
metadata as artifacts. Five samples per mode alternate execution order.

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
already saturates all workers. No additional adaptive throttling or retry policy
is introduced. Existing provider rate limits remain in force.

The normal **Build** workflow also runs the full Rust regression suite, including
real audio-tag write/read-back, report parity and skip-marker tests on all
supported desktop targets. The browser smoke suite verifies the visible
provider order against the tagging request after toggling a provider. No live-provider credentials are used by these
tests, and PR artifacts are never published as releases.
