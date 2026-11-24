# Auto-tag provider scheduling

For the complete API inventory and the distinction between bulk IDs, release
tracklists and ordinary searches, see [provider request capabilities](provider-api-capabilities.md).

AutoTagger searches enabled providers in the order shown by their cards. Drag a
card to change its priority. Re-enabling a card puts it back at its displayed
position in the search order. A track that misses on one provider can immediately
try the next provider while other tracks are still being searched.

With **Multiplatform** off, a successful provider lookup and tag save normally ends the search for that file. A provider may supply a subset of the requested fields. When its detail request fails, the available fields can still be saved. The next providers fill the outstanding fields in order, using the original overwrite settings for those fields. A search interrupted by a rate limit also continues to the next provider. Files awaiting a retry stay in place when **Move failed files** is enabled.

Enable **Multiplatform** to visit every selected provider in order, with one writer at a time for each file. Each provider uses the configured overwrite permissions.

**Skip tagged** reads the AutoTagger marker when the run first reads each file.
A marker from an earlier run makes the file ineligible. A marker written during
the current run still allows its remaining providers and album pass to run.

Global overwrite and per-tag overwrite permissions control which fields a
provider can replace. Earlier values survive when a populated field is protected.
Later providers can update populated fields when overwrite is allowed.
**Merge genres** unions the file's existing genres with the provider's genres
and needs overwrite permission for Genre to save that union to a populated field.

The **Needs another lookup** filter groups partial saves and deferred lookups. A file leaves the filter when a later provider completes it. Each provider's status tooltip explains its warning. Reports record each attempt, its retry delay when supplied, and the file's before and after values. For Workbench v1/v2 compatibility, a partial save has status `written` and outcome `partial`. A deferred attempt has status `skipped` and outcome `deferred`.

**Manual Tag** searches the chosen providers concurrently. It merges the results
you select in their selection order. Separate editions remain distinct when their recording IDs coincide. Scalar fields use the first present value,
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

Release enrichment reuses validated successful detail responses for Beatport,
MusicBrainz, Deezer, Traxsource and Spotify; Discogs retains its existing release
reuse with separate master/release keys. Each cache holds at most 256 responses
for ten minutes. Beatport, MusicBrainz, Deezer and Traxsource builders share their
caches across sources. Spotify caches belong to its authenticated source and its
clones; Deezer's key includes the content language. No tokens are written to a
response cache. Errors, rate limits and responses identifying another release
are not cached. Search requests and their candidate ordering remain unchanged.

Discogs and iTunes count their proactive interval from the start of the preceding
request, allowing the response time to count toward that interval. MusicBrainz
shares one request per second across all its workers. These budgets also cover
manual lookups. A cache hit consumes no network request. Provider cooldowns still
take precedence after a quota response.

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

The same GitHub command also compiles the production response cache and request
pacer directly and checks concurrent duplicate fetches, independent release
progress, expiry, bounded storage, failed-fetch recovery and shared admission.
Its separate request-reuse benchmark makes real HTTP requests to a loopback
fixture: 1,000 synthetic songs, 16 workers, five alternating samples per mode and
a 2 ms fixture response delay. Each cached result must equal its uncached
counterpart, including the selected release identity. The summary records detail
HTTP requests and timings for unique releases, ten songs per release and one
shared release. It does not call live services or measure search accuracy.

The normal **Build** workflow also runs the full Rust regression suite, including
real audio-tag write/read-back, report parity and skip-marker tests on all
supported desktop targets. The browser smoke suite verifies the visible
provider order against the tagging request after toggling a provider. It also
verifies that a deferred lookup appears in the Provider pace panel.
