use std::sync::{Arc, Mutex};
use std::error::Error;
use std::time::Duration;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use chrono::NaiveDate;
use scraper::{Html, Selector};
use serde::de::DeserializeOwned;
use serde::{Serialize, Deserialize};
use onetagger_tag::FrameName;
use onetagger_tagger::{Track, TaggerConfig, AutotaggerSource, AudioFileInfo, MatchingUtils, TrackNumber, AutotaggerSourceBuilder, PlatformInfo, PlatformCustomOptions, PlatformCustomOptionValue, supported_tags, SupportedTag};
use serde_json::Value;

const INVALID_ART: &'static str = "ab2d1d04-233d-4b08-8234-9782b34dcab8";


pub struct Beatport {
    client: Client,
    access_token: Arc<Mutex<Option<BeatportOAuth>>>
}

impl Beatport {
    /// Create new instance
    pub fn new(access_token: Arc<Mutex<Option<BeatportOAuth>>>) -> Beatport {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:85.0) Gecko/20100101 Firefox/85.0")
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();
        Beatport {
            client, access_token
        }
    }

    /// Search for tracks on beatport
    pub fn search(&self, query: &str, page: i32, results_per_page: usize) -> Result<BeatportTrackResults, Box<dyn Error>> {
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
    fn get_next_data<T: DeserializeOwned>(&self, response: &str) -> Result<T, Box<dyn Error>> {
        let document = Html::parse_document(&response);
        let selector = Selector::parse("script#__NEXT_DATA__").unwrap();
        let script = document.select(&selector).next().ok_or("Missing __NEXT_DATA__")?.text().collect::<String>();
        let value: Value = serde_json::from_str(&script)?;
        Ok(serde_json::from_value(value["props"]["pageProps"]["dehydratedState"]["queries"][0]["state"]["data"].to_owned())?)
    }

    /// Update embed auth token
    pub fn update_token(&self) -> Result<String, Box<dyn Error>> {
        let mut token = self.access_token.lock().unwrap();
        // Fetch new if doesn't exist
        if (*token).is_none() {
            let mut response: BeatportOAuth = self.client.get("https://embed.beatport.com/token")
                .send()?.json()?;
            response.expires_in = response.expires_in * 1000 + timestamp!() - 60000;
            *token = Some(response);
            debug!("OAuth: {:?}", token);
        }
        // Expired
        let t = token.clone().unwrap();
        if t.expires_in <= timestamp!() {
            *token = None;
            return self.update_token();
        }
        Ok(t.access_token)
    }

    /// Fetch track using API
    pub fn track(&self, id: i64) -> Result<Option<BeatportTrack>, Box<dyn Error>> {
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
    pub fn release(&self, id: i64) -> Result<BeatportRelease, Box<dyn Error>> {
        let token = self.update_token()?;
        let response = self.client.get(&format!("https://api.beatport.com/v4/catalog/releases/{}", id))
            .bearer_auth(token)
            .send()?.json()?;
        Ok(response)
    }

    /// Extend the track with release info
    pub fn extend_track(&self, track: &mut Track) -> Result<(), Box<dyn Error>> {
        debug!("Extending track...");
        let release = self.release(track.release_id.parse()?)?;
        track.track_total = release.track_count;
        track.album_artists = match release.artists {
            Some(a) => a.into_iter().map(|a| a.name).collect(),
            None => vec![],
        };
        Ok(())
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


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportOAuth {
    pub access_token: String,
    pub expires_in: u128
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

impl BeatportTrackResult {
    pub fn to_track(self, include_version: bool) -> Track {
        Track {
            platform: "beatport".to_string(),
            title: self.track_name,
            track_id: Some(self.track_id.to_string()),
            artists: self.artists.unwrap_or(vec![]).into_iter().map(|a| a.artist_name).collect(),
            version: match include_version {
                true => self.mix_name,
                false => None
            },
            duration: Duration::from_millis(self.length.unwrap_or(0)),
            isrc: self.isrc,
            ..Default::default()
        }
    }
}

impl BeatportTrack {
    pub fn to_track(self, art_resolution: u32) -> Track {
        let art = self.get_art(art_resolution);

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
            release_id: self.release.id.to_string(),
            duration: Duration::from_millis(self.length_ms.unwrap_or(0)),
            remixers: self.remixers.into_iter().map(|r| r.name).collect(),
            track_number: self.number.map(|n| TrackNumber::Number(n as i32)),
            isrc: self.isrc,
            release_year: self.new_release_date.as_ref().map(|d| d.chars().take(4).collect::<String>().parse().ok()).flatten(),
            publish_year: self.publish_date.as_ref().map(|d| d.chars().take(4).collect::<String>().parse().ok()).flatten(),
            release_date: self.new_release_date.as_ref().map_or(None, |d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
            publish_date: self.publish_date.as_ref().map_or(None, |d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
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
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>> {       
        // Load custom config
        let custom_config: BeatportConfig = config.get_custom("beatport")?;

        // Fetch by ID
        if let Some(id) = info.tags.get("BEATPORT_TRACK_ID").map(|t| t.first().map(|id| id.trim().replace("\0", "").parse().ok()).flatten()).flatten() {
            info!("Fetching by ID: {}", id);
            match self.track(id) {
                Ok(Some(api_track)) => {
                    let mut track = api_track.to_track(custom_config.art_resolution);
                    // Extend
                    if config.tag_enabled(SupportedTag::AlbumArtist) || config.tag_enabled(SupportedTag::TrackTotal) {
                        if let Err(e) = self.extend_track(&mut track) {
                            warn!("Failed extending track: {e}");
                        }
                    }
                    return Ok(Some((1.0, track)));
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
                                let mut track = track.to_track(custom_config.art_resolution);
                                info!("Matched track by ISRC");
                                // Extend
                                if config.tag_enabled(SupportedTag::AlbumArtist) || config.tag_enabled(SupportedTag::TrackTotal) {
                                    if let Err(e) = self.extend_track(&mut track) {
                                        warn!("Failed extending track: {e}");
                                    }
                                }
                                return Ok(Some((1.0, track)));
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
            match self.search(&query, page, 25) {
                Ok(res) => {
                    // Match
                    let mut tracks = res.data.into_iter().map(|t| t.to_track(!custom_config.ignore_version)).collect();

                    // Fallback with restricted
                    let (acc, mut track) = loop {
                        match MatchingUtils::match_track(&info, &tracks, &config, true) {
                            Some((acc, track)) => {
                                let id = track.track_id.as_ref().unwrap().parse().unwrap();
                                match self.track(id)? {
                                    Some(track) => break (acc, track.to_track(custom_config.art_resolution)),
                                    None => {
                                        // Track restricted
                                        warn!("Got restricted track, fallback");
                                        tracks = tracks.into_iter().filter(|t| t.track_id != track.track_id).collect();
                                        continue;
                                    },
                                };
                            },
                            None => return Ok(None),
                        }
                    };

                    // Extend
                    if config.tag_enabled(SupportedTag::AlbumArtist) || config.tag_enabled(SupportedTag::TrackTotal) {
                        if let Err(e) = self.extend_track(&mut track) {
                            warn!("Failed extending track: {e}");
                        }
                    }
                    return Ok(Some((acc, track)));
                },
                Err(e) => {
                    warn!("Beatport search failed, query: {}. {}", query, e);
                    return Ok(None);
                }
            }
        }
        Ok(None)
    }
}

/// For creating Beatport instances
pub struct BeatportBuilder {
    access_token: Arc<Mutex<Option<BeatportOAuth>>>
}

impl AutotaggerSourceBuilder for BeatportBuilder {
    fn new() -> BeatportBuilder {
        BeatportBuilder {
            access_token: Arc::new(Mutex::new(None))
        }
    }

    fn get_source(&mut self, _config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Box<dyn Error>> {
        Ok(Box::new(Beatport::new(self.access_token.clone())))
    }

    fn info(&self) -> PlatformInfo {
        PlatformInfo {
            id: "beatport".to_string(),
            name: "Beatport".to_string(),
            description: "Overall more specialized in Techno".to_string(),
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