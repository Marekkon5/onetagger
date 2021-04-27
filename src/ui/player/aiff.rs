use std::error::Error;
use std::time::Duration;
use std::io::SeekFrom;
use rodio::Source;
use sndfile::{ReadOptions, SndFile, SndFileIO};
use crate::ui::player::AudioSource;

pub struct AIFFSource {
    path: String,
    snd: SndFile,
    len: u64,
    buffer: Vec<i16>,
    position: usize
}

impl AIFFSource {
    //Load from path
    pub fn new(path: &str) -> Result<AIFFSource, Box<dyn Error>> { 
        let mut snd = sndfile::OpenOptions::ReadOnly(ReadOptions::Auto).from_path(path)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:?}", e)))?;
        let len = snd.len().unwrap();
        snd.seek(SeekFrom::Start(0)).unwrap();

        let aiff = AIFFSource {
            path: path.to_owned(),
            snd,
            len,
            buffer: Vec::new(), 
            position: 0
        };
        Ok(aiff)
    }

}

impl AudioSource for AIFFSource {
    //Get duration
    fn duration(&self) -> u128 {
        (self.len as u128 / self.snd.get_samplerate() as u128) * 1000
    }

    //Get rodio source
    fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Box<dyn Error>> {
        Ok(Box::new(AIFFSource::new(&self.path)?))
    }
}

impl Source for AIFFSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.snd.get_channels() as u16
    }

    fn sample_rate(&self) -> u32 {
        self.snd.get_samplerate() as u32
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Iterator for AIFFSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        //Load more data
        //TODO: Currently shitty but works, streaming was broke
        if self.buffer.is_empty() {
            self.buffer = self.snd.read_all_to_vec().ok()?;
            self.position = 0;
        }
        if self.position >= self.buffer.len() {
            return None;
        }

        //Get sample
        let s = self.buffer[self.position];
        self.position += 1;
        Some(s)
    }
}