use std::error::Error;
use chrono::{DateTime, Utc};
use onetagger_tag::FrameName;
use onetagger_tagger::{AutotaggerSourceBuilder, TaggerConfig, AutotaggerSource, PlatformInfo, PlatformCustomOptions, PlatformCustomOptionValue, AudioFileInfo, Track, MatchingUtils};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Serialize, Deserialize};
use serde_json::json;

struct BPMSupreme {
    client: Client,
}

impl BPMSupreme {
    /// Create new instance
    pub fn new(token: &str) -> BPMSupreme {
        let mut header_map = HeaderMap::new();
        header_map.append("Cookie", HeaderValue::from_str(&format!("bpm_session={token}")).unwrap());
        BPMSupreme {
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
        let res: BPMSupremeResponse<BPMSupremeUser> = client.post("https://api.bpmsupreme.com/v3/login")
            .json(&json!({
                "device": {
                    "app_version": "2.0",
                    "build_version": "1",
                    "debug": true,
                    "device_data_os": "web",
                    "device_uuid": "b9e709ad12df28dd5f06ac07933254bf",
                    "language": "en-US"
                },
                "email": email,
                "password": password
            }))
            .send()?
            .error_for_status()?
            .json()?;
        Ok(res.data.session_token)
    }

    /// Search for tracks
    pub fn search(&self, query: &str) -> Result<Vec<BPMSupremeTrack>, Box<dyn Error>> {
        let res: BPMSupremeResponse<Vec<BPMSupremeTrack>> = self.client.get("https://api.bpmsupreme.com/v1.2/search/audio")
            .query(&[
                ("keywords", query),
                ("limit", "100"),
                ("skip", "0"),
            ])
            .send()?
            .error_for_status()?
            .json()?;
        Ok(res.data)
    }
}

impl AutotaggerSource for BPMSupreme {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>> {
        // Search and match
        let query = format!("{} {}", MatchingUtils::clean_title(info.title()?), info.artist()?);
        let tracks = self.search(&query)?.into_iter().map(|t| t.into()).collect::<Vec<Track>>();
        Ok(MatchingUtils::match_track(info, &tracks, config, true))
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
struct BPMSupremeTrack {
    pub artist: String,
    pub bpm_count: i64,
    pub category: BPMSupremeCategory,
    pub cover: String,
    pub depth_analysis: Option<DepthAnalysis>,
    pub genre: BPMSupremeGenre,
    pub key: Option<String>,
    pub label: String,
    pub song_name: String,
    pub created_at: DateTime<Utc>,
    pub id: i64
}

impl Into<Track> for BPMSupremeTrack {
    fn into(self) -> Track {
        //TODO: Maybe mood field in `Track`
        // Mood
        let mut other = vec![];
        if let Some(da) = self.depth_analysis {
            other.push((FrameName::new("TMOO", "MOOD", "com.apple.iTunes:MOOD"), da.mood));
        }

        Track {
            platform: "bpmsupreme".to_string(),
            artists: vec![self.artist],
            title: self.song_name,
            bpm: Some(self.bpm_count),
            art: Some(self.cover),
            genres: vec![self.genre.name],
            key: self.key,
            label: Some(self.label),
            release_date: Some(self.created_at.naive_utc().date()),
            track_id: Some(self.id.to_string()),
            other,
            ..Default::default()
        }
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


pub struct BPMSupremeBuilder {
    token: Option<String>
}

impl AutotaggerSourceBuilder for BPMSupremeBuilder {
    fn new() -> Self {
        BPMSupremeBuilder { token: None }
    }

    fn get_source(&mut self, config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Box<dyn Error>> {
        // Get token
        let token = match &self.token {
            Some(token) => token.to_string(),
            None => {
                // Try to login
                let custom = config.custom.get("bpmsupreme").ok_or("Missing bpmsupreme config!")?;
                let token = BPMSupreme::login(
                    &custom.get_str("email").ok_or("Missing email!")?, 
                    &custom.get_str("password").ok_or("Missing password!")?
                )?;
                self.token = Some(token.to_string());
                token
            }
        };
        
        Ok(Box::new(BPMSupreme::new(&token)))
    }

    fn info(&self) -> PlatformInfo {
        PlatformInfo {
            id: "bpmsupreme".to_string(),
            name: "BPM Supreme".to_string(),
            //TODO: Add description
            description: "//todo".to_string(),
            version: "1.0.0".to_string(),
            //TODO: Icon
            icon: &[0u8],
            //TODO: test if more doable
            max_threads: 1,
            custom_options: PlatformCustomOptions::new()
                .add("email", "Email", PlatformCustomOptionValue::String { 
                    value: String::new(), hidden: Some(false) 
                })
                .add("password", "Password", PlatformCustomOptionValue::String {
                    value: String::new(), hidden: Some(true)
                }),
        }
    }
}