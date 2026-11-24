use std::path::{PathBuf, Path};
use anyhow::Error;
use std::io::BufReader;
use std::fs::File;
use rodio::{Source, Decoder};
use crate::AudioSource;
use lofty::file::AudioFile;

pub struct WAVSource {
    path: PathBuf,
    duration: u128
}

impl WAVSource {
    pub fn new(path: impl AsRef<Path>) -> Result<WAVSource, Error> {
        let mut wav = WAVSource {
            path: path.as_ref().to_owned(),
            duration: 0
        };
        // Get duration from decoder
        wav.duration = lofty::read_from_path(&path)?.properties().duration().as_millis();

        Ok(wav)
    }
}

impl AudioSource for WAVSource {
    // Get duration
    fn duration(&self) -> u128 {
        self.duration
    }

    // Get rodio decoder
    fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Error> {
        Ok(crate::with_duration(Decoder::new_wav(BufReader::new(File::open(&self.path)?))?, self.duration))
    }
}