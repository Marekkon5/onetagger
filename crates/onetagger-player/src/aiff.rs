use std::path::{Path, PathBuf};
use anyhow::Error;
use lofty::file::AudioFile;
use std::fs::File;
use std::io::Read;
use std::time::Duration;
use std::num::{NonZeroU16, NonZeroU32};
use pacmog::PcmReader;
use rodio::Source;

use crate::AudioSource;

pub struct AIFFSource {
    path: PathBuf,
    duration: Duration
}

impl AIFFSource {
    // Load from path
    pub fn new(path: impl AsRef<Path>) -> Result<AIFFSource, Error> { 
        // Get duration
        let file = lofty::read_from_path(&path)?;
        let duration = file.properties().duration();

        Ok(AIFFSource {
            path: path.as_ref().to_owned(),
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
    fn get_source(&self) -> Result<Box<dyn Source<Item = f32> + Send>, Error> {
        let source = AIFFDecoder::load(&self.path)?;
        Ok(Box::new(source))
    }
}

struct AIFFDecoder {
    channels: u32,
    samples: u32,
    sample_rate: u32,
    index: usize,
    buffer: Vec<f32>
}

impl AIFFDecoder {
    /// Load file into memory
    pub fn load(path: impl AsRef<Path>) -> Result<AIFFDecoder, Error> {
        // Load file
        let mut data = vec![];
        File::open(path)?.read_to_end(&mut data)?;

        // Parse metadata (catch panic, because weird library)
        let reader = std::panic::catch_unwind(|| {
            PcmReader::new(&mut &data[..])
        }).map_err(|e| anyhow!("Not an AIFF file: {e:?}"))??;
        let specs = reader.get_pcm_specs();

        // Decode the file (because the library is weeeird)
        // TODO: Make better using symphonia / new rodio
        let mut samples = vec![0f32; specs.num_channels as usize * specs.num_samples as usize];
        let mut i = 0;
        for sample in 0..specs.num_samples {
            for channel in 0..specs.num_channels {
                let s = std::panic::catch_unwind(|| {
                    reader.read_sample::<f32>(channel, sample)
                }).map_err(|e| anyhow!("Failed decoding AIFF: {e:?}"))?.map_err(|e| anyhow!("Failed decoding AIFF: {e}"))?;
                samples[i] = s;
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
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> NonZeroU16 {
        NonZeroU16::new(self.channels as u16).unwrap_or(NonZeroU16::new(1).unwrap())
    }

    fn sample_rate(&self) -> NonZeroU32 {
        NonZeroU32::new(self.sample_rate).unwrap_or(NonZeroU32::new(44100).unwrap())
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(self.samples as f32 / self.sample_rate as f32))
    }
}

impl Iterator for AIFFDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.buffer.len() {
            return None;
        }
        let sample = self.buffer[self.index];
        self.index += 1;
        Some(sample)
    }
}