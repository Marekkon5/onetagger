use std::error::Error;
use std::time::Duration;
use chrono::{NaiveDate, Datelike};
use onetagger_tagger::{AutotaggerSourceBuilder, AutotaggerSource, TaggerConfig, PlatformInfo, Track, AudioFileInfo, MatchingUtils, supported_tags};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use serde_json::{json, Value};

pub struct Bandcamp {
    client: Client
}

impl Bandcamp {
    /// Create new instance
    pub fn new() -> Bandcamp {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:85.0) Gecko/20100101 Firefox/85.0")
            .build()
            .unwrap();
        Bandcamp {
            client
        }
    }

    /// Search for tracks
    fn search_tracks(&self, query: &str) -> Result<Vec<BandcampSearchResult>, Box<dyn Error>> {
        let response = self.client.post("https://bandcamp.com/api/bcsearch_public_api/1/autocomplete_elastic")
            .json(&json!({
                "fan_id": null,
                "full_page": false,
                "search_filter": "t",
                "search_text": query
            }))
            .send()?;
        // No rate limit
        if response.status().is_success() {
            let r: Value = response.json()?;
            let results = serde_json::from_value(r["auto"]["results"].to_owned())?;
            return Ok(results)
        }
        // Rate limit
        warn!("Bandcamp rate limit... Waiting for few seconds");
        std::thread::sleep(Duration::from_secs(3));
        self.search_tracks(query)
    }

    /// Get data from track page
    fn track_page(&self, url: &str) -> Result<BandcampTrack, Box<dyn Error>> {
        // Fetch with rate limit
        let response = self.client.get(url).send()?;
        if response.status().is_client_error() {
            warn!("Bandcamp track page rate limit... Waiting for few seconds");
            std::thread::sleep(Duration::from_secs(3));
            return self.track_page(url);
        }
        let response = response.text()?;
        // Get <script> tag
        let document = Html::parse_document(&response);
        let selector = Selector::parse("script[type=\"application/ld+json\"]").unwrap();
        let elem = document.select(&selector).next().ok_or(format!("Missing <script> tag with data on: {url}"))?;
        let data: BandcampTrack = serde_json::from_str(&elem.text().collect::<Vec<_>>().join(""))?;
        Ok(data)
    }

}

impl AutotaggerSource for Bandcamp {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>> {
        // Search
        let query = format!("{} {}", info.artist()?, MatchingUtils::clean_title(info.title()?));
        debug!("Bandcamp q: {query}");
        let results = self.search_tracks(&query)?;
        let results: Vec<Track> = results.into_iter().map(|r| r.into()).collect();
        match MatchingUtils::match_track(info, &results, config, true) {
            Some((acc, track)) => {
                // Extend the track
                let track = self.track_page(&track.url)?;
                Ok(Some((acc, track.into())))
            },
            None => return Ok(None)
        }
    }
}

pub struct BandcampBuilder;

impl AutotaggerSourceBuilder for BandcampBuilder {
    fn new() -> Self where Self: Sized {
        BandcampBuilder
    }

    fn get_source(&mut self, _config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Box<dyn Error>> {
        Ok(Box::new(Bandcamp::new()))
    }

    fn info(&self) -> PlatformInfo {
        PlatformInfo {
            id: "bandcamp".to_string(),
            name: "Bandcamp".to_string(),
            description: "Specialized in indie artists. Limited amount of tags".to_string(),
            version: "1.0.0".to_string(),
            icon: include_bytes!("../assets/bandcamp.png"),
            max_threads: 4,
            custom_options: Default::default(),
            requires_auth: false,
            supported_tags: supported_tags!(Title, Artist, ReleaseDate, Album, Artist, Label, AlbumArt, Style, Genre, TrackId, URL, ReleaseId, TrackTotal)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BandcampSearchResult {
    pub id: i64,
    pub album_id: Option<i64>,
    pub band_id: i64,
    pub band_name: String,
    pub album_name: Option<String>,
    pub name: String,
    pub item_url_path: String
}

impl Into<Track> for BandcampSearchResult {
    fn into(self) -> Track {
        Track {
            platform: "bandcamp".to_string(),
            track_id: Some(self.id.to_string()),
            title: self.name,
            artists: vec![self.band_name],
            album: self.album_name,
            release_id: self.album_id.map(|a| a.to_string()).unwrap_or(String::new()),
            url: self.item_url_path,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BandcampTrack {
    pub name: String,
    pub description: Option<String>,
    pub duration: Option<String>,
    pub date_published: String,
    pub in_album: BandcampAlbumSmall,
    pub by_artist: BandcampArtistSmall,
    pub publisher: BandcampPublisherSmall,
    pub keywords: Option<Vec<String>>,
    pub image: String,
    #[serde(rename = "@id")]
    pub id: String
}

impl BandcampTrack {
    /// Try to parse date published
    pub fn date_published(&self) -> Option<NaiveDate> {
        let d = self.date_published.chars().take(11).collect::<String>();
        NaiveDate::parse_from_str(&d, "%d %b %Y").ok()
    }  
}

impl Into<Track> for BandcampTrack {
    fn into(self) -> Track {
        let genre = self.publisher.genre();
        Track {
            platform: "bandcamp".to_string(),
            release_date: self.date_published(),
            release_year: self.date_published().map(|d| d.year() as i16),
            title: self.name,
            album: Some(self.in_album.name),
            // Prioritize album artist, because it is more likely the artist
            artists: vec![self.in_album.by_artist.map(|a| a.name.to_owned()).unwrap_or(self.by_artist.name)],
            label: Some(self.publisher.name),
            art: Some(self.image),
            styles: self.keywords.unwrap_or(vec![]).into_iter()
                .filter(|k| 
                    Some(k.to_lowercase()) != genre.as_ref().map(|g| g.to_lowercase()) && 
                    crate::bandcamp_genres::GENRES.contains(&k.to_lowercase().trim())
                )
                .map(|s| capitalize(&s.replace(" and ", " & ")))
                .collect::<Vec<_>>(),
            genres: genre.map(|g| vec![g]).unwrap_or(vec![]),
            track_id: Some(self.id.clone()),
            url: self.id,
            release_id: self.in_album.id.unwrap_or(String::new()),
            track_total: self.in_album.num_tracks,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BandcampAlbumSmall {
    pub name: String,
    pub num_tracks: Option<u16>,
    #[serde(rename = "@id")]
    pub id: Option<String>,
    pub by_artist: Option<BandcampArtistSmall>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BandcampArtistSmall {
    pub name: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BandcampPublisherSmall {
    pub name: String,
    pub genre: Option<String>
}

impl BandcampPublisherSmall {
    /// Get genre of this song from url
    pub fn genre(&self) -> Option<String> {
        let genre = self.genre.as_ref()?.rsplit("/").next().unwrap().to_string();
        Some(capitalize(&genre))
    }
}

/// Capitalize every word
/// https://stackoverflow.com/questions/38406793/why-is-capitalizing-the-first-letter-of-a-string-so-convoluted-in-rust/38406885#38406885
fn capitalize(input: &str) -> String {
    input.split(" ").map(|w| {
        let mut c = w.trim().chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str()
        }
    }).collect::<Vec<_>>().join(" ")
}

#[test]
fn test_bandcamp() {
    let b = Bandcamp::new();
    let results = b.search_tracks("mr kill myself").unwrap();
    for result in results {
        println!("{}", result.item_url_path);
        b.track_page(&result.item_url_path).unwrap().date_published().unwrap();
    }
}