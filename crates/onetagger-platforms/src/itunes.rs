use std::error::Error;
use std::time::Duration;
use std::thread::sleep;
use chrono::NaiveDate;
use reqwest::blocking::{Client, Response};
use serde::{Serialize, Deserialize};
use onetagger_tagger::{AutotaggerSource, AudioFileInfo, TaggerConfig, Track, MatchingUtils, AutotaggerSourceBuilder, PlatformInfo};

pub struct ITunes {
    client: Client,
    // Requests per minute
    pub rate_limit: i16,
    last_request: u128,
}

impl ITunes {
    /// Create new instance
    pub fn new() -> ITunes {
        ITunes {
            client: Client::builder()
                .user_agent("OneTagger/1.0")
                .build()
                .unwrap(),
            rate_limit: 20,
            last_request: 0
        }
    }

    // /// Set rate limit, -1 for no rate limit
    // pub fn set_rate_limit(&mut self, rate_limit: i16) {
    //     self.rate_limit = rate_limit;
    // }

    /// Make get request to API
    fn get(&mut self, path: &str, query: &[(&str, &str)]) -> Result<Response, Box<dyn Error>> {
        debug!("iTunes GET: {} {:?}", path, query);
        // Rate limit
        if self.last_request > 0 && self.rate_limit != -1 {
            let diff = timestamp!() - self.last_request;
            let req_ms = 1000_f64 / (self.rate_limit as f64 / 60_f64);
            let wait = diff as f64 - req_ms;
            if wait < 0_f64 {
                info!("iTunes rate limit delay: {}", -wait);
                sleep(Duration::from_millis(-wait as u64));
            }
        }

        // Do request
        let res = self.client.get(&format!("https://itunes.apple.com{}", path))
            .query(query)
            .send()?;
        self.last_request = timestamp!();
        Ok(res)
    }

    /// Search the iTunes API
    pub fn search(&mut self, query: &str) -> Result<SearchResults, Box<dyn Error>> {
        Ok(self.get("/search", &[("term", query)])?.json()?)
    }
}

impl AutotaggerSource for ITunes {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>> {
        // Search
        let query = format!("{} {}", info.artist()?, MatchingUtils::clean_title(info.title()?));
        let results = self.search(&query)?;
        let tracks: Vec<Track> = results.results.iter().filter_map(|r| r.into_track()).collect();
        if let Some((f, track)) = MatchingUtils::match_track(info, &tracks, config, true) {
            return Ok(Some((f, track)));
        }
        Ok(None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub result_count: usize,
    pub results: Vec<SearchResult>
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "wrapperType")]
pub enum SearchResult {
    #[serde(rename_all = "camelCase")]
    Track {
        kind: TrackKind,
        artist_id: Option<i64>,
        collection_id: Option<i64>,
        track_id: i64,
        artist_name: Option<String>,
        collection_name: Option<String>,
        track_name: String,
        disc_count: Option<i16>,
        disc_number: Option<i16>,
        track_count: Option<i16>,
        track_number: Option<i32>,
        country: String,
        track_view_url: String,
        track_time_millis: Option<u64>,
        primary_genre_name: String,
        release_date: Option<String>,
        artwork_url100: Option<String>
    }
}

impl SearchResult {
    pub fn into_track(&self) -> Option<Track> {
        match self {
            SearchResult::Track { collection_id, track_id, artist_name, collection_name, track_name, track_view_url, track_time_millis, primary_genre_name, release_date, track_number, artwork_url100, .. } => {
                Some(Track {
                    platform: "itunes".to_string(),
                    title: track_name.clone(),
                    artists: artist_name.clone().map(|a| vec![a]).unwrap_or(vec![]),
                    album: collection_name.clone(),
                    url: track_view_url.to_string(),
                    track_id: Some(track_id.to_string()),
                    release_id: collection_id.map(|c| c.to_string()).unwrap_or_default(),
                    duration: track_time_millis.map(|d| Duration::from_millis(d)).unwrap_or(Duration::ZERO),
                    genres: vec![primary_genre_name.to_string()],
                    release_date: release_date.as_ref().map(|release_date| NaiveDate::parse_from_str(&release_date[0..10], "%Y-%m-%d").ok()).flatten(),
                    track_number: track_number.map(|t| t.into()),
                    art: artwork_url100.clone(),
                    ..Default::default()
                })
            },
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrackKind {
    Song,
    Podcast,
    #[serde(other)]
    Other
}

pub struct ITunesBuilder;

impl AutotaggerSourceBuilder for ITunesBuilder {
    fn new() -> ITunesBuilder {
        ITunesBuilder
    }

    fn get_source(&mut self, _config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Box<dyn Error>> {
        Ok(Box::new(ITunes::new()))
    }

    fn info(&self) -> PlatformInfo {
        PlatformInfo {
            id: "itunes".to_string(),
            name: "iTunes".to_string(),
            description: "Slow due rate limits (~20 tracks / min)".to_string(),
            icon: include_bytes!("../assets/itunes.png"),
            max_threads: 1,
            custom_options: Default::default(),
        }
    }
}