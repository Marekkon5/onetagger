## Unreleased (fantuz fork)

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

