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

Published limits and application identification:
[MusicBrainz rate limiting](https://musicbrainz.org/doc/MusicBrainz_API/Rate_Limiting),
[Discogs rate limiting](https://www.discogs.com/developers/#page:home,header:home-rate-limiting).
