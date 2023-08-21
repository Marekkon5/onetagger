use std::error::Error;
use std::time::Duration;
use chrono::NaiveDate;
use onetagger_tagger::{Track, AutotaggerSourceBuilder, PlatformInfo, TaggerConfig, AutotaggerSource, PlatformCustomOptions, PlatformCustomOptionValue, AudioFileInfo, MatchingUtils, TrackNumber, supported_tags, TrackMatch};
use reqwest::blocking::Client;
use serde::de::DeserializeOwned;

/// Rate limit error code
const RATE_LIMIT_CODE: i32 = 4;

struct Deezer {
    client: Client,
    config: DeezerConfig,
}

impl Deezer {
    /// Create new instance
    pub fn new(config: DeezerConfig) -> Deezer {
        Deezer {
            client: Client::new(),
            config
        }
    }

    /// GET with rate limit wrap
    fn get<D: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<D, Box<dyn Error>> {
        let r: DeezerResponse<D> = self.client.get(format!("https://api.deezer.com{path}"))
            .query(query)
            .send()?.json()?;
        if r.error_code() == Some(RATE_LIMIT_CODE) {
            warn!("Deezer Rate Limit, sleeping for 3s...");
            std::thread::sleep(Duration::from_secs(3));
            return self.get(path, query);
        }
        Ok(r.into_result()?)
    }

    /// Search tracks on Deezer api
    pub fn search_tracks(&self, query: &str) -> Result<SearchResults<DeezerTrack>, Box<dyn Error>> {
        Ok(self.get("/search/track", &[("q", query)])?)
    }

    /// Get full track info
    pub fn track(&self, id: i64) -> Result<DeezerTrackFull, Box<dyn Error>> {
        Ok(self.get(&format!("/track/{id}"), &[])?)
    }

    /// Get full album info
    pub fn album(&self, id: i64) -> Result<DeezerAlbumFull, Box<dyn Error>> {
        Ok(self.get(&format!("/album/{id}"), &[])?)
    }

    /// Generate Deezer image url
    pub fn image_url(image_type: &str, md5: &str, resolution: u16) -> String {
        format!("https://e-cdns-images.dzcdn.net/images/{image_type}/{md5}/{resolution}x{resolution}-000000-80-0-0.jpg")
    }
}

impl AutotaggerSource for Deezer {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Vec<TrackMatch>, Box<dyn Error>> {
        // Search
        let query = format!("{} {}", info.artist()?, MatchingUtils::clean_title(info.title()?));
        let tracks = self.search_tracks(&query)?.data.into_iter().map(|t| t.into()).collect::<Vec<_>>();
        let mut tracks = MatchingUtils::match_track(info, &tracks, config, true);
        // Inject art
        tracks.iter_mut().for_each(|track| track.track.art = Some(Self::image_url("cover", track.track.art.as_ref().unwrap(), self.config.art_resolution)));
        Ok(tracks)
    }

    fn extend_track(&mut self, track: &mut Track, config: &TaggerConfig) -> Result<(), Box<dyn Error>> {
        // Extend with full track data
        if config.any_tag_enabled(&supported_tags!(TrackNumber, DiscNumber, BPM, ISRC, ReleaseDate)) {
            let id = track.track_id.as_ref().unwrap().parse().unwrap();
            match self.track(id) {
                Ok(t) => {
                    track.track_number = t.track_position.map(|t| TrackNumber::Number(t));
                    track.disc_number = t.disk_number;
                    if let Some(bpm) = t.bpm {
                        if bpm > 1.0 {
                            track.bpm = Some(bpm as i64);
                        }
                    }
                    track.isrc = t.isrc;
                    track.release_date = NaiveDate::parse_from_str(&t.release_date, "%Y-%m-%d").ok();
                },
                Err(e) => warn!("Failed extending Deezer track ID {id}: {e}")
            }
        }

        // Extend with album data
        if config.any_tag_enabled(&supported_tags!(Genre, TrackTotal, Label, AlbumArtist)) {
            let id = track.release_id.parse().unwrap();
            match self.album(id) {
                Ok(album) => {
                    track.genres = album.genres.data.into_iter().map(|g| g.name).collect();
                    track.track_total = Some(album.nb_tracks);
                    track.label = Some(album.label);
                    track.album_artists = album.contributors.into_iter().map(|a| a.name).collect();
                },
                Err(e) => warn!("Failed extending Deezer track (album {id}): {e}")
            }
        }
        Ok(())
    }

}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum DeezerResponse<D> {
    Ok(D),
    Error { error: DeezerError }
}

impl<D> DeezerResponse<D> {
    /// Convert into result
    pub fn into_result(self) -> Result<D, Box<dyn Error>> {
        match self {
            DeezerResponse::Ok(data) => Ok(data),
            DeezerResponse::Error { error: DeezerError { message, code, .. } } => Err(format!("Deezer API Error {code}: {message}").into()),
        }
    }

    /// Get error code
    pub fn error_code(&self) -> Option<i32> {
        match self {
            Self::Error { error: DeezerError { code, .. } } => Some(*code),
            _ => None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeezerError {
    pub code: i32, 
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearchResults<I> {
    pub data: Vec<I>,
    pub total: usize,
    pub next: Option<String>
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct DeezerTrack {
    pub id: i64,
    pub readable: bool,
    pub title: String,
    pub title_short: String,
    pub title_version: Option<String>,
    pub link: String,
    pub duration: i64,
    pub rank: i64,
    pub artist: DeezerArtist,
    pub album: DeezerAlbum,
    pub explicit_lyrics: Option<bool>,
    pub explicit_content_lyrics: Option<i8>
}

impl Into<Track> for DeezerTrack {
    fn into(self) -> Track {
        Track {
            platform: "deezer".to_string(),
            title: self.title_short,
            version: self.title_version,
            artists: vec![self.artist.name],
            album_artists: vec![],
            album: Some(self.album.title),
            url: self.link,
            catalog_number: Some(self.id.to_string()),
            track_id: Some(self.id.to_string()),
            release_id: self.album.id.to_string(),
            duration: Duration::from_secs(self.duration as u64),
            explicit: self.explicit_lyrics.or(self.explicit_content_lyrics.map(|i| i == 1)),
            thumbnail: Some(Deezer::image_url("cover", &self.album.md5_image, 150)),
            art: Some(self.album.md5_image),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeezerArtist {
    pub id: i64,
    pub name: String,
    pub picture: String,
    pub picture_small: String,
    pub picture_medium: String,
    pub picture_big: String,
    pub picture_xl: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeezerAlbum {
    pub id: i64,
    pub title: String,
    // pub cover: String,
    // pub cover_small: String,
    // pub cover_medium: String,
    // pub cover_big: String,
    // pub cover_xl: String,
    pub md5_image: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeezerTrackFull {
    pub id: i64,
    pub title: String,
    pub title_short: String,
    pub title_version: Option<String>,
    pub isrc: Option<String>,
    pub link: String,
    pub duration: i64,
    pub track_position: Option<i32>,
    pub disk_number: Option<u16>,
    pub release_date: String,
    pub bpm: Option<f64>,
    pub gain: Option<f64>,
    pub contributors: Vec<DeezerArtist>,
    pub md5_image: String,
    pub artist: DeezerArtist,
    pub album: DeezerAlbum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeezerAlbumFull {
    pub id: i64,
    pub title: String,
    pub upc: Option<String>,
    pub link: String,
    pub share: String,
    // pub cover: String,
    // pub cover_small: String,
    // pub cover_medium: String,
    // pub cover_big: String,
    // pub cover_xl: String,
    pub md5_image: String,
    pub genre_id: i64,
    pub genres: DeezerGenres,
    pub label: String,
    pub nb_tracks: u16,
    pub duration: i64,
    pub release_date: String,
    pub contributors: Vec<DeezerArtist>,
    pub artist: DeezerArtist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeezerGenre {
    pub id: i64,
    pub name: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeezerGenres {
    pub data: Vec<DeezerGenre>
}

pub struct DeezerBuilder;

impl AutotaggerSourceBuilder for DeezerBuilder {
    fn new() -> Self {
        DeezerBuilder
    }

    fn get_source(&mut self, config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Box<dyn Error>> {
        let deezer_config: DeezerConfig = config.get_custom("deezer")?;
        Ok(Box::new(Deezer::new(deezer_config)))
    }

    fn info(&self) -> PlatformInfo {
        PlatformInfo {
            id: "deezer".to_string(),
            name: "Deezer".to_string(),
            description: "Spotify alternative, but faster. No login required".to_string(),
            version: "1.0.0".to_string(),
            icon: include_bytes!("../assets/deezer.png"),
            max_threads: 2,
            requires_auth: false, 
            supported_tags: supported_tags!(Title, Version, Album, AlbumArtist, Artist, AlbumArt, URL, CatalogNumber, TrackId, ReleaseId, Duration, Genre, TrackTotal, Label, ISRC, ReleaseDate, TrackNumber, DiscNumber, Explicit, BPM),
            custom_options: PlatformCustomOptions::new()
                .add("art_resolution", "Album Art Resolution", PlatformCustomOptionValue::Number { min: 100, max: 1600, step: 100, value: 1200 }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeezerConfig {
    pub art_resolution: u16
}
