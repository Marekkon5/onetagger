use std::error::Error;
use std::time::Duration;
use chrono::{DateTime, Utc};
use onetagger_tagger::{AutotaggerSourceBuilder, TaggerConfig, AutotaggerSource, PlatformInfo, PlatformCustomOptions, PlatformCustomOptionValue, AudioFileInfo, Track, MatchingUtils, supported_tags, TrackMatch};
use regex::Regex;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use serde::{Serialize, Deserialize};
use serde_json::json;

struct BPMSupreme {
    client: Client,
    library: BPMMusicLibrary
}

impl BPMSupreme {
    /// Create new instance
    pub fn new(token: &str, library: BPMMusicLibrary) -> BPMSupreme {
        let mut header_map = HeaderMap::new();
        header_map.append("Cookie", HeaderValue::from_str(&format!("bpm_session={token}")).unwrap());
        BPMSupreme {
            library,
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.67 Safari/537.36.")
                .default_headers(header_map)
                .build()
                .unwrap(),
        }
    }

    /// Login with email and password and get token
    pub fn login(email: &str, password: &str) -> Result<String, Box<dyn Error>> {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.67 Safari/537.36.")
            .build()
            .unwrap();
        let res: BPMSupremeResponse<BPMSupremeUser> = client.post("https://api.bpmsupreme.com/v4/login")
            .json(&json!({
                "device": {
                    "app_version": "2.0",
                    "build_version": "1",
                    "debug": false,
                    "device_data_os": "web",
                    "device_uuid": "d2d9dc2f7cf311a3bff7f3ea6df3ba9b",
                    "language": "en-US"
                },
                "email": email,
                "password": password,
                "from": "global-login"
            }))
            .send()?
            .error_for_status()?
            .json()?;
        Ok(res.data.session_token)
    }

    /// Wrapper for GET request with rate limit
    fn get<T: DeserializeOwned>(&self, url: &str, query: &[(&str, &str)]) -> Result<T, Box<dyn Error>> {
        let res = self.client.get(url)
            .query(query)
            .send()?;

        // Rate limit
        if res.status() == StatusCode::TOO_MANY_REQUESTS {
            let delay = res.headers().get("retry-after").map(|h| h.to_str().unwrap().parse().ok()).flatten().unwrap_or(3);
            warn!("BPM Supreme rate limited, waiting for: {delay}s");
            std::thread::sleep(Duration::from_secs(delay));
            return self.get(url, query);
        }

        Ok(res.error_for_status()?.json()?)
    }

    /// Search for tracks
    pub fn search(&self, query: &str, library: BPMMusicLibrary) -> Result<Vec<BPMSupremeSong>, Box<dyn Error>> {
        debug!("Library: {library:?}");
        let res: BPMSupremeResponse<Vec<BPMSupremeSong>> = self.get(
            "https://api.download.bpmsupreme.com/v1/albums",
            &[
                ("term", query),
                ("limit", "100"),
                ("skip", "0"),
                ("library", library.library()),
                ("hide_remix", "0")
            ]
        )?;
        Ok(res.data)
    }
}

impl AutotaggerSource for BPMSupreme {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Vec<TrackMatch>, Box<dyn Error>> {
        // Search and match
        let re = Regex::new(" \\(.*\\)$").unwrap();
        let title = MatchingUtils::clean_title(info.title()?);
        let title = re.replace(&title, "");
        let query = format!("{title} {}", MatchingUtils::clean_title(info.artist()?));
        debug!("{query}");
        let tracks = self.search(&query, self.library)?.into_iter().map(|t| t.into_tracks()).flatten().collect::<Vec<Track>>();
        Ok(MatchingUtils::match_track(info, &tracks, config, true))
    }

    fn extend_track(&mut self, _track: &mut Track, _config: &TaggerConfig) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BPMSupremeResponse<D> {
    pub data: D
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BPMSupremeUser {
    pub session_token: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BPMSupremeSong {
    pub artist: String,
    pub bpm: i64,
    pub category: BPMSupremeCategory,
    pub cover_url: String,
    pub depth_analysis: Option<DepthAnalysis>,
    pub genre: BPMSupremeGenre,
    pub key: Option<String>,
    pub label: String,
    pub title: String,
    pub created_at: Option<DateTime<Utc>>,
    pub id: i64,
    pub media: Vec<BPMSupremeMedia>
}

impl BPMSupremeSong {
    /// Convert self and all versions into tracks
    pub fn into_tracks(self) -> Vec<Track> {
        let base = Track {
            platform: "bpmsupreme".to_string(),
            artists: vec![self.artist],
            title: self.title,
            bpm: Some(self.bpm),
            genres: vec![self.genre.name],
            key: self.key,
            label: Some(self.label),
            release_date: self.created_at.map(|c| c.naive_utc().date()),
            track_id: Some(self.id.to_string()),
            mood: self.depth_analysis.map(|da| da.mood),
            url: format!("https://app.bpmsupreme.com/d/album/{}", self.id),
            catalog_number: Some(self.id.to_string()),
            thumbnail: if self.cover_url.contains("default_cover.png") { None } else { Some(format!("{}?dw=112", self.cover_url)) },
            art: if self.cover_url.contains("default_cover.png") { None } else { Some(self.cover_url) },
            ..Default::default()
        };
        // Different versions
        let mut output = vec![base.clone()];
        for media in self.media {
            let mut t = base.clone();
            t.title = media.name;
            output.push(t);
        }
        output
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
struct BPMSupremeCategory {
    pub name: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DepthAnalysis {
    pub mood: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BPMSupremeGenre {
    pub name: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BPMSupremeMedia {
    name: String
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq, Default)]
pub enum BPMMusicLibrary {
    #[default]
    Supreme,
    Latino
}

impl BPMMusicLibrary {
    /// Get the library parameter of this library
    pub fn library(&self) -> &'static str {
        match self {
            BPMMusicLibrary::Supreme => "music",
            BPMMusicLibrary::Latino => "latin",
        }
    }
}

pub struct BPMSupremeBuilder {
    token: Option<String>,
    library: BPMMusicLibrary
}

impl AutotaggerSourceBuilder for BPMSupremeBuilder {
    fn new() -> Self {
        BPMSupremeBuilder { token: None, library: BPMMusicLibrary::Supreme }
    }

    fn get_source(&mut self, config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Box<dyn Error>> {
        // Get token
        let token = match &self.token {
            Some(token) => token.to_string(),
            None => {
                // Try to login
                let custom: BPMSupremeConfig = config.get_custom("bpmsupreme")?;
                let token = BPMSupreme::login(&custom.email, &custom.password)?;
                self.token = Some(token.to_string());
                self.library = custom.library;
                token
            }
        };
        
        Ok(Box::new(BPMSupreme::new(&token, self.library)))
    }

    fn info(&self) -> PlatformInfo {
        PlatformInfo {
            id: "bpmsupreme".to_string(),
            name: "BPM Supreme".to_string(),
            description: "Specialized in chart & open-format. Requires a free account".to_string(),
            version: "1.0.0".to_string(),
            icon: include_bytes!("../assets/bpmsupreme.png"),
            max_threads: 1,
            requires_auth: true,
            supported_tags: supported_tags!(Artist, Title, BPM, AlbumArt, Genre, Key, Label, ReleaseDate, TrackId, Mood, URL),
            custom_options: PlatformCustomOptions::new()
                .add("email", "Email", PlatformCustomOptionValue::String { 
                    value: String::new(), hidden: Some(false) 
                })
                .add("password", "Password", PlatformCustomOptionValue::String {
                    value: String::new(), hidden: Some(true)
                })
                .add("library", "Library", PlatformCustomOptionValue::Option { 
                    values: vec!["Supreme".to_string(), "Latino".to_string()], value: "Supreme".to_string()
                }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BPMSupremeConfig {
    pub email: String,
    pub password: String,
    pub library: BPMMusicLibrary
}