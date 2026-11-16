use anyhow::Error;
use std::io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};
use std::fs::File;
use std::collections::HashMap;
use std::path::Path;
use id3::{Tag, TagLike, Frame, Version};
use once_cell::sync::Lazy;
use riff::{Chunk, ChunkId, LIST_ID, RIFF_ID, ChunkContents};

/// RIFF `id3 ` chunk
static ID3_ID_1: ChunkId = ChunkId { value: [0x69, 0x64, 0x33, 0x20] };
static ID3_ID_2: ChunkId = ChunkId { value: [0x49, 0x44, 0x33, 0x20] };
/// RIFF WAVE chunk
static WAVE_ID:  ChunkId = ChunkId { value: [0x57, 0x41, 0x56, 0x45] };
static INFO_ID:  ChunkId = ChunkId { value: [0x49, 0x4E, 0x46, 0x4F] };

/// ID3 Frame to RIFF ChunkID
static ID3_RIFF: Lazy<HashMap<&'static str, ChunkId>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("TIT2", ChunkId::new("INAM").unwrap());
    m.insert("TALB", ChunkId::new("IPRD").unwrap());
    m.insert("TPE1", ChunkId::new("IART").unwrap());
    m.insert("COMM", ChunkId::new("ICMT").unwrap());
    m.insert("TCON", ChunkId::new("IGNR").unwrap());
    m.insert("TSRC", ChunkId::new("ISRC").unwrap());
    m
});

/// Write wav to path
/// Will copy ID3 meta into RIFF INFO chunk
pub(crate) fn write_wav(path: impl AsRef<Path>, tag: Tag, version: Version) -> Result<(), Error> {
    let mut file = BufReader::new(File::open(&path)?);
    let file_len = validate_header(&mut file)?;
    let mut offset = 0;
    // Read all the chunks
    let mut riff_chunks = vec![];
    let mut list_chunks = vec![];
    loop {
        let chunk = match Chunk::read(&mut file, offset) {
            Ok(chunk) => chunk,
            Err(_) => break
        };
        // Search for parent chunks
        if chunk.id() != LIST_ID && chunk.id() != RIFF_ID {
            offset += 4;
            continue;
        }
        if chunk.len() < 4 || chunk.offset() + 8 + u64::from(chunk.len()) > file_len {
            return Err(anyhow!("Invalid or truncated WAV container chunk"));
        }
        // Save chunks
        for child in chunk.iter(&mut file) {
            let child = child?;
            if chunk.id() == LIST_ID {
                list_chunks.push(child);
            } else {
                riff_chunks.push(child);
            }
        }
        
        offset += 8 + u64::from(chunk.len()) + u64::from(chunk.len() % 2);
    }


    // Get all the RIFF chunks
    let mut resolved_riff_chunks = vec![];
    for chunk in riff_chunks {
        // Resolve LIST chunks
        if chunk.id() == LIST_ID {
            for child in chunk.iter(&mut file) {
                let child = child?;
                if child.id() != RIFF_ID && child.id() != LIST_ID {
                    list_chunks.push(child);
                }
            }
            continue;
        }
        // Resolve nested (because of SOME apps)
        if chunk.id() == RIFF_ID {
            for child in chunk.iter(&mut file) {
                let child = child?;
                if child.id() != RIFF_ID && child.id() != LIST_ID {
                    resolved_riff_chunks.push(child);
                }
            }
            continue;
        }
        resolved_riff_chunks.push(chunk);
    }
    let riff_chunks = resolved_riff_chunks;

    // Generate the RIFF chunk
    let mut riff_data = vec![];
    for chunk in riff_chunks {
        // skip old ID3 chunk
        if chunk.id() == ID3_ID_1 || chunk.id() == ID3_ID_2 {
            continue;
        }
        // Passthru
        let data = ChunkContents::Data(chunk.id(), chunk.read_contents(&mut file)?);
        riff_data.push(data);
    }
    // Add ID3 chunk
    let mut out = vec![];
    tag.write_to(Cursor::new(&mut out), version)?;
    riff_data.push(ChunkContents::Data(ID3_ID_1.clone(), out));
    let riff_chunk = ChunkContents::Children(RIFF_ID.clone(), WAVE_ID.clone(), riff_data);

    // Generate LIST chunk
    let mut list_data = vec![];
    for frame in tag.frames() {
        if let Some(chunk_id) = ID3_RIFF.get(frame.id()) {
            if let Some(text) = frame.content().text() {
                let data = ChunkContents::Data(chunk_id.clone(), format!("{text}").as_bytes().to_owned());
                list_data.push(data);
            }
        }
    }
    
    // Add original LIST chunks
    for chunk in list_chunks {
        if list_data.iter().any(|c| matches!(c, ChunkContents::Data(id, _) if id == &chunk.id())) {
            continue;
        }
        let data = ChunkContents::Data(chunk.id(), chunk.read_contents(&mut file)?);
        list_data.push(data);
    }
    let list_chunk = ChunkContents::Children(LIST_ID.clone(), INFO_ID.clone(), list_data);

    // The caller writes a private staged copy. Close the reader before truncation (Windows).
    drop(file);
    let mut file = BufWriter::new(File::create(path)?);
    riff_chunk.write(&mut file)?;
    list_chunk.write(&mut file)?;
    file.flush()?;
    file.get_ref().sync_all()?;

    Ok(())
}

/// Read WAV from file, will copy missing tags from RIFF to ID3
pub(crate) fn read_wav(path: impl AsRef<Path>) -> Result<Tag, Error> {
    let mut file = BufReader::new(File::open(path)?);
    let file_len = validate_header(&mut file)?;
    let mut offset = 0;
    // Read all the chunks
    let mut chunks = vec![];
    loop {
        let chunk = match Chunk::read(&mut file, offset) {
            Ok(chunk) => chunk,
            Err(_) => break
        };
        // Search for parent chunks
        if chunk.id() != LIST_ID && chunk.id() != RIFF_ID {
            offset += 4;
            continue;
        }
        if chunk.len() < 4 || chunk.offset() + 8 + u64::from(chunk.len()) > file_len {
            return Err(anyhow!("Invalid or truncated WAV container chunk"));
        }
        for child in chunk.iter(&mut file) {
            chunks.push(child?);
        }
        offset += 8 + u64::from(chunk.len()) + u64::from(chunk.len() % 2);
    }

    // Resolve nested chunks because SOME apps do that
    let mut new_chunks = vec![];
    for chunk in chunks {
        // Resolve nested
        if chunk.id() == RIFF_ID || chunk.id() == LIST_ID {
            for c in chunk.iter(&mut file) {
                new_chunks.push(c?);
            }
            continue;
        }
        new_chunks.push(chunk);
    }
    let chunks = new_chunks;

    // Read ID3 using the new ID3 reader
    file.seek(SeekFrom::Start(0))?;
    let mut id3 = id3::no_tag_ok(Tag::read_from2(&mut file))?.unwrap_or_default();

    // Copy tags from RIFF to ID3 if missing
    for (frame_name, chunk_id) in ID3_RIFF.iter() {
        if id3.get(frame_name).is_none() {
            if let Some(chunk) = chunks.iter().find(|c| &c.id() == chunk_id) {
                let data = chunk.read_contents(&mut file)?;
                if let Ok(data) = String::from_utf8(data) {
                    id3.add_frame(Frame::text(frame_name, data.replace("\0", "")));
                }
            }
        }
    }

    Ok(id3)
}

// Validate before parsing or staging a replacement; a zero-length RIFF/LIST must not loop.
fn validate_header(file: &mut BufReader<File>) -> Result<u64, Error> {
    let mut header = [0u8; 12];
    file.read_exact(&mut header)?;
    if &header[..4] != b"RIFF" || &header[8..] != b"WAVE" {
        return Err(anyhow!("Invalid WAV header"));
    }
    let declared_len = u64::from(u32::from_le_bytes(header[4..8].try_into().unwrap())) + 8;
    let file_len = file.get_ref().metadata()?.len();
    if declared_len < 12 || declared_len > file_len {
        return Err(anyhow!("Invalid or truncated WAV container"));
    }
    file.seek(SeekFrom::Start(0))?;
    Ok(file_len)
}
