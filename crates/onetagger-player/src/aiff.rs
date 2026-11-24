use std::{fs::File, io::BufReader, path::{Path, PathBuf}};
use anyhow::Error;
use rodio::{Decoder, Source};
use crate::AudioSource;

pub struct AIFFSource { path: PathBuf, duration: u128 }
impl AIFFSource {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        let decoder = Decoder::new(BufReader::new(File::open(&path)?))?;
        let duration = decoder.total_duration().ok_or(anyhow!("Missing AIFF duration"))?.as_millis();
        Ok(Self { path: path.as_ref().into(), duration })
    }
}
impl AudioSource for AIFFSource {
    fn duration(&self) -> u128 { self.duration }
    fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Error> {
        Ok(Box::new(Decoder::new(BufReader::new(File::open(&self.path)?))?))
    }
}
