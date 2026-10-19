use anyhow::Error;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread::Builder;
use std::time::{Duration, Instant, SystemTime};
use onetagger_player::rodio::source::UniformSourceIterator;
use reqwest::blocking::Client;
use serde::{Serialize, Deserialize};
use serde_json::json;
use songrec::{DecodedSignature, SignatureGenerator};
use onetagger_player::AudioSources;
use uuid::Uuid;

/// Minimum delay between two requests to Shazam (shared by all threads).
/// Shazam returns 429 after ~20 back to back requests, ~3s between requests is sustainable.
const REQUEST_INTERVAL: Duration = Duration::from_millis(3200);
/// Initial cooldown after hitting rate limit, doubles on every consecutive 429
const RATE_LIMIT_COOLDOWN: Duration = Duration::from_secs(20);
const RATE_LIMIT_MAX_COOLDOWN: Duration = Duration::from_secs(300);
const MAX_RETRIES: u32 = 10;

lazy_static::lazy_static! {
    static ref CLIENT: Client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    /// Earliest time the next request may be sent
    static ref NEXT_REQUEST: Mutex<Instant> = Mutex::new(Instant::now());
    /// Cached results, so every platform doesn't recognize the same file again
    static ref CACHE: Mutex<HashMap<PathBuf, Option<(ShazamTrack, u128)>>> = Mutex::new(HashMap::new());
}

pub struct Shazam;

impl Shazam {
    /// Recognize song on Shazam from path, returns Track, Duration
    pub fn recognize_from_file(path: impl AsRef<Path>) -> Result<(ShazamTrack, u128), Error> {
        let path = path.as_ref();
        if let Some(cached) = CACHE.lock().unwrap().get(path) {
            debug!("Using cached Shazam result for: {path:?}");
            return cached.clone().ok_or(anyhow!("Shazam returned no matches!"));
        }

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
        let response = Self::recognize_signature(&signature)?;
        let result = response.track.map(|t| (t, duration));
        CACHE.lock().unwrap().insert(path.to_owned(), result.clone());
        result.ok_or(anyhow!("Shazam returned no matches!"))
    }

    /// Wait for a free request slot (global across threads), aborts when tagging is stopped
    fn wait_for_slot() -> Result<(), Error> {
        let slot = {
            let mut next = NEXT_REQUEST.lock().unwrap();
            let slot = (*next).max(Instant::now());
            *next = slot + REQUEST_INTERVAL;
            slot
        };
        while Instant::now() < slot {
            if crate::STOP_TAGGING.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(anyhow!("Stopped"));
            }
            std::thread::sleep((slot - Instant::now()).min(Duration::from_millis(250)));
        }
        Ok(())
    }

    /// Delay all future requests (rate limit hit)
    fn cooldown(duration: Duration) {
        let mut next = NEXT_REQUEST.lock().unwrap();
        *next = (*next).max(Instant::now() + duration);
    }

    /// Send signature to Shazam, handles rate limiting
    fn recognize_signature(signature: &DecodedSignature) -> Result<ShazamResponse, Error> {
        let uri = signature.encode_to_uri().map_err(|e| anyhow!("Failed encoding signature: {e}"))?;
        let samplems = (signature.number_samples as f32 / signature.sample_rate_hz as f32 * 1000.) as u32;
        let mut cooldown = RATE_LIMIT_COOLDOWN;

        for attempt in 0..=MAX_RETRIES {
            Self::wait_for_slot()?;
            let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_millis() as u32;
            let body = json!({
                "geolocation": { "altitude": 300, "latitude": 45, "longitude": 2 },
                "signature": { "samplems": samplems, "timestamp": timestamp, "uri": uri },
                "timestamp": timestamp,
                "timezone": "Europe/Paris"
            });
            let url = format!(
                "https://amp.shazam.com/discovery/v5/en/US/android/-/tag/{}/{}",
                Uuid::new_v4().hyphenated().to_string().to_uppercase(), Uuid::new_v4().hyphenated()
            );
            let response = CLIENT.post(&url)
                .query(&[
                    ("sync", "true"),
                    ("webv3", "true"),
                    ("sampling", "true"),
                    ("connected", ""),
                    ("shazamapiversion", "v3"),
                    ("sharehub", "true"),
                    ("video", "v3")
                ])
                .header("User-Agent", "Dalvik/2.1.0 (Linux; U; Android 13; Pixel 6 Build/TQ3A.230901.001)")
                .header("Content-Language", "en_US")
                .json(&body)
                .send();
            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    warn!("Shazam request failed (attempt {}): {e}", attempt + 1);
                    Self::cooldown(Duration::from_secs(5));
                    continue;
                }
            };

            let status = response.status();
            // Rate limited, all threads wait
            if status.as_u16() == 429 || status.is_server_error() {
                warn!("Shazam rate limit ({status}), pausing for {}s...", cooldown.as_secs());
                Self::cooldown(cooldown);
                cooldown = (cooldown * 2).min(RATE_LIMIT_MAX_COOLDOWN);
                continue;
            }

            let text = response.text()?;
            if !status.is_success() {
                return Err(anyhow!("Shazam returned {status}: {}", text.chars().take(200).collect::<String>()));
            }
            return serde_json::from_str(&text)
                .map_err(|e| anyhow!("Invalid Shazam response ({e}): {}", text.chars().take(200).collect::<String>()));
        }
        Err(anyhow!("Shazam rate limit, giving up after {MAX_RETRIES} retries"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShazamResponse {
    /// Missing when there are no matches
    pub timestamp: Option<u64>,
    pub tagid: Option<String>,
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