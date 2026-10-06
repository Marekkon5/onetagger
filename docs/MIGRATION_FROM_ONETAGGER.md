# Migration from OneTagger

## Reused concepts

- provider registry / builder separation
- provider-specific request wrappers, rate limits and caching concepts
- matching as ranked candidates rather than one opaque answer
- human review before committing metadata
- provider-specific configuration
- artwork and external-ID enrichment

## Retired from the active product

- individual-file scanner
- AutoTagger track loop
- QuickTag
- track renamer
- Shazam-per-track flow
- Spotify AudioFeatures flow
- desktop WebView message bridge and socket assumptions
- direct media-file tag writes as the primary persistence model

History remains in Git and upstream remains the fork parent.

## Current archive workflow

The active flow is provider discovery → curator-selected import → server enrichment → field-level Review → shared detail/index pages. OneTagger's ranked-candidate and explicit-review ideas are retained, while its Rust/WebView/socket bridge and file-tag write path are not used by the shared archive.

The Vue client reads canonical records from the Node API. SQLite is the source of truth; localStorage is retained only as a migration source/cache. On curator login, a browser with a saved library can submit an idempotent migration batch. The server excludes the four shipped demo IDs, keeps valid records if other entries fail, preserves covers and valid waveform PNGs, and stores legacy-ID redirects. The browser copy is never removed by the migration request.

Each browser performs its own authorized migration, so a second browser's local records are never uploaded from someone else's session. Two browsers importing the same source converge on one canonical record because the provider identity is unique in the database, not because the browser checked for a duplicate first.

What OneTagger contributed is still visible in the workflow: ranked candidates, explicit human review, and provider-specific configuration all survived. What did not survive is the assumption that metadata lives in the file or the browser — it lives on the server, with the browser as a reader and the curator as the only writer.

## New aggregate root

`MixSet` replaces the `AudioFileInfo -> Track` workflow. It carries long duration, artist/crew/event context, multiple public sources, candidates, provenance, confidence and raw provider metadata.
