use std::path::{PathBuf, Path};
use anyhow::Error;
use std::io::BufReader;
use std::fs::File;
use rodio::{Source, Decoder};
use crate::AudioSource;
use lofty::file::AudioFile;

pub struct FLACSource {
    path: PathBuf,
    duration: u128
}

impl FLACSource {
    pub fn new(path: impl AsRef<Path>) -> Result<FLACSource, Error> {
        let mut flac = FLACSource {
            path: path.as_ref().to_owned(),
            duration: 0
        };
        // Get duration from decoder
        flac.duration = lofty::read_from_path(&path)?.properties().duration().as_millis();

        Ok(flac)
    }
}

impl AudioSource for FLACSource {
    // Get duration
    fn duration(&self) -> u128 {
        self.duration
    }

    // Get rodio decoder
    fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Error> {
        Ok(crate::with_duration(Decoder::new_flac(BufReader::new(File::open(&self.path)?))?, self.duration))
    }
}