use std::path::Path;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::time::Duration;
use lofty::AudioFile;
use pacmog::PcmReader;
use rodio::Source;

use crate::AudioSource;

pub struct AIFFSource {
    path: String,
    duration: Duration
}

impl AIFFSource {
    // Load from path
    pub fn new(path: &str) -> Result<AIFFSource, Box<dyn Error>> { 
        // Get duration
        let file = lofty::read_from_path(&path)?;
        let duration = file.properties().duration();

        Ok(AIFFSource {
            path: path.to_owned(),
            duration
        })
    }

}

impl AudioSource for AIFFSource {
    // Get duration
    fn duration(&self) -> u128 {
        self.duration.as_millis()

    }

    // Get rodio source
    fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Box<dyn Error>> {
        let source = AIFFDecoder::load(&self.path)?;
        Ok(Box::new(source.convert_samples()))
    }
}

struct AIFFDecoder {
    channels: u32,
    samples: u32,
    sample_rate: u32,
    index: usize,
    buffer: Vec<i16>
}

impl AIFFDecoder {
    /// Load file into memory
    pub fn load(path: impl AsRef<Path>) -> Result<AIFFDecoder, Box<dyn Error>> {
        // Load file
        let mut data = vec![];
        File::open(path)?.read_to_end(&mut data)?;

        // Parse metadata (catch panic, because weird library)
        let reader = std::panic::catch_unwind(|| {
            PcmReader::new(&data)
        }).map_err(|e| format!("Not an AIFF file: {e:?}"))?;
        let specs = reader.get_pcm_specs();

        // Decode the file (because the library is weeeird)
        let mut samples = vec![0i16; specs.num_channels as usize * specs.num_samples as usize];
        let mut i = 0;
        for sample in 0..specs.num_samples {
            for channel in 0..specs.num_channels {
                let s = std::panic::catch_unwind(|| {
                    reader.read_sample(channel as u32, sample)
                }).map_err(|e| format!("Failed decoding AIFF: {e:?}"))?.map_err(|e| format!("Failed decoding AIFF: {e}"))?;
                samples[i] = (s * i16::MAX as f32) as i16;
                i += 1;
            }
        }

        Ok(AIFFDecoder {
            channels: specs.num_channels as u32,
            samples: specs.num_samples,
            sample_rate: specs.sample_rate,
            index: 0,
            buffer: samples,
        })
    }
}

impl Source for AIFFDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels as u16
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(self.samples as f32 / self.sample_rate as f32))
    }
}

impl Iterator for AIFFDecoder {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.buffer.len() {
            return None;
        }
        let sample = self.buffer[self.index];
        self.index += 1;
        Some(sample)
    }
}