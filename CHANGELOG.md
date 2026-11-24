## Unreleased (fantuz fork)

## 2.2.1 — 2026-11-24

- Fix Windows tag saves while an audio decoder has the original file open, including tracks selected before playback starts. Close the staging writer and use the pinned Rust toolchain's Windows rename fallback for atomic replacement; retain the original on genuine access/sharing failures and retry short sharing locks within a bounded interval.
- Show and log the complete Quick Tag save error, including the Windows error and file path. Keep failed edits pending and confirm success only after the saved tags are available.
- Share successful release-detail requests across Beatport, MusicBrainz, Deezer and Traxsource workers, and reuse Spotify album/artist details within the authenticated source. Concurrent requests for the same data share one fetch; different releases can progress independently. Each response cache is bounded to 256 entries with a ten-minute lifetime. Failed or incorrectly identified responses are not reused.
- Keep Discogs master IDs separate from release IDs and bound its existing release cache. Pace Discogs and iTunes from request starts, allowing network time to count toward the interval. Share MusicBrainz's one-request-per-second budget across workers and sessions, including retry cooldowns.
- Preserve provider order, candidate selection, matching thresholds and per-file write ordering. Document all twelve built-in providers' bulk-lookup support and limitations, including Spotify app eligibility and the difference between multiple results and multiple independent requests.
- Validate request coalescing, unchanged metadata and actual HTTP request counts with a loopback fixture on GitHub. Benchmark figures describe synthetic release enrichment and do not predict live whole-library speed or change match accuracy.

[Full changes since v2.2.0](https://github.com/fantuz/onetagger/compare/v2.2.0...v2.2.1)

## 2.2.0 — 2026-11-24

- Cache Quick Tag metadata in an incremental, local SQLite index. Re-read changed files, retain existing separator behavior, and fall back to direct reads if the disposable cache is unavailable.
- Show Quick Tag results progressively with cancellable loading and recycled rows in detailed and compact views. Stop traversal at the normal 500-file cap. Treat search text literally and avoid repeated sorting for each fast batch.
- Acknowledge Quick Tag writes by request and file. Retain unconfirmed edits after errors or disconnections, and wait for confirmed saves before autosave navigation or exit.
- Stream original audio to the browser with HTTP byte-range support, using server-issued IDs for validated tracks. Use a bounded disk WAV preview when its codec is unsupported, and avoid opening the server's audio device for browser playback.
- Use streaming, seekable native decoders, including AIFF and ALAC. Cache RMS waveform summaries, cancel obsolete waveform work, follow actual playback position and accept zero volume.
- Load the selected language and English fallback at startup; fetch other catalogs and Quasar language packs on selection. Keep all 14 interface languages in packaged builds.
- Stop repeated singleton access from creating extra player instances and timers. Move filesystem, decoder, renaming and manual-result work off network workers with bounded admission.
- Add synthetic codec, index, save-failure, large-library and language-loading regressions plus GitHub metadata-cache measurements. See [library loading and playback](https://github.com/fantuz/onetagger/blob/master/docs/library-performance.md) for behavior and limits.

[Full changes since v2.1.5](https://github.com/fantuz/onetagger/compare/v2.1.5...v2.2.0)

## 2.1.5 — 2026-11-20

- On Linux, OneTagger waits for its local listener before opening the desktop window. Launch with `./onetagger --software-rendering` to use software rendering from the first frame. If WebKit's page process exits, it reopens the session with software rendering. A second failure within a minute presents native controls to reopen the view or continue in a browser.
- The interface reconnects after a dropped WebSocket and can recover the current run summary. Settings, tagging and stop requests can run without opening an audio device.
- In Auto Tag results, **Retry unfinished lookups** reruns only partial and deferred files. The queue retains outstanding fields and Library Workbench track IDs. A countdown shows when a rate-limited provider can be tried again.
- In Auto Tag advanced settings, **Rotate first provider between tracks** lets different tracks start at different providers. Each track then follows the configured order around the list. Ordered matching remains the default. iTunes now honors HTTP 429 retry delays.

[Full changes since v2.1.4](https://github.com/fantuz/onetagger/compare/v2.1.4...v2.1.5)

## 2.1.4 — 2026-11-20

### Library integrity

- Album tagging checks each selected file against the release using recording identity or an unambiguous title match, available artist evidence, and the configured duration and confidence limits. Track and album passes count each file once in progress.
- Quick Tag preserves other players’ ID3 ratings and play counters. Clearing stars also clears the Windows Media Player compatibility rating.
- M3U imports preserve literal filenames and decode local file URIs once. Large Quick Tag playlist exports append paths in one pass. The CLI accepts a single audio file as well as a playlist or directory.

### Providers and status

- AutoTagger distinguishes partial writes and deferred lookups. Ordered fallbacks can supply outstanding fields, and a rate limit alone no longer sends a file to the failed-files folder. Workbench reports retain their compatible status fields and add retry evidence.
- MusicBrainz enriches the selected release edition. Manual Tag groups duplicate results while retaining distinct editions. Oldest and Newest prioritize dated releases.
- The live status screen counts each track incrementally, displays the newest results without shifting the entire list, and offers a filter for partial or deferred tracks.

[Full changes since v2.1.3](https://github.com/fantuz/onetagger/compare/v2.1.3...v2.1.4)

## 2.1.3 — 2026-11-19

Maintenance release updating the application, CLI, Windows installer and macOS bundle versions. This repackages [v2.1.2](https://github.com/fantuz/onetagger/releases/tag/v2.1.2) without application or third-party dependency changes.

[Full changes since v2.1.2](https://github.com/fantuz/onetagger/compare/v2.1.2...v2.1.3)

## 2.1.2 — 2026-11-19

### AutoTagger

- Continue searching with the next enabled provider when a provider is rate limited.
- Initialize providers independently so a slow startup does not delay other providers.
- Show each provider’s observed lookup pace and count of deferred lookups. The pace includes completed attempts that found no match.

### Build validation

- Share one Apple Silicon runner between scheduler tests, benchmarks and native packaging, retaining all native platforms and benchmark reports. Allow 60 minutes for the combined job, with the scheduler step bounded to 10 minutes.

[Full changes since v2.1.1](https://github.com/fantuz/onetagger/compare/v2.1.1...v2.1.2)

## 2.1.1 — 2026-11-19

### AutoTagger

- Use the displayed order of enabled provider cards for each track's search priority. Drag cards to change priority. Re-enabling a provider uses its current position.
- Apply **Skip tagged** using the marker first read for each file. Multiplatform and album tagging can continue after the current run writes its own marker. Files with an existing AutoTagger marker are skipped before forced Shazam recognition.

### Windows

- Reduce the installer download by about 64% in GitHub builds by removing the bundled Visual C++ Redistributable. Reuse an installed WebView2 Runtime and run the bootstrapper when needed.
- Verify executable versions and exercise silent installation and uninstallation in GitHub CI.

### Dependencies

- Update the locked Tokio version to 1.54.1.

[Full changes since v2.1.0](https://github.com/fantuz/onetagger/compare/v2.1.0...v2.1.1)

## 2.1.0 — 2026-11-19

OneTagger now offers 14 interface languages in the desktop app and browser mode.

### Interface languages

- Add Portuguese for Portugal and Brazil, Czech, Polish, Japanese, and Arabic. The Portuguese options use region-specific wording and plural rules.
- Revise Greek navigation, tagging messages, file operation prompts, and in-app help.
- Mirror the interface in Arabic. File paths, renamer templates, and the playback waveform retain their left-to-right order.
- Bundle the catalogs and Quasar control translations for offline use. Language choice is saved per client.

### Dependencies

- Update Tokio to the 1.51.6 LTS patch release. Terminal styling remains available on Windows with Crossterm's event support disabled.
- Update the Vue Router, Quasar extras, ID3, Clap, Muda, and convert_case dependencies included since v2.0.0.

[Full changes since v2.0.0](https://github.com/fantuz/onetagger/compare/v2.0.0...v2.1.0)

## 2.0.0 — 2026-11-19

OneTagger 2.0.0 brings seven new interface languages to the desktop app and browser mode, along with a faster way to handle provider fallbacks in AutoTagger.

### Interface languages

- Use Italian, French, Dutch, German, Spanish, Greek, or Russian alongside English. OneTagger remembers your language choice. The translations cover navigation, settings, in-app help, and common tagging messages, and are bundled for offline use.
- Long translated navigation labels remain reachable with scroll controls. English is available as the fallback for unsupported locales and missing messages.

### AutoTagger

- When a provider returns no match, a track can move to the next provider immediately while other tracks are still being processed. Each track keeps the configured provider order and first-match behavior, and requests respect the provider and global concurrency limits.
- Album tagging retains its grouped processing. Tagging results, report formats, and the reviewed rekordbox Migration Helper import workflow remain compatible.

### Release and validation

- GitHub validates the application and CLI packages for Windows x64, Linux x64, Intel macOS, and Apple Silicon. CI covers the localized workflows in Chromium, Firefox, and WebKit, the Linux native WebView, and provider scheduling tests and benchmarks on all four build platforms.
- A successful build of `master` can now publish a pending version even if a later commit introduced a CI fix. The publisher verifies artifacts and their checksums before publishing.

**Full changelog** [v1.7.4…v2.0.0](https://github.com/fantuz/onetagger/compare/v1.7.4...v2.0.0)

## 1.7.4 — 2026-11-18

- Let Autotagger steps fit their content instead of reserving a fixed viewport-sized panel, so short settings pages and hidden options release unused space.
- Reset scrolling when switching Autotagger steps or presentation modes, and prevent outgoing and incoming pages from stacking during navigation.
- Keep the step navigation bar fixed while preserving access to Start and CLI controls.
- Remove redundant per-update scroll-style timers and animated step-height transitions.
- Give advanced toggles accessible names and align the native minimum window width with the frontend's 1200-pixel requirement.
- Expand layout regression coverage for scrolling, conditional fields, route navigation and window resizing across Chromium, Firefox and WebKit on Linux, plus Chromium on Windows.
- Extend the real Linux executable's UI checks with resized navigation, layout assertions and system/WebKit diagnostics.

The separate Linux native-window freeze investigation is deferred; this release does not claim to resolve it. Browser mode (`onetagger --server --browser`) remains available.

## 1.7.3 — 2026-11-17

- Add opt-in tagging-results JSON in AutoTagger and the CLI, recording actual before/after tags, provider attempts and file identity for reviewed imports into rekordbox Migration Helper.
- Accept Helper selection manifests to keep tagging within the selected files and preserve the selection identity in the report.
- Extend negotiated version-2 reports with stored label and composer metadata. Older selection manifests retain the original five-field version-1 report; this does not add automatic composer lookup by providers.
- Acknowledge Stop requests, show stopping/stopped states, preserve partial progress and ignore updates after completion. Keep blocking progress reception off the async runtime and prevent overlapping browser runs from resetting cancellation.
- Recheck cancellation after provider lookups and before tag writes, and skip queued album work after Stop. Already-running requests and writes may need to finish.
- Filter successful paths once per provider batch, reducing repeated scans while preserving provider fallback and multiplatform behavior.
- Use stable provider IDs for draggable cards and keep checkbox clicks out of drag gestures.
- Verify the real producer-to-importer round trip on GitHub, including MP3/M4A, duplicate selected IDs, provider fallback, legacy reports, cancellation and stale-file rejection. Keep the Helper importer revision pinned.
- Expand browser regression coverage to 60 test executions across Chromium, Firefox and WebKit, including mouse/keyboard controls and cancellation. Exercise the actual Linux executable through WebKitGTK with three navigation cycles and 48 checkbox toggles; stabilize the native test's scrolling around page transitions and fixed navigation.
- Add the README credit: Build pipelines and security audit by Massimiliano Fantuzzi.

Native Linux CI uses a virtual display and does not establish a fix for every Wayland/GPU-specific freeze. Results reports support reviewed Helper imports; they do not directly update a live rekordbox database.

## 1.7.2 — 2026-11-17

- Keep album tagging within the selected playlist/file set, including when neighbouring tracks share the same folder.
- Keep final success/failure results disjoint across provider fallbacks, preventing successful tracks from entering failed-file moves or playlists.
- Preserve existing MP4/M4A artwork when a replacement download is unavailable or invalid; validate downloaded images before replacing covers.
- Retain editor changes until the matching save acknowledgement, preserve failed edits for retry, and wait for successful autosave before switching tracks or pages.
- Allow first-time ID3 saves in the tag editor while continuing to reject corrupt tags and I/O errors.
- Add backend file-integrity regressions and 45 browser test executions across Chromium, Firefox and WebKit, covering session handling, editing, failed/delayed saves, retries, disconnects and navigation.
- Refresh locked Rust/frontend dependencies, including Wry, scraper, Quasar and vue-tsc; adopt TypeScript 6 with bundler module resolution and compatible editor payload types.
- Reuse pnpm and Python package caches and automatically remove Rust caches belonging to closed pull requests.
- Refresh README badges and platform download links, document the rekordbox Migration Helper handoff, and update build/security audit credits.

## 1.7.1 — 2026-11-16

First published release of the maintained fantuz fork, based on upstream OneTagger 1.7.0. Original authorship and the GPL-3.0 license are preserved.

### Tagging and file handling

- Fix Beatport token refresh deadlocking after expiry; concurrent workers share one refresh and failed refreshes remain retryable.
- Enforce duration filtering in fuzzy matching as well as exact matching.
- Stage tag updates beside the original and publish only after successful writes and synchronization.
- Preserve malformed-tag and I/O errors instead of replacing unreadable ID3 metadata with an empty tag.
- Preflight renamer collisions, preserve source paths on failed moves, and show per-file outcomes in the UI/CLI.
- Preserve unrelated files in the Windows installation folder during installation and removal.

### Credentials and browser sessions

- Remove provider authentication payloads from debug logs, redact credential-bearing debug output, and strip Musixmatch token URLs from request errors. Logs from older builds are not scrubbed.
- Authenticate browser, WebSocket and media access with per-process sessions and same-origin checks; redact command payloads from error logs.
- Escape renamer template previews and display release-note text without interpreting HTML.

### Builds and downloads

- Publish desktop and CLI packages for Linux x64, Windows x64, macOS Intel and macOS Apple Silicon, plus a Windows installer, platform checksums and source manifests.
- Automate releases after validated version-bump PRs. Publication verifies artifacts from the exact successful master build and supports resuming an incomplete draft.
- Synchronize VERSION, application/CLI versions, the macOS bundle and Cargo lockfile; derive the Windows installer version from the application.
- Refresh Quasar, its Vite plugin, Vue Router and the Rust time dependency with reviewed lockfiles. Pin Rust and both dependency lockfiles; run tests, Clippy, frontend typechecks and native release builds on GitHub.
- Validate release tooling and package manifests in CI, and group routine Dependabot updates by ecosystem.
- Update download badges and installation links, document release recovery and security reporting, and retain upstream attribution and provider-repository links.

### Usage notes

- Keep library backups and try a small selection before bulk editing. See the documented file-operation guarantees and limitations.
- A valid application session retains filesystem access with the server account's permissions; this release does not add a per-user or library-folder sandbox.
- Online provider availability is not covered by offline CI. The inherited Spotify Audio Features page remains unavailable; Spotify metadata tagging is a separate feature.

# 1.7.0
**(03.08.2023)**

### Auto Tag:
- Updated Beatport to support the new site
- Improved match rates
- Selectable overwrite
- Internal refactor to how tags work
- Explicit tag
- BPMSupreme > Latino

### Quick Tag:
- **Multiple files mode**
- Thin view mode
- External player support

### Renamer:
- Fixed input updating
- BPM in MP4

### Other:
- Various bug fixes and improvements in all sections


# 1.6.0
**(14.04.2023)**

### OneTagger:
- Support for WAV (ID3) and OGG files

### Auto Tag:
- **Android version**
- Added Bandcamp
- Added Musixmatch
- Added Deezer
- Lyrics support
- Logging in custom platforms
- Stop button
- Better platform info (supported tags, whether auth is required)
- Profiles (multiple configurations)

### Other
- Many bug fixes in all sections
- Dropped dependency on libsndfile, so compilling 1T should be less painful and more portable


# 1.5.1
**(31.10.2022)**

### OneTagger:
- Older MacOS warning and server version restart

### Auto Tag:
- HOTFIX: Make sure the Move files feature is more safe


# 1.5.0
**(14.10.2022)**

### OneTagger:
- **Rewrite entire frontend to Vue3 + Typescript**

### Auto Tag:
- Only write year
- Track/disc number/total tags
- BPMSupreme, iTunes improvements
- Tag each track using multiple platforms
- Merging styles and genres fixes
- Regex title cleanup
- Move failed/successful files

### Audio Features:
- Spofiy rate limits
- Bug fixes

### Quick Tag:
- List of failed files with reasons


# 1.4.0
**(20.04.2022)**

### OneTagger:
- Added Auto Rename tab (rename your files by to your tags)
- CLI version

### Auto Tag:
- Internal platform system redesign
- Custom platforms support (https://github.com/Marekkon5/onetagger-platform-template/)
- Update Spotify implementation
- Improved match rates, less skipped files
- Various minor platform fixes
- Added reason of failure into status list

### Quick Tag:
- Redesign
- Sorting
- Multiple genres
- File browser
- Subgenres


### Other:
- Windows install to registry
- New logging system
- Split to bunch of different crates
- Minor improvements and bug fixes


# 1.3.0:
**(18.11.2021)**

### Auto Tag:
 - Match by exact ID for Discogs, Beatport
 - Filename template fixes
 - Duration tag
 - `VINYLTRACK` Tag for Discogs
 - Discogs now faster for smaller batches
 - Album artist tag
 - iTunes, Musicbrainz, Beatsource and Spotify support
 - Beatport subgenres, more tags
 - Meta tag
 - Remixer tag
 - Track number tag
 - ISRC tag
 - Shazam to find songs without tag and filename parsing
 - Filter in status page

### Audio Features:
 - Added popularity tag
 - Renamed danceability value to `dance-high, dance-med, dance-low`

### Quick Tag:
 - Internal rewrite, cleaner code, more stable
 - Search and filter

### Tag Editor:
 - CTRL + S
 - Filtering
 - Refactored some code

### Other:
 - General UI improvements
 - Windows: Replace CEF with webview2 - smaller install sizes, more portable.
 - `--expose` command line option to make the server listen on 0.0.0.0
 - Updated dependencies
 - Bug fixes
 - .mp4 Extension support


# 1.2.1:
**(03.07.2021)**

### Auto Tagger
- Fixed bug in 1.2.0 causing Beatport and Traxsource having low match rate

### Other
- Added more info to logs for debugging
- Fixed path pickers not opening with bad path


# 1.2.0
**(02.07.2021)**

### Shared:
- Added M3U playlist support with drag and drop

### Auto Tag:
- Added catalog number, track ID, release ID, version, URL tags
- Added duration matching (WARNING: strict, should be used only in specific situations)
- Tag files without metadata (using filename with custom templates)
- Single page design changes
- Improved matching rates, bug fixes

### Quick Tag:
- Option to load recursively

### Tag Editor:
- Minor design changes

### Other
- Benchmark mode (for testing / debugging purproses, can be ran with `--benchmark` command line argument)
- If you specify path as command line argument, it will be automatically prefilled.



# 1.1.0
**(31.05.2021)**

### Shared:
- Added MP4/M4A support

### Auto Tag:
- Redesign
- Better status page
- Single page setup (enable in settings)
- Camelot key notation
- Juno Download is single thread now
- Bug fixes and improvements related to matching

### Audio Features:
- Fix searching in some edge cases
- Cache Spotify token

### Quick Tag:
- Player UI improvements
- Autosave, Autoplay (can be enabled in settings)
- Bug fixes

## **WARNING: Due to many breaking changes, SETTINGS WILL BE RESET TO DEFAULT. This will hopefully not happen in future again. Sorry for the inconvenience.**



# 1.0.0
**(13.05.2021)**

First public release

