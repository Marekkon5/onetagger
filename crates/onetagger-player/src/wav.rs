use std::error::Error;
use std::fs::File;
use rodio::{Source, Decoder};
use crate::AudioSource;

pub struct WAVSource {
    path: String,
    duration: u128
}

impl WAVSource {
    pub fn new(path: &str) -> Result<WAVSource, Box<dyn Error>> {
        let mut wav = WAVSource {
            path: path.to_string(),
            duration: 0
        };
        // Get duration from decoder
        wav.duration = wav.get_source()?.total_duration().ok_or("Missing duration")?.as_millis();

        Ok(wav)
    }
}

impl AudioSource for WAVSource {
    // Get duration
    fn duration(&self) -> u128 {
        self.duration
    }

    // Get rodio decoder
    fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Box<dyn Error>> {
        Ok(Box::new(Decoder::new_wav(File::open(&self.path)?)?))
    }
}