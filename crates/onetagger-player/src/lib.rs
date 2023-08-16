use std::error::Error;
use std::io::Cursor;
use std::path::Path;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread;
use hound::{WavSpec, SampleFormat, WavWriter};
use rodio::{Sink, Source};
use onetagger_shared::OTError;

pub mod mp3;
pub mod mp4;
pub mod wav;
pub mod ogg;
pub mod alac;
pub mod flac;
pub mod aiff;

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
            // Create sink
            let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
            let mut sink = Sink::try_new(&stream_handle).unwrap();
            sink.set_volume(volume);
            sink.pause();
            // Wait for messages
            for action in rx {
                match action {
                    PlayerAction::Volume(v) => {
                        sink.set_volume(v);
                        volume = v;
                    },
                    PlayerAction::Stop => {
                        sink.stop();
                    }
                    PlayerAction::Play => sink.play(),
                    PlayerAction::Pause => sink.pause(),
                    // Play new source
                    PlayerAction::Load(audio_source) => {
                        // Create new sink
                        sink.stop();
                        sink = Sink::try_new(&stream_handle).unwrap();
                        sink.set_volume(volume);
                        sink.pause();
                        // Append source
                        if let Ok(s) = audio_source.get_source() {
                            sink.append(s);
                        }
                        // Save source
                        source = Some(audio_source);
                    },
                    // Seek by re-creating new source
                    PlayerAction::Seek(pos) => {
                        if source.is_some() {
                            // Create new sink
                            let paused = sink.is_paused();
                            sink.stop();
                            sink = Sink::try_new(&stream_handle).unwrap();
                            sink.set_volume(volume);
                            if paused {
                                sink.pause();
                            }
                            // Add source again
                            let s = source.as_ref().unwrap();
                            if let Ok(mut s) = s.get_source() {
                                // Skip manually because some sources are kinda bugged
                                let n_skip = s.sample_rate() as f32 * s.channels() as f32 * pos as f32 / 1000.0;
                                for _ in 0..n_skip as u64 {
                                    if s.next().is_none() {
                                        break;
                                    }
                                }
                                sink.append(s);
                            }
                            // Sync
                            tx.send(!sink.is_paused()).ok();
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
    pub fn from_path(path: impl AsRef<Path>) -> Result<Box<dyn AudioSource + Send + 'static>, Box<dyn Error>> {
        let p = path.as_ref().extension().ok_or("Missing extension")?.to_ascii_lowercase();
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

        Err(OTError::new("Unsupported format!").into())
    }
}

pub trait AudioSource {
    /// Duration in ms
    fn duration(&self) -> u128;
    /// Rodio Source
    fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Box<dyn Error>>;

    /// Stream generate 2D waveform, in thread, stream
    fn generate_waveform(&self, bars: i16) -> Result<(Receiver<f32>, Sender<bool>), Box<dyn Error>> {
        let source = self.get_source()?;
        // Calculate n samples per bar
        let sample_rate = source.sample_rate() as f32;
        let channels = source.channels() as f32;
        let duration = self.duration() as f32 / 1000.0;
        let n_samples = (sample_rate * channels * (duration / bars as f32)).round() as usize;

        // Create thread
        let (tx, rx) = channel();
        let (tx1, rx1) = channel();
        thread::spawn(move || {
            // Get samples
            let mut samples: Vec<i16> = vec![];
            for sample in source {
                // Cancel
                if rx1.try_recv().is_ok() {
                    break;
                }

                samples.push(sample);

                // Buffer full
                if samples.len() >= n_samples {
                    let sum: i64 = samples.iter().fold(0, |s, v| s + *v as i64);
                    let wave: f64 = sum as f64 / samples.len() as f64;
                    tx.send(((wave.abs() + 1.0).log2() / 10.0) as f32).ok();
                    samples = vec![];
                }
            }
        });

        // tx1 = for canceling
        Ok((rx, tx1))
    }

    /// Generate wav for streaming in browser
    fn generate_wav(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let source = self.get_source()?;
        let spec = WavSpec {
            channels: source.channels(),
            sample_rate: source.sample_rate(),
            bits_per_sample: 16,
            sample_format: SampleFormat::Int
        };
        // Generate wav
        let mut buf = vec![];
        {
            let mut cursor = Cursor::new(&mut buf);
            let mut writer = WavWriter::new(&mut cursor, spec)?;
            for s in source {
                writer.write_sample(s)?;
            }
            writer.finalize()?;
        }
        Ok(buf)
    }
}