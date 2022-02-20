use std::error::Error;
use std::fs::File;
use rodio::{Source, Decoder};
use crate::AudioSource;

pub struct FLACSource {
    path: String,
    duration: u128
}

impl FLACSource {
    pub fn new(path: &str) -> Result<FLACSource, Box<dyn Error>> {
        let mut flac = FLACSource {
            path: path.to_string(),
            duration: 0
        };
        // Get duration from decoder
        flac.duration = flac.get_source()?.total_duration().ok_or("Missing duration")?.as_millis();

        Ok(flac)
    }
}

impl AudioSource for FLACSource {
    // Get duration
    fn duration(&self) -> u128 {
        self.duration
    }

    // Get rodio decoder
    fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Box<dyn Error>> {
        Ok(Box::new(Decoder::new_flac(File::open(&self.path)?)?))
    }
}