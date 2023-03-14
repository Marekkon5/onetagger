use std::error::Error;
use std::time::Duration;
use chrono::NaiveDate;
use onetagger_tagger::{Track, TaggerConfig, AutotaggerSource, AudioFileInfo, MatchingUtils, TrackNumber, AutotaggerSourceBuilder, PlatformInfo, PlatformCustomOptions, PlatformCustomOptionValue};
use reqwest::blocking::Client;
use scraper::{Html, Selector};


struct RateYourMusic {
    client: Client,
    config: RYMConfig
}

impl RateYourMusic {
    /// Create new instance
    pub fn new(config: RYMConfig) -> RateYourMusic {
        RateYourMusic { 
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:85.0) Gecko/20100101 Firefox/85.0")
                .build()
                .unwrap(),
            config
        }
    }

    /// Search for albums
    pub fn search(&self, query: &str) -> Result<Vec<RYMSearchRelease>, Box<dyn Error>> {
        let data = self.client.get("https://rateyourmusic.com/search")
            .query(&[
                ("searchterm", query),
                ("searchtype", "l")
            ])
            .send()?
            .error_for_status()?
            .text()?;

        // Parse
        let data = String::from_utf8(minify_html::minify(data.as_bytes(), &minify_html::Cfg::spec_compliant()))?;
        let document = Html::parse_document(&data);
        
        // Get album links
        let mut releases = vec![];
        let selector = Selector::parse(".searchpage").unwrap();
        for element in document.select(&selector) {
            // let name = element.text().collect::<String>();
            if let Some(url) = element.value().attr("href") {
                releases.push(RYMSearchRelease { url: format!("https://rateyourmusic.com{url}") });
            }
        }

        Ok(releases)
    }

    /// Get release
    pub fn release(&self, url: &str) -> Result<RYMRelease, Box<dyn Error>> {
        // Fetch, parse
        let data = self.client.get(url).send()?.error_for_status()?.text()?;
        let data = String::from_utf8(minify_html::minify(data.as_bytes(), &minify_html::Cfg::spec_compliant()))?;
        let document = Html::parse_document(&data);
        let mut release = RYMRelease::default();
        release.url = url.to_string();

        // Get title
        let mut selector = Selector::parse(".album_title").unwrap();
        release.title = document.select(&selector).next().ok_or("Missing title")?.text().collect::<String>().trim().to_string();
        selector = Selector::parse(".album_title .album_artist_small").unwrap();
        let artist_trash = document.select(&selector).next().unwrap().text().collect::<String>().trim().to_string();
        release.title = release.title.replace(&artist_trash, "").trim().to_string();

        // Get art
        selector = Selector::parse(".page_release_art_frame img").unwrap();
        release.art = format!("https:{}", document.select(&selector).next().unwrap().value().attr("src").unwrap());

        // ID
        selector = Selector::parse(".album_title input").unwrap();
        release.id = document.select(&selector).next().unwrap().value().attr("value").unwrap().to_string();

        // Get meta
        selector = Selector::parse(".album_info tr").unwrap();
        for row in document.select(&selector) {
            // Get name and value
            let mut selector = Selector::parse("th.info_hdr").unwrap();
            let name = row.select(&selector).next().unwrap().text().collect::<String>();
            selector = Selector::parse("td").unwrap();
            let value = row.select(&selector).next().unwrap().text().collect::<String>().trim().to_string();

            // Parse
            match name.to_lowercase().replace(":", "").trim() {
                "artist" => {
                    let mut artists = vec![];
                    let selector = Selector::parse("td .artist").unwrap();
                    for artist in row.select(&selector) {
                        artists.push(artist.text().collect::<String>().trim().to_string());
                    }
                    release.artists = artists;
                },
                "genres" => {
                    selector = Selector::parse("span.release_pri_genres").unwrap();
                    release.genres = row.select(&selector).next().unwrap().text().collect::<String>().trim().split(", ").map(String::from).collect::<Vec<_>>();
                    selector = Selector::parse("span.release_sec_genres").unwrap();
                    if let Some(e) = row.select(&selector).next() {
                        release.sub_genres = e.text().collect::<String>().trim().split(", ").map(String::from).collect::<Vec<_>>();
                    }
                },
                "descriptors" => {
                    release.descriptors = value.split(", ").map(String::from).collect::<Vec<_>>()
                },
                "released" => {
                    // Year
                    if value.len() == 4 {
                        release.release_year = value.parse().ok();
                    } else {
                        // Zero pad the day because there is no formatting for non padded day
                        let day = format!("{:02}", value.chars().take(2).collect::<String>().trim().parse::<i8>().unwrap());
                        let rest = value.chars().skip(2).collect::<String>();
                        let value = format!("{day} {}", rest.trim());
                        release.release_date = NaiveDate::parse_from_str(&value, "%d %B %Y").ok()
                    }
                    
                }
                _ => {}
            }
        }

        // Get tracks
        selector = Selector::parse("ul#tracks li.track").unwrap();
        let mut tracks = vec![];
        for track_elem in document.select(&selector) {
            // Last element is track total
            let selector = Selector::parse("span.tracklist_total").unwrap();
            if track_elem.select(&selector).next().is_some() {
                break;
            }
            // Get meta
            let selector = Selector::parse(".tracklist_num").unwrap();
            let track_number = track_elem.select(&selector).next().unwrap().text().collect::<String>();
            let selector = Selector::parse(".tracklist_title .rendered_text").unwrap();
            let title = track_elem.select(&selector).next().unwrap().text().collect::<String>();
            let selector = Selector::parse(".tracklist_title .tracklist_duration").unwrap();
            let duration = track_elem.select(&selector).next().unwrap().value().attr("data-inseconds").unwrap();
            // Create track
            tracks.push(RYMTrack {
                number: track_number.trim().parse().unwrap(),
                title,
                duration: Duration::from_secs(duration.trim().parse().unwrap()),
            });
        }
        release.tracks = tracks;

        Ok(release)
    }

}

impl AutotaggerSource for RateYourMusic {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>> {
        let query = format!("{} {}", info.artist()?, MatchingUtils::clean_title(info.title()?));
        let releases = self.search(&query)?;
        for release in releases.into_iter().take(self.config.max_albums) {
            let release = self.release(&release.url)?;
            let tracks = release.into_tracks(&self.config);
            match MatchingUtils::match_track(info, &tracks, config, true) {
                Some(r) => return Ok(Some(r)),
                None => continue,
            }
        }
        Ok(None)
    }
}


#[derive(Debug, Clone)]
struct RYMSearchRelease {
    url: String
}

#[derive(Debug, Clone)]
struct RYMTrack {
    number: i32,
    title: String,
    duration: Duration
}


#[derive(Debug, Clone, Default)]
struct RYMRelease {
    id: String,
    title: String,
    url: String,
    art: String,
    artists: Vec<String>,
    release_date: Option<NaiveDate>,
    release_year: Option<i16>,
    genres: Vec<String>,
    sub_genres: Vec<String>,
    descriptors: Vec<String>,
    tracks: Vec<RYMTrack>
}

impl RYMRelease {
    /// Convert self into tracks
    pub fn into_tracks(self, config: &RYMConfig) -> Vec<Track> {
        let mut genres = self.genres;
        if config.genres_and_subgenres {
            genres.extend(self.sub_genres.clone());
        }
        let styles = match config.subgenres_to_styles {
            true => self.sub_genres,
            false => self.descriptors
        };
        let track_total = self.tracks.len();

        self.tracks.into_iter().map(|track| {
            Track {
                platform: "rateyourmusic".to_string(),
                title: track.title,
                artists:self.artists.clone(),
                album_artists: self.artists.clone(),
                album: Some(self.title.to_string()),
                genres: genres.clone(),
                styles: styles.clone(),
                art: Some(self.art.to_owned()),
                url: self.url.to_string(),
                catalog_number: Some(self.id.to_string()),
                release_id: self.id.to_string(),
                duration: track.duration,
                track_number: Some(TrackNumber::Number(track.number)),
                track_total: Some(track_total as u16),
                release_date: self.release_date.clone(),
                release_year: self.release_year.clone(),
                ..Default::default()
            }
        }).collect()
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
struct RYMConfig {
    genres_and_subgenres: bool,
    subgenres_to_styles: bool,
    max_albums: usize
}

impl Default for RYMConfig {
    fn default() -> Self {
        Self { genres_and_subgenres: true, subgenres_to_styles: false, max_albums: 5 }
    }
}

pub struct RateYourMusicBuilder;

impl AutotaggerSourceBuilder for RateYourMusicBuilder {
    fn new() -> Self where Self: Sized {
        RateYourMusicBuilder
    }

    fn get_source(&mut self, config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Box<dyn Error>> {
        let rymconfig: RYMConfig = config.get_custom("rateyourmusic")?;
        Ok(Box::new(RateYourMusic::new(rymconfig)))
    }

    fn info(&self) -> PlatformInfo {
        PlatformInfo {
            id: "rateyourmusic".to_string(),
            name: "RateYourMusic".to_string(),
            description: "//TODO: Write desc".to_string(),
            version: "1.0.0".to_string(),
            icon: &[0u8], //TODO: Icon
            max_threads: 4,
            custom_options: PlatformCustomOptions::new()
                .add("genres_and_subgenres", "Merge genres and subgenres into genres", PlatformCustomOptionValue::Boolean { value: true })
                .add("subgenres_to_styles", "Write subgenres as styles", PlatformCustomOptionValue::Boolean { value: false })
                .add("max_albums", "Max albums to search thru", PlatformCustomOptionValue::Number { min: 1, max: 20, step: 1, value: 5 })
        }
    }
}


#[test]
fn test() {
    let rym = RateYourMusic::new(Default::default());
    let results = rym.search("Porter Robinson Hear The Bells").unwrap();
    for result in results {
        let _release = rym.release(&result.url).unwrap();
    }
}