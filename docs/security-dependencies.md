# Consolidated dependency maintenance

The security maintenance PR incorporates these Dependabot updates into one tested
dependency graph. Routine maintenance updates are not assertions that every package
had a disclosed vulnerability.

| Dependabot PR | Update |
| --- | --- |
| #70 | setup-python 6 → 7.1.0 in localization workflows |
| #71 | Quasar 2.35.5, Vue 3.5.46, vue-i18n 11.4.23 |
| #72 | Vite 6.4.5 → 8.3.8 |
| #73 | id3 1.17.4, regex 1.13.2 and related patch dependencies |
| #74 | Wry 0.54.2 → 0.55.1 and its dependency graph |
| #75 | clap 4.5.60 → 4.6.9 and its dependency graph |
| #76 | minify-html 0.16.4 → 0.16.5 |
| #77 | webbrowser 1.0.5 → 1.2.7 and its dependency graph |

Both lockfiles are resolved on GitHub against the current application source.
Vite uses the existing explicit browser target instead of silently raising the
native WebKit/WebView2 compatibility floor with its new default.

The Dependency security workflow compares RustSec and npm advisory reports for
the proposed graph and the PR's base commit. New advisories fail the check;
previously existing advisories remain visible in the uploaded full JSON reports
and summary. A green comparison does not assert that existing advisories are absent.
Registry or scanner failures fail the check instead of being treated as clean.

The first GitHub audit found nine advisory/package pairs in the base Rust lockfile
and eight after the Dependabot updates, with no frontend advisories. Additional
targeted updates address the available fixes:

| Dependency | Version | Advisory |
| --- | --- | --- |
| cookie | 0.18.3 | RUSTSEC-2026-0594 |
| crossbeam-epoch | 0.9.20 | RUSTSEC-2026-0204 |
| rusqlite / libsqlite3-sys | 0.41.0 / 0.39.1 (SQLite 3.54.1) | RUSTSEC-2026-0459 |
| quinn-udp | 0.5.16 | RUSTSEC-2026-0538 |
| rkyv | 0.7.46 | RUSTSEC-2026-0001 |
| url | 2.5.9 | RUSTSEC-2026-0592 |
| webpki-roots | 1.0.10 | RUSTSEC-2026-0381 |

### Remaining transitive advisory

`RUSTSEC-2026-0235` affects archive validation in rkyv 0.7, which upstream no longer
supports. The available fix is in 0.8.17+, incompatible with the 0.7 requirement of
`parcel_sourcemap`, reached through `minify-html → lightningcss → parcel_sourcemap`.
It is reported by the scanner and is not hidden by an ignore entry.

The application passes HTML to the minifier with its default CSS and JavaScript
minification disabled. It does not call rkyv archive parsing or source-map archive
deserialization. This is a limited reachability assessment, not an upstream fix or
a claim that the dependency is vulnerability-free. Replacing the HTML minifier
or updating its source-map dependency requires separate parser compatibility work;
the current provider extraction and matching behavior are preserved here.

Sources: [rkyv archive advisory](https://rustsec.org/advisories/RUSTSEC-2026-0235.html),
[minify-html 0.16.5 defaults](https://github.com/wilsonzlin/minify-html/blob/v0.16.5/minify-html/src/cfg/mod.rs),
[parcel_sourcemap dependency](https://github.com/parcel-bundler/source-map/blob/master/parcel_sourcemap/Cargo.toml).

The new configuration field changes the Rust plugin ABI. Custom provider
compatibility is incremented from 45 to 46 so old binaries are rejected before a
configuration reference crosses that boundary. Rebuild custom providers against
this source; the built-in providers are included in every application build.

Published limits and application identification:
[MusicBrainz rate limiting](https://musicbrainz.org/doc/MusicBrainz_API/Rate_Limiting),
[Discogs rate limiting](https://www.discogs.com/developers/#page:home,header:home-rate-limiting).
