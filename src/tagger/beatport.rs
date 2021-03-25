use std::error::Error;
use regex::{Regex, Captures};
use reqwest::blocking::Client;
use serde_json::Value;
use chrono::NaiveDate;
use scraper::{Html, Selector};

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
    pub fn search_tracks(&self, query: &str, page: i64, config: &TaggerConfig) -> Result<Vec<Track>, Box<dyn Error>> {
        let response = self.client.get("https://www.beatport.com/search/tracks")
            .query(&[("q", query), ("page", &page.to_string())])
            .send()?
            .text()?;

        //Get JSON
        let document = Html::parse_document(&response);
        let selector = Selector::parse("script#data-objects").unwrap();
        let script = document.select(&selector).next().ok_or("No data found")?.text().collect::<Vec<_>>().join("");
        let start = script.find("window.Playables =").ok_or("No data found")? + 18;
        let end = script.find("window.Sliders = []").unwrap_or_else(|| script.len());
        let mut data = script[start..end].trim().to_owned();
        //Remove `;`
        data.pop();
        //Parse JSON
        let json: Value = serde_json::from_str(&data)?;
        let tracks = json["tracks"].as_array().ok_or("Missing tracks")?.iter().filter_map(
            |t| self.parse_track(&t, config.beatport.art_resolution)
        ).collect();

        Ok(tracks)
    }

    fn parse_track(&self, json: &Value, art_resolution: i64) -> Option<Track> {
        //Get title
        let title = if let Some(title) = json["title"].as_str() {
            if title.len() == 0 || title == " " {
                //Empty title = use name + mix
                if let Some(mix) = json["mix"].as_str() {
                    format!("{} ({})", json["name"].as_str()?, mix)
                } else {
                    json["name"].as_str()?.to_owned()
                }
            } else {
                title.to_owned()
            }
        } else {
            //Empty title = use name + mix
            if let Some(mix) = json["mix"].as_str() {
                format!("{} ({})", json["name"].as_str()?, mix)
            } else {
                json["name"].as_str()?.to_owned()
            }
        };
        //Used for json unwrapping
        let empty_vec: Vec<Value> = Vec::new();
        
        //Parse
        Some(Track {
            platform: MusicPlatform::Beatport, 
            title,
            version: json["mix"].as_str().map(String::from),
            artists: json["artists"].as_array().unwrap_or(&empty_vec).iter().filter_map(
                |a| a["name"].as_str().map(String::from)
            ).collect(),
            album: json["release"]["name"].as_str().map(String::from),
            key: if let Some(key) = json["key"].as_str() {
                //Convert to ID3 key
                Some(key.replace("♭", "b")
                    .replace("♯", "#")
                    .replace("min", "m")
                    .replace("maj", "")
                    .replace(" ", ""))
            } else {None},
            bpm: json["bpm"].as_i64(),
            art: if let Some(url) = json["images"]["dynamic"]["url"].as_str() 
                {Some(self.art_url(url, art_resolution))} else {None},
            genres: json["genres"].as_array().unwrap_or(&empty_vec).iter().filter_map(
                |g| g["name"].as_str().map(String::from)
            ).collect(),
            styles: vec![],
            url: Some(format!("https://beatport.com/track/{}/{}", json["slug"].as_str()?, json["id"].as_i64()?)),
            //Parse year only if 4 digits
            release_year: if let Some(date) = json["date"]["released"].as_str() {
                if date.len() == 4 {
                    date.parse().ok()
                } else {None}} else {None},
            publish_year: if let Some(date) = json["date"]["published"].as_str() {
                if date.len() == 4 {
                    date.parse().ok()
                } else {None}} else {None},
            //Date
            release_date: if let Some(date) = json["date"]["released"].as_str() 
                { NaiveDate::parse_from_str(date, "%Y-%m-%d").ok() } else { None },
            publish_date: if let Some(date) = json["date"]["published"].as_str() 
                { NaiveDate::parse_from_str(date, "%Y-%m-%d").ok() } else { None },
            label: json["label"]["name"].as_str().map(String::from)
        })
    }

    //Generate art url from dynamic one
    fn art_url(&self, dynamic: &str, resolution: i64) -> String {
        let r = resolution.to_string();
        //Normal dynamic
        if dynamic.contains("{w}") || dynamic.contains("{x}") {
            return dynamic.replace("{w}", &r)
                .replace("{h}", &r)
                .replace("{x}", &r)
                .replace("{y}", &r)
                .to_owned();
        }
        //Undocumented dynamic
        if dynamic.contains("/image_size/") {
            let re = Regex::new(r"/image_size/\d+x\d+/").unwrap();
            return re.replace(&dynamic, |_: &Captures| format!("/image_size/{}x{}/", r, r)).to_string();
        }

        dynamic.to_owned()
    }   

}


//Match track
impl TrackMatcher for Beatport {
    fn match_track(&self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>> {
        //Search
        let query = format!("{} {}", info.artists.first().unwrap(), MatchingUtils::clean_title(&info.title));
        for page in 1..config.beatport.max_pages+1 {
            match self.search_tracks(&query, page, &config) {
                Ok(tracks) => {
                    //Got tracks
                    if tracks.is_empty() {
                        return Ok(None);
                    }
                    //Match
                    if let Some((f, track)) = MatchingUtils::match_track(&info, &tracks, &config) {
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