use std::error::Error;
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;
use regex::Regex;
use chrono::NaiveDate;
use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use serde_json::Value;
use serde::{Serialize, Deserialize};
use serde;
use crate::tagger::{MusicPlatform, Track, TrackMatcherST, TaggerConfig, AudioFileInfo, MatchingUtils, ITunesStyles, parse_duration};

pub struct ITunes {
    client: Client,
    token: Option<String>,
    // Requests per minute
    pub rate_limit: i16,
    last_request: u128,
    // Cache of ID:Value
    release_cache: HashMap<i64, ReleaseMaster>
}

impl ITunes {
    // Create new instance
    pub fn new() -> ITunes {
        let client = Client::builder()
            .user_agent("OneTagger/1.0")
            .build()
            .unwrap();
        ITunes {
            client,
            token: None, //Info about Enterprise for unblock the 20 x minute limit: https://affiliate.itunes.apple.com/resources/documentation/itunes-enterprise-partner-feed/ 
            rate_limit: 15, //In the public API documentation says 20 x minute aproximately, but let's play safe. (https://developer.apple.com/library/archive/documentation/AudioVideo/Conceptual/iTuneSearchAPI/Searching.html#//apple_ref/doc/uid/TP40017632-CH5-SW1)
            last_request: 0,
            release_cache: HashMap::new()
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
        match self.get(" https://itunes.apple.com/search", vec![("q", "test")]) {
            Ok(res) => if res.status() == StatusCode::OK { true } else { 
                error!("Failed validating iTunes token: {}", res.status());
                debug!("{:?}", res.text());
                false 
            },
            Err(e) => {
                error!("Failed validating iTunes token: {}", e);
                false
            }
        }
    }

    // Get request wrapper with rate limit
    fn get(&mut self, url: &str, query: Vec<(&str, &str)>) -> Result<Response, Box<dyn Error>> {
        debug!("iTunes GET {}", url);
        // Rate limit
        if self.last_request > 0 && self.rate_limit != -1 {
            let diff = timestamp!() - self.last_request;
            let req_ms = 1000_f64 / (self.rate_limit as f64 / 60_f64);
            let wait = diff as f64 - req_ms;
            if wait < 0_f64 {
                debug!("iTunes rate limit delay: {}", -wait);
                sleep(Duration::from_millis(-wait as u64));
            }
        }
        // Create request
        let mut request = self.client.get(url).query(&query);
        if self.token.is_some() {
            request = request.header("Authorization", format!("iTunes token={}", self.token.as_ref().unwrap()));
        }
        let response = request.send()?;
        
        // Rate limit
        let rate_limit_remaining = response.headers().get("X-iTunes-Ratelimit-Remaining")
            .map(|v| v.to_str().unwrap().parse::<i32>().ok()).flatten().unwrap_or(100);
        debug!("iTunes rate limit remaining: {}", rate_limit_remaining);
        if rate_limit_remaining < 1 {
            warn!("iTunes rate limit hit! Waiting 10s...");
            sleep(Duration::from_secs(10));
            return self.get(url, query);
        }

        // Save last reqeust time for rate limitting
        self.last_request = timestamp!();
        Ok(response)
    }

    // Search for tracks on iTunes
    pub fn search(&mut self, result_type: Option<&str>, query: Option<&str>, country: Option<&str>, language: Option<&str>) -> Result<Vec<ReleaseMasterSearchResult>, Box<dyn Error>> {

        // Generate parameters
        let mut qp = vec![];
        qp.push(("limit", "200")); //&TaggerConfig.itunes.max_results.to_string()));
        qp.push(("media", "music"));
        if let Some(t) = result_type {
            qp.push(("entity", t)); //song
        }
        if let Some(q) = query {
            qp.push(("term", q));
        }
        /*
        if let Some(p) = limit {
            qp.push(("limit", p.to_string()));
        }
        */
        if let Some(c) = country {
            qp.push(("country", c));
        }
        if let Some(l) = language {
            qp.push(("language", l));
        }
        // Get
        let response: Value = self.get("https://itunes.apple.com/search?", qp)?.json()?;
        let empty: Vec<Value> = vec![];
        let results: Vec<ReleaseMasterSearchResult> = response["results"].as_array().unwrap_or(&empty).into_iter().filter_map(|r| {
            // Filter only releases and masters
            let t = r["type"].as_str().unwrap_or("");
            if t == "release" || t == "master" {
                serde_json::from_value(r.to_owned()).ok()
            } else {
                None
            }
        }).collect();
        Ok(results)
    }

    // Get full release info
    pub fn full_release(&mut self, release_type: ReleaseType, id: i64) -> Result<ReleaseMaster, Box<dyn Error>> {
        // Check if cached
        if self.release_cache.contains_key(&id) {
            return Ok(self.release_cache.get(&id).unwrap().to_owned());
        }
        // Get 
        let rtype = match release_type {
            ReleaseType::Master => "masters",
            ReleaseType::Release => "releases"
        };
        let response = self.get(&format!("https://api.discogs.com/{}/{}", rtype, id), vec![])?.json()?;
        let release: ReleaseMaster = serde_json::from_value(response)?;
        // Cache
        self.release_cache.insert(id, release.clone());
        Ok(release)
    }
}

impl TrackMatcherST for ITunes {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>> {
        // Exact ID match
        if config.match_by_id && info.ids.discogs_release_id.is_some() {
            let release = self.full_release(ReleaseType::Release, info.ids.discogs_release_id.unwrap())?;
            // Exact track number match
            if let Some(track_number) = info.track_number {
                return Ok(Some((1.0, release.get_track(track_number as usize - 1, &config.itunes.styles))))
            }
            // Match inside release
            let mut tracks = vec![];
            for i in 0..release.tracks.len() {
                tracks.push(release.get_track(i, &config.itunes.styles));
            }
            return Ok(MatchingUtils::match_track(&info, &tracks, &config, false));
        }
        
        // Search
        let query = format!("{} {}", MatchingUtils::clean_title(info.title()?), info.artist()?);
        let mut results = self.search(Some("song"), Some(&query), None, None)?;
        // Fallback
        if results.is_empty() {
            info!("iTunes fallback search!");
            //results = self.search(Some("song"), None, Some(&MatchingUtils::clean_title(info.title()?)), Some(&info.artists.first().unwrap()))?;
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
            // Match artist
            // if !MatchingUtils::match_artist(&info.artists, &release.artists.iter().map(|a| a.name.clone()).collect(), config.strictness) {
            //     continue;
            // }
            // Match track
            let mut tracks = vec![];
            for i in 0..release.tracks.len() {
                tracks.push(release.get_track(i, &config.itunes.styles));
            }
            if let Some((acc, mut track)) = MatchingUtils::match_track(&info, &tracks, &config, false) {
                // Get catalog number if enabled from release rather than master
                if ((config.catalog_number && track.catalog_number.is_none()) || (config.label && track.label.is_none())) && (release.labels.is_none() && release.main_release.is_some()) {
                    info!("iTunes fetching release for catalog number/label...");
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
                        },
                        Err(e) => warn!("Failed fetching release info for catalog number! {}", e)
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
    Master
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
    pub id: i64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraArtist {
    pub name: String,
    pub id: i64,
    pub role: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ITunesTrack {
    pub position: String,
    pub title: String,
    pub artists: Option<Vec<ExtraArtist>>,
    pub duration: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: i64,
    pub name: String,
    pub catno: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub height: i64,
    pub width: i64,
    #[serde(rename = "uri")]
    pub url: String,
    #[serde(rename = "type")]
    pub image_type: String
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
    pub tracks: Vec<ITunesTrack>,
    pub released: Option<String>,
    pub main_release: Option<i64>
}

impl ReleaseMaster {
    // Remove (n) at end of artist
    pub fn clean_artist(input: &str) -> String {
        let re = Regex::new(r" \(\d{1,2}\)$").unwrap();
        re.replace(input, "").to_string()
    }

    pub fn get_track(&self, track_index: usize, styles_option: &ITunesStyles) -> Track {
        // Parse release date
        let release_date = match &self.released {
            Some(r) => NaiveDate::parse_from_str(&r, "%Y-%m-%d").ok(),
            None => None
        };

        // Generate styles and genres
        let mut styles = vec![];
        let mut genres = vec![];
        let styles_o = self.styles.clone().unwrap_or(vec![]);
        let genres_o = self.genres.clone();
        match styles_option {
            /*
            ITunesStyles::OnlyGenres => genres = genres_o,
            ITunesStyles::OnlyStyles => styles = styles_o,
            ITunesStyles::MergeToGenres => {
                genres = genres_o;
                genres.extend(styles_o);
            },
            ITunesStyles::MergeToStyles => {
                styles = styles_o;
                styles.extend(genres_o);
            },
            ITunesStyles::GenresToStyle => styles = genres_o,
            ITunesStyles::StylesToGenre => genres = styles_o,
            */
            // Default and custom
            _ => {
                genres = genres_o;
                styles = styles_o;
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
            platform: MusicPlatform::ITunes,
            title: self.tracks[track_index].title.to_string(),
            version: None,
            artists: match self.tracks[track_index].artists.as_ref() {
                // Use track artists if available
                Some(artists) => artists.iter().map(|a| ReleaseMaster::clean_artist(&a.name).to_string()).collect(),
                None => self.artists.iter().map(|a| ReleaseMaster::clean_artist(&a.name).to_string()).collect()
            },
            album_artists: self.artists.iter().map(|a| ReleaseMaster::clean_artist(&a.name).to_string()).collect(),
            album: Some(self.title.to_string()),
            key: None,
            bpm: None,
            genres,
            styles,
            art: match self.images.as_ref().unwrap_or(&Vec::new()).first() {
                Some(image) => Some(image.url.to_string()),
                None => None
            },
            url: self.url.to_string(),
            label: if let Some(labels) = &self.labels {
                if labels.is_empty() {
                    None
                } else {
                    Some(ReleaseMaster::clean_artist(&labels[0].name).to_string())
                }
            } else {None},
            release_year: self.year,
            release_date,
            publish_date: None,
            publish_year: None,
            catalog_number,
            other: vec![("VINYLTRACK".to_string(), self.tracks[track_index].position.to_string())],
            track_id: None,
            release_id: self.id.to_string(),
            duration: parse_duration(&self.tracks[track_index].duration).unwrap_or(Duration::ZERO)
        }
    }
}