use anyhow::Error;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;
use std::num::{NonZeroU16, NonZeroU32};
use rodio::Source;
use alac::{Reader, Samples, StreamInfo};

pub struct ALACSource {
    samples: Samples<BufReader<File>, i32>,
    stream_info: StreamInfo
}

impl ALACSource {
    // Read alac from file
    pub fn new(path: impl AsRef<Path>) -> Result<ALACSource, Error> {
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
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> NonZeroU16 {
        NonZeroU16::new(self.stream_info.channels() as u16).unwrap_or(NonZeroU16::new(1).unwrap())
    }

    fn sample_rate(&self) -> NonZeroU32 {
        NonZeroU32::new(self.stream_info.sample_rate()).unwrap_or(NonZeroU32::new(44100).unwrap())
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Iterator for ALACSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        // Wrapper against samples
        if let Some(r) = self.samples.next() {
            if let Ok(s) = r {
                return Some(((s >> 16) as f32) / 32768.0);
            }
        }
        None
    }
}