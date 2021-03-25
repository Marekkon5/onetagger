use std::error::Error;
use rodio::Source;
use crate::ui::player::AudioSource;

pub struct AIFFSource {
    path: String,
    duration: u128
}

impl AIFFSource {
    //Load from path
    pub fn new(path: &str) -> Result<AIFFSource, Box<dyn Error>> { 
        let mut aiff = AIFFSource {
            path: path.to_owned(),
            duration: 0
        };
        //Get duration from decoder
        aiff.duration = aiff.get_source()?.total_duration().ok_or("Missing duration")?.as_millis();

        Ok(aiff)
    }
}

impl AudioSource for AIFFSource {
    //Get duration
    fn duration(&self) -> u128 {
        self.duration
    }

    //Get rodio source
    fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Box<dyn Error>> {
        #[cfg(target_os = "windows")]
        {
            //Create temp file for reading, it is because saving to aiff while player/waveform is open on Windows breaks
            use tempfile::NamedTempFile;
            use std::fs::File;
            use std::io::copy;
            let mut tmp_file = NamedTempFile::new()?;
            let mut file = File::open(&self.path)?;
            copy(&mut file, &mut tmp_file)?;
            return Ok(Box::new(ffmpeg_decoder::Decoder::open(tmp_file.path())?));
        }

        Ok(Box::new(ffmpeg_decoder::Decoder::open(&self.path)?))
    }
}