#[macro_use] extern crate log;
#[macro_use] extern crate anyhow;

use anyhow::Error;
use std::io::Cursor;
use std::path::Path;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread;
use hound::{WavSpec, SampleFormat, WavWriter};
use rodio::{Sink, Source};

pub mod mp3;
pub mod mp4;
pub mod wav;
pub mod ogg;
pub mod alac;
pub mod flac;
pub mod aiff;

/// Re-Export to prevent dependency issues
pub use rodio;

#[derive(Clone)]
pub struct AudioPlayer { tx: Sender<PlayerAction> }

#[derive(Debug, Default, Clone, Copy)]
pub struct PlayerState { pub position: u64, pub playing: bool, pub ended: bool }

impl AudioPlayer {
    pub fn new() -> AudioPlayer {
        let (tx, rx) = channel();
        thread::spawn(move || {
            let Ok((_stream, stream_handle)) = rodio::OutputStream::try_default() else {
                warn!("No audio output is available"); return;
            };
            let Ok(mut sink) = Sink::try_new(&stream_handle) else { warn!("Cannot open the audio sink"); return; };
            let mut volume = 0.5;
            let mut position_base = 0;
            let mut source: Option<Box<dyn AudioSource + Send>> = None;
            sink.pause();
            for action in rx {
                match action {
                    PlayerAction::Volume(value) => { volume = value.clamp(0.0, 1.0); sink.set_volume(volume); },
                    PlayerAction::Stop => { sink.stop(); source = None; },
                    PlayerAction::Play => sink.play(),
                    PlayerAction::Pause => sink.pause(),
                    PlayerAction::Load(next, reply) => {
                        let result = (|| -> Result<(), Error> {
                            let decoder = next.get_source()?;
                            sink.stop();
                            sink = Sink::try_new(&stream_handle)?;
                            sink.set_volume(volume); sink.pause(); sink.append(decoder);
                            source = Some(next); position_base = 0;
                            Ok(())
                        })();
                        reply.send(result.map_err(|error| error.to_string())).ok();
                    },
                    PlayerAction::Seek(position, reply) => {
                        let result = (|| -> Result<PlayerState, Error> {
                            let source = source.as_ref().ok_or(anyhow!("No track is loaded"))?;
                            let position = position.min(source.duration() as u64);
                            if sink.empty() {
                                // An exhausted sink no longer owns a decoder. Reopen and
                                // seek that decoder directly, without decoding the prefix.
                                let mut decoder = source.get_source()?;
                                decoder.try_seek(std::time::Duration::from_millis(position)).map_err(|error| anyhow!(error.to_string()))?;
                                let paused = sink.is_paused();
                                sink = Sink::try_new(&stream_handle)?;
                                sink.set_volume(volume);
                                if paused { sink.pause(); }
                                sink.append(decoder);
                                position_base = position;
                            } else {
                                sink.try_seek(std::time::Duration::from_millis(position)).map_err(|error| anyhow!(error.to_string()))?;
                                position_base = 0;
                            }
                            Ok(PlayerState { position, playing: !sink.is_paused() && !sink.empty(), ended: sink.empty() })
                        })();
                        reply.send(result.map_err(|error| error.to_string())).ok();
                    },
                    PlayerAction::State(reply) => {
                        let ended = source.is_some() && sink.empty();
                        reply.send(PlayerState {
                            position: if ended { source.as_ref().unwrap().duration() as u64 }
                                else { position_base + sink.get_pos().as_millis() as u64 },
                            playing: !sink.is_paused() && !sink.empty(), ended,
                        }).ok();
                    },
                }
            }
        });
        AudioPlayer { tx }
    }

    pub fn load_file(&self, source: Box<dyn AudioSource + Send + 'static>) -> Result<(), Error> {
        let (reply, receiver) = channel();
        self.tx.send(PlayerAction::Load(source, reply)).map_err(|_| anyhow!("Audio output is unavailable"))?;
        receiver.recv_timeout(std::time::Duration::from_secs(10))?.map_err(Error::msg)
    }
    pub fn play(&self) { self.tx.send(PlayerAction::Play).ok(); }
    pub fn pause(&self) { self.tx.send(PlayerAction::Pause).ok(); }
    pub fn volume(&self, volume: f32) { self.tx.send(PlayerAction::Volume(volume)).ok(); }
    pub fn stop(&self) { self.tx.send(PlayerAction::Stop).ok(); }
    pub fn seek(&self, position: u64) -> Result<PlayerState, Error> {
        let (reply, receiver) = channel();
        self.tx.send(PlayerAction::Seek(position, reply)).map_err(|_| anyhow!("Audio output is unavailable"))?;
        receiver.recv_timeout(std::time::Duration::from_secs(10))?.map_err(Error::msg)
    }
    pub fn state(&self) -> Result<PlayerState, Error> {
        let (reply, receiver) = channel();
        self.tx.send(PlayerAction::State(reply)).map_err(|_| anyhow!("Audio output is unavailable"))?;
        Ok(receiver.recv_timeout(std::time::Duration::from_secs(2))?)
    }
}

enum PlayerAction {
    Play, Pause, Stop, Volume(f32),
    Load(Box<dyn AudioSource + Send + 'static>, Sender<Result<(), String>>),
    Seek(u64, Sender<Result<PlayerState, String>>),
    State(Sender<PlayerState>),
}

/// Wrapper for getting audio sources
// Rodio 0.20.1's Symphonia adapter mis-converts fractional durations (including
// integer seconds). Keep the existing dependency API, report container metadata,
// and handle an explicit end seek without using its incorrect duration.
struct TimedSource<S> { source: S, duration: std::time::Duration, ended: bool }
impl<S: Source<Item = i16>> Iterator for TimedSource<S> {
    type Item = i16;
    fn next(&mut self) -> Option<i16> { if self.ended { None } else { self.source.next() } }
    fn size_hint(&self) -> (usize, Option<usize>) { if self.ended { (0, Some(0)) } else { self.source.size_hint() } }
}
impl<S: Source<Item = i16>> Source for TimedSource<S> {
    fn current_frame_len(&self) -> Option<usize> { if self.ended { Some(0) } else { self.source.current_frame_len() } }
    fn channels(&self) -> u16 { self.source.channels() }
    fn sample_rate(&self) -> u32 { self.source.sample_rate() }
    fn total_duration(&self) -> Option<std::time::Duration> { Some(self.duration) }
    fn try_seek(&mut self, position: std::time::Duration) -> Result<(), rodio::source::SeekError> {
        if position >= self.duration { self.ended = true; return Ok(()); }
        self.source.try_seek(position)?;
        self.ended = false;
        Ok(())
    }
}
pub(crate) fn with_duration<S: Source<Item = i16> + Send + 'static>(source: S, milliseconds: u128)
    -> Box<dyn Source<Item = i16> + Send> {
    Box::new(TimedSource { source, duration: std::time::Duration::from_millis(milliseconds.min(u64::MAX as u128) as u64), ended: false })
}

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
    /// Rodio Source
    fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Error>;

    /// Constant-memory RMS summary. Cancellation is checked at bounded sample intervals.
    fn waveform(&self, bars: usize, cancelled: &dyn Fn() -> bool) -> Result<Vec<f32>, Error> {
        if bars == 0 || bars > 4096 { return Err(anyhow!("Invalid waveform size")); }
        let source = self.get_source()?;
        let samples_per_bar = ((source.sample_rate() as f64 * source.channels() as f64
            * self.duration() as f64 / 1000.0 / bars as f64).ceil() as usize).max(1);
        let mut output = Vec::with_capacity(bars);
        let (mut squares, mut count) = (0.0f64, 0usize);
        for (index, sample) in source.enumerate() {
            if index % 4096 == 0 && cancelled() { return Err(anyhow!("Waveform cancelled")); }
            let value = sample as f64 / 32768.0;
            squares += value * value; count += 1;
            if count >= samples_per_bar {
                output.push((squares / count as f64).sqrt() as f32);
                squares = 0.0; count = 0;
                if output.len() == bars { break; }
            }
        }
        if count > 0 && output.len() < bars { output.push((squares / count as f64).sqrt() as f32); }
        output.resize(bars, 0.0);
        Ok(output)
    }

    /// Compatibility adapter for callers using the original streamed interface.
    fn generate_waveform(&self, bars: i16) -> Result<(Receiver<f32>, Sender<bool>), Error> {
        let peaks = self.waveform(bars.max(1) as usize, &|| false)?;
        let (sender, receiver) = std::sync::mpsc::sync_channel(8);
        let (cancel, cancelled) = channel();
        thread::spawn(move || {
            for peak in peaks {
                if cancelled.try_recv().is_ok() || sender.send(peak).is_err() { break; }
            }
        });
        Ok((receiver, cancel))
    }

    fn write_wav(&self, output: &mut dyn WriteSeek) -> Result<(), Error> {
        let source = self.get_source()?;
        let spec = WavSpec { channels: source.channels(), sample_rate: source.sample_rate(),
            bits_per_sample: 16, sample_format: SampleFormat::Int };
        let estimated = self.duration().saturating_mul(spec.sample_rate as u128)
            .saturating_mul(spec.channels as u128).saturating_mul(2) / 1000;
        if estimated > 1024 * 1024 * 1024 { return Err(anyhow!("Audio preview exceeds the 1 GiB conversion limit")); }
        let mut writer = WavWriter::new(output, spec)?;
        for (index, sample) in source.enumerate() {
            if index >= 512 * 1024 * 1024 { return Err(anyhow!("Audio preview exceeds the 1 GiB conversion limit")); }
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
        Ok(())
    }

    /// Generate wav for streaming in browser
    fn generate_wav(&self) -> Result<Vec<u8>, Error> {
        let mut buffer = Cursor::new(Vec::new());
        self.write_wav(&mut buffer)?;
        Ok(buffer.into_inner())
    }
}

pub trait WriteSeek: std::io::Write + std::io::Seek {}
impl<T: std::io::Write + std::io::Seek> WriteSeek for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn each_shipped_decoder_streams_and_seeks_to_the_end_of_a_real_fixture() {
        for name in ["tone.wav", "tone.aiff", "tone.flac", "tone.mp3", "tone.ogg", "tone-aac.m4a", "tone-alac.m4a"] {
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-fixtures").join(name);
            let source = AudioSources::from_path(&path).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert!((1900..=2300).contains(&source.duration()), "{name}: unexpected duration {}", source.duration());
            let mut decoder = source.get_source().unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!(decoder.total_duration().unwrap().as_millis(), source.duration(), "{name}");
            assert_eq!(decoder.channels(), 2, "{name}");
            assert_eq!(decoder.sample_rate(), 44100, "{name}");
            assert!(decoder.by_ref().take(1000).any(|sample| sample != 0), "{name}: silent decoder");
            decoder.try_seek(Duration::from_millis(1800)).unwrap_or_else(|error| panic!("{name}: {error}"));
            let remaining = decoder.count();
            assert!(remaining > 1000 && remaining < 44100, "{name}: seek left {remaining} samples");
            let mut decoder = source.get_source().unwrap();
            decoder.try_seek(Duration::from_millis(source.duration() as u64)).unwrap();
            assert_eq!(decoder.next(), None, "{name}: seeking to the end must exhaust playback");
            decoder.try_seek(Duration::from_millis(500)).unwrap();
            assert!(decoder.next().is_some(), "{name}: seeking back must restore playback");
        }
    }

    struct Alternating;
    impl AudioSource for Alternating {
        fn duration(&self) -> u128 { 1000 }
        fn get_source(&self) -> Result<Box<dyn Source<Item = i16> + Send>, Error> {
            Ok(Box::new(rodio::buffer::SamplesBuffer::new(1, 8000,
                (0..8000).map(|index| if index % 2 == 0 { 8192i16 } else { -8192i16 }).collect::<Vec<_>>())))
        }
    }

    #[test]
    fn waveform_uses_energy_without_phase_cancellation_and_obeys_cancellation() {
        let peaks = Alternating.waveform(4, &|| false).unwrap();
        assert_eq!(peaks.len(), 4);
        assert!(peaks.iter().all(|value| (*value - 0.25).abs() < 0.0001));
        assert!(Alternating.waveform(180, &|| true).is_err());
        assert!(Alternating.waveform(0, &|| false).is_err());
    }
}
