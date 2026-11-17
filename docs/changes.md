# Combined changes report

Two independent fixes, ready to merge together:

1. **Live view / tagging status** (client, Vue): the "newest first" button on the tagging status page.
2. **Duplicate GEOB / RVA2 frames** (backend, Rust): Serato data lost when saving ID3 tags.

| | |
|---|---|
| Baseline | `f7adb26` (`master` of https://github.com/rosgr100/onetagger) |
| Fix 2 commit | `d20cd1c` on `fix/preserve-duplicate-geob-rva2` (local, not pushed) |
| Fix 1 | uncommitted edits in this working tree |
| Date | 2026-11-17 |

Fix 1 touches only `client/`; fix 2 touches only `crates/onetagger-tag/`. No file overlaps, so they merge without conflicts.

---

## Part 1: Live view in the tagging status page

### Problem
The Live view button (⚡) on the status page should keep the newest tagged file at the top of the list. It did not.

### Root causes
1. **Sorting was one-way bound.** `:pagination='pagination'` ignored the table's `update:pagination` event, so clicking a column header never re-sorted.
2. **No column reflected tagging order**, so there was nothing to sort by for "newest".
3. **The first attempt at fixing this (see `auto-tagger-status.md`) sorted the wrong way.** It assumed new files are appended to `statuses`. They are not: `onetagger.ts` uses `statuses.unshift(...)`, so index 0 is the newest. With `_seq = idx` sorted descending, the **oldest** file was on top. `isCurrentRow` (`length - 1`) highlighted the oldest file too.
4. **Auto-enable on mount never fired (race).** `Autotagger.vue` navigates to the status page after 10 ms, but `startTagging` is sent after 100 ms and `lock.locked` only becomes `true` when the server replies. On mount `locked` was still `false`.
5. A file that received a second platform result stayed at its original position.

### Final changes (replace the earlier, faulty attempt)

#### `client/src/views/AutotaggerStatus.vue`
| # | Change | Effect |
|---|---|---|
| 1 | `:pagination` → `v-model:pagination` | Header-click sorting works again |
| 2 | New hidden `_seq` column (`display: none`, sortable) | Gives the table a sortable "tag order" field |
| 3 | `statuses` computed replaced by `allRows` (unfiltered, carries `_seq` and `_status`) plus `rows` (filtered by status) | Filtering no longer loses the tag order |
| 4 | **`_seq = statuses.length - 1 - idx`** (was `idx`) | Higher = newer, stable as files arrive; descending sort now puts the newest on top |
| 5 | Title row button, icon `mdi-sort-clock-descending-outline` (on) / `mdi-sort-clock-ascending-outline` (off), tooltip "Newest first (live)" (was `mdi-flash` / `mdi-flash-off`) | More representative icon |
| 6 | `toggleLiveMode()` sets `sortBy: '_seq'`, `descending: true` (on) or `filename` ascending (off) | The toggle |
| 7 | `enableLiveMode()` helper, used by both the mount check and the new watch | One place that turns live mode on |
| 8 | **`watch` on `$1t.lock.value.locked`** turns live mode on when tagging starts | Fixes the mount race; the mount check is kept for opening the page mid-tag |
| 9 | `watch` on pagination turns live mode off if the user sorts by something else | Manual sort wins |
| 10 | `isCurrentRow()` + `.current-tag-row` CSS | Highlights the newest row while tagging (now correct because of #4) |
| 11 | `onTaggingDone` turns live mode off **and** resets sort to `filename` ascending | Toggle state and table agree after finishing |
| 12 | `watch` added to the `vue` import | Needed by #8 and #9 |

#### `client/src/scripts/onetagger.ts` (`taggingProgress` handler, dedupe branch)
Before: a later result for a known file was pushed into its existing entry, which stayed in place.
After: the entry is removed, the new result is pushed, and the entry is `unshift`ed to the front, so "newest first" means **last updated**.

```ts
const entry = this.taggerStatus.value.statuses.splice(i, 1)[0];
entry.push(json.status);
this.taggerStatus.value.statuses.unshift(entry);
```
`statuses` is only used for counting, the table rows and the platform list, none of which depend on order.

#### `client/src/views/Autotagger.vue`
**No changes.** It was reviewed because it starts tagging and routes to the status page; the race in cause 4 is handled in `AutotaggerStatus.vue` instead of changing the 10 ms / 100 ms timers.

### Verification status
Checked by the author in the running app and **confirmed fixed**: the newest file stays on top during tagging, and the new icons render.

Checks to repeat on review:
- the newest file stays on top and the highlight follows it;
- clicking a column header turns live mode off, and the toggle turns it back on;
- opening the page mid-tag works;
- the audio-features route (`/audiofeatures/status`, same component) works.

Behaviour note: the lock watch re-enables live view at the start of every run, so a sort chosen on a previous run does not carry over.

### Obsolete guidance
`docs/auto-tagger-status.md` describes the first attempt (`_seq: idx`, append-order assumption). Where it differs from this report, **this report wins**.

---

## Part 2: Duplicate GEOB / empty RVA2 frames lost when saving ID3 tags

Full write-up: [geob-rva2-fix.md](geob-rva2-fix.md). Commit details: [commit-report.md](commit-report.md). PR text: [pull-request-draft.md](pull-request-draft.md). Affected files: [affected-files.csv](affected-files.csv).

### Problem
Serato (and Mixed In Key's Serato export) write several `GEOB` frames that share a description but differ in MIME type and payload (`Serato Markers_` and `Serato Markers2`, each as `application/octet-stream` and `application/json`). The `id3` crate treats GEOB frames with the same description as the same frame, so reading keeps only the last one and saving destroys the others (hotcues, loops, colours).

Measured on 7,108 MP3s: **334 files (4.7 %)** lost GEOB frames (typically 12 → 10). 0 of 761 files with an `RVA2` frame lost it; it is protected by the same mechanism as a precaution.

### Fix
New module `crates/onetagger-tag/src/id3_raw.rs`:
1. On load (MP3, AIFF `ID3 ` chunk, WAV `id3 ` chunk) the raw ID3v2.3/2.4 tag bytes are scanned frame by frame.
2. Every `GEOB` and `RVA2` frame body is kept as raw bytes, duplicates included, in file order.
3. The crate's deduplicated frames for those IDs are removed and the raw bodies are re-added as `Content::Unknown`, which is never merged and is written verbatim.
4. When saving as ID3v2.3, GEOB frames with v2.4-only encodings (UTF-8, UTF-16BE) are rewritten to UTF-16 with BOM; the payload is untouched.

Fallback to the old behaviour for: tag-level unsync, non-syncsafe v2.4 sizes, frames running past the tag, invalid frame IDs, compressed/encrypted/grouped/unsynchronised frames, and ID3v2.2. No other frame handling changes.

### Files
| File | Change |
|---|---|
| `crates/onetagger-tag/src/id3_raw.rs` | **new** (515 lines): raw extraction, restore, v2.3 GEOB conversion, tests |
| `crates/onetagger-tag/src/id3.rs` | `use crate::id3_raw`; `preserve_raw_frames` in the MP3/AIFF/WAV branches of `load_file`; `geob_for_v23` in `save_file` when `!self.id3v24` (+13 / -3) |
| `crates/onetagger-tag/src/lib.rs` | `#[cfg(feature = "tag")] mod id3_raw;` (+2) |

### Verification
- `cargo test -p onetagger-tag --lib id3_raw`: 5 passed, 1 ignored (bulk).
- Bulk over 7,108 MP3s: 0 mismatches (334 files / 752 frames previously lost, now none).
- Real file `Aarv - Everybody (Clean).mp3`: 12 GEOB + 1 RVA2 identical, also when saved as v2.3.
- Serato DJ Pro: track loads with all 6 hotcues and labels intact.

### Known limits
- WAV and AIFF are implemented but untested on real files.
- Real-file coverage is ID3v2.4 only; v2.3 is covered by synthetic and converted files.
- `RVA2` is now written after the `GEOB` frames; order within an ID is preserved, across IDs it is not significant.

---

## Merge checklist
1. Commit the client changes (`AutotaggerStatus.vue`, `onetagger.ts`) on the same branch as `d20cd1c`, or on a separate branch; there are no overlapping files.
2. Part 1 has been tested and confirmed fixed by the author.
3. Push `fix/preserve-duplicate-geob-rva2` to `rosgr100/onetagger` (not done) and open the PR using `pull-request-draft.md`; add a short note for the client changes if they ship in the same PR.

### Suggested commit message for Part 1
```
Fix Live view in tagging status: newest file on top

The status list stores files newest-first (unshift), but Live view sorted a
hidden index column descending, putting the oldest file on top. Make the
tag-order column count from oldest to newest, bind pagination two-way so
sorting works, enable Live view when tagging actually starts (the lock is set
after the page mounts), highlight the current row, and reset the sort when
tagging finishes. Files that receive a further platform result move to the
front. Replace the flash icon with a sort-by-time icon.
```
