# OneTagger results exchange, version 1

This integration extends the existing Helper selection manifest (schema 1). It is a local, same-computer workflow: export a selection from rekordbox Migration Helper, load its JSON manifest in OneTagger, tag the selected files, then import the results JSON into that saved Helper selection.

## Report contract

Reports identify their format with `schema: "onetagger.tagging-results"` and `version: 1`. They contain a unique run ID, application version/commit, creation/completion timestamps and an optional selection identity. Selection identity preserves `id`, `snapshot`, `snapshot_digest`, `scope_key` and `mapping_revision` without reinterpretation.

Each track records its Helper track ID (when present), original local path, final path, before/after tag snapshots, final file state, read errors and provider attempts. Attempts include provider ID, provider track/release IDs when known, matching score/reason and written/failed/skipped outcome. Match scores are provider evidence, not calibrated probabilities. Configuration, credentials, tokens and provider response bodies are excluded.

The initial tag snapshot covers title, artist, album, genre and year. Snapshots describe values read from disk, not proposed provider values. Missing values are null and never imply permission to erase library values. Missing or failed read-back remains visible. Cancelled/unprocessed selections must remain in the report.

## Review boundary

The Helper accepts only the supported report version and an exact saved selection identity, membership and path mapping. It reads files through its saved selection, never through arbitrary paths from the report. Current file state and tags must agree with reported read-back before any field becomes eligible for review. Changed mappings, snapshots, files, reviews or staged metadata invalidate stale approval.

Import only prepares a review. The existing explicit field selection, edit unlock, revision checks and atomic staging remain required. Failed/skipped/unprocessed rows are evidence, not automatic edits. Personal stars, cues, beat grids, play counts and playlist membership are outside this first contract. Library export continues through the Helper's existing independently verified XML/native ZIP writers.

Move/rename operations are excluded from Helper-bound tagging runs. A new selection is required after moving files. Generic reports can record final paths but do not authorize the Helper to relocate a library.

## Validation

GitHub Actions runs synthetic, offline tests for successful and partial outcomes, fallback providers, cancellation, malformed/unsupported reports, duplicate identities, Unicode/percent filenames, changed files/mappings and preservation of unselected library fields. Browser checks cover loading a selection, obtaining a report and explicitly reviewing imported fields. No real provider credentials or personal music files are used.

Implementation and validation status are tracked in the linked pull requests. BPM/key/label editing and performance changes are follow-up work.
