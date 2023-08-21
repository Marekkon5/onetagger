use std::time::Duration;
use anyhow::Error;
use std::path::{PathBuf, Path};
use std::io::BufReader;
use std::fs::File;
use lofty::AudioFile;
use rodio::{Source, Decoder};

use crate::AudioSource;

pub struct OGGSource {
    path: PathBuf,
    duration: Duration,
}

impl OGGSource {
    pub fn new(path: impl AsRef<Path>) -> Result<OGGSource, Error> {
        // Get duration
        let file = lofty::read_from_path(&path)?;
        let duration = file.properties().duration();

        Ok(OGGSource {
            duration,
            path: path.as_ref().into()
        })
    }
}

impl AudioSource for OGGSource {
    fn duration(&self) -> u128 {
        self.duration.as_millis()
    }

    fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Error> {
        // Use rodio vorbis
        Ok(Box::new(Decoder::new_vorbis(BufReader::new(File::open(&self.path)?))?))
    }
}

