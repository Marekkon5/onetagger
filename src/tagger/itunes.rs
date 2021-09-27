use chrono::NaiveDate;
use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use crate::tagger::matcher::Matcher;
use crate::tagger::{Explicitness, ITunesID, MusicPlatform, TaggerConfig, Track, TrackMatcherST};

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
            last_request: 0,
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
        let res = self
            .client
            .get(&format!("https://itunes.apple.com{}", path))
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

impl TrackMatcherST for ITunes {
    fn match_track(
        &mut self,
        local: &Track,
        config: &TaggerConfig,
    ) -> Result<Option<(f64, Track)>, Box<dyn Error>> {
        // Search
        let query = format!(
            "{} {}",
            local.artist.as_ref().unwrap(),
            local.title.as_ref().unwrap()
        );
        let results = self.search(&query)?;
        let tracks: Vec<Track> = results
            .results
            .iter()
            .filter_map(|r| r.get_metadata())
            .collect();
        if let Some((f, track)) = Matcher::match_track(local, &tracks, config) {
            return Ok(Some((f, track)));
        }
        Ok(None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub result_count: usize,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "wrapperType")]
pub enum SearchResult {
    #[serde(rename_all = "camelCase")]
    Track {
        kind: TrackKind,
        track_name: String,
        track_censored_name: String,
        track_id: i64,

        artist_name: String,
        artist_id: i64,

        collection_name: String,
        collection_censored_name: String,
        collection_id: i64,

        primary_genre_name: String,
        track_time_millis: u64,
        release_date: String,

        track_number: i64,
        track_count: i64,
        disc_number: i64,
        disc_count: i64,

        track_explicitness: Explicitness,
        collection_explicitness: Explicitness,

        is_streamable: bool,
        preview_url: String,

        track_view_url: String,
        artist_view_url: String,
        collection_view_url: String,
        artwork_url30: String,
        artwork_url60: String,
        artwork_url100: String,

        collection_price: f64,
        track_price: f64,
        currency: String,
        country: String,
    },
}

impl SearchResult {
    pub fn get_metadata(&self) -> Option<Track> {
        match self {
            SearchResult::Track {
                track_name,
                track_id,

                artist_name,
                artist_id,

                collection_name,
                collection_id,

                primary_genre_name,
                track_time_millis,
                release_date,

                track_number,
                track_count,
                disc_number,
                disc_count,

                track_explicitness,

                preview_url,
                track_view_url,
                artist_view_url,
                collection_view_url,
                ..
            } => Some(Track {
                platform: Some(MusicPlatform::ITunes),
                title: Some(track_name.to_owned()),
                artist: Some(artist_name.to_owned()),
                album: Some(collection_name.to_owned()),
                duration: Some(Duration::from_millis(*track_time_millis)),
                genre: Some(primary_genre_name.to_owned()),
                track_number: Some(track_number.to_owned()),
                track_count: Some(track_count.to_owned()),
                disc_number: Some(disc_number.to_owned()),
                disc_count: Some(disc_count.to_owned()),
                release_date: Some(
                    NaiveDate::parse_from_str(&release_date[0..10], "%Y-%m-%d").ok(),
                )
                .flatten(),
                itunes: Some(ITunesID {
                    track_id: track_id.to_owned(),
                    release_id: collection_id.to_owned(),
                    preview_url: preview_url.to_owned(),
                    track_url: track_view_url.to_owned(),
                    release_url: collection_view_url.to_owned(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrackKind {
    Song,
    Podcast,
    #[serde(other)]
    Other,
}
