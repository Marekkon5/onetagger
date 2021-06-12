use std::error::Error;
use std::collections::HashMap;
use regex::{Regex, Captures};
use reqwest::blocking::Client;
use chrono::NaiveDate;
use scraper::{Html, Selector};
use serde::{Serialize, Deserialize};

use crate::tagger::{Track, TaggerConfig, MusicPlatform, TrackMatcher, AudioFileInfo, MatchingUtils};

pub struct Beatport {
    client: Client
}

impl Beatport {
    //Create instance
    pub fn new() -> Beatport {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:85.0) Gecko/20100101 Firefox/85.0")
            .build()
            .unwrap();
        Beatport {
            client
        }
    }

    //Search for tracks on beatport
    pub fn search(&self, query: &str, page: i64) -> Result<BeatportSearchResults, Box<dyn Error>> {
        let response = self.client.get("https://www.beatport.com/search/tracks")
            .query(&[("q", query), ("page", &page.to_string())])
            .send()?
            .text()?;
        
        //Parse JSON
        let json = self.get_playables(&response)?;
        let results: BeatportSearchResults = serde_json::from_str(&json)?;
        Ok(results)
    }

    //Get JSON data from website
    fn get_playables(&self, response: &str) -> Result<String, Box<dyn Error>> {
        let document = Html::parse_document(&response);
        let selector = Selector::parse("script#data-objects").unwrap();
        let script = document.select(&selector).next().ok_or("No data found")?.text().collect::<Vec<_>>().join("");
        let start = script.find("window.Playables =").ok_or("No data found")? + 18;
        let end = script.find("window.Sliders =").unwrap_or_else(|| script.len());
        let mut data = script[start..end].trim().to_owned();
        //Remove trailing characters
        while !data.ends_with('}') {
            data.pop();
        }
        Ok(data)
    }

    //Get release info
    pub fn fetch_release(&self, slug: &str, id: i64) -> Result<BeatportRelease, Box<dyn Error>> {
        let response = self.client.get(format!("https://www.beatport.com/release/{}/{}", slug, id))
            .send()?
            .text()?;
        //Parse
        let json = self.get_playables(&response)?;
        let results: BeatportSearchResults = serde_json::from_str(&json)?;
        Ok(results.releases.first().ok_or("Missing release!")?.to_owned())
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatportSearchResults {
    pub tracks: Vec<BeatportTrack>,
    pub releases: Vec<BeatportRelease>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
}

impl BeatportTrack {
    pub fn to_track(&self, art_resolution: i64) -> Track {
        //If no title use name + mix
        let mut title = self.title.as_ref().unwrap_or(&String::new()).to_owned();
        if title.trim().is_empty() {
            if let Some(mix) = &self.mix {
                title = format!("{} ({})", self.name, mix);
            } else {
                title = self.name.to_string();
            }
        }

        Track {
            platform: MusicPlatform::Beatport,
            title,
            version: self.mix.as_ref().map(String::from),
            artists: self.artists.iter().map(|a| a.name.to_string()).collect(),
            album: Some(self.release.name.to_string()),
            bpm: self.bpm.clone(),
            genres: self.genres.iter().map(|g| g.name.to_string()).collect(),
            styles: vec![],
            label: self.label.as_ref().map(|l| l.name.to_string()),
            url: Some(format!("https://beatport.com/track/{}/{}", &self.slug, &self.id)),
            //Parse year only if 4 digits
            release_year: if let Some(date) = &self.date.released {
                if date.len() == 4 { date.parse().ok() } else { None }
            } else { None },
            publish_year: if let Some(date) = &self.date.published {
                if date.len() == 4 { date.parse().ok() } else { None }
            } else { None },
            //Dates
            release_date: self.date.released.as_ref().map_or(None, |d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
            publish_date: self.date.published.as_ref().map_or(None, |d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
            //Key
            key: self.key.as_ref().map(|k| k
                .replace("♭", "b")
                .replace("♯", "#")
                .replace("min", "m")
                .replace("maj", "")
                .replace(" ", "")
                .to_owned()
            ),
            catalog_number: None,
            art: self.get_image().map(|i| i.get_url(art_resolution)).flatten()
        }
    }

    //Get dynamic or first image
    fn get_image(&self) -> Option<BeatportImage> {
        Some(self.images.get("dynamic").unwrap_or(self.images.values().next()?).clone())
    }
}

//Generic container 
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

//Currently only used for catalog number
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatportRelease {
    pub id: i64,
    pub slug: String,
    pub catalog: Option<String>
}

impl BeatportImage {
    pub fn get_url(&self, resolution: i64) -> Option<String> {
        if self.url.contains("ab2d1d04-233d-4b08-8234-9782b34dcab8") {
            return None;
        }

        let r = resolution.to_string();
        let dynamic = &self.url;
        //Normal dynamic
        if dynamic.contains("{w}") || dynamic.contains("{x}") {
            return Some(dynamic
                .replace("{w}", &r)
                .replace("{h}", &r)
                .replace("{x}", &r)
                .replace("{y}", &r)
                .to_owned());
        }
        //Undocumented dynamic
        if dynamic.contains("/image_size/") {
            let re = Regex::new(r"/image_size/\d+x\d+/").unwrap();
            return Some(re.replace(&dynamic, |_: &Captures| format!("/image_size/{}x{}/", r, r)).to_string());
        }
        Some(dynamic.to_owned())
    }
}

//Match track
impl TrackMatcher for Beatport {
    fn match_track(&self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>> {
        //Search
        let query = format!("{} {}", info.artists.first().unwrap(), MatchingUtils::clean_title(&info.title));
        for page in 1..config.beatport.max_pages+1 {
            match self.search(&query, page) {
                Ok(res) => {
                    //Convert tracks
                    let tracks = res.tracks.iter().map(|t| t.to_track(config.beatport.art_resolution)).collect();

                    //Match
                    if let Some((f, mut track)) = MatchingUtils::match_track(&info, &tracks, &config) {
                        //Get catalog number
                        if config.catalog_number {
                            let i = tracks.iter().position(|t| t == &track).unwrap();
                            match self.fetch_release(&res.tracks[i].release.slug, res.tracks[i].release.id) {
                                Ok(r) => track.catalog_number = r.catalog,
                                Err(e) => warn!("Beatport failed fetching release for catalog number! {}", e)
                            }
                        }
                        
                        return Ok(Some((f, track)));
                    }
                },
                Err(_) => {
                    return Ok(None);
                }
            }
        }
        Ok(None)
    }
}