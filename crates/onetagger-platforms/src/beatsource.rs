use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use chrono::NaiveDate;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use serde_json::Value;
use serde::{Serialize, Deserialize};

use onetagger_tagger::{AutotaggerSource, Track, TaggerConfig, AudioFileInfo, MatchingUtils, AutotaggerSourceBuilder, PlatformInfo, PlatformCustomOptions, PlatformCustomOptionValue};

pub struct Beatsource {
    client: Client,
    token_manager: BeatsourceTokenManager
}

impl Beatsource {
    /// Create new instance
    pub fn new(token_manager: BeatsourceTokenManager) -> Beatsource {
        Beatsource {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:85.0) Gecko/20100101 Firefox/85.0")
                .build()
                .unwrap(),
            token_manager
        }
    }

    /// Search for tracks
    pub fn search(&self, query: &str) -> Result<BeatsourceSearchResponse, Box<dyn Error>> {
        let res: BeatsourceSearchResponse = self.client.get("https://api.beatsource.com/v4/catalog/search")
            .query(&[
                ("pubper_page", "100"),
                ("page", "1"),
                ("type", "tracks"),
                ("q", query)
            ])
            .bearer_auth(self.token_manager.token()?)
            .send()?
            .json()?;
        Ok(res)
    }
}

impl AutotaggerSource for Beatsource {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>> {
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
            art: self.release.image.map(|i| i.dynamic_uri
                .replace("{w}", &config.art_resolution.to_string())
                .replace("{h}", &config.art_resolution.to_string())
            ),
            url: format!("https://beatsource.com/track/{}/{}", self.slug, self.id),
            label: Some(self.release.label.name),
            catalog_number: Some(self.catalog_number),
            track_id: Some(self.id.to_string()),
            release_id: self.release.id.to_string(),
            duration: self.length_ms.map(|ms| Duration::from_millis(ms)).unwrap_or(Duration::ZERO),
            remixers: self.remixers.into_iter().map(|r| r.name).collect(),
            release_date: NaiveDate::parse_from_str(&self.publish_date, "%Y-%m-%d").ok(),
            isrc: self.isrc,
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
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:85.0) Gecko/20100101 Firefox/85.0")
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
    pub fn token(&self) -> Result<String, Box<dyn Error>> {
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

    /// Fetch token from homepage
    fn fetch_token(&self) -> Result<BeatsourceToken, Box<dyn Error>> {
        debug!("Updating Beatsource token!");
        let body = self.client.get("https://www.beatsource.com").send()?.text()?;
        let document = Html::parse_document(&body);
        let selector = Selector::parse("script#__NEXT_DATA__").unwrap();
        let elem = document.select(&selector).next().ok_or("Missing __NEXT_DATA__")?;
        let text = elem.text().collect::<Vec<_>>().join("");
        let json: Value = serde_json::from_str(&text)?;
        let token = json["props"]["rootStore"]["authStore"]["user"]["access_token"].as_str().ok_or("Missing access_token")?;
        let expires = json["props"]["rootStore"]["authStore"]["user"]["expires_in"].as_u64().unwrap();
        debug!("New Beatsource token: {}", token);
        Ok(BeatsourceToken {
            token: token.to_string(),
            expires: timestamp!() + expires as u128 - 100
        })
    }
}

pub struct BeatsourceBuilder {
    token_manager: BeatsourceTokenManager
}

impl AutotaggerSourceBuilder for BeatsourceBuilder {
    fn new() -> BeatsourceBuilder {
        BeatsourceBuilder {
            token_manager: BeatsourceTokenManager::new()
        }
    }

    fn get_source(&mut self, _config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Box<dyn Error>> {
        Ok(Box::new(Beatsource::new(self.token_manager.clone())))
    }

    fn info(&self) -> PlatformInfo {
        PlatformInfo {
            id: "beatsource".to_string(),
            name: "Beatsource".to_string(),
            description: "Overall more specialized in open-format (Hip Hop/Latin/Dancehall)".to_string(),
            icon: include_bytes!("../assets/beatsource.png"),
            max_threads: 0,
            version: "1.0.0".to_string(),
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