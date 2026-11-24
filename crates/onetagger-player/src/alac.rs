use std::{fs::File, io::BufReader, path::Path, time::Duration};
use anyhow::Error;
use rodio::{Decoder, Source, source::SeekError};

/// Streaming, seekable ALAC compatibility wrapper.
pub struct ALACSource { decoder: Decoder<BufReader<File>> }
impl ALACSource {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        Ok(Self { decoder: Decoder::new(BufReader::new(File::open(path)?))? })
    }
}
impl Iterator for ALACSource {
    type Item = i16;
    fn next(&mut self) -> Option<i16> { self.decoder.next() }
}
impl Source for ALACSource {
    fn current_frame_len(&self) -> Option<usize> { self.decoder.current_frame_len() }
    fn channels(&self) -> u16 { self.decoder.channels() }
    fn sample_rate(&self) -> u32 { self.decoder.sample_rate() }
    fn total_duration(&self) -> Option<Duration> { self.decoder.total_duration() }
    fn try_seek(&mut self, position: Duration) -> Result<(), SeekError> { self.decoder.try_seek(position) }
}
