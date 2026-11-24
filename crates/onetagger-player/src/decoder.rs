//! File-backed Symphonia source: known lengths, reusable PCM buffers and accurate seeks.
use std::{fs::File, path::Path, time::Duration};
use anyhow::Error;
use rodio::{Source, source::SeekError};
use symphonia::core::{audio::SampleBuffer, codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL},
    errors::Error as DecodeError, formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
    io::MediaSourceStream, meta::MetadataOptions, probe::Hint, units::{Time, TimeBase}};

pub(crate) struct FileDecoder {
    format: Box<dyn FormatReader>, decoder: Box<dyn Decoder>, track: u32,
    time_base: Option<TimeBase>, buffer: Option<SampleBuffer<i16>>, offset: usize,
    rate: u32, channels: u16, duration: Duration, ended: bool,
}

impl FileDecoder {
    pub(crate) fn open(path: &Path, milliseconds: u128) -> Result<Self, Error> {
        // File implements MediaSource with its real length. Wrapping a generic
        // Read+Seek in Rodio 0.20 loses that length and disables FLAC seeking.
        let input = MediaSourceStream::new(Box::new(File::open(path)?), Default::default());
        let mut hint = Hint::new();
        if let Some(extension) = path.extension().and_then(|extension| extension.to_str()) { hint.with_extension(extension); }
        let options = FormatOptions { enable_gapless: true, ..Default::default() };
        let probed = symphonia::default::get_probe().format(&hint, input, &options, &MetadataOptions::default())
            .map_err(|error| anyhow!(error.to_string()))?;
        let selected = probed.format.tracks().iter().filter(|track| track.codec_params.codec != CODEC_TYPE_NULL)
            .find_map(|track| symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())
                .ok().map(|decoder| (track.id, track.codec_params.time_base, decoder)))
            .ok_or(anyhow!("No supported audio track"))?;
        let mut source = Self { format: probed.format, decoder: selected.2, track: selected.0, time_base: selected.1,
            buffer: None, offset: 0, rate: 0, channels: 0,
            duration: Duration::from_millis(milliseconds.min(u64::MAX as u128) as u64), ended: false };
        source.fill().map_err(|error| anyhow!(error.to_string()))?.ok_or(anyhow!("Audio track has no samples"))?;
        Ok(source)
    }

    /// Decode the next packet for this audio track, reusing the sample buffer.
    fn fill(&mut self) -> Result<Option<u64>, DecodeError> {
        let mut failures = 0;
        loop {
            let packet = match self.format.next_packet() {
                Ok(packet) => packet,
                Err(DecodeError::IoError(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof => { self.ended = true; return Ok(None); },
                Err(error) => return Err(error),
            };
            if packet.track_id() != self.track { continue; }
            let decoded = match self.decoder.decode(&packet) {
                Ok(decoded) => decoded,
                Err(error @ DecodeError::DecodeError(_)) if failures < 3 => { failures += 1; debug!("Skipping damaged audio packet: {error}"); continue; },
                Err(error) => return Err(error),
            };
            let rate = decoded.spec().rate;
            let channels = decoded.spec().channels.count() as u16;
            if rate == 0 || channels == 0 { return Err(DecodeError::DecodeError("Invalid audio signal specification")); }
            if self.buffer.as_ref().is_none_or(|buffer| buffer.capacity() < decoded.capacity() * channels as usize)
                || self.rate != rate || self.channels != channels {
                self.buffer = Some(SampleBuffer::new(decoded.capacity() as u64, *decoded.spec()));
            }
            self.rate = rate; self.channels = channels;
            let buffer = self.buffer.as_mut().unwrap();
            buffer.copy_interleaved_ref(decoded);
            self.offset = 0;
            if !buffer.samples().is_empty() { return Ok(Some(packet.ts())); }
        }
    }

    fn seek(&mut self, position: Duration) -> Result<(), DecodeError> {
        if position >= self.duration { self.ended = true; return Ok(()); }
        let channel = self.offset % self.channels as usize;
        let target = self.format.seek(SeekMode::Accurate,
            SeekTo::Time { time: Time::from(position.as_secs_f64()), track_id: Some(self.track) })?;
        self.decoder.reset(); self.ended = false;
        while let Some(timestamp) = self.fill()? {
            let delta = target.required_ts.saturating_sub(timestamp);
            let frames = self.time_base.map_or(delta, |base| {
                let time = base.calc_time(delta);
                ((time.seconds as f64 + time.frac) * self.rate as f64).round() as u64
            });
            let skip = frames.saturating_mul(self.channels as u64).saturating_add(channel as u64);
            if skip < self.buffer.as_ref().unwrap().samples().len() as u64 { self.offset = skip as usize; return Ok(()); }
        }
        Err(DecodeError::DecodeError("No samples after seek"))
    }
}

impl Iterator for FileDecoder {
    type Item = i16;
    fn next(&mut self) -> Option<i16> {
        if self.ended { return None; }
        if self.offset >= self.buffer.as_ref()?.samples().len() {
            match self.fill() {
                Ok(Some(_)) => {}, Ok(None) => return None,
                Err(error) => { warn!("Audio decoding stopped: {error}"); self.ended = true; return None; },
            }
        }
        let sample = *self.buffer.as_ref()?.samples().get(self.offset)?;
        self.offset += 1; Some(sample)
    }
}
impl Source for FileDecoder {
    fn current_frame_len(&self) -> Option<usize> { if self.ended { Some(0) } else { None } }
    fn channels(&self) -> u16 { self.channels }
    fn sample_rate(&self) -> u32 { self.rate }
    fn total_duration(&self) -> Option<Duration> { Some(self.duration) }
    fn try_seek(&mut self, position: Duration) -> Result<(), SeekError> {
        self.seek(position).map_err(|error| SeekError::Other(Box::new(std::io::Error::other(error.to_string()))))
    }
}
