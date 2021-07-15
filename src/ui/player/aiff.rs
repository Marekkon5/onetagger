use std::error::Error;
use std::time::Duration;
use std::io::SeekFrom;
use ndarray::ArrayViewMut2;
use rodio::Source;
use sndfile::{ReadOptions, SndFile, SndFileNDArrayIO};
use crate::ui::player::AudioSource;

pub struct AIFFSource {
    path: String,
    snd: SndFile,
    len: u64,
    buffer: Vec<i16>,
    position: usize
}

impl AIFFSource {
    // Load from path
    pub fn new(path: &str) -> Result<AIFFSource, Box<dyn Error>> { 
        let mut snd = sndfile::OpenOptions::ReadOnly(ReadOptions::Auto).from_path(path)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:?}", e)))?;
        let len = snd.len().ok().ok_or("Invalid length")?;
        snd.seek(SeekFrom::Start(0)).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{:?}", e)))?;


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
    // Get duration
    fn duration(&self) -> u128 {
        (self.len as u128 / self.snd.get_samplerate() as u128) * 1000
    }

    // Get rodio source
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
        // Load more data
        if self.position >= self.buffer.len() {
            // Read to ndarray, because reading to slice produced bad data
            let mut buffer = vec![0; 2048];
            let nd_buffer = ArrayViewMut2::<i16>::from_shape((1024, 2), &mut buffer).ok()?;
            let read = self.snd.read_to_ndarray(nd_buffer).ok()?;
            if read == 0 {
                return None;
            }

            self.buffer = buffer;
            self.position = 0;
        }

        // Get sample
        let s = self.buffer[self.position];
        self.position += 1;
        Some(s)
    }
}