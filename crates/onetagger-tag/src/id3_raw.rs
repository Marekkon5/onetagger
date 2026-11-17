//! Lossless handling of `GEOB` and `RVA2` frames.
//!
//! The `id3` crate treats frames with the same content descriptor as the same frame, so when
//! reading a tag only the last of several `GEOB` frames with an identical description survives.
//! Serato (and Mixed In Key) write exactly that: `Serato Markers_` and `Serato Markers2` once as
//! `application/octet-stream` and once as `application/json`, with different payloads.
//!
//! To keep them, the raw tag bytes are scanned *before* the `id3` crate's view of the tag is used.
//! Every `GEOB` / `RVA2` frame body is taken as bytes and stored in the tag as an opaque
//! `Content::Unknown` frame. Unknown frames are never compared for equality (so they are never
//! merged) and are written back verbatim by the encoder, in MP3, AIFF and WAV alike.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use id3::{Tag, Frame, Content, TagLike};
use id3::frame::Unknown;

/// Frame IDs handled losslessly
const RAW_IDS: [&str; 2] = ["GEOB", "RVA2"];

/// Container the ID3 tag lives in
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Container {
    /// ID3v2 at the start of the file
    Mp3,
    /// `ID3 ` chunk in a FORM/AIFF file (big endian)
    Aiff,
    /// `id3 ` chunk in a RIFF/WAVE file (little endian)
    Wav,
}

/// Decode a syncsafe integer, None if any byte has the high bit set
fn syncsafe(b: &[u8]) -> Option<usize> {
    let mut n = 0usize;
    for x in b {
        if x & 0x80 != 0 {
            return None;
        }
        n = (n << 7) | *x as usize;
    }
    Some(n)
}

/// Raw frames (id, body) found in an ID3v2.3/2.4 tag, only for IDs where *every* frame
/// could be extracted safely. `None` if the tag can't be parsed with confidence, in which
/// case the normal `id3` behaviour is kept.
pub(crate) fn parse_raw_frames(data: &[u8]) -> Option<Vec<(String, Vec<u8>)>> {
    if data.len() < 10 || &data[..3] != b"ID3" {
        return None;
    }
    let version = data[3];
    if version != 3 && version != 4 {
        return None;
    }
    let flags = data[5];
    // Tag level unsynchronisation, leave to the library
    if flags & 0x80 != 0 {
        return None;
    }
    let size = syncsafe(&data[6..10])?;
    if 10 + size > data.len() {
        return None;
    }
    let body = &data[10..10 + size];

    // Skip extended header
    let mut pos = 0usize;
    if flags & 0x40 != 0 {
        if body.len() < 4 {
            return None;
        }
        pos = if version == 4 {
            syncsafe(&body[..4])?
        } else {
            4 + u32::from_be_bytes([body[0], body[1], body[2], body[3]]) as usize
        };
    }

    let mut found: Vec<(String, Vec<u8>)> = vec![];
    let mut unsafe_ids: Vec<String> = vec![];
    while pos + 10 <= body.len() && body[pos] != 0 {
        let id = &body[pos..pos + 4];
        if !id.iter().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
            return None;
        }
        let id = String::from_utf8_lossy(id).to_string();
        let n = if version == 4 {
            syncsafe(&body[pos + 4..pos + 8])?
        } else {
            u32::from_be_bytes([body[pos + 4], body[pos + 5], body[pos + 6], body[pos + 7]]) as usize
        };
        let format_flags = body[pos + 9];
        if pos + 10 + n > body.len() {
            return None;
        }
        if RAW_IDS.contains(&id.as_str()) {
            // Compression, encryption, grouping, per-frame unsync, data length indicator
            let plain = if version == 4 { format_flags & 0x4F == 0 } else { format_flags & 0xE0 == 0 };
            if plain {
                found.push((id, body[pos + 10..pos + 10 + n].to_vec()));
            } else if !unsafe_ids.contains(&id) {
                unsafe_ids.push(id);
            }
        }
        pos += 10 + n;
    }
    found.retain(|(id, _)| !unsafe_ids.contains(id));
    Some(found)
}

/// Read the raw bytes of the ID3 tag from a file
pub(crate) fn read_tag_bytes(path: &Path, container: Container) -> Option<Vec<u8>> {
    let mut file = File::open(path).ok()?;
    let start = match container {
        Container::Mp3 => 0u64,
        Container::Aiff | Container::Wav => {
            let mut head = [0u8; 12];
            file.read_exact(&mut head).ok()?;
            let big = container == Container::Aiff;
            if (big && &head[..4] != b"FORM") || (!big && &head[..4] != b"RIFF") {
                return None;
            }
            let len = file.metadata().ok()?.len();
            let mut offset = 12u64;
            loop {
                if offset + 8 > len {
                    return None;
                }
                file.seek(SeekFrom::Start(offset)).ok()?;
                let mut ch = [0u8; 8];
                file.read_exact(&mut ch).ok()?;
                let size = if big {
                    u32::from_be_bytes([ch[4], ch[5], ch[6], ch[7]])
                } else {
                    u32::from_le_bytes([ch[4], ch[5], ch[6], ch[7]])
                } as u64;
                if ch[..4].eq_ignore_ascii_case(b"ID3 ") {
                    break offset + 8;
                }
                offset += 8 + size + (size & 1);
            }
        }
    };
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut header = [0u8; 10];
    file.read_exact(&mut header).ok()?;
    if &header[..3] != b"ID3" {
        return None;
    }
    let size = syncsafe(&header[6..10])?;
    let mut data = header.to_vec();
    data.resize(10 + size, 0);
    file.read_exact(&mut data[10..]).ok()?;
    Some(data)
}

/// Replace `GEOB` / `RVA2` frames the `id3` crate parsed (and deduplicated) with the raw ones from the file
pub(crate) fn preserve_raw_frames(tag: &mut Tag, path: &Path, container: Container) {
    let Some(bytes) = read_tag_bytes(path, container) else { return };
    let Some(frames) = parse_raw_frames(&bytes) else { return };
    let version = tag.version();
    for id in RAW_IDS {
        if !frames.iter().any(|(i, _)| i == id) {
            continue;
        }
        tag.remove(id);
        for (_, data) in frames.iter().filter(|(i, _)| i == id) {
            tag.add_frame(Frame::with_content(id, Content::Unknown(Unknown { data: data.clone(), version })));
        }
    }
}

/// Before writing ID3v2.3: UTF-8 (3) and UTF-16BE (2) text encodings only exist in v2.4,
/// so rewrite those GEOB frames to UTF-16 with BOM (1). Everything else is untouched.
pub(crate) fn geob_for_v23(tag: &mut Tag) {
    let converted: Vec<Vec<u8>> = tag.frames()
        .filter(|f| f.id() == "GEOB")
        .filter_map(|f| match f.content() {
            Content::Unknown(u) => geob_to_utf16(&u.data),
            _ => None
        })
        .collect();
    if converted.is_empty() {
        return;
    }
    // Rebuild in original order
    let originals: Vec<Frame> = tag.frames().filter(|f| f.id() == "GEOB").cloned().collect();
    tag.remove("GEOB");
    let mut converted = converted.into_iter();
    for frame in originals {
        let data = match frame.content() {
            Content::Unknown(u) => match geob_to_utf16(&u.data) {
                Some(_) => converted.next().unwrap(),
                None => u.data.clone()
            },
            _ => {
                tag.add_frame(frame);
                continue;
            }
        };
        tag.add_frame(Frame::with_content("GEOB", Content::Unknown(Unknown { data, version: id3::Version::Id3v23 })));
    }
}

/// None if the frame doesn't need converting (or is malformed)
fn geob_to_utf16(data: &[u8]) -> Option<Vec<u8>> {
    let enc = *data.first()?;
    if enc != 2 && enc != 3 {
        return None;
    }
    let rest = &data[1..];
    // MIME is always ISO-8859-1
    let mime_end = rest.iter().position(|b| *b == 0)?;
    let (mime, mut rest) = (&rest[..mime_end], &rest[mime_end + 1..]);
    let mut strings = vec![];
    for _ in 0..2 {
        let (text, remaining) = if enc == 3 {
            let end = rest.iter().position(|b| *b == 0)?;
            (String::from_utf8_lossy(&rest[..end]).to_string(), &rest[end + 1..])
        } else {
            let mut end = None;
            for i in (0..rest.len().saturating_sub(1)).step_by(2) {
                if rest[i] == 0 && rest[i + 1] == 0 {
                    end = Some(i);
                    break;
                }
            }
            let end = end?;
            let units = rest[..end].chunks(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect::<Vec<_>>();
            (String::from_utf16_lossy(&units), &rest[end + 2..])
        };
        strings.push(text);
        rest = remaining;
    }
    let mut out = vec![1u8];
    out.extend_from_slice(mime);
    out.push(0);
    for s in strings {
        out.extend_from_slice(&[0xFF, 0xFE]);
        for u in s.encode_utf16() {
            out.extend_from_slice(&u.to_le_bytes());
        }
        out.extend_from_slice(&[0, 0]);
    }
    out.extend_from_slice(rest);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id3::ID3Tag;
    use crate::TagImpl;

    fn syncsafe_bytes(n: usize) -> [u8; 4] {
        [((n >> 21) & 0x7F) as u8, ((n >> 14) & 0x7F) as u8, ((n >> 7) & 0x7F) as u8, (n & 0x7F) as u8]
    }

    fn frame_v24(id: &str, body: &[u8]) -> Vec<u8> {
        let mut f = id.as_bytes().to_vec();
        f.extend_from_slice(&syncsafe_bytes(body.len()));
        f.extend_from_slice(&[0, 0]);
        f.extend_from_slice(body);
        f
    }

    fn geob(enc: u8, mime: &str, desc: &str, payload: &[u8]) -> Vec<u8> {
        let mut b = vec![enc];
        b.extend_from_slice(mime.as_bytes());
        b.push(0);
        match enc {
            3 | 0 => { b.push(0); b.extend_from_slice(desc.as_bytes()); b.push(0); }
            _ => { b.extend_from_slice(&[0xFF, 0xFE, 0, 0, 0xFF, 0xFE]); for u in desc.encode_utf16() { b.extend_from_slice(&u.to_le_bytes()); } b.extend_from_slice(&[0, 0]); }
        }
        b.extend_from_slice(payload);
        b
    }

    const AUDIO: &[u8] = &[0xFF, 0xFB, 0x90, 0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

    /// ID3v2.4 mp3 with two GEOBs sharing a description, and an empty RVA2
    fn mock_frames() -> Vec<(&'static str, Vec<u8>)> {
        vec![
            ("TIT2", [&[3u8][..], b"Title"].concat()),
            ("GEOB", geob(0, "application/octet-stream", "Serato Markers_", &[1, 2, 3, 0, 255, 254])),
            ("GEOB", geob(0, "application/json", "Serato Markers_", &[9, 8, 7, 6])),
            ("RVA2", b"SeratoGain\0".to_vec()),
            ("TBPM", [&[3u8][..], b"128"].concat()),
        ]
    }

    fn mock_mp3(path: &Path, frames: &[(&str, Vec<u8>)]) {
        let body: Vec<u8> = frames.iter().flat_map(|(id, b)| frame_v24(id, b)).collect();
        let mut data = b"ID3\x04\x00\x00".to_vec();
        data.extend_from_slice(&syncsafe_bytes(body.len() + 100));
        data.extend_from_slice(&body);
        data.extend_from_slice(&[0u8; 100]);
        data.extend_from_slice(AUDIO);
        std::fs::write(path, data).unwrap();
    }

    /// Independent raw frame scanner (does not use the id3 crate)
    fn scan(path: &Path) -> Vec<(String, Vec<u8>)> {
        let bytes = std::fs::read(path).unwrap();
        let v = bytes[3];
        let size = syncsafe(&bytes[6..10]).unwrap();
        let t = &bytes[10..10 + size];
        let (mut i, mut out) = (0, vec![]);
        while i + 10 <= t.len() && t[i] != 0 {
            let n = if v == 4 { syncsafe(&t[i + 4..i + 8]).unwrap() } else { u32::from_be_bytes([t[i + 4], t[i + 5], t[i + 6], t[i + 7]]) as usize };
            out.push((String::from_utf8_lossy(&t[i..i + 4]).to_string(), t[i + 10..i + 10 + n].to_vec()));
            i += 10 + n;
        }
        out
    }

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("onetagger_raw_{}_{}", std::process::id(), name))
    }

    #[test]
    fn duplicate_geob_and_empty_rva2_survive_save() {
        let path = tmp("dup.mp3");
        let frames = mock_frames();
        mock_mp3(&path, &frames);

        // Control: the id3 crate alone loses one of the two GEOBs
        let plain = Tag::read_from_path(&path).unwrap();
        assert_eq!(plain.encapsulated_objects().count(), 1);

        let mut tag = ID3Tag::load_file(&path).unwrap();
        tag.save_file(&path).unwrap();

        let out = scan(&path);
        for id in ["GEOB", "RVA2"] {
            let expected: Vec<_> = frames.iter().filter(|(i, _)| *i == id).map(|(_, b)| b.clone()).collect();
            let got: Vec<_> = out.iter().filter(|(i, _)| i == id).map(|(_, b)| b.clone()).collect();
            assert_eq!(got, expected, "{id} frames must be byte-identical and in order");
        }
        assert_eq!(out.iter().filter(|(i, _)| i == "GEOB").count(), 2);
        assert_eq!(out.iter().filter(|(i, _)| i == "RVA2").count(), 1);
        // Other frames still there and audio untouched
        assert!(out.iter().any(|(i, _)| i == "TIT2") && out.iter().any(|(i, _)| i == "TBPM"));
        assert!(std::fs::read(&path).unwrap().ends_with(AUDIO));

        // Second round trip is stable
        let mut tag = ID3Tag::load_file(&path).unwrap();
        tag.save_file(&path).unwrap();
        assert_eq!(scan(&path).iter().filter(|(i, _)| i == "GEOB").count(), 2);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn utf8_geob_converted_for_id3v23() {
        let path = tmp("v23.mp3");
        let mut frames = mock_frames();
        frames[1].1 = geob(3, "application/octet-stream", "Serato Markers_", &[1, 2, 3]);
        frames[2].1 = geob(3, "application/json", "Serato Markers_", &[4, 5, 6]);
        mock_mp3(&path, &frames);
        let mut tag = ID3Tag::load_file(&path).unwrap();
        tag.set_id3v24(false);
        tag.save_file(&path).unwrap();
        let out = scan(&path);
        let geobs: Vec<_> = out.iter().filter(|(i, _)| i == "GEOB").collect();
        assert_eq!(geobs.len(), 2);
        assert!(geobs.iter().all(|(_, b)| b[0] == 1), "converted to UTF-16 with BOM");
        assert!(geobs[0].1.ends_with(&[1, 2, 3]) && geobs[1].1.ends_with(&[4, 5, 6]));
        assert_eq!(Tag::read_from_path(&path).unwrap().version(), id3::Version::Id3v23);
        std::fs::remove_file(&path).ok();
    }

    /// Real Serato / Mixed In Key tagged file, skipped if the sample isn't present
    #[test]
    fn real_serato_file() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../TEST FILES/8th Wonder 1311/Aarv - Everybody (Clean).mp3");
        if !src.exists() {
            return;
        }
        let path = tmp("real.mp3");
        std::fs::copy(&src, &path).unwrap();
        let before = scan(&path);
        assert_eq!(before.iter().filter(|(i, _)| i == "GEOB").count(), 12);

        let mut tag = ID3Tag::load_file(&path).unwrap();
        tag.save_file(&path).unwrap();
        let after = scan(&path);

        // all GEOB / RVA2 frames identical, same order
        // (order across different frame IDs is not significant, order within an ID is kept)
        let pick = |v: &Vec<(String, Vec<u8>)>, id: &str| v.iter().filter(|(i, _)| i == id).cloned().collect::<Vec<_>>();
        assert_eq!(pick(&before, "GEOB"), pick(&after, "GEOB"));
        assert_eq!(pick(&before, "RVA2"), pick(&after, "RVA2"));
        assert_eq!(pick(&after, "GEOB").len(), 12);
        assert_eq!(pick(&after, "RVA2").len(), 1);

        // Compare every other frame to a plain id3 round trip: the fix must not alter anything else
        let base = tmp("base.mp3");
        std::fs::copy(&src, &base).unwrap();
        let t = Tag::read_from_path(&base).unwrap();
        id3::Encoder::new().version(id3::Version::Id3v24).padding(2048).write_to_path(&t, &base).unwrap();
        let others = |v: Vec<(String, Vec<u8>)>| v.into_iter().filter(|(i, _)| i != "GEOB" && i != "RVA2").collect::<Vec<_>>();
        assert_eq!(others(scan(&path)), others(scan(&base)));
        std::fs::remove_file(&path).ok();
        std::fs::remove_file(&base).ok();
    }

    /// Every mp3 in TEST FILES: `cargo test --release -- --ignored all_test_files --nocapture`
    #[test]
    #[ignore]
    fn all_test_files() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../TEST FILES");
        let (out_p, base_p) = (tmp("bulk.mp3"), tmp("bulk_base.mp3"));
        let mut stack = vec![root];
        let (mut n, mut had_dup, mut bad, mut skipped) = (0, 0, 0, 0);
        while let Some(p) = stack.pop() {
            if p.is_dir() {
                stack.extend(std::fs::read_dir(&p).unwrap().map(|e| e.unwrap().path()));
                continue;
            }
            if p.extension().map(|e| e != "mp3").unwrap_or(true) {
                continue;
            }
            let Ok(plain) = Tag::read_from_path(&p) else { skipped += 1; continue };
            n += 1;
            std::fs::copy(&p, &out_p).unwrap();
            std::fs::copy(&p, &base_p).unwrap();
            let before = scan(&p);
            let before_v = std::fs::read(&p).map(|b| b[3]).unwrap_or(0);
            let version = plain.version();
            let mut tag = ID3Tag::load_file(&p).unwrap();
            tag.save_file(&out_p).unwrap();
            id3::Encoder::new().version(version).padding(2048).write_to_path(&plain, &base_p).unwrap();
            let after = scan(&out_p);
            let pick = |v: &Vec<(String, Vec<u8>)>, id: &str| v.iter().filter(|(i, _)| i == id).cloned().collect::<Vec<_>>();
            // Compared as sorted sets: OneTagger's existing TRCK/TPOS rewrite reorders those frames
            let others = |v: Vec<(String, Vec<u8>)>| { let mut v = v.into_iter().filter(|(i, _)| i != "GEOB" && i != "RVA2").collect::<Vec<_>>(); v.sort(); v };
            if pick(&before, "GEOB").len() > plain.encapsulated_objects().count() {
                had_dup += 1;
            }
            if pick(&before, "GEOB") != pick(&after, "GEOB") || pick(&before, "RVA2") != pick(&after, "RVA2") || others(after) != others(scan(&base_p)) {
                bad += 1;
                println!("MISMATCH v2.{} {:?}", before_v, p);
            }
        }
        std::fs::remove_file(&out_p).ok();
        std::fs::remove_file(&base_p).ok();
        println!("files={n} unreadable={skipped} files_with_collapsed_geob={had_dup} mismatches={bad}");
        assert_eq!(bad, 0);
    }

    /// Native ID3v2.3 file (big endian, non syncsafe frame sizes) with duplicate GEOBs and an empty RVA2
    #[test]
    fn native_id3v23_roundtrip() {
        let path = tmp("native23.mp3");
        let mut frames = mock_frames();
        frames[0].1 = [&[0u8][..], b"Title"].concat();
        frames[4].1 = [&[0u8][..], b"128"].concat();
        // enc 1 (UTF-16) GEOBs are what v2.3 writers produce
        frames[1].1 = geob(1, "application/octet-stream", "Serato Markers2", &[1, 2, 3]);
        frames[2].1 = geob(1, "application/json", "Serato Markers2", &[4, 5, 6]);
        let body: Vec<u8> = frames.iter().flat_map(|(id, b)| {
            let mut f = id.as_bytes().to_vec();
            f.extend_from_slice(&(b.len() as u32).to_be_bytes());
            f.extend_from_slice(&[0, 0]);
            f.extend_from_slice(b);
            f
        }).collect();
        let mut data = b"ID3  ".to_vec();
        data.extend_from_slice(&syncsafe_bytes(body.len() + 100));
        data.extend_from_slice(&body);
        data.extend_from_slice(&[0u8; 100]);
        data.extend_from_slice(AUDIO);
        std::fs::write(&path, data).unwrap();

        let mut tag = ID3Tag::load_file(&path).unwrap();
        assert!(!tag.id3v24);
        tag.save_file(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes[3], 3, "stays ID3v2.3");
        let out = scan(&path);
        for id in ["GEOB", "RVA2"] {
            let expected: Vec<_> = frames.iter().filter(|(i, _)| *i == id).map(|(_, b)| b.clone()).collect();
            let got: Vec<_> = out.iter().filter(|(i, _)| i == id).map(|(_, b)| b.clone()).collect();
            assert_eq!(got, expected, "{id}");
        }
        assert!(bytes.ends_with(AUDIO));
        std::fs::remove_file(&path).ok();
    }

    /// Real v2.4 Serato file converted to v2.3 on save
    #[test]
    fn real_serato_file_saved_as_id3v23() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../TEST FILES/8th Wonder 1311/Aarv - Everybody (Clean).mp3");
        if !src.exists() {
            return;
        }
        let path = tmp("real23.mp3");
        std::fs::copy(&src, &path).unwrap();
        let mut tag = ID3Tag::load_file(&path).unwrap();
        tag.set_id3v24(false);
        tag.save_file(&path).unwrap();
        let out = scan(&path);
        assert_eq!(std::fs::read(&path).unwrap()[3], 3);
        assert_eq!(out.iter().filter(|(i, _)| i == "GEOB").count(), 12);
        assert_eq!(out.iter().filter(|(i, _)| i == "RVA2").count(), 1);
        // payloads (after the text header) all still present
        let before = scan(&src);
        let tail = |b: &Vec<u8>| b[b.len().min(20)..].to_vec();
        for (_, b) in before.iter().filter(|(i, _)| i == "GEOB") {
            assert!(out.iter().any(|(i, o)| i == "GEOB" && o.ends_with(&tail(b)[tail(b).len().saturating_sub(10)..])), "payload missing");
        }
        std::fs::remove_file(&path).ok();
    }
}
