use anyhow::Error;
use mp4parse::{SampleEntry, CodecType};
use rodio::decoder::Mp4Type;
use std::io::BufReader;
use std::fs::File;
use std::path::{PathBuf, Path};
use rodio::{Source, Decoder};

use crate::AudioSource;
use crate::alac::ALACSource;

pub struct MP4Source {
    path: PathBuf,
    duration: u128,
    alac: bool
}

impl MP4Source {
    pub fn new(path: impl AsRef<Path>) -> Result<MP4Source, Error> {
        let file = File::open(&path)?;
        let mp4 = mp4parse::read_mp4(&mut BufReader::new(file))?;
        let track = mp4.tracks.first().ok_or(anyhow!("No MP4 tracks"))?;
        let duration = track.duration.ok_or(anyhow!("Missing duration"))?.0 as f32 / track.timescale.ok_or(anyhow!("Missing timescale"))?.0 as f32;
        // Check if alac
        let mut alac = false;
        if let SampleEntry::Audio(entry) = track.stsd.as_ref().ok_or(anyhow!("Missing stsd"))?.descriptions.first().ok_or(anyhow!("Missing first stsd"))? {
            alac = entry.codec_type == CodecType::ALAC
        }

        debug!("Creating MP4 source ok, alac: {}", alac);

        Ok(MP4Source {
            path: path.as_ref().to_owned(),
            duration: (duration * 1000.0) as u128,
            alac
        })
    }
}

impl AudioSource for MP4Source {
    fn duration(&self) -> u128 {
        self.duration
    }

    fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Error> {
        // ALAC MP4
        if self.alac {
            let alac = ALACSource::new(&self.path)?;
            return Ok(Box::new(alac));
        }
        
        // Symphonia 
        let decoder = Decoder::new_mp4(BufReader::new(File::open(&self.path)?), Mp4Type::M4a)?;
        return Ok(Box::new(decoder));
        
    }
}
