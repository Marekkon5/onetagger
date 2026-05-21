use anyhow::{anyhow, Error};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use chrono::NaiveDate;
use reqwest::blocking::Client;
use serde::{Serialize, Deserialize};
use serde_json::json;

use onetagger_tagger::{AutotaggerSource, Track, TaggerConfig, AudioFileInfo, MatchingUtils, AutotaggerSourceBuilder, PlatformInfo, PlatformCustomOptions, PlatformCustomOptionValue, supported_tags, TrackMatch};

pub struct Beatsource {
    client: Client,
    token_manager: BeatsourceTokenManager
}

impl Beatsource {
    /// Create new instance
    pub fn new(token_manager: BeatsourceTokenManager) -> Beatsource {
        Beatsource {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36")
                .timeout(Duration::from_secs(60))
                .build()
                .unwrap(),
            token_manager
        }
    }

    /// Strip ALL parentheses to prevent Beatsource API 403/400 decoding errors
    pub fn clear_search_query(query: &str) -> String {
        query.replace("(", "").replace(")", "")
    }

    /// Search for tracks
    pub fn search(&self, query: &str) -> Result<BeatsourceSearchResponse, Error> {
        // Run the query through the cleaner before sending it to the API
        let safe_query = Self::clear_search_query(query);

        let res: BeatsourceSearchResponse = self.client.get("https://api.beatsource.com/v4/catalog/search")
            .query(&[
                ("per_page", "100"), // Fixed typo from pubper_page
                ("page", "1"),
                ("type", "tracks"),
                ("q", safe_query.as_str()) // Use the cleaned query here
            ])
            .bearer_auth(self.token_manager.token()?)
            .send()?
            .json()?;
        Ok(res)
    }
}

impl AutotaggerSource for Beatsource {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Vec<TrackMatch>, Error> {
        let beatsource_config: BeatsourceConfig = config.get_custom("beatsource")?;
        
        // Search
        let query = format!("{} {}", info.artist()?, MatchingUtils::clean_title(info.title()?));
        let res = match self.search(&query) {
            Ok(r) => r,
            Err(e) => {
                error!("Beatsource search failed: {}", e);
                return Err(e);
            }
        };
        let tracks: Vec<Track> = res.tracks.into_iter().map(|t| t.into_track(&beatsource_config)).collect();
        let matched = MatchingUtils::match_track(&info, &tracks, config, true);
        Ok(matched)
    }

    fn extend_track(&mut self, _track: &mut Track, _config: &TaggerConfig) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatsourceSearchResponse {
    pub count: usize,
    pub tracks: Vec<BeatsourceTrack>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatsourceTrack {
    pub artists: Vec<BeatsourceSmall>,
    pub bpm: Option<i64>,
    pub catalog_number: String,
    pub genre: BeatsourceSmall,
    pub id: i64,
    pub isrc: Option<String>,
    pub key: Option<BeatsourceKey>,
    pub length_ms: Option<u64>,
    pub mix_name: Option<String>,
    pub name: String,
    /// YYYY-MM-DD
    pub publish_date: String,
    pub release: BeatsourceRelease,
    pub remixers: Vec<BeatsourceSmall>,
    pub slug: String
}

impl BeatsourceTrack {
    pub fn into_track(self, config: &BeatsourceConfig) -> Track {
        Track {
            platform: "beatsource".to_string(),
            title: self.name,
            version: self.mix_name,
            artists: self.artists.into_iter().map(|a| a.name).collect(),
            album: Some(self.release.name),
            key: self.key.map(|k| k.name
                .replace("Major", "")
                .replace("Minor", "m")
                .replace(" ", "")
                .trim()
                .to_string()
            ),
            bpm: self.bpm,
            genres: vec![self.genre.name],
            art: self.release.image.as_ref().map(|i| i.dynamic_uri
                .replace("{w}", &config.art_resolution.to_string())
                .replace("{h}", &config.art_resolution.to_string())
            ),
            url: format!("https://beatsource.com/track/{}/{}", self.slug, self.id),
            label: Some(self.release.label.name),
            catalog_number: Some(self.catalog_number),
            track_id: Some(self.id.to_string()),
            release_id: Some(self.release.id.to_string()),
            duration: self.length_ms.map(|ms| Duration::from_millis(ms)).unwrap_or(Duration::ZERO).into(),
            remixers: self.remixers.into_iter().map(|r| r.name).collect(),
            release_date: NaiveDate::parse_from_str(&self.publish_date, "%Y-%m-%d").ok(),
            isrc: self.isrc,
            thumbnail: self.release.image.map(|i| i.dynamic_uri
                .replace("{w}", "150")
                .replace("{h}", "150")
            ),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatsourceSmall {
    pub id: i64,
    pub name: String,
    pub slug: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatsourceKey {
    pub name: String,
    pub id: i64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatsourceRelease {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub image: Option<BeatsourceImage>,
    pub label: BeatsourceSmall
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatsourceImage {
    pub id: i64,
    pub dynamic_uri: String,
    pub uri: String
}

#[derive(Debug, Clone)]
pub struct BeatsourceToken {
    pub token: String,
    pub expires: u128
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatsourceOAuth {
    pub access_token: String,
    pub expires_in: u128
}

/// Manages the OAuth token
#[derive(Debug, Clone)]
pub struct BeatsourceTokenManager {
    token: Arc<Mutex<BeatsourceToken>>,
    client: Client
}

impl BeatsourceTokenManager {
    /// Create new instance and fetch token
    pub fn new() -> BeatsourceTokenManager {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36")
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();
        BeatsourceTokenManager {
            token: Arc::new(Mutex::new(BeatsourceToken {
                token: String::new(),
                // forces refresh
                expires: 0
            })),
            client
        }
    }

    /// Get and refresh token
    pub fn token(&self) -> Result<String, Error> {
        let mut token = self.token.lock().unwrap();
        // Valid
        if token.expires > timestamp!() {
            return Ok(token.token.to_string())
        }
        // Refresh
        let new_token = self.fetch_token()?;
        let code = new_token.token.clone();
        *token = new_token;
        Ok(code)
    }

    /// Fetch token via API directly
    fn fetch_token(&self) -> Result<BeatsourceToken, Error> {
        debug!("Updating Beatsource token!");
        
        // Beatsource and Beatport share the same account API. 
        // We can safely request an OAuth token using the embed client_credentials.
        let response: BeatsourceOAuth = self.client.post("https://account.beatport.com/o/token/")
            .form(&json!({
                "client_id": "2tiTbKxmQFwnbFjMONU4k7njMRZmV3ZMwRBndiZs",
                "client_secret": "RDUJyAk4zFEGtQ8rsTmylDSfxmALRNBn3D1BsRr7MKi3oa1TL9Mq9QxqUPK7loiumXolEWbJcWa4IGAhtwnTz1cSXClGJ1tkkNCNWwRwjxIKTZJKOJxbwaNt0Rm3WG0v",
                "grant_type": "client_credentials"
            }))
            .send()?
            .json()?;

        debug!("New Beatsource token retrieved.");
        Ok(BeatsourceToken {
            token: response.access_token,
            expires: response.expires_in * 1000 + timestamp!() - 10_000
        })
    }
}

#[derive(Debug, Clone)]
pub struct BeatsourceBuilder {
    token_manager: BeatsourceTokenManager
}

impl AutotaggerSourceBuilder for BeatsourceBuilder {
    fn new() -> BeatsourceBuilder {
        BeatsourceBuilder {
            token_manager: BeatsourceTokenManager::new()
        }
    }

    fn get_source(&mut self, _config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Error> {
        Ok(Box::new(Beatsource::new(self.token_manager.clone())))
    }

    fn info(&self) -> PlatformInfo {
        PlatformInfo {
            id: "beatsource".to_string(),
            name: "Beatsource".to_string(),
            description: "Overall more specialized in open-format (Hip Hop/Latin/Dancehall)".to_string(),
            icon: include_bytes!("../assets/beatsource.png"),
            max_threads: 0,
            version: "1.0.1".to_string(),
            requires_auth: false,
            supported_tags: supported_tags!(Title, Version, Artist, Album, Key, BPM, Genre, AlbumArt, URL, Label, CatalogNumber, TrackId, ReleaseId, Duration, Remixer, ReleaseDate, ISRC),
            custom_options: PlatformCustomOptions::new()
                .add_tooltip("art_resolution", "Album art resolution", "Select album art resolution", PlatformCustomOptionValue::Number {
                    min: 100, max: 1600, step: 100, value: 500
                }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatsourceConfig {
    pub art_resolution: i32
}


#[test]
/// Make sure it doesn't panic / response format ok
fn test_beatsource() {
    let token_manager = BeatsourceTokenManager::new();
    let _token = token_manager.token().unwrap();
    let b = Beatsource::new(token_manager);
    b.search("martin garrix").unwrap();
    b.search("illenium").unwrap();
    b.search("test").unwrap();
}
