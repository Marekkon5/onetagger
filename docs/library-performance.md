# Library loading and playback

The browser application and native desktop window use the same library loader,
save protocol and Rust backend. These changes are included in OneTagger 2.2.0.

## Incremental metadata index

Quick Tag keeps a disposable SQLite index named `library-index-v1.sqlite` in
OneTagger's settings directory. Every visit still enumerates the selected files
and checks their filesystem metadata. It reuses parsed tags only when the file
identity, size, timestamps and configured tag separators match. Changed and new
files are parsed again; files no longer present stop appearing in the list.

The index contains metadata and waveform summaries. It does not replace music
files, playlists, Rekordbox databases or OneTagger settings. Deleting it while
OneTagger is closed causes the next visit to rebuild it. SQLite is bundled with
the application, so a system SQLite installation is unnecessary.

Quick Tag, Tag Editor and Manual Tag writes invalidate the affected cache entry.
Other writers are detected through filesystem metadata on the next visit. A
corrupt, locked, unwritable or newer index falls back to reading the files.
Unchanged file timestamps are not a cryptographic proof of unchanged contents;
after restoring files together with preserved filesystem metadata, remove the
index to force a complete re-read.

Completed and cancelled scans trim metadata entries to 100,000 records and
128 MiB of JSON payload, expiring entries unused for 90 days. Individual payloads
larger than 512 KiB are read without caching. SQLite journal/free pages are
additional disk space, and an active scan can temporarily exceed the trim limits.
Waveform summaries retain at most 10,000 entries. Cache contents remain local.

## Progressive Quick Tag loading

Directory traversal and tag parsing run on bounded background workers. Results
arrive in batches, with flush thresholds of 64 files, 256 KiB or 50 ms of work;
a single file with unusually large tags can exceed the byte threshold. The first
rows can be used while the scan continues. **Cancel** stops further loading and
keeps the rows already received. Leaving Quick Tag or choosing another source
closes that loading job; obsolete results cannot replace a newer selection.

The normal 500-file directory cap stops traversal after finding enough eligible files.
Explicit playlists retain their complete selection.
**Show all** removes that cap. Detailed and compact views recycle rows outside
the viewport. Filtering searches literal text, including characters such as `[`,
and coalesces incoming batches to limit repeated sorting.

The cache avoids repeated metadata parsing. It does not eliminate filesystem
enumeration, JSON transfer, frontend track models, network filesystem latency or
provider requests. Loading every file still stores the loaded track models in
the frontend; virtual scrolling bounds rendered rows rather than all library RAM.

## Save completion

Every Quick Tag save carries a request ID and file path. A successful response
updates that file's baseline; errors, disconnections and missing acknowledgements
release the pending operation while retaining unconfirmed edits. A batch permits
at most four outstanding saves. Save-and-exit and autosave navigation proceed
only after successful acknowledgements. Writes are never replayed automatically
after reconnection: an unconfirmed write may already have reached the file.

## Audio playback and seeking

With **Client side player** enabled, the browser requests original audio with
HTTP byte-range support. Supported codecs can start without a whole-file WAV
conversion. Browser codec support varies; if the browser cannot decode the
original, OneTagger prepares a 16-bit WAV preview on disk using bounded-memory
decoding. That fallback must finish conversion before playback starts.

Conversions are serialized, individual previews are limited to 1 GiB, and the
process retains at most two previews with a combined 1 GiB cache budget. Active
responses retain their temporary file until they close. Converted previews
support byte ranges too. Originals are never modified. This mode does not open
the server's audio output device.

Native playback uses streaming Symphonia decoders for MP3, FLAC, WAV, Vorbis,
AAC, ALAC and AIFF. Seeking uses decoder seek operations instead of decoding and
discarding the prefix. The UI obtains native playback position from the backend;
browser playback follows its audio element's time and ended events.

Waveforms use a constant-memory RMS calculation. Summaries are cached, delivered
in one response and cancelled when their track is replaced. Playback does not
wait for a waveform. Tagging format support and browser codec support remain
distinct from the native decoder formats listed above.

## Language loading

Startup loads the chosen language and the English fallback. Other application
catalogs and Quasar language packs load on selection and are reused thereafter.
A switch applies only after its resources arrive; an obsolete response cannot
override a newer language choice. A failed download retains the current language
and can be retried. All 14 languages remain available offline in the packaged app.

## GitHub validation and measurements

The normal GitHub matrix tests the decoder fixtures and index behavior on Linux,
Windows, Intel macOS and Apple Silicon. Browser tests exercise progressive
5,000-file loading, bounded rendered rows, literal filtering, failed-save retry,
and language chunk requests. Linux additionally exercises native audio load,
seek, zero volume and pause through the packaged Wry/WebKitGTK application.

The Linux job runs `library::tests::library_benchmark` separately and retains
`library-performance`. It compares five direct-read, cold-index and warm-index passes over
1,000 synthetic files, then checks that one changed file causes exactly one
reparse. The operating-system file cache is warm; this is metadata-index evidence,
not a claim about complete-library, provider or playback throughput. Timings are
informational; cache membership, outcomes and rendering bounds are asserted.

Codec fixtures are two-second synthetic tones, generated on GitHub by
`ci/audio_fixtures.py`. Their manifest records the encoder and file hashes. No
user music or external provider account is required for these regressions.
