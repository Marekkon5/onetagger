use anyhow::Error;
use std::path::Path;
use std::sync::LazyLock;
use std::thread::Builder;
use onetagger_player::rodio::source::UniformSourceIterator;
use serde::{Serialize, Deserialize};
use songrec::{DecodedSignature, SignatureGenerator};
use onetagger_player::AudioSources;
use onetagger_platforms::RequestCache;

static RECOGNITIONS: LazyLock<RequestCache<Vec<u8>, ShazamTrack>> = LazyLock::new(RequestCache::default);

pub struct Shazam;

impl Shazam {
    /// Recognize song on Shazam from path, returns Track, Duration
    pub fn recognize_from_file(path: impl AsRef<Path>) -> Result<(ShazamTrack, u128), Error> {
        // Load file
        let source = AudioSources::from_path(path)?;
        let duration = source.duration();
        let conv = UniformSourceIterator::new(source.get_source()?, 1, 16000);
        // Get 12s part from middle
        let buffer = if duration >= 12000 {
            // ((duration / 1000) * 16KHz) / 2 (half duration) - (6 * 16KHz) seconds.
            conv.skip((duration * 8 - 96000) as usize).take(16000 * 12).collect::<Vec<i16>>()
        } else {
            conv.collect::<Vec<i16>>()
        };
        // Calculating singnature requires 6MB stack, because it allocates >2MB of buffers for some reason
        let signature = Builder::new()
            .stack_size(1024 * 1024 * 6)
            .spawn(move || { SignatureGenerator::make_signature_from_buffer(&buffer) })
            .unwrap()
            .join()
            .unwrap();
        let track = Self::recognize_signature(&RECOGNITIONS, &signature, || {
            songrec::recognize_song_from_signature(&signature, 0).map_err(|e| anyhow!("{e:?}"))
        })?;
        Ok((track, duration))
    }

    fn recognize_signature(
        cache: &RequestCache<Vec<u8>, ShazamTrack>,
        signature: &DecodedSignature,
        lookup: impl FnOnce() -> Result<serde_json::Value, Error>,
    ) -> Result<ShazamTrack, Error> {
        // Different local paths and tag rewrites can produce the same recognition request.
        let key = signature.encode_to_binary().map_err(|e| anyhow!("{e}"))?;
        cache.get_or_try_init(key, || {
            let response: ShazamResponse = serde_json::from_value(lookup()?)?;
            response.track.ok_or_else(|| anyhow!("Shazam returned no matches!"))
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShazamResponse {
    pub timestamp: u64,
    pub tagid: String,
    pub track: Option<ShazamTrack>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShazamTrack {
    pub albumadamid: Option<String>,
    pub artists: Option<Vec<ShazamSmall>>,
    pub genres: Option<ShazamGenres>,
    pub images: Option<ShazamImages>,
    pub isrc: Option<String>,
    pub key: String,
    pub sections: Vec<ShazamSection>,
    /// Song title
    pub title: String,
    /// Artist
    pub subtitle: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShazamSmall {
    pub adamid: String,
    pub id: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShazamGenres {
    pub primary: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShazamImages {
    pub background: String,
    pub coverart: String,
    pub coverarthq: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ShazamSection {
    MetaSection {
        metadata: Vec<ShazamMetadataSection>
    },
    ArtistSection {
        id: String,
        name: String,
        tabname: String,
        // Has to == "ARTIST"
        #[serde(rename = "type")]
        _type: String
    },
    Other {}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShazamMetadataSection {
    pub text: String,
    pub title: String
}

#[cfg(test)]
mod tests {
    use super::*;
    use songrec::{FrequencyBand, FrequencyPeak};
    use std::sync::{Barrier, atomic::{AtomicUsize, Ordering}};

    fn signature(bin: u16) -> DecodedSignature {
        let peak = FrequencyPeak {
            fft_pass_number: 1, peak_magnitude: 32, corrected_peak_frequency_bin: bin, sample_rate_hz: 16000,
        };
        DecodedSignature {
            sample_rate_hz: 16000, number_samples: 192000,
            frequency_band_to_sound_peaks: [(FrequencyBand::_250_520, vec![peak])].into(),
        }
    }

    fn matched(title: &str) -> serde_json::Value {
        serde_json::json!({
            "tagid": "test", "timestamp": 123,
            "track": { "title": title, "subtitle": "Artist", "key": "1", "sections": [], "url": "" }
        })
    }

    #[test]
    fn simultaneous_identical_signatures_share_lookup_and_distinct_audio_gets_its_own_result() {
        let cache = RequestCache::default();
        let starts = Barrier::new(8);
        let calls = AtomicUsize::new(0);
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    starts.wait();
                    let track = Shazam::recognize_signature(&cache, &signature(1200), || {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Ok(matched("First"))
                    }).unwrap();
                    assert_eq!(track.title, "First");
                });
            }
        });
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let other = Shazam::recognize_signature(&cache, &signature(1250), || Ok(matched("Second"))).unwrap();
        assert_eq!(other.title, "Second");
    }

    #[test]
    fn unsuccessful_recognition_can_be_retried() {
        let cache = RequestCache::default();
        let signature = signature(1250);
        assert!(Shazam::recognize_signature(&cache, &signature, || Err(anyhow!("offline"))).is_err());
        let miss = serde_json::json!({"tagid": "test", "timestamp": 123, "track": null});
        assert!(Shazam::recognize_signature(&cache, &signature, || Ok(miss)).is_err());
        let next = Shazam::recognize_signature(&cache, &signature, || Ok(matched("Recovered"))).unwrap();
        assert_eq!(next.title, "Recovered");
    }
}
