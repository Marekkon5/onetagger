use std::sync::{Arc, Mutex};
use std::error::Error;
use std::time::Duration;
use std::collections::HashMap;
use regex::{Regex, Captures};
use reqwest::blocking::Client;
use chrono::NaiveDate;
use scraper::{Html, Selector};
use serde::{Serialize, Deserialize};
use onetagger_tag::FrameName;
use onetagger_tagger::{Track, TaggerConfig, MusicPlatform, AutotaggerSource, AudioFileInfo, MatchingUtils, StylesOptions, TrackNumber, AutotaggerSourceBuilder, PlatformInfo};

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
            .build()
            .unwrap();
        Beatport {
            client, access_token
        }
    }

    /// Search for tracks on beatport
    pub fn search(&self, query: &str, page: i64, results_per_page: usize) -> Result<BeatportSearchResults, Box<dyn Error>> {
        let response = self.client.get("https://www.beatport.com/search/tracks")
            .query(&[
                ("q", query), 
                ("page", &page.to_string()),
                ("per-page", &results_per_page.to_string())
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
        let script = document.select(&selector).next().ok_or("No data found")?.text().collect::<Vec<_>>().join("");
        let start = script.find("window.Playables =").ok_or("No data found")? + 18;
        let end = script.find("window.Sliders =").unwrap_or_else(|| script.len());
        let mut data = script[start..end].trim().to_owned();
        // Remove trailing characters
        while !data.ends_with('}') {
            data.pop();
        }
        Ok(data)
    }

    /// Get full release info
    pub fn fetch_release(&self, slug: &str, id: i64) -> Result<BeatportRelease, Box<dyn Error>> {
        let response = self.client.get(format!("https://www.beatport.com/release/{}/{}", slug, id))
            .send()?
            .text()?;
        // Parse
        let json = self.get_playables(&response)?;
        let results: BeatportSearchResults = serde_json::from_str(&json)?;
        Ok(results.releases.first().ok_or("Missing release!")?.to_owned())
    }

    /// Get full track details
    pub fn fetch_track(&self, slug: &str, id: i64) -> Result<BeatportTrack, Box<dyn Error>> {
        let response = self.client.get(format!("https://www.beatport.com/track/{}/{}", slug, id))
            .send()?.text()?;
        let json = self.get_playables(&response)?;
        let results: BeatportSearchResults = serde_json::from_str(&json)?;
        Ok(results.tracks.first().ok_or("Missing track data!")?.to_owned())
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
        let response: BeatportAPITrack = self.client.get(&format!("https://api.beatport.com/v4/catalog/tracks/{}", id))
            .bearer_auth(token)
            .send()?.json()?;
        Ok(response)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportOAuth {
    pub access_token: String,
    pub expires_in: u128
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportSearchResults {
    pub tracks: Vec<BeatportTrack>,
    pub releases: Vec<BeatportRelease>
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
    pub remixers: Option<Vec<BeatportSmall>>,
    pub exclusive: Option<bool>
}

// TODO: Track from private API has different data!
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportAPITrack {
    pub slug: String,
    pub id: i64,
    pub number: i32,
    pub isrc: Option<String>
}

impl BeatportAPITrack {
    /// Get track number struct
    pub fn track_number(&self) -> TrackNumber {
        if self.number == 0 {
            TrackNumber::Number(1)
        } else {
            TrackNumber::Number(self.number)
        }
    }
}

impl BeatportTrack {
    pub fn to_track(&self, art_resolution: u32) -> Track {
        let mut t = Track {
            platform: MusicPlatform::Beatport,
            title: self.name.to_string(),
            version: self.mix.as_ref().map(String::from),
            artists: self.artists.iter().map(|a| a.name.to_string()).collect(),
            album_artists: vec![],
            album: Some(self.release.name.to_string()),
            bpm: self.bpm.clone(),
            genres: self.genres.iter().map(|g| g.name.to_string()).collect(),
            styles: self.sub_genres.as_ref().unwrap_or(&Vec::new()).iter().map(|g| g.name.to_string()).collect(),
            label: self.label.as_ref().map(|l| l.name.to_string()),
            url: format!("https://beatport.com/track/{}/{}", &self.slug, &self.id),
            // Parse year only if 4 digits
            release_year: if let Some(date) = &self.date.released {
                if date.len() == 4 { date.parse().ok() } else { None }
            } else { None },
            publish_year: if let Some(date) = &self.date.published {
                if date.len() == 4 { date.parse().ok() } else { None }
            } else { None },
            // Dates
            release_date: self.date.released.as_ref().map_or(None, |d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
            publish_date: self.date.published.as_ref().map_or(None, |d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
            // Key
            key: self.key.as_ref().map(|k| k
                .replace("♭", "b")
                .replace("♯", "#")
                .replace("min", "m")
                .replace("maj", "")
                .replace(" ", "")
                .to_owned()
            ),
            catalog_number: None,
            art: self.get_image().map(|i| i.get_url(art_resolution)).flatten(),
            other: vec![
                (FrameName::same("UNIQUEFILEID"), format!("https://beatport.com|{}", &self.id))
            ],
            track_id: Some(self.id.to_string()),
            release_id: self.release.id.to_string(),
            duration: self.duration.to_duration(),
            remixers: self.remixers.clone().unwrap_or(vec![]).into_iter().map(|r| r.name).collect(),
            track_number: None,
            isrc: None
        };

        // Exclusive beatport tag
        if self.exclusive.is_some() && self.exclusive.unwrap() {
            t.other.push((FrameName::same("BEATPORT_EXCLUSIVE"), "1".to_string()));
        }
        t
    }

    // Get dynamic or first image
    fn get_image(&self) -> Option<&BeatportImage> {
        // Prioritize dynamic image
        if let Some(image) = self.images.get("dynamic") {
            if !image.url.contains(INVALID_ART) {
                return Some(image);
            }
        }
        self.images.iter().filter(|(k, v)| {
            *k != "dynamic" && !v.url.contains(INVALID_ART)
        }).map(|(_k, v)| v).next()
    }
}

// Generic container 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportSmall {
    pub id: i64,
    pub name: String,
    pub slug: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportDate {
    pub published: Option<String>,
    pub released: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportImage {
    pub id: i64,
    pub url: String
}

// Currently only used for catalog number
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatportRelease {
    pub id: i64,
    pub slug: String,
    pub catalog: Option<String>,
    pub artists: Vec<BeatportSmall>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatportDuration {
    pub milliseconds: Option<u64>,
    pub minutes: Option<String>
}

impl BeatportDuration {
    pub fn to_duration(&self) -> Duration {
        if let Some(ms) = self.milliseconds {
            return Duration::from_millis(ms);
        }
        if let Some(m) = self.minutes.as_ref() {
            return MatchingUtils::parse_duration(&m).unwrap_or(Duration::ZERO);
        }
        Duration::ZERO
    }
}

impl BeatportImage {
    pub fn get_url(&self, resolution: u32) -> Option<String> {
        if self.url.contains("ab2d1d04-233d-4b08-8234-9782b34dcab8") {
            return None;
        }

        let r = resolution.to_string();
        let dynamic = &self.url;
        // Normal dynamic
        if dynamic.contains("{w}") || dynamic.contains("{x}") {
            return Some(dynamic
                .replace("{w}", &r)
                .replace("{h}", &r)
                .replace("{x}", &r)
                .replace("{y}", &r)
                .to_owned());
        }
        // Undocumented dynamic
        if dynamic.contains("/image_size/") {
            let re = Regex::new(r"/image_size/\d+x\d+/").unwrap();
            return Some(re.replace(&dynamic, |_: &Captures| format!("/image_size/{}x{}/", r, r)).to_string());
        }
        Some(dynamic.to_owned())
    }
}

// Match track
impl AutotaggerSource for Beatport {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>> {       
        // Fetch by ID
        if let Some(id) = info.ids.beatport_track_id {
            info!("Fetching by ID: {}", id);
            // TODO: Serialize properly the private API response, rather than double request
            let api_track = self.fetch_track_embed(id)?;
            let bp_track = self.fetch_track(&api_track.slug, api_track.id)?;
            let mut track = bp_track.to_track(config.beatport.art_resolution);
            track.isrc = api_track.isrc.clone();
            track.track_number = Some(api_track.track_number());
            return Ok(Some((1.0, track)));
        }

        // Search
        let query = format!("{} {}", info.artist()?, MatchingUtils::clean_title(info.title()?));
        for page in 1..config.beatport.max_pages+1 {
            match self.search(&query, page, 25) {
                Ok(res) => {
                    // Convert tracks
                    let tracks = res.tracks.iter().map(|t| t.to_track(config.beatport.art_resolution)).collect();
                    // Match
                    if let Some((f, mut track)) = MatchingUtils::match_track(&info, &tracks, &config, true) {
                        let i = tracks.iter().position(|t| t == &track).unwrap();
                        // Data from release
                        if config.catalog_number || config.album_artist {
                            info!("Fetching full release for extra metadata.");
                            match self.fetch_release(&res.tracks[i].release.slug, res.tracks[i].release.id) {
                                Ok(r) => {
                                    track.catalog_number = r.catalog;
                                    track.album_artists = r.artists.into_iter().map(|a| a.name).collect();
                                },
                                Err(e) => warn!("Beatport failed fetching release for catalog number! {}", e)
                            }
                        }
                        // Get style info
                        if config.style && track.styles.is_empty() {
                            info!("Fetching full track for subgenres!");
                            match self.fetch_track(&res.tracks[i].slug, res.tracks[i].id) {
                                Ok(t) => {
                                    debug!("New subgenres: {:?}", t.sub_genres);
                                    track.styles = t.sub_genres.unwrap_or(Vec::new()).into_iter().map(|g| g.name).collect();
                                },
                                Err(e) => warn!("Beatport failed fetching full track data for subgenres! {}", e)
                            }
                        }
                        // Data from API for track number
                        if config.track_number || config.isrc {
                            info!("Fetching track info from API for track number!");
                            match self.fetch_track_embed(res.tracks[i].id) {
                                Ok(t) => {
                                    track.track_number = Some(t.track_number());
                                    track.isrc = t.isrc;
                                },
                                Err(e) => warn!("Beatport failed fetching full API track data for track number! {}", e)
                            }
                        }

                        // Apply style config similar way to Discogs
                        let genres = track.genres.clone();
                        let styles = track.styles.clone();
                        match config.styles_options {
                            StylesOptions::OnlyGenres => track.styles = vec![],
                            StylesOptions::OnlyStyles => track.genres = vec![],
                            StylesOptions::MergeToGenres => {
                                track.genres.extend(styles);
                                track.styles = vec![];
                            },
                            StylesOptions::MergeToStyles => {
                                track.styles.extend(genres);
                                track.genres = vec![];
                            },
                            StylesOptions::StylesToGenre => {
                                track.genres = styles;
                                track.styles = vec![];
                            },
                            StylesOptions::GenresToStyle => {
                                track.styles = genres;
                                track.genres = vec![];
                            },
                            _ => {}
                        }
                        
                        return Ok(Some((f, track)));
                    }
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
    fn new(_config: &TaggerConfig) -> BeatportBuilder {
        BeatportBuilder {
            access_token: Arc::new(Mutex::new(None))
        }
    }

    fn get_source(&mut self) -> Result<Box<dyn AutotaggerSource>, Box<dyn Error>> {
        Ok(Box::new(Beatport::new(self.access_token.clone())))
    }

    fn info(&self) -> PlatformInfo {
        todo!()
    }
}