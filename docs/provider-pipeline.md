# Auto-tag provider scheduling

For track tagging, providers are tried in the order selected in Auto-tag. A track
that misses on one provider can immediately try the next provider while other
tracks are still being searched. Provider order is never randomized. A successful
track stops searching, unless **Multiplatform** is enabled; in that case the same
track still visits providers in order, with only one writer at a time.

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
real audio-tag write/read-back and report parity tests for both scheduling modes,
on all supported desktop targets. No live-provider credentials are used by these
tests, and PR artifacts are never published as releases.
