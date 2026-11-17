# Duplicate GEOB / empty RVA2 frames lost when saving ID3 tags

Related report: https://github.com/quodlibet/mutagen/issues/754 (same underlying problem in Mutagen)

## Summary

OneTagger silently dropped Serato cue/marker data when saving MP3, AIFF and WAV files.
Serato, and Mixed In Key's Serato export, write several `GEOB` frames that **share one description but differ in MIME type and payload**:

| Description | MIME | Content |
|---|---|---|
| `Serato Markers_` | `application/octet-stream` | cue positions, loop slot colours, track colour |
| `Serato Markers_` | `application/json` | same structure, **different** positions and colours |
| `Serato Markers2` | `application/octet-stream` | Base64 markers (names like `Energy 6`, `COLOR`, `BPMLOCK`) |
| `Serato Markers2` | `application/json` | Base64 markers (names like `Cue 1`, no `COLOR`/`BPMLOCK`) |

The `id3` crate considers GEOB frames with the same description to be the same frame (`Content::unique()` for
`EncapsulatedObject` compares only the description), so while *reading* a tag the later frame replaces the earlier one.
By the time OneTagger saved the file, one frame of each pair was gone.

## Findings

Measured on `TEST FILES` (7,108 MP3s), reading and writing each file with `id3` 1.16.4 using OneTagger's own write settings
(`Encoder::new().padding(2048)`):

* **334 files (4.7 %)** lost GEOB frames, typically 12 → 10 (`Serato Markers_` and `Serato Markers2`, the octet-stream copy lost).
  The frames are on disk in the input but never reach the writer.
* **0 of 761** files with an `RVA2` frame lost it. The `id3` crate keeps the empty `SeratoGain\0` RVA2 as an opaque
  `Unknown` frame. Nothing in OneTagger strips it. It is protected by the same mechanism as a precaution.
* `Serato Overview`, `Serato Analysis`, `Serato Autotags`, `Serato BeatGrid`, `CuePoints`, `Key`, `Energy`, `BeatGrid`
  have unique descriptions and were never affected.
* Reference file: `8th Wonder 1311/Aarv - Everybody (Clean).mp3`: 12 GEOB + 1 RVA2 in, 10 GEOB + 1 RVA2 out before the fix.

A misleading early result worth knowing about: re-reading a tag written **without padding** through the `id3` crate also
hides frames. That is a reader quirk, the bytes are on disk. OneTagger writes with padding, so it is not affected,
but it means verification must scan raw frames instead of trusting a re-parse.

## Fix

New module `crates/onetagger-tag/src/id3_raw.rs`.

1. **On load** (`ID3Tag::load_file`, MP3 / AIFF / WAV) the raw ID3v2 tag bytes are located
   (start of file for MP3, the `ID3 ` / `id3 ` chunk for AIFF / WAV) and scanned frame by frame.
2. Every `GEOB` and `RVA2` frame body is kept as raw bytes, duplicates included, in file order.
3. The frames the `id3` crate parsed for those IDs are removed and the raw bodies are re-added as `Content::Unknown`.
   `Unknown` is `Incomparable` in the crate, so it is never merged with another frame and the encoder writes the bytes verbatim.
4. **On save** nothing special is needed for ID3v2.4. For **ID3v2.3**, GEOB frames using the v2.4-only text encodings
   (UTF-8 = 3, UTF-16BE = 2) are rewritten to UTF-16 with BOM (1); the payload is untouched.

Because the raw frames travel inside the normal `Tag`, the existing writers (`Encoder` for MP3, `write_to_path` for AIFF,
`wav::write_wav`) need no changes, and padding, version handling and audio data are unchanged.

### Safety fallbacks

The raw path is skipped, and the old `id3` behaviour is used, when the tag cannot be parsed with confidence:

* tag-level unsynchronisation, non-syncsafe v2.4 frame sizes, a frame running past the tag, invalid frame IDs
* a `GEOB` / `RVA2` frame with compression, encryption, grouping, per-frame unsynchronisation or a data length indicator
  (that frame ID stays on the old path entirely)
* ID3v2.2 tags

### Scope

Only `GEOB` and `RVA2` handling changed. All other frames (BPM, key, title, ...) go through exactly the same code as before.

## Files changed

| File | Change |
|---|---|
| `crates/onetagger-tag/src/id3_raw.rs` | **new**: raw frame extraction, restore, v2.3 GEOB conversion, tests |
| `crates/onetagger-tag/src/id3.rs` | `use crate::id3_raw`; call `preserve_raw_frames` in the MP3 / AIFF / WAV branches of `load_file`; call `geob_for_v23` in `save_file` when saving ID3v2.3 |
| `crates/onetagger-tag/src/lib.rs` | `mod id3_raw;` |

## Tests

`cargo test -p onetagger-tag --lib id3_raw` (5 tests):

* `duplicate_geob_and_empty_rva2_survive_save`: synthetic ID3v2.4 file with two GEOBs sharing a description and an empty RVA2,
  saved through `ID3Tag::save_file`; frames compared byte for byte, in order, with an independent raw scanner.
  Also asserts the plain `id3` crate loses one (control), the audio bytes are untouched, and a second round trip is stable.
* `native_id3v23_roundtrip`: native ID3v2.3 file (big-endian frame sizes), same assertions, stays v2.3.
* `utf8_geob_converted_for_id3v23`: v2.4 UTF-8 GEOBs saved as v2.3 become UTF-16, payloads intact.
* `real_serato_file`: the Aarv file: 12 GEOB + 1 RVA2 identical after save, and every other frame identical to a plain `id3` write.
* `real_serato_file_saved_as_id3v23`: same real file converted to v2.3, all 12 GEOB + RVA2 present.

Bulk test (slow, ignored by default): `cargo test --release -p onetagger-tag --lib -- --ignored all_test_files`
runs every MP3 in `TEST FILES`. Result: **7,108 files, 334 previously affected, 0 mismatches**; GEOB / RVA2 byte-identical,
all other frames identical to a plain `id3` write (compared as sorted sets because OneTagger's existing `TRCK` / `TPOS`
rewrite moves those frames to the end).

## Known limits

* WAV and AIFF paths are implemented but untested on real files (the sample set contains only MP3).
* Frames order across different IDs is not preserved (RVA2 now follows the GEOBs); order within an ID is.
* Not verified inside Serato itself.
