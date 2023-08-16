use std::error::Error;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;
use rodio::Source;
use alac::{Reader, Samples, StreamInfo};

pub struct ALACSource {
    samples: Samples<BufReader<File>, i32>,
    stream_info: StreamInfo
}

impl ALACSource {
    // Read alac from file
    pub fn new(path: impl AsRef<Path>) -> Result<ALACSource, Box<dyn Error>> {
        let file = File::open(path)?;
        let r = BufReader::new(file);
        let reader = Reader::new(r)?;
        let stream_info = reader.stream_info().to_owned();
        Ok(ALACSource {
            samples: reader.into_samples(),
            stream_info
        })
    }
}

impl Source for ALACSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.stream_info.channels() as u16
    }

    fn sample_rate(&self) -> u32 {
        self.stream_info.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Iterator for ALACSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        // Wrapper against samples
        if let Some(r) = self.samples.next() {
            if let Ok(s) = r {
                return Some((s >> 16) as i16);
            }
        }
        None
    }
}