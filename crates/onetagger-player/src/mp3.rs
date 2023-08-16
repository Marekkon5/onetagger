use std::path::{Path, PathBuf};
use std::error::Error;
use std::io::BufReader;
use std::fs::File;
use rodio::{Source, Decoder};
use crate::AudioSource;

pub struct MP3Source {
    path: PathBuf,
    duration: u128
}
impl MP3Source {
    pub fn new(path: impl AsRef<Path>) -> Result<MP3Source, Box<dyn Error>> {
        // Get duration
        let duration = mp3_duration::from_path(&path)?.as_millis();

        Ok(MP3Source {
            path: path.as_ref().to_owned(),
            duration
        })
    }
}

impl AudioSource for MP3Source {
    // Get duration
    fn duration(&self) -> u128 {
        self.duration
    }

    // Get rodio decoder
    fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Box<dyn Error>> {
        Ok(Box::new(Decoder::new_mp3(BufReader::new(File::open(&self.path)?))?))
    }
}