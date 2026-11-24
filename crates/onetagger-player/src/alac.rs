use std::{path::Path, time::Duration};
use anyhow::Error;
use rodio::{Source, source::SeekError};
use lofty::file::AudioFile;

/// Streaming, seekable ALAC compatibility wrapper.
pub struct ALACSource { decoder: Box<dyn Source<Item = i16> + Send> }
impl ALACSource {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        let duration = lofty::read_from_path(&path)?.properties().duration().as_millis();
        Ok(Self { decoder: crate::file_source(path.as_ref(), duration)? })
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
