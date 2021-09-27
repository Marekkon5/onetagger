use chrono::NaiveDate;
use regex::{Captures, Regex};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::tagger::matcher::Matcher;
use crate::tagger::{
    parse_duration, BeatportID, MusicPlatform, StylesOptions, TaggerConfig, Track, TrackMatcher,
};

const INVALID_ART: &'static str = "ab2d1d04-233d-4b08-8234-9782b34dcab8";

pub struct Beatport {
    client: Client,
    // TODO: Share token if used properly in future
    access_token: Arc<Mutex<Option<BeatportOAuth>>>,
}

impl Beatport {
    /// Create new instance
    pub fn new() -> Beatport {
        let client = Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:85.0) Gecko/20100101 Firefox/85.0",
            )
            .build()
            .unwrap();
        Beatport {
            client,
            access_token: Arc::new(Mutex::new(None)),
        }
    }

    /// Search for tracks on beatport
    pub fn search(
        &self,
        query: &str,
        page: i64,
        results_per_page: usize,
    ) -> Result<BeatportSearchResults, Box<dyn Error>> {
        let response = self
            .client
            .get("https://www.beatport.com/search/tracks")
            .query(&[
                ("q", query),
                ("page", &page.to_string()),
                ("per-page", &results_per_page.to_string()),
            ])
            .send()?
            .text()?;
        // Parse JSON
        let json = self.get_playables(&response)?;
        let results: BeatportSearchResults = serde_json::from_str(&json)?;
        Ok(results)
    }

    /// Get JSON data from website
    fn get_playables(&self, response: &str) -> Result<String, Box<dyn Error>> {
        let document = Html::parse_document(&response);
        let selector = Selector::parse("script#data-objects").unwrap();
        let script = document
            .select(&selector)
            .next()
            .ok_or("No data found")?
            .text()
            .collect::<Vec<_>>()
            .join("");
        let start = script.find("window.Playables =").ok_or("No data found")? + 18;
        let end = script
            .find("window.Sliders =")
            .unwrap_or_else(|| script.len());
        let mut data = script[start..end].trim().to_owned();
        // Remove trailing characters
        while !data.ends_with('}') {
            data.pop();
        }
        Ok(data)
    }

    /// Get full release info
    pub fn fetch_release(&self, slug: &str, id: i64) -> Result<BeatportRelease, Box<dyn Error>> {
        let response = self
            .client
            .get(format!("https://www.beatport.com/release/{}/{}", slug, id))
            .send()?
            .text()?;
        // Parse
        let json = self.get_playables(&response)?;
        let results: BeatportSearchResults = serde_json::from_str(&json)?;
        Ok(results
            .releases
            .first()
            .ok_or("Missing release!")?
            .to_owned())
    }

    /// Get full track details
    pub fn fetch_track(&self, slug: &str, id: i64) -> Result<BeatportTrack, Box<dyn Error>> {
        let response = self
            .client
            .get(format!("https://www.beatport.com/track/{}/{}", slug, id))
            .send()?
            .text()?;
        let json = self.get_playables(&response)?;
        let results: BeatportSearchResults = serde_json::from_str(&json)?;
        Ok(results
            .tracks
            .first()
            .ok_or("Missing track data!")?
            .to_owned())
    }

    /// Update embed auth token
    pub fn update_token(&self) -> Result<String, Box<dyn Error>> {
        let mut token = self.access_token.lock().unwrap();
        // Fetch new if doesn't exist
        if (*token).is_none() {
            let mut response: BeatportOAuth = self
                .client
                .get("https://embed.beatport.com/token")
                .send()?
                .json()?;
            response.expires_in = response.expires_in * 1000 + timestamp!() - 60000;
            *token = Some(response);
        }
        // Expired
        let t = token.clone().unwrap();
        if t.expires_in <= timestamp!() {
            *token = None;
            return self.update_token();
        }
        debug!("OAuth: {:?}", t);
        Ok(t.access_token)
    }

    /// Fetch track using private embed API
    pub fn fetch_track_embed(&self, id: i64) -> Result<BeatportAPITrack, Box<dyn Error>> {
        let token = self.update_token()?;
        let response: BeatportAPITrack = self
            .client
            .get(&format!(
                "https://api.beatport.com/v4/catalog/tracks/{}",
                id
            ))
            .bearer_auth(token)
            .send()?
            .json()?;
        Ok(response)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportOAuth {
    pub access_token: String,
    pub expires_in: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportSearchResults {
    pub tracks: Vec<BeatportTrack>,
    pub releases: Vec<BeatportRelease>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportTrack {
    pub artists: Vec<BeatportSmall>,
    pub bpm: Option<i64>,
    pub date: BeatportDate,
    pub genres: Vec<BeatportSmall>,
    pub id: i64,
    pub images: HashMap<String, BeatportImage>,
    pub key: Option<String>,
    pub label: Option<BeatportSmall>,
    pub mix: Option<String>,
    pub name: String,
    pub release: BeatportSmall,
    pub slug: String,
    pub title: Option<String>,
    pub duration: BeatportDuration,
    pub sub_genres: Option<Vec<BeatportSmall>>,
}

// TODO: Track from private API has different data!
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportAPITrack {
    pub slug: String,
    pub id: i64,
}

impl BeatportTrack {
    pub fn to_track(&self, art_resolution: i64) -> Track {
        Track {
            platform: Some(MusicPlatform::Beatport),
            title: self.title,
            name: Some(self.name),
            mix: self.mix,
            artists: Some(self.artists.iter().map(|a| a.name.to_string()).collect()),
            album: Some(self.release.name.to_string()),
            bpm: self.bpm.clone(),
            genres: Some(self.genres.iter().map(|g| g.name.to_string()).collect()),
            styles: Some(
                self.sub_genres
                    .as_ref()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .map(|g| g.name.to_string())
                    .collect(),
            ),
            label: self.label.as_ref().map(|l| l.name.to_string()),
            //url: format!("https://beatport.com/track/{}/{}", &self.slug, &self.id),
            // Parse year only if 4 digits
            release_year: if let Some(date) = &self.date.released {
                if date.len() == 4 {
                    date.parse().ok()
                } else {
                    None
                }
            } else {
                None
            },
            publish_year: if let Some(date) = &self.date.published {
                if date.len() == 4 {
                    date.parse().ok()
                } else {
                    None
                }
            } else {
                None
            },
            // Dates
            release_date: self
                .date
                .released
                .as_ref()
                .map_or(None, |d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
            publish_date: self
                .date
                .published
                .as_ref()
                .map_or(None, |d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
            // Key
            key: self.key.as_ref().map(|k| {
                k.replace("♭", "b")
                    .replace("♯", "#")
                    .replace("min", "m")
                    .replace("maj", "")
                    .replace(" ", "")
                    .to_owned()
            }),
            artwork_url: self
                .get_image()
                .map(|i| i.get_url(art_resolution))
                .flatten(),
            duration: Some(self.duration.to_duration()),
            beatport: Some(BeatportID {
                track_id: self.id,
                release_id: self.release.id,
                track_url: format!("https://beatport.com/track/{}/{}", &self.slug, &self.id),
                release_url: format!(
                    "https://beatport.com/release/{}/{}",
                    &self.release.slug, &self.release.id
                ),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    // Get dynamic or first image
    fn get_image(&self) -> Option<&BeatportImage> {
        // Prioritize dynamic image
        if let Some(image) = self.images.get("dynamic") {
            if !image.url.contains(INVALID_ART) {
                return Some(image);
            }
        }
        self.images
            .iter()
            .filter(|(k, v)| *k != "dynamic" && !v.url.contains(INVALID_ART))
            .map(|(_k, v)| v)
            .next()
    }
}

// Generic container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportSmall {
    pub id: i64,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportDate {
    pub published: Option<String>,
    pub released: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportImage {
    pub id: i64,
    pub url: String,
}

// Currently only used for catalog number
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatportRelease {
    pub id: i64,
    pub slug: String,
    pub catalog: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportDuration {
    pub milliseconds: Option<u64>,
    pub minutes: Option<String>,
}

impl BeatportDuration {
    pub fn to_duration(&self) -> Duration {
        if let Some(ms) = self.milliseconds {
            return Duration::from_millis(ms);
        }
        if let Some(m) = self.minutes.as_ref() {
            return parse_duration(&m).unwrap_or(Duration::ZERO);
        }
        Duration::ZERO
    }
}

impl BeatportImage {
    pub fn get_url(&self, resolution: i64) -> Option<String> {
        if self.url.contains("ab2d1d04-233d-4b08-8234-9782b34dcab8") {
            return None;
        }

        let r = resolution.to_string();
        let dynamic = &self.url;
        // Normal dynamic
        if dynamic.contains("{w}") || dynamic.contains("{x}") {
            return Some(
                dynamic
                    .replace("{w}", &r)
                    .replace("{h}", &r)
                    .replace("{x}", &r)
                    .replace("{y}", &r)
                    .to_owned(),
            );
        }
        // Undocumented dynamic
        if dynamic.contains("/image_size/") {
            let re = Regex::new(r"/image_size/\d+x\d+/").unwrap();
            return Some(
                re.replace(&dynamic, |_: &Captures| format!("/image_size/{}x{}/", r, r))
                    .to_string(),
            );
        }
        Some(dynamic.to_owned())
    }
}

// Match track
impl TrackMatcher for Beatport {
    fn match_track(
        &self,
        local: &Track,
        config: &TaggerConfig,
    ) -> Result<Option<(f64, Track)>, Box<dyn Error>> {
        // Fetch by ID
        /*
        if let Some(id) = local.ids.beatport_track_id {
            info!("Fetching by ID: {}", id);
            // TODO: Serialize properly the private API response, rather than double request
            let track = self.fetch_track_embed(id)?;
            let track = self.fetch_track(&track.slug, track.id)?;
            return Ok(Some((1.0, track.to_track(config.beatport.art_resolution))));
        }
        */

        // Search
        let query = format!(
            "{} {}",
            local.artist.unwrap_or_default(),
            local.title.unwrap_or_default()
        );
        for page in 1..config.beatport.max_pages + 1 {
            match self.search(&query, page, 150) {
                Ok(res) => {
                    // Convert tracks
                    let tracks = res
                        .tracks
                        .iter()
                        .map(|t| t.to_track(config.beatport.art_resolution))
                        .collect();
                    // Match
                    if let Some((f, mut track)) = Matcher::match_track(&local, &tracks, &config) {
                        let i = tracks.iter().position(|t| t == &track).unwrap();
                        // Get catalog number
                        if config.catalog_number {
                            info!("Fetching full release for catalog number!");
                            match self.fetch_release(
                                &res.tracks[i].release.slug,
                                res.tracks[i].release.id,
                            ) {
                                Ok(r) => track.catalog_number = r.catalog,
                                Err(e) => warn!(
                                    "Beatport failed fetching release for catalog number! {}",
                                    e
                                ),
                            }
                        }
                        // Get style info
                        if config.style && track.styles.unwrap().is_empty() {
                            info!("Fetching full track for subgenres!");
                            match self.fetch_track(&res.tracks[i].slug, res.tracks[i].id) {
                                Ok(t) => {
                                    debug!("New subgenres: {:?}", t.sub_genres);
                                    track.styles = Some(
                                        t.sub_genres
                                            .unwrap_or(Vec::new())
                                            .into_iter()
                                            .map(|g| g.name)
                                            .collect(),
                                    );
                                }
                                Err(e) => warn!(
                                    "Beatport failed fetching full track data for subgenres! {}",
                                    e
                                ),
                            }
                        }

                        // Apply style config similar way to Discogs
                        let genres = track.genres.clone();
                        let styles = track.styles.clone();
                        match config.styles_options {
                            StylesOptions::OnlyGenres => track.styles = None,
                            StylesOptions::OnlyStyles => track.genres = None,
                            /*
                            StylesOptions::MergeToGenres => {
                                track.genres = track.genres.extend(styles);
                                track.styles = None;
                            }
                            StylesOptions::MergeToStyles => {
                                track.styles = track.styles.extend(genres);
                                track.genres = None;
                            }
                            StylesOptions::StylesToGenre => {
                                track.genres = styles;
                                track.styles = None;
                            }
                            StylesOptions::GenresToStyle => {
                                track.styles = genres;
                                track.genres = None;
                            }
                            */
                            _ => {}
                        }
                        return Ok(Some((f, track)));
                    }
                }
                Err(e) => {
                    warn!("Beatport search failed, query: {}. {}", query, e);
                    return Ok(None);
                }
            }
        }
        Ok(None)
    }
}
