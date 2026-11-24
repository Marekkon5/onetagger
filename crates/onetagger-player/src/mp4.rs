use std::path::{Path, PathBuf};
use anyhow::Error;
use lofty::file::AudioFile;
use rodio::Source;
use crate::AudioSource;

pub struct MP4Source { path: PathBuf, duration: u128 }
impl MP4Source {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        let duration = lofty::read_from_path(&path)?.properties().duration().as_millis();
        Ok(Self { path: path.as_ref().into(), duration })
    }
}
impl AudioSource for MP4Source {
    fn duration(&self) -> u128 { self.duration }
    fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Error> { crate::file_source(&self.path, self.duration) }
}
