use std::error::Error;
use rspotify::clients::BaseClient;
use rspotify::model::{SearchType, TrackId, Id, AlbumId, ArtistId, Modality};
use rspotify::{Credentials, Config, ClientCredsSpotify};
use rspotify::model::album::FullAlbum;
use rspotify::model::artist::FullArtist;
use rspotify::model::search::SearchResult;
use rspotify::model::track::FullTrack;
use rspotify::model::audio::AudioFeatures;
use onetagger_shared::Settings;
use onetagger_tagger::{AutotaggerSource, Track, TaggerConfig, AudioFileInfo, MatchingUtils, TrackNumber, AutotaggerSourceBuilder, PlatformInfo};

/// Reexport, beacause the rspotify dependency is git
pub use rspotify;


static PITCH_CLASS_MAJOR: [&'static str; 12] = ["C", "C#",   "D",  "D#",  "E",  "F",  "F#",  "G",  "G#",  "A",  "A#",  "B" ];
static PITCH_CLASS_MINOR: [&'static str; 12] = ["Cm", "Dbm", "Dm", "Ebm", "Em", "Fm", "Gbm", "Gm", "Abm", "Am", "Bbm", "Bm"];

#[derive(Clone)]
pub struct Spotify {
    pub spotify: ClientCredsSpotify
}

impl Spotify {
    /// Create ClientCredsSpotify with parameters
    pub fn create_client(client_id: &str, client_secret: &str) -> ClientCredsSpotify {
        let credentials = Credentials::new(client_id, client_secret);
        let mut config = Config::default();
        config.cache_path = Settings::get_folder().unwrap().join("spotify_token_cache.json");
        config.token_cached = true;
        config.token_refreshing = true;
        let client = ClientCredsSpotify::with_config(credentials, config);
        client
    }

    /// Request auth token
    pub fn authorize(mut client: ClientCredsSpotify) -> Result<Spotify, Box<dyn Error>> {
        client.request_token()?;
        Ok(Spotify { spotify: client })
    }

    /// Search tracks by query
    pub fn search_tracks(&self, query: &str, limit: u32) -> Result<Vec<FullTrack>, Box<dyn Error>> {
        // rspotify 0.10 doesn't url encode for some reason
        // let q = urlencoding::encode(query);
        let results = self.spotify.search(query, &SearchType::Track, None, None, Some(limit), None)?;
        let mut tracks = vec![];
        if let SearchResult::Tracks(tracks_page) = results {
            tracks = tracks_page.items;
        }
        Ok(tracks)
    }

    /// Fetch audio features for track id
    pub fn audio_features(&self, id: &TrackId) -> Result<AudioFeatures, Box<dyn Error>> {
        Ok(self.spotify.track_features(id)?)
    }

    /// Fetch full album
    pub fn album(&self, id: &AlbumId) -> Result<FullAlbum, Box<dyn Error>> {
        Ok(self.spotify.album(id)?)
    }

    /// Fetch full artist
    pub fn artist(&self, id: &ArtistId) -> Result<FullArtist, Box<dyn Error>> {
        Ok(self.spotify.artist(id)?)
    }
}

impl AutotaggerSource for Spotify {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>> {
        let query = format!("{} {}", info.artist()?, MatchingUtils::clean_title(info.title()?));
        let results = self.search_tracks(&query, 20)?;
        let tracks: Vec<Track> = results.clone().into_iter().map(|t| full_track_to_track(t)).collect();
        if let Some((acc, mut track)) = MatchingUtils::match_track(info, &tracks, config, true) {
            // Fetch album
            if config.label {
                match self.album(&AlbumId::from_id(&track.release_id)?) {
                    Ok(album) => {
                        track.label = album.label;
                    }
                    Err(e) => warn!("Failed to fetch album data: {}", e),
                }
            }
            // Fetch artist
            if config.genre {
                // Get artist id
                let t = results.iter().find(|t| t.id.as_ref().map(|i| i.id().to_string()) == track.track_id).unwrap();
                if let Some(artist_id) = t.artists.first().map(|a| a.id.clone()).flatten() {
                    match self.artist(&artist_id) {
                        Ok(artist) => {
                            track.genres = artist.genres;
                        },
                        Err(e) => warn!("Failed to fetch artist data: {}", e)
                    }
                } else {
                    warn!("Missing artist ID");
                }
                
            }
            // Fetch audio features
            if config.key {
                let t = results.iter().find(|t| t.id.as_ref().map(|i| i.id()) == track.track_id.as_ref().map(|s| s.as_str())).unwrap();
                if let Some(track_id) = &t.id {
                    match self.audio_features(track_id) {
                        Ok(features) => {
                            if features.key < 0 || features.key >= 12 {
                                warn!("Spotify returned unkown key!");
                            } else {
                                match features.mode {
                                    Modality::Major => track.key = Some(PITCH_CLASS_MAJOR[features.key as usize].to_string()),
                                    Modality::Minor => track.key = Some(PITCH_CLASS_MINOR[features.key as usize].to_string()),
                                    v => warn!("Invalid audio features mode: {v:?}")
                                }
                            }
                        },
                        Err(e) => warn!("Failed to fetch audio features: {e}")
                    }
                }
            }

            return Ok(Some((acc, track)));
        }
        Ok(None)
    }
   
}

fn full_track_to_track(track: FullTrack) -> Track {
    Track {
        platform: "spotify".to_string(),
        title: track.name,
        version: None,
        artists: track.artists.into_iter().map(|a| a.name).collect(),
        album_artists: track.album.artists.into_iter().map(|a| a.name).collect(),
        album: Some(track.album.name),
        art: track.album.images.first().map(|i| i.url.to_string()),
        url: format!("https://open.spotify.com/track/{}", track.id.as_ref().map(|i| i.id()).unwrap_or("")),
        track_id: track.id.map(|i| i.id().to_string()),
        release_id: track.album.id.map(|i| i.id().to_string()).unwrap_or(String::new()),
        duration: track.duration,
        track_number: Some(TrackNumber::Number(track.track_number as i32)),
        isrc: track.external_ids.into_iter().find(|(k, _)| k == "isrc").map(|(_, v)| v.to_string()),
        release_year: track.album.release_date.map(|d| if d.len() > 4 { d[0..4].to_string().parse().ok() } else { None }).flatten(),
        ..Default::default()
    }
}

/// For creating instance of Spotify AT plugin
pub struct SpotifyBuilder;

impl AutotaggerSourceBuilder for SpotifyBuilder {
    fn new() -> SpotifyBuilder {
        SpotifyBuilder
    }

    fn get_source(&mut self, config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Box<dyn Error>> {
        let config = config.spotify.clone().ok_or("Missing Spotify config!")?;
        let spotify = Spotify::create_client(&config.client_id, &config.client_secret);
        let spotify = Spotify::authorize(spotify)?;
        Ok(Box::new(spotify))
    }

    fn info(&self) -> PlatformInfo {
        PlatformInfo {
            id: "spotify".to_string(),
            name: "Spotify".to_string(),
            description: "Requires a free account".to_string(),
            icon: include_bytes!("../assets/spotify.png"),
            max_threads: 1,
            version: "1.0.0".to_string(),
            custom_options: Default::default()
        }
    }
}