//! Read-only workaround for Symphonia 0.5.5's AIFF SSND length calculation.
//!
//! SoundChunk::parse consumes the eight offset/block-size bytes but includes
//! them in the remaining sound length. A trailing ID3 chunk is then decoded as
//! four extra 16-bit samples; without a following chunk the final packet can be
//! lost to EOF. Present a corrected length to that demuxer, without changing the
//! file, buffering its audio, or changing sample/seek behavior for other formats.
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use symphonia::core::io::MediaSource;

pub(crate) struct AiffInput {
    file: File,
    length: u64,
    position: u64,
    patch_at: u64,
    sound_length: [u8; 4],
}

impl AiffInput {
    pub(crate) fn new(mut file: File) -> io::Result<Self> {
        let invalid = || io::Error::new(io::ErrorKind::InvalidData, "Invalid AIFF sound chunk");
        let length = file.metadata()?.len();
        let mut header = [0; 12];
        file.read_exact(&mut header)?;
        if &header[..4] != b"FORM" || !matches!(&header[8..], b"AIFF" | b"AIFC") {
            return Err(invalid());
        }
        let end = 8 + u64::from(u32::from_be_bytes(header[4..8].try_into().unwrap()));
        if end < 12 || end > length { return Err(invalid()); }
        let mut position = 12;
        // Bound malformed inputs with thousands of empty chunks.
        for _ in 0..65_536 {
            if position + 8 > end { break; }
            let mut chunk = [0; 8];
            file.read_exact(&mut chunk)?;
            let size = u32::from_be_bytes(chunk[4..].try_into().unwrap());
            let next = position + 8 + u64::from(size) + u64::from(size & 1);
            if next > end { return Err(invalid()); }
            if &chunk[..4] == b"SSND" {
                let size = size.checked_sub(8).ok_or_else(invalid)?;
                file.seek(SeekFrom::Start(0))?;
                return Ok(Self { file, length, position: 0, patch_at: position + 4,
                    sound_length: size.to_be_bytes() });
            }
            file.seek(SeekFrom::Start(next))?;
            position = next;
        }
        Err(invalid())
    }
}

impl Read for AiffInput {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let start = self.position;
        let count = self.file.read(output)?;
        self.position += count as u64;
        let from = start.max(self.patch_at);
        let to = self.position.min(self.patch_at + 4);
        if from < to {
            output[(from - start) as usize..(to - start) as usize]
                .copy_from_slice(&self.sound_length[(from - self.patch_at) as usize..(to - self.patch_at) as usize]);
        }
        Ok(count)
    }
}

impl Seek for AiffInput {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.position = self.file.seek(position)?;
        Ok(self.position)
    }
}

impl MediaSource for AiffInput {
    fn is_seekable(&self) -> bool { true }
    fn byte_len(&self) -> Option<u64> { Some(self.length) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn split_reads_and_seeks_overlay_only_the_demuxer_length_and_leave_the_file_unchanged() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-fixtures/tone.aiff");
        let original = std::fs::read(&path).unwrap();
        let mut input = AiffInput::new(File::open(&path).unwrap()).unwrap();
        let offset = input.patch_at as usize;
        let mut expected = original.clone();
        expected[offset..offset + 4].copy_from_slice(&input.sound_length);
        let mut actual = vec![];
        let mut block = [0; 3];
        loop {
            let count = input.read(&mut block).unwrap();
            if count == 0 { break; }
            actual.extend_from_slice(&block[..count]);
        }
        assert_eq!(actual, expected);
        input.seek(SeekFrom::Start(input.patch_at - 1)).unwrap();
        let mut across = [0; 6];
        input.read_exact(&mut across).unwrap();
        assert_eq!(across, expected[offset - 1..offset + 5]);
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
}
