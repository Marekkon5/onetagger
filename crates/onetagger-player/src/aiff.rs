use std::{fs::File, io::BufReader, path::{Path, PathBuf}};
use anyhow::Error;
use rodio::{Decoder, Source};
use crate::AudioSource;
use lofty::file::AudioFile;

pub struct AIFFSource { path: PathBuf, duration: u128 }
impl AIFFSource {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        let duration = lofty::read_from_path(&path)?.properties().duration().as_millis();
        Ok(Self { path: path.as_ref().into(), duration })
    }
}
impl AudioSource for AIFFSource {
    fn duration(&self) -> u128 { self.duration }
    fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Error> {
        Ok(crate::with_duration(Decoder::new(BufReader::new(File::open(&self.path)?))?, self.duration))
    }
}
