use chrono::NaiveDate;
use regex::Regex;
use reqwest::blocking::{Client, Response};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use crate::tagger::matcher::Matcher;
use crate::tagger::{
    parse_duration, MusicPlatform, StylesOptions, TaggerConfig, Track, TrackMatcherST,
};

pub struct Discogs {
    client: Client,
    token: Option<String>,
    // Requests per minute
    pub rate_limit: i16,
    last_request: u128,
    // Cache of ID:Value
    release_cache: HashMap<i64, ReleaseMaster>,
}

impl Discogs {
    // Create new instance
    pub fn new() -> Discogs {
        let client = Client::builder()
            .user_agent("OneTagger/1.0")
            .build()
            .unwrap();
        Discogs {
            client,
            token: None,
            rate_limit: 25,
            last_request: 0,
            release_cache: HashMap::new(),
        }
    }

    /// Set rate limit, -1 for no rate limit
    pub fn set_rate_limit(&mut self, rate_limit: i16) {
        self.rate_limit = rate_limit;
    }

    // Set authorization token and update rate limit
    pub fn set_auth_token(&mut self, token: &str) {
        self.token = Some(token.to_string());
        self.rate_limit = 60;
    }

    // Check if token is valid
    pub fn validate_token(&mut self) -> bool {
        match self.get(
            "https://api.discogs.com/database/search",
            vec![("q", "test")],
        ) {
            Ok(res) => {
                if res.status() == StatusCode::OK {
                    true
                } else {
                    error!("Failed validating Discogs token: {}", res.status());
                    debug!("{:?}", res.text());
                    false
                }
            }
            Err(e) => {
                error!("Failed validating Discogs token: {}", e);
                false
            }
        }
    }

    // Get request wrapper with rate limit
    fn get(&mut self, url: &str, query: Vec<(&str, &str)>) -> Result<Response, Box<dyn Error>> {
        debug!("Discogs GET {}", url);
        // Rate limit
        if self.last_request > 0 && self.rate_limit != -1 {
            let diff = timestamp!() - self.last_request;
            let req_ms = 1000_f64 / (self.rate_limit as f64 / 60_f64);
            let wait = diff as f64 - req_ms;
            if wait < 0_f64 {
                debug!("Discogs rate limit delay: {}", -wait);
                sleep(Duration::from_millis(-wait as u64));
            }
        }
        // Create request
        let mut request = self.client.get(url).query(&query);
        if self.token.is_some() {
            request = request.header(
                "Authorization",
                format!("Discogs token={}", self.token.as_ref().unwrap()),
            );
        }
        let response = request.send()?;
        // Rate limit
        let rate_limit_remaining = response
            .headers()
            .get("X-Discogs-Ratelimit-Remaining")
            .map(|v| v.to_str().unwrap().parse::<i32>().ok())
            .flatten()
            .unwrap_or(100);
        debug!("Discogs rate limit remaining: {}", rate_limit_remaining);
        if rate_limit_remaining < 1 {
            warn!("Discogs rate limit hit! Waiting 10s...");
            sleep(Duration::from_secs(10));
            return self.get(url, query);
        }

        // Save last reqeust time for rate limitting
        self.last_request = timestamp!();
        Ok(response)
    }

    pub fn search(
        &mut self,
        result_type: Option<&str>,
        query: Option<&str>,
        title: Option<&str>,
        artist: Option<&str>,
    ) -> Result<Vec<ReleaseMasterSearchResult>, Box<dyn Error>> {
        // Generate parameters
        let mut qp = vec![];
        if let Some(t) = result_type {
            qp.push(("type", t));
        }
        if let Some(q) = query {
            qp.push(("q", q));
        }
        if let Some(t) = title {
            qp.push(("title", t));
        }
        if let Some(a) = artist {
            qp.push(("artist", a));
        }
        // Get
        let response: Value = self
            .get("https://api.discogs.com/database/search", qp)?
            .json()?;
        let empty: Vec<Value> = vec![];
        let results: Vec<ReleaseMasterSearchResult> = response["results"]
            .as_array()
            .unwrap_or(&empty)
            .into_iter()
            .filter_map(|r| {
                // Filter only releases and masters
                let t = r["type"].as_str().unwrap_or("");
                if t == "release" || t == "master" {
                    serde_json::from_value(r.to_owned()).ok()
                } else {
                    None
                }
            })
            .collect();
        Ok(results)
    }

    // Get full release info
    pub fn full_release(
        &mut self,
        release_type: ReleaseType,
        id: i64,
    ) -> Result<ReleaseMaster, Box<dyn Error>> {
        // Check if cached
        if self.release_cache.contains_key(&id) {
            return Ok(self.release_cache.get(&id).unwrap().to_owned());
        }
        // Get
        let rtype = match release_type {
            ReleaseType::Master => "masters",
            ReleaseType::Release => "releases",
        };
        let response = self
            .get(&format!("https://api.discogs.com/{}/{}", rtype, id), vec![])?
            .json()?;
        let release: ReleaseMaster = serde_json::from_value(response)?;
        // Cache
        self.release_cache.insert(id, release.clone());
        Ok(release)
    }
}

impl TrackMatcherST for Discogs {
    fn match_track(
        &mut self,
        local: &Track,
        config: &TaggerConfig,
    ) -> Result<Option<(f64, Track)>, Box<dyn Error>> {
        // Exact ID match
        if config.match_by_id && local.discogs.is_some() {
            let release =
                self.full_release(ReleaseType::Release, local.discogs.unwrap().release_id)?;
            // Exact track number match
            if let Some(track_number) = local.track_number {
                return Ok(Some((
                    1.0,
                    release.get_track(track_number as usize - 1, &config.styles_options),
                )));
            }
            // Match inside release
            let mut tracks = vec![];
            for i in 0..release.tracks.len() {
                tracks.push(release.get_track(i, &config.styles_options));
            }
            return Ok(Matcher::match_track(&local, &tracks, &config));
        }
        // Search
        let query = format!(
            "{} {}",
            local.artist.unwrap_or_default(),
            local.title.unwrap_or_default()
        );
        let mut results = self.search(Some("release,master"), Some(&query), None, None)?;
        // Fallback
        if results.is_empty() {
            info!("Discogs fallback search!");
            results = self.search(
                Some("release,master"),
                None,
                Some(&local.title.unwrap_or_default()),
                Some(&local.artists.unwrap().first().unwrap()),
            )?;
        }
        if results.is_empty() {
            return Ok(None);
        }
        // Turncate
        results.truncate(config.discogs.max_results as usize);
        for release_data in results {
            // Get full
            let r = self.full_release(release_data.rtype, release_data.id);
            if r.is_err() {
                error!("{:?}", r);
                continue;
            }
            let release = r.unwrap();
            let mut tracks = vec![];
            for i in 0..release.tracks.len() {
                tracks.push(release.get_track(i, &config.styles_options));
            }
            if let Some((acc, mut track)) = Matcher::match_track(&local, &tracks, &config) {
                // Get catalog number if enabled from release rather than master
                if ((config.catalog_number && track.catalog_number.is_none())
                    || (config.label && track.label.is_none()))
                    && (release.labels.is_none() && release.main_release.is_some())
                {
                    info!("Discogs fetching release for catalog number/label...");
                    match self.full_release(ReleaseType::Release, release.main_release.unwrap()) {
                        // Get CN, label from release
                        Ok(r) => {
                            if let Some(labels) = r.labels {
                                if let Some(label) = labels.first() {
                                    if track.label.is_none() {
                                        track.label = Some(label.name.to_string());
                                    }
                                    if let Some(cn) = label.catno.as_ref() {
                                        if cn != "none" {
                                            track.catalog_number = Some(cn.to_owned());
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => warn!("Failed fetching release info for catalog number! {}", e),
                    }
                }
                return Ok(Some((acc, track)));
            }
        }
        Ok(None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseType {
    Release,
    Master,
}

// Used in search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseMasterSearchResult {
    pub id: i64,
    // pub country: String,
    // pub year: Option<String>,
    // pub label: Vec<String>,
    // pub genre: Vec<String>,
    // pub style: Vec<String>,
    #[serde(rename = "type")]
    pub rtype: ReleaseType,
    #[serde(rename = "resource_url")]
    pub url: String,
    // #[serde(rename = "cover_image")]
    // pub album_art: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    pub name: String,
    pub id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraArtist {
    pub name: String,
    pub id: i64,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscogsTrack {
    pub position: String,
    pub title: String,
    pub artists: Option<Vec<ExtraArtist>>,
    pub duration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: i64,
    pub name: String,
    pub catno: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub height: i64,
    pub width: i64,
    #[serde(rename = "uri")]
    pub url: String,
    #[serde(rename = "type")]
    pub image_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseMaster {
    pub id: i64,
    pub styles: Option<Vec<String>>,
    pub genres: Vec<String>,
    pub year: Option<i64>,
    pub artists: Vec<Artist>,
    #[serde(rename = "extraartists")]
    pub extra_artists: Option<Vec<ExtraArtist>>,
    pub country: Option<String>,
    #[serde(rename = "uri")]
    pub url: String,
    pub labels: Option<Vec<Label>>,
    pub title: String,
    pub images: Option<Vec<Image>>,
    #[serde(rename = "tracklist")]
    pub tracks: Vec<DiscogsTrack>,
    pub released: Option<String>,
    pub main_release: Option<i64>,
}

impl ReleaseMaster {
    // Remove (n) at end of artist
    pub fn clean_artist(input: &str) -> String {
        let re = Regex::new(r" \(\d{1,2}\)$").unwrap();
        re.replace(input, "").to_string()
    }

    pub fn get_track(&self, track_index: usize, styles_option: &StylesOptions) -> Track {
        // Parse release date
        let release_date = match &self.released {
            Some(r) => NaiveDate::parse_from_str(&r, "%Y-%m-%d").ok(),
            None => None,
        };

        // Generate styles and genres
        let mut styles: Option<Vec<String>>;
        let mut genres: Option<Vec<String>>;
        let styles_o = self.styles.clone().unwrap_or(vec![]);
        let genres_o = self.genres.clone();
        match styles_option {
            StylesOptions::OnlyGenres => genres = Some(genres_o),
            StylesOptions::OnlyStyles => styles = Some(styles_o),
            /*
            StylesOptions::MergeToGenres => {
                genres = Some(genres_o.extend(styles_o));
            }
            StylesOptions::MergeToStyles => {
                styles = Some(styles_o.extend(genres_o));
            }
            */
            StylesOptions::GenresToStyle => styles = Some(genres_o),
            StylesOptions::StylesToGenre => genres = Some(styles_o),
            // Default and custom
            _ => {
                genres = Some(genres_o);
                styles = Some(styles_o);
            }
        }

        // Get catalog number
        let mut catalog_number = None;
        if let Some(labels) = &self.labels {
            if let Some(label) = labels.first() {
                if let Some(cn) = label.catno.as_ref() {
                    if cn != "none" {
                        catalog_number = Some(cn.to_string());
                    }
                }
            }
        }

        // Generate track
        Track {
            platform: Some(MusicPlatform::Discogs),
            title: Some(self.tracks[track_index].title.to_string()),
            artists: match self.tracks[track_index].artists.as_ref() {
                // Use track artists if available
                Some(artists) => Some(
                    artists
                        .iter()
                        .map(|a| ReleaseMaster::clean_artist(&a.name).to_string())
                        .collect(),
                ),
                None => Some(
                    self.artists
                        .iter()
                        .map(|a| ReleaseMaster::clean_artist(&a.name).to_string())
                        .collect(),
                ),
            },
            album: Some(self.title.to_string()),
            genres,
            styles,
            artwork_url: match self.images.as_ref().unwrap_or(&Vec::new()).first() {
                Some(image) => Some(image.url.to_string()),
                None => None,
            },
            //url: self.url.to_string(),
            label: if let Some(labels) = &self.labels {
                if labels.is_empty() {
                    None
                } else {
                    Some(ReleaseMaster::clean_artist(&labels[0].name).to_string())
                }
            } else {
                None
            },
            release_year: self.year,
            release_date,
            catalog_number,
            /*
            other: vec![(
                "VINYLTRACK".to_string(),
                self.tracks[track_index].position.to_string(),
            )],
            */
            //release_id: self.id.to_string(),
            duration: Some(
                parse_duration(&self.tracks[track_index].duration).unwrap_or(Duration::ZERO),
            ),
            ..Default::default()
        }
    }
}
