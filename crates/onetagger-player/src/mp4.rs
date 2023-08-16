use std::error::Error;
use std::io::BufReader;
use std::fs::File;
use std::path::{PathBuf, Path};
use rodio::Source;
use redlux::Decoder;
use mp4::Mp4Reader;

use crate::AudioSource;
use crate::alac::ALACSource;

pub struct MP4Source {
    path: PathBuf,
    duration: u128,
    alac: bool
}

impl MP4Source {
    pub fn new(path: impl AsRef<Path>) -> Result<MP4Source, Box<dyn Error>> {
        let file = File::open(&path)?;
        let metadata = file.metadata()?;
        let mp4 = Mp4Reader::read_header(BufReader::new(file), metadata.len())?;
        let track = mp4.tracks().values().next().ok_or("No tracks!")?;
        // ALAC will fail on this function so i guess dirty but works
        let alac = track.audio_profile().is_err();

        Ok(MP4Source {
            path: path.as_ref().to_owned(),
            duration: mp4.duration().as_millis(),
            alac
        })
    }
}

impl AudioSource for MP4Source {
    fn duration(&self) -> u128 {
        self.duration
    }

    fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Box<dyn Error>> {
        // ALAC MP4
        if self.alac {
            let alac = ALACSource::new(&self.path)?;
            return Ok(Box::new(alac));
        }
        
        // AAC MP4
        let f = File::open(&self.path)?;
        let meta = f.metadata()?;
        let reader = BufReader::new(f);
        let mut decoder = Decoder::new_mpeg4(reader, meta.len())?;
        // Decode first sample otherwise for some reason the channels and sample rate is 0
        decoder.decode_next_sample()?.ok_or("No samples!")?;
        Ok(Box::new(decoder))
    }
}
