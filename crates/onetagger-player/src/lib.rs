#[macro_use] extern crate log;
#[macro_use] extern crate anyhow;

use anyhow::Error;
use std::io::Cursor;
use std::path::Path;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread;
use hound::{WavSpec, SampleFormat, WavWriter};
use rodio::{Source};

pub mod mp3;
pub mod mp4;
pub mod wav;
pub mod ogg;
pub mod alac;
pub mod flac;
pub mod aiff;

/// Re-Export to prevent dependency issues
pub use rodio;

pub struct AudioPlayer {
    tx: Sender<PlayerAction>,
    rx: Receiver<bool>,
}

impl AudioPlayer {
    /// Create new instance
    pub fn new() -> AudioPlayer {
        // Create thread, becuase for some reason it stops working after closure ends
        let (tx_main, rx) = channel();
        let (tx, rx_main) = channel();
        thread::spawn(move || {
            let mut volume = 0.5;
            let mut source = None;
            // Create sink (Now DeviceSinkBuilder in rodio 0.22)
            let stream_handle = rodio::DeviceSinkBuilder::open_default_sink().unwrap();
            let mut player = rodio::Player::connect_new(&stream_handle.mixer());
            player.set_volume(volume);
            player.pause();
            // Wait for messages
            for action in rx {
                match action {
                    PlayerAction::Volume(v) => {
                        player.set_volume(v);
                        volume = v;
                    },
                    PlayerAction::Stop => {
                        player.stop();
                    }
                    PlayerAction::Play => player.play(),
                    PlayerAction::Pause => player.pause(),
                    // Play new source
                    PlayerAction::Load(audio_source) => {
                        // Create new sink
                        player.stop();
                        player = rodio::Player::connect_new(&stream_handle.mixer());
                        player.set_volume(volume);
                        player.pause();
                        // Append source
                        if let Ok(s) = audio_source.get_source() {
                            player.append(s);
                        }
                        // Save source
                        source = Some(audio_source);
                    },
                    // Seek by re-creating new source
                    PlayerAction::Seek(pos) => {
                        if source.is_some() {
                            // Create new sink
                            let paused = player.is_paused();
                            player.stop();
                            player = rodio::Player::connect_new(&stream_handle.mixer());
                            player.set_volume(volume);
                            if paused {
                                player.pause();
                            }
                            // Add source again
                            let s = source.as_ref().unwrap();
                            if let Ok(mut s) = s.get_source() {
                                // Skip manually because some sources are kinda bugged
                                let n_skip = s.sample_rate().get() as f32 * s.channels().get() as f32 * pos as f32 / 1000.0;
                                for _ in 0..n_skip as u64 {
                                    if s.next().is_none() {
                                        break;
                                    }
                                }
                                player.append(s);
                            }
                            // Sync
                            tx.send(!player.is_paused()).ok();
                        }
                    }
                }
            }
        });

        AudioPlayer {
            tx: tx_main,
            rx: rx_main
        }
    }

    // Load file
    pub fn load_file(&self, source: Box<dyn AudioSource + Send + 'static>) {
        self.tx.send(PlayerAction::Load(source)).ok();
    }

    pub fn play(&self) {
        self.tx.send(PlayerAction::Play).ok();
    }

    pub fn pause(&self) {
        self.tx.send(PlayerAction::Pause).ok();
    }

    pub fn seek(&self, pos: u64) -> bool {
        self.tx.send(PlayerAction::Seek(pos)).ok();
        // Wait for ready
        self.rx.recv().unwrap()
    }

    pub fn volume(&self, volume: f32) {
        self.tx.send(PlayerAction::Volume(volume)).ok();
    }

    pub fn stop(&self) {
        self.tx.send(PlayerAction::Stop).ok();
    }
}

enum PlayerAction {
    Play,
    Pause,
    Load(Box<dyn AudioSource + Send + 'static>),
    /// ms
    Seek(u64),
    /// 0.0 - 1.0
    Volume(f32),
    Stop,
}

/// Wrapper for getting audio sources
pub struct AudioSources {}
impl AudioSources {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Box<dyn AudioSource + Send + 'static>, Error> {
        let p = path.as_ref().extension().ok_or(anyhow!("Missing extension"))?.to_ascii_lowercase();
        // MP3
        if p == "mp3" {
            return Ok(Box::new(mp3::MP3Source::new(path)?));
        }
        // FLAC
        if p == "flac" {
            return Ok(Box::new(flac::FLACSource::new(path)?));
        }
        // AIFF
        if p == "aif" || p == "aiff" {
            return Ok(Box::new(aiff::AIFFSource::new(path)?));
        }
        // MP4
        if p == "m4a" || p == "mp4" {
            return Ok(Box::new(mp4::MP4Source::new(path)?));
        }
        // WAV
        if p == "wav" {
            return Ok(Box::new(wav::WAVSource::new(path)?));
        }
        // OGG
        if p == "ogg" || p == "opus" || p == "oga" || p == "spx" {
            return Ok(Box::new(ogg::OGGSource::new(path)?));
        }

        Err(anyhow!("Unsupported format!").into())
    }
}

pub trait AudioSource {
    /// Duration in ms
    fn duration(&self) -> u128;
    /// Rodio Source (Now strictly f32 in rodio 0.22)
    fn get_source(&self) -> Result<Box<dyn Source<Item = f32> + Send>, Error>;

    /// Stream generate 2D waveform, in thread, stream
    fn generate_waveform(&self, bars: i16) -> Result<(Receiver<f32>, Sender<bool>), Error> {
        let source = self.get_source()?;
        // Calculate n samples per bar
        let sample_rate = source.sample_rate().get() as f32;
        let channels = source.channels().get() as f32;
        let duration = self.duration() as f32 / 1000.0;
        let n_samples = (sample_rate * channels * (duration / bars as f32)).round() as usize;

        // Create thread
        let (tx, rx) = channel();
        let (tx1, rx1) = channel();
        thread::spawn(move || {
            // Get samples
            let mut samples: Vec<f32> = vec![];
            for sample in source {
                // Cancel
                if rx1.try_recv().is_ok() {
                    break;
                }

                samples.push(sample);

                // Buffer full
                if samples.len() >= n_samples {
                    // Re-scale the f32 samples (-1.0 to 1.0) back up to the old i16 scale logic so the UI waveform stays identical
                    let sum: f64 = samples.iter().fold(0.0, |s, v| s + (*v as f64 * 32768.0));
                    let wave: f64 = sum / samples.len() as f64;
                    tx.send(((wave.abs() + 1.0).log2() / 10.0) as f32).ok();
                    samples = vec![];
                }
            }
        });

        // tx1 = for canceling
        Ok((rx, tx1))
    }

    /// Generate wav for streaming in browser
    fn generate_wav(&self) -> Result<Vec<u8>, Error> {
        let source = self.get_source()?;
        let spec = WavSpec {
            channels: source.channels().get(),
            sample_rate: source.sample_rate().get(),
            bits_per_sample: 16,
            sample_format: SampleFormat::Int
        };
        // Generate wav
        let mut buf = vec![];
        {
            let mut cursor = Cursor::new(&mut buf);
            let mut writer = WavWriter::new(&mut cursor, spec)?;
            for s in source {
                writer.write_sample((s * 32767.0) as i16)?;
            }
            writer.finalize()?;
        }
        Ok(buf)
    }
}