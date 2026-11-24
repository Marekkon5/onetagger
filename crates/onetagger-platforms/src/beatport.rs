use std::sync::{Arc, Mutex};
use anyhow::Error;
use std::time::Duration;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use chrono::NaiveDate;
use scraper::{Html, Selector};
use serde::de::DeserializeOwned;
use serde::{Serialize, Deserialize};
use onetagger_tag::FrameName;
use onetagger_tagger::{supported_tags, Album, AudioFileInfo, AutotaggerSource, AutotaggerSourceBuilder, MatchingUtils, PlatformCustomOptionValue, PlatformCustomOptions, PlatformInfo, SupportedTag, TaggerConfig, Track, TrackMatch, TrackNumber};
use serde_json::{json, Value};
use crate::request_cache::RequestCache;

const INVALID_ART: &'static str = "ab2d1d04-233d-4b08-8234-9782b34dcab8";


pub struct Beatport {
    client: Client,
    access_token: Arc<Mutex<Option<BeatportOAuth>>>,
    release_cache: RequestCache<i64, BeatportRelease>,
}

impl Beatport {
    /// Create new instance
    pub fn new(access_token: Arc<Mutex<Option<BeatportOAuth>>>) -> Beatport {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36")
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();
        Beatport {
            client, access_token, release_cache: RequestCache::default(),
        }
    }

    /// Search for tracks on beatport
    pub fn search(&self, query: &str, page: i32, results_per_page: usize) -> Result<BeatportTrackResults, Error> {
        let query = Self::clear_search_query(query);
        let response = self.client.get("https://www.beatport.com/search/tracks")
            .query(&[
                ("q", &query), 
                ("page", &page.to_string()),
                ("per-page", &results_per_page.to_string())
            ])
            .send()?
            .text()?;
        
        // Parse JSON
        let results: BeatportTrackResults = self.get_next_data(&response)?;
        Ok(results)
    }
    
    /// Extract next hydratation data from html
    fn get_next_data<T: DeserializeOwned>(&self, response: &str) -> Result<T, Error> {
        let document = Html::parse_document(&response);
        let selector = Selector::parse("script#__NEXT_DATA__").unwrap();
        let script = document.select(&selector).next().ok_or(anyhow!("Missing __NEXT_DATA__"))?.text().collect::<String>();
        let value: Value = serde_json::from_str(&script)?;
        Ok(serde_json::from_value(value["props"]["pageProps"]["dehydratedState"]["queries"][0]["state"]["data"].to_owned())?)
    }

    /// Return the cached token, refreshing it once when missing or expired.
    pub fn update_token(&self) -> Result<String, Error> {
        self.update_token_with(|| self.fetch_token(), || timestamp!())
    }

    fn fetch_token(&self) -> Result<BeatportOAuth, Error> {
        // Taken from https://embed.beatport.com/?id=4418593&type=release
        let response: BeatportOAuth = self.client.post("https://account.beatport.com/o/token/")
            .form(&json!({
                "client_id": "2tiTbKxmQFwnbFjMONU4k7njMRZmV3ZMwRBndiZs",
                "client_secret": "RDUJyAk4zFEGtQ8rsTmylDSfxmALRNBn3D1BsRr7MKi3oa1TL9Mq9QxqUPK7loiumXolEWbJcWa4IGAhtwnTz1cSXClGJ1tkkNCNWwRwjxIKTZJKOJxbwaNt0Rm3WG0v",
                "grant_type": "client_credentials"
            }))
            .send()?
            .error_for_status()?
            .json()?;
        Ok(response)
    }

    fn update_token_with<F, N>(&self, fetch: F, now: N) -> Result<String, Error>
    where
        F: FnOnce() -> Result<BeatportOAuth, Error>,
        N: Fn() -> u128,
    {
        let mut token = self.access_token.lock()
            .map_err(|_| anyhow!("Beatport token cache lock poisoned"))?;
        let started = now();
        if let Some(cached) = token.as_ref() {
            if !cached.access_token.is_empty() && cached.expires_in > started {
                return Ok(cached.access_token.clone());
            }
        }

        // Keep the shared lock during the bounded HTTP request so other workers
        // reuse this refresh. Never recurse or acquire this mutex a second time.
        // A failed refresh must not leave an expired token available to callers.
        *token = None;
        let mut response = fetch()?;
        let lifetime = response.expires_in.checked_mul(1000)
            .filter(|lifetime| *lifetime > 0)
            .ok_or_else(|| anyhow!("Invalid Beatport token lifetime"))?;
        // Retain the usual ten-second margin, allowing short-lived tokens too.
        let margin = 10_000.min(lifetime / 2);
        let expires = started.checked_add(lifetime - margin)
            .ok_or_else(|| anyhow!("Invalid Beatport token lifetime"))?;
        if response.access_token.is_empty() || expires <= now() {
            return Err(anyhow!("Beatport returned an empty or expired token"));
        }
        response.expires_in = expires;
        let access_token = response.access_token.clone();
        *token = Some(response);
        debug!("Beatport OAuth token refreshed");
        Ok(access_token)
    }

    /// Fetch track using API
    pub fn track(&self, id: i64) -> Result<Option<BeatportTrack>, Error> {
        let token = self.update_token()?;
        let response = self.client.get(&format!("https://api.beatport.com/v4/catalog/tracks/{}", id))
            .bearer_auth(token)
            .send()?;

        // Restricted / deleted track
        if response.status() == StatusCode::FORBIDDEN {
            return Ok(None);
        }

        Ok(response.json()?)
    }

    /// Fetch release using API
    pub fn release(&self, id: i64) -> Result<BeatportRelease, Error> {
        self.release_with(id, || {
            let token = self.update_token()?;
            let response = self.client.get(&format!("https://api.beatport.com/v4/catalog/releases/{}", id))
                .bearer_auth(token)
                .send()?.error_for_status()?.json()?;
            Ok(response)
        })
    }

    fn release_with(&self, id: i64, fetch: impl FnOnce() -> Result<BeatportRelease, Error>) -> Result<BeatportRelease, Error> {
        self.release_cache.get_or_try_init(id, || {
            let release = fetch()?;
            anyhow::ensure!(release.id == id, "Beatport returned a different release");
            Ok(release)
        })
    }

    /// Get tracks from release
    pub fn release_tracks(&self, id: i64) -> Result<Vec<BeatportTrack>, Error> {
        let token = self.update_token()?;
        let response: BeatportPagination<BeatportTrack> = self.client.get(&format!("https://api.beatport.com/v4/catalog/releases/{}/tracks?per_page=200", id))
            .bearer_auth(token)
            .send()?.json()?;
        Ok(response.results)
    }


    /// Beatport returns 403 if you have more than single () pair
    pub fn clear_search_query(query: &str) -> String {
        let mut open = 0;
        let mut closed = 0;

        query.chars().filter(|c| {
            match c {
                '(' if open > 0 => false,
                '(' => { open += 1; true },
                ')' if closed > 0 => false,
                ')' => { closed += 1; true },
                _ => true
            }
        }).collect()
    }
}


#[derive(Clone, Serialize, Deserialize)]
pub struct BeatportOAuth {
    pub access_token: String,
    pub expires_in: u128
}

impl std::fmt::Debug for BeatportOAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BeatportOAuth")
            .field("access_token", &"[redacted]")
            .field("expires_in", &self.expires_in)
            .finish()
    }
}

/// When searching for tracks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportTrackResults {
    pub data: Vec<BeatportTrackResult>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportTrackResult {
    pub track_id: i64,
    pub track_name: String,
    pub artists: Option<Vec<BeatportArtist>>,
    pub isrc: Option<String>,
    pub length: Option<u64>,
    pub mix_name: Option<String>,
    pub release: Option<BeatportTrackResultRelease>,
    pub genre: Option<Vec<BeatportTrackResultsGenre>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportTrackResultRelease {
    pub release_id: i64,
    pub release_image_uri: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportTrackResultsGenre {
    pub genre_name: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportArtist {
    pub artist_id: i64,
    pub artist_name: String,
    pub artist_type_name: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportTrack {
    pub artists: Vec<BeatportGeneric>,
    pub bpm: Option<i64>,
    pub catalog_number: Option<String>,
    pub exclusive: bool,
    pub genre: BeatportGeneric,
    pub id: i64,
    pub image: Option<BeatportImage>,
    pub isrc: Option<String>,
    pub key: Option<BeatportGeneric>,
    pub length_ms: Option<u64>,
    pub mix_name: String,
    pub name: String,
    pub number: Option<i64>,
    pub publish_date: Option<String>,
    pub release: BeatportRelease,
    pub remixers: Vec<BeatportGeneric>,
    pub slug: String,
    pub sub_genre: Option<BeatportGeneric>,
    pub new_release_date: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportGeneric {
    pub id: i64,
    pub name: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportImage {
    pub id: i64,
    pub dynamic_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportRelease {
    pub id: i64,
    pub name: String,
    pub label: BeatportGeneric,
    pub image: BeatportImage,
    pub upc: Option<String>,
    pub track_count: Option<u16>,
    pub artists: Option<Vec<BeatportGeneric>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BeatportPagination<T> {
    pub results: Vec<T>
}

impl BeatportTrackResult {
    pub fn to_track(self, include_version: bool) -> Track {
        Track {
            platform: "beatport".to_string(),
            url: format!("https://www.beatport.com/track/{}/{}", self.track_name.to_lowercase().replace(" ", "-"), self.track_id),
            title: self.track_name,
            track_id: Some(self.track_id.to_string()),
            artists: self.artists.unwrap_or(vec![]).into_iter().map(|a| a.artist_name).collect(),
            version: match include_version {
                true => self.mix_name,
                false => None
            },
            duration: Duration::from_millis(self.length.unwrap_or(0)).into(),
            isrc: self.isrc,
            thumbnail: self.release.map(|r| r.release_image_uri).flatten(),
            genres: self.genre.map(|g| g.into_iter().map(|g| g.genre_name).collect()).unwrap_or(vec![]),
            ..Default::default()
        }
    }
}

impl BeatportTrack {
    pub fn to_track(self, art_resolution: u32) -> Track {
        let art = self.get_art(art_resolution);
        let thumbnail = self.get_art(150);

        let mut track = Track {
            platform: "beatport".to_string(),
            title: self.name,
            version: Some(self.mix_name),
            artists: self.artists.into_iter().map(|a| a.name).collect(),
            album: Some(self.release.name),
            key: self.key.map(|k| k.name.replace(" Major", "").replace(" Minor", "m")),
            bpm: self.bpm,
            genres: vec![self.genre.name],
            styles: match self.sub_genre {
                Some(s) => vec![s.name],
                None => vec![],
            },
            art,
            url: format!("https://www.beatport.com/track/{}/{}", self.slug, self.id),
            label: Some(self.release.label.name),
            catalog_number: self.catalog_number,
            other: vec![
                (FrameName::same("UNIQUEFILEID"), vec![format!("https://beatport.com|{}", &self.id)])
            ],
            track_id: Some(self.id.to_string()),
            release_id: Some(self.release.id.to_string()),
            duration: Duration::from_millis(self.length_ms.unwrap_or(0)).into(),
            remixers: self.remixers.into_iter().map(|r| r.name).collect(),
            track_number: self.number.map(|n| TrackNumber::Number(n as i32)),
            isrc: self.isrc,
            release_year: self.new_release_date.as_ref().map(|d| d.chars().take(4).collect::<String>().parse().ok()).flatten(),
            publish_year: self.publish_date.as_ref().map(|d| d.chars().take(4).collect::<String>().parse().ok()).flatten(),
            release_date: self.new_release_date.as_ref().map_or(None, |d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
            publish_date: self.publish_date.as_ref().map_or(None, |d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
            thumbnail,
            ..Default::default()
        };

        // Exclusive
        if self.exclusive {
            track.other.push((FrameName::same("BEATPORT_EXCLUSIVE"), vec!["1".to_string()]));
        }

        track
    }

    /// Get album art URL
    pub fn get_art(&self, art_resolution: u32) -> Option<String> {
        if self.release.image.dynamic_uri.contains(&INVALID_ART) {
            return None;
        }
        let r = art_resolution.to_string();
        Some(self.release.image.dynamic_uri.replace("{w}", &r).replace("{h}", &r).replace("{x}", &r).replace("{y}", &r))
    }
}

// Match track
impl AutotaggerSource for Beatport {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Vec<TrackMatch>, Error> {       
        // Load custom config
        let custom_config: BeatportConfig = config.get_custom("beatport")?;
        let mut output = vec![];

        // Fetch by ID
        if let Some(id) = info.tags.get("BEATPORT_TRACK_ID").map(|t| t.first().map(|id| id.trim().replace("\0", "").parse().ok()).flatten()).flatten() {
            info!("Fetching by ID: {}", id);
            match self.track(id) {
                Ok(Some(api_track)) => {
                    let track = TrackMatch::new_id(api_track.to_track(custom_config.art_resolution));
                    if !config.fetch_all_results {
                        return Ok(vec![track]);   
                    }
                    output.push(track);
                },
                Ok(None) => warn!("Matching by ID failed, track restricted, matching normally"),
                Err(e) => {
                    warn!("Matching by ID failed, matching normally: {e}");
                }
            }
        }

        // Fetch by ISRC
        if let Some(isrc) = info.isrc.as_ref() {
            match self.search(isrc, 1, 25) {
                Ok(results) => {
                    if !results.data.is_empty() {
                        let track = self.track(results.data[0].track_id)?;
                        match track {
                            Some(track) => {
                                let track = TrackMatch::new_isrc(track.to_track(custom_config.art_resolution));
                                if !config.fetch_all_results {
                                    return Ok(vec![track]);   
                                }
                                output.push(track);
                            },
                            None => warn!("Matching by ISRC failed, track restricted, trying normal."),
                        }
                    }
                },
                Err(e) => {
                    warn!("Failed fetching track by ISRC: {e}");
                },
            }
        }

        // Search
        let query = format!("{} {}", info.artist()?, MatchingUtils::clean_title(info.title()?));
        debug!("BP Query: {}", query);
        for page in 1..custom_config.max_pages+1 {
            match self.search(&query, page, 50) {
                Ok(res) => {
                    // Match
                    let tracks = res.data
                        .into_iter()
                        .map(|t| t.to_track(true))
                        .collect::<Vec<_>>();

                    // Ignore version match
                    let tracks = if custom_config.ignore_version {
                        let t = tracks.clone().into_iter().map(|mut t| { t.version = None; t }).collect();
                        // Copy back versions
                        MatchingUtils::match_track(info, &t, config, true)
                            .into_iter()
                            .map(|mut t| {
                                if let Some(ot) = tracks.iter().find(|ot| ot.url == t.track.url) {
                                    t.track.version = ot.version.to_owned();
                                }
                                t
                            })
                            .collect()
                    } else {
                        MatchingUtils::match_track(info, &tracks, config, true)
                    };

                    // Return
                    output.extend(tracks);
                    if config.fetch_all_results {
                        continue;
                    }
                    return Ok(output);
                },
                Err(e) => {
                    warn!("Beatport search failed, query: {}. {}", query, e);
                    return Ok(output);
                }
            }
        }
        Ok(output)
    }


    fn extend_track(&mut self, track: &mut Track, config: &TaggerConfig) -> Result<(), Error> {
        let custom_config: BeatportConfig = config.get_custom("beatport")?;

        // Extend search results track
        if track.other.is_empty() {
            let id = track.track_id.as_ref().unwrap().parse().unwrap();
            *track = self.track(id)?.ok_or(anyhow!("Restricted track"))?.to_track(custom_config.art_resolution);
        }

        // Ignore extending track
        if !config.tag_enabled(SupportedTag::AlbumArtist) && !config.tag_enabled(SupportedTag::TrackTotal) {
            return Ok(());
        }

        let release = self.release(track.release_id.as_ref().ok_or(anyhow!("Missing release_id"))?.parse()?)?;
        track.track_total = release.track_count;
        track.album_artists = match release.artists {
            Some(a) => a.into_iter().map(|a| a.name).collect(),
            None => vec![],
        };
        Ok(())
    }
    
    fn get_album(&mut self, id: &str, config: &TaggerConfig) -> Result<Option<Album>, Error> {
        let custom_config: BeatportConfig = config.get_custom("beatport")?; 
        let id: i64 = id.trim().parse()?;
        let release = self.release(id)?;
        let tracks = self.release_tracks(id)?;

        let album = Album {
            id: id.to_string(),
            name: release.name,
            tracks: tracks.into_iter().map(|t| t.to_track(custom_config.art_resolution)).collect()
        };

        Ok(Some(album))
    }

}

/// For creating Beatport instances
#[derive(Debug, Clone)]
pub struct BeatportBuilder {
    access_token: Arc<Mutex<Option<BeatportOAuth>>>,
    release_cache: RequestCache<i64, BeatportRelease>,
}

impl BeatportBuilder {
    fn source(&self) -> Beatport {
        let mut source = Beatport::new(self.access_token.clone());
        source.release_cache = self.release_cache.clone();
        source
    }
}

impl AutotaggerSourceBuilder for BeatportBuilder {
    fn new() -> BeatportBuilder {
        BeatportBuilder {
            access_token: Arc::new(Mutex::new(None)),
            release_cache: RequestCache::default(),
        }
    }

    fn get_source(&mut self, _config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Error> {
        Ok(Box::new(self.source()))
    }

    fn info(&self) -> PlatformInfo {
        PlatformInfo {
            id: "beatport".to_string(),
            name: "Beatport".to_string(),
            description: "Overall more specialized in Techno, can match using ISRC".to_string(),
            icon: include_bytes!("../assets/beatport.png"),
            max_threads: 0,
            version: "1.0.0".to_string(),
            requires_auth: false,
            supported_tags: supported_tags!(Title, Version, Artist, AlbumArtist, Album, BPM, Genre, Style, Label, URL, ReleaseDate, PublishDate, Key, AlbumArt, OtherTags, TrackId, ReleaseId, Duration, Remixer, CatalogNumber, TrackTotal, ISRC, TrackNumber),
            custom_options: PlatformCustomOptions::new()
                // Album art resolution
                .add("art_resolution", "Album art resolution", PlatformCustomOptionValue::Number {
                    min: 200, max: 1600, step: 100, value: 500 
                })
                // Max pages to search
                .add_tooltip("max_pages", "Max pages", "How many pages of search results to scan for tracks", PlatformCustomOptionValue::Number {
                    min: 1, max: 10, step: 1, value: 1
                })
                // Ignore version
                .add_tooltip("ignore_version", "Ignore version when matching", "Ignores (Extended Mix), (Original Mix) and such", PlatformCustomOptionValue::Boolean { 
                    value: false
                })
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
struct BeatportConfig {
    pub art_resolution: u32,
    pub max_pages: i32,
    pub ignore_version: bool
}

#[test]
#[ignore = "Live provider integration test; run explicitly with --ignored"]
fn test_album() {
    let mut builder = BeatportBuilder::new();
    let mut config = TaggerConfig::default();
    let custom_config = builder.info().custom_options.get_defaults();
    config.custom.0.insert("beatport".to_string(), custom_config);

    let mut bp = builder.get_source(&config).unwrap();
    bp.get_album("2174307", &config).unwrap();
}

#[cfg(test)]
mod token_tests {
    use super::*;
    use std::sync::{atomic::{AtomicUsize, Ordering}, mpsc, Barrier};

    const NOW: u128 = 1_000_000;
    const SECRET: &str = "beatport-secret-regression-sentinel";

    fn token(expires_in: u128) -> BeatportOAuth {
        BeatportOAuth { access_token: SECRET.to_string(), expires_in }
    }

    #[test]
    fn valid_token_is_reused_without_fetching() {
        let bp = Beatport::new(Arc::new(Mutex::new(Some(token(NOW + 1)))));
        assert_eq!(bp.update_token_with(|| panic!("unexpected refresh"), || NOW).unwrap(), SECRET);
    }

    #[test]
    fn expired_token_refresh_completes_and_is_cached() {
        // A recursive-lock regression must fail with a timeout, not hang CI.
        let (tx, rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let cache = Arc::new(Mutex::new(Some(token(NOW))));
            let bp = Beatport::new(cache.clone());
            let fresh = bp.update_token_with(|| Ok(token(60)), || NOW).unwrap();
            let cached = bp.update_token_with(|| panic!("duplicate refresh"), || NOW + 1).unwrap();
            tx.send((fresh, cached, cache.lock().unwrap().as_ref().unwrap().expires_in)).unwrap();
        });
        let (fresh, cached, expires) = rx.recv_timeout(Duration::from_secs(5))
            .expect("expired token refresh did not complete");
        worker.join().unwrap();
        assert_eq!(fresh, SECRET);
        assert_eq!(cached, SECRET);
        assert_eq!(expires, NOW + 50_000);
    }

    #[test]
    fn concurrent_workers_share_one_refresh() {
        const WORKERS: usize = 8;
        let cache = Arc::new(Mutex::new(Some(token(NOW))));
        let calls = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(WORKERS));
        let bp = Arc::new(Beatport::new(cache));
        let (tx, rx) = mpsc::channel();
        let mut workers = Vec::new();
        for _ in 0..WORKERS {
            let (bp, calls, barrier, tx) = (bp.clone(), calls.clone(), barrier.clone(), tx.clone());
            workers.push(std::thread::spawn(move || {
                barrier.wait();
                let result = bp.update_token_with(|| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(token(60))
                }, || NOW);
                tx.send(result).unwrap();
            }));
        }
        drop(tx);
        for _ in 0..WORKERS {
            assert_eq!(rx.recv_timeout(Duration::from_secs(5))
                .expect("concurrent token refresh did not complete").unwrap(), SECRET);
        }
        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn failed_refresh_clears_expired_cache_and_can_recover() {
        let cache = Arc::new(Mutex::new(Some(token(NOW - 1))));
        let bp = Beatport::new(cache.clone());
        assert!(bp.update_token_with(|| Err(anyhow!("provider unavailable")), || NOW).is_err());
        assert!(cache.lock().unwrap().is_none());
        assert_eq!(bp.update_token_with(|| Ok(token(60)), || NOW).unwrap(), SECRET);
    }

    #[test]
    fn invalid_responses_are_not_cached() {
        let cache = Arc::new(Mutex::new(None));
        let bp = Beatport::new(cache.clone());
        for response in [token(0), token(u128::MAX), BeatportOAuth {
            access_token: String::new(), expires_in: 60,
        }] {
            assert!(bp.update_token_with(|| Ok(response), || NOW).is_err());
            assert!(cache.lock().unwrap().is_none());
        }
        assert!(bp.update_token_with(|| Ok(token(60)), || u128::MAX - 1).is_err());
        assert!(cache.lock().unwrap().is_none());
    }

    #[test]
    fn short_lifetimes_work_and_slow_responses_are_rejected() {
        let cache = Arc::new(Mutex::new(None));
        let bp = Beatport::new(cache.clone());
        assert_eq!(bp.update_token_with(|| Ok(token(1)), || NOW).unwrap(), SECRET);
        assert_eq!(cache.lock().unwrap().as_ref().unwrap().expires_in, NOW + 500);
        *cache.lock().unwrap() = None;
        let ticks = std::cell::Cell::new(NOW);
        assert!(bp.update_token_with(|| {
            ticks.set(NOW + 60_000);
            Ok(token(60))
        }, || ticks.get()).is_err());
        assert!(cache.lock().unwrap().is_none());
    }

    thread_local! {
        static MESSAGES: std::cell::RefCell<Option<Vec<String>>> = const { std::cell::RefCell::new(None) };
    }

    struct TestLogger;
    impl log::Log for TestLogger {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool { true }
        fn log(&self, record: &log::Record<'_>) {
            MESSAGES.with(|messages| {
                if let Some(messages) = messages.borrow_mut().as_mut() {
                    messages.push(record.args().to_string());
                }
            });
        }
        fn flush(&self) {}
    }

    #[test]
    fn refresh_logs_and_nested_debug_redact_credentials() {
        static LOGGER: TestLogger = TestLogger;
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            log::set_logger(&LOGGER).unwrap();
            log::set_max_level(log::LevelFilter::Debug);
        });
        MESSAGES.with(|messages| *messages.borrow_mut() = Some(Vec::new()));
        let cache = Arc::new(Mutex::new(None));
        let bp = Beatport::new(cache.clone());
        assert_eq!(bp.update_token_with(|| Ok(token(60)), || NOW).unwrap(), SECRET);
        debug!("{:?}", BeatportBuilder { access_token: cache.clone(), release_cache: RequestCache::default() });
        let messages = MESSAGES.with(|messages| messages.borrow_mut().take().unwrap().join("\n"));
        assert!(messages.contains("Beatport OAuth token refreshed"));
        assert!(messages.contains("[redacted]"));
        assert!(!messages.contains(SECRET));
        // Redaction only affects diagnostics, not the token used by requests.
        let serialized = serde_json::to_value(cache.lock().unwrap().as_ref().unwrap()).unwrap();
        assert_eq!(serialized["access_token"], SECRET);
    }

    #[test]
    fn worker_sources_share_only_validated_successful_release_details() {
        use crate::ProviderRateLimited;
        use std::sync::atomic::{AtomicUsize, Ordering};
        let builder = BeatportBuilder::new();
        let sources: Vec<_> = (0..16).map(|_| builder.source()).collect();
        let calls = AtomicUsize::new(0);
        let fixture = |id| BeatportRelease { id, name: "Edition A".into(),
            label: BeatportGeneric { id: 1, name: "Label".into() },
            image: BeatportImage { id: 2, dynamic_uri: "https://fixture/{w}/{h}".into() },
            upc: None, track_count: Some(10), artists: None };
        std::thread::scope(|scope| {
            for source in &sources {
                let fixture = &fixture; let calls = &calls;
                scope.spawn(move || {
                    let release = source.release_with(42, || {
                        calls.fetch_add(1, Ordering::SeqCst); Ok(fixture(42))
                    }).unwrap();
                    assert_eq!(release.id, 42); assert_eq!(release.track_count, Some(10));
                });
            }
        });
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(sources[0].release_with(7, || Ok(fixture(42))).is_err());
        let error = sources[0].release_with(7, || Err(ProviderRateLimited::retry_after(Duration::from_secs(9)).into())).unwrap_err();
        assert!(error.is::<ProviderRateLimited>());
        assert_eq!(sources[1].release_with(7, || Ok(fixture(7))).unwrap().id, 7);
    }
}
