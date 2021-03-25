use std::error::Error;
use std::fs::File;
use rodio::{Source, Decoder};
use crate::ui::player::AudioSource;

pub struct MP3Source {
    path: String,
    duration: u128
}
impl MP3Source {
    pub fn new(path: &str) -> Result<MP3Source, Box<dyn Error>> {
        //Get duration
        let duration = mp3_duration::from_path(path)?.as_millis();

        Ok(MP3Source {
            path: path.to_owned(),
            duration
        })
    }
}

impl AudioSource for MP3Source {
    //Get duration
    fn duration(&self) -> u128 {
        self.duration
    }

    //Get rodio decoder
    fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Box<dyn Error>> {
        Ok(Box::new(Decoder::new_mp3(File::open(&self.path)?)?))
    }
}