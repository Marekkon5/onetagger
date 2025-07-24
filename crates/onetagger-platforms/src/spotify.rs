use anyhow::Error;
use chrono::NaiveDate;
use std::time::Duration;
use rspotify::clients::{BaseClient, OAuthClient};
use rspotify::model::{SearchType, TrackId, Id, AlbumId, ArtistId, Modality};
use rspotify::{Credentials, Config, AuthCodeSpotify, OAuth, scopes, ClientError, ClientResult};
use rspotify::model::album::FullAlbum;
use rspotify::model::artist::FullArtist;
use rspotify::model::search::SearchResult;
use rspotify::model::track::FullTrack;
use rspotify::model::audio::AudioFeatures;
use onetagger_shared::{Settings, WEBSERVER_CALLBACKS, PORT};
use onetagger_tagger::{AutotaggerSource, Track, TaggerConfig, AudioFileInfo, MatchingUtils, TrackNumber, AutotaggerSourceBuilder, PlatformInfo, supported_tags, SupportedTag, TrackMatch};

/// Reexport, beacause the rspotify dependency is git
pub use rspotify;


static PITCH_CLASS_MAJOR: [&'static str; 12] = ["C", "C#",   "D",  "D#",  "E",  "F",  "F#",  "G",  "G#",  "A",  "A#",  "B" ];
static PITCH_CLASS_MINOR: [&'static str; 12] = ["Cm", "Dbm", "Dm", "Ebm", "Em", "Fm", "Gbm", "Gm", "Abm", "Am", "Bbm", "Bm"];

#[derive(Clone)]
pub struct Spotify {
    pub spotify: AuthCodeSpotify
}

impl Spotify {
    /// Create AuthCodeSpotify with parameters
    pub fn create_client(client_id: &str, client_secret: &str) -> AuthCodeSpotify {
        let credentials = Credentials::new(client_id, client_secret);
        let mut config = Config::default();
        config.cache_path = Settings::get_folder().unwrap().join("spotify_token_cache.json");
        config.token_cached = true;
        config.token_refreshing = true;
        let mut oauth = OAuth::default();
        oauth.scopes = scopes!("user-read-private");
        oauth.redirect_uri = format!("http://127.0.0.1:{}/spotify", PORT);
        let client = AuthCodeSpotify::with_config(credentials, oauth, config);
        client
    }

    /// Generate OAuth authorization URL
    pub fn generate_auth_url(client_id: &str, client_secret: &str) -> Result<(String, AuthCodeSpotify), Error> {
        let client = Self::create_client(client_id, client_secret);
        Ok((client.get_authorize_url(false)?, client ))
    }

    /// Try to authorize spotify from cached token
    pub fn try_cached_token(client_id: &str, client_secret: &str) -> Option<Spotify> {
        let client = Self::create_client(client_id, client_secret);
        let token = client.read_token_cache(true).ok()??;
        *client.token.lock().unwrap() = Some(token);
        client.refresh_token().ok()?;
        client.auto_reauth().ok()?;
        Some(Spotify { spotify: client })
    }

     /// Authentication server for callback from spotify
     pub fn auth_server(spotify: AuthCodeSpotify) -> Result<Spotify, Error> {
        // Wait for auth token
        { WEBSERVER_CALLBACKS.lock().unwrap().remove("spotify") };
        let token = loop {
            if let Some(token) = { WEBSERVER_CALLBACKS.lock().unwrap().get("spotify").map(String::from) } {
                break format!("http://127.0.0.1:{}{}", PORT, token);
            }
            std::thread::sleep(Duration::from_millis(100));
        };

        // Auth
        let code = spotify.parse_response_code(token.trim()).ok_or(anyhow!("Invalid token url!"))?;
        spotify.request_token(&code)?;
        spotify.auto_reauth()?;
        spotify.write_token_cache()?;
        Ok(Spotify {
            spotify
        })
    }

    /// Authorize from URL
    pub fn auth_token_code(spotify: AuthCodeSpotify, url: &str) -> Result<Spotify, Error> {
        let code = spotify.parse_response_code(url).ok_or(anyhow!("Invalid token url!"))?;
        spotify.request_token(&code)?;
        spotify.auto_reauth()?;
        spotify.write_token_cache()?;
        Ok(Spotify {
            spotify
        })
    }

    /// Wrapper for rate limit 
    fn rate_limit_wrap<F, R>(&self, f: F) -> Result<R, Error>
    where
        F: Fn(&Spotify) -> ClientResult<R>
    {
        match f(self) {
            Ok(r) => {
                // Intentionally slow down to prevent rate limits
                std::thread::sleep(Duration::from_millis(250));
                Ok(r)
            },
            Err(ClientError::Http(http)) => {
                match *http {
                    rspotify::http::HttpError::StatusCode(r) => {
                        // Rate limit
                        if r.status() == 429 {
                            let delay = r.header("Retry-After").map(|v| v.parse().ok()).flatten().unwrap_or(3);
                            if delay > 600 {
                                error!("Spotify rate limit too high!: {delay}");
                                return Err(anyhow!("Too high rate limit by Spotify"));
                            }

                            warn!("Spotify rate limit hit, sleeping for: {delay}s...");
                            std::thread::sleep(Duration::from_secs(delay));
                            return self.rate_limit_wrap(f);
                        }
                        return Err(anyhow!("Unknown Spotify status code: {}", r.status()));
                    },
                    e => return Err(e.into())
                }
            },
            Err(e) => Err(e.into()),
        }
    }

    /// Search tracks by query
    pub fn search_tracks(&self, query: &str, limit: u32) -> Result<Vec<FullTrack>, Error> {
        let results = self.rate_limit_wrap(|s| s.spotify.search(query, SearchType::Track, None, None, Some(limit), None))?;
        let mut tracks = vec![];
        if let SearchResult::Tracks(tracks_page) = results {
            tracks = tracks_page.items;
        }
        Ok(tracks)
    }

    /// Fetch audio features for track id
    pub fn audio_features(&self, _id: &TrackId) -> Result<AudioFeatures, Error> {
        // self.rate_limit_wrap(|s| s.spotify.track_features(id.to_owned()))
        return Err(anyhow!("Spotify deprecated AudioFeatures"))
    }

    /// Fetch full album
    pub fn album(&self, id: &AlbumId) -> Result<FullAlbum, Error> {
        self.rate_limit_wrap(|s| s.spotify.album(id.to_owned(), None))
    }

    /// Fetch full artist
    pub fn artist(&self, id: &ArtistId) -> Result<FullArtist, Error> {
        self.rate_limit_wrap(|s| s.spotify.artist(id.to_owned()))
    }


    /// Extend track for autotagger
    fn extend_track_spotify(&self, track: &mut Track, config: &TaggerConfig) -> Result<(), Error> {
        // Fetch album
        if config.tag_enabled(SupportedTag::Label) || config.tag_enabled(SupportedTag::Genre) {
            match self.album(&AlbumId::from_id(track.release_id.as_ref().ok_or(anyhow!("Missing release_id"))?)?) {
                Ok(album) => {
                    track.label = album.label;
                    track.genres.extend(album.genres.into_iter());
                }
                Err(e) => warn!("Failed to fetch album data: {}", e),
            }
        }

        // Fetch genres from artist if album has them unavailable
        if config.tag_enabled(SupportedTag::Genre) && track.custom.get("artist_ids").is_some() {
            for id in track.custom.get("artist_ids").unwrap().split("\0") {
                debug!("artist id: {id}");
                match self.artist(&ArtistId::from_uri(id)?) {
                    Ok(artist) => {
                        for genre in artist.genres {
                            if !track.genres.contains(&genre) {
                                track.genres.push(genre);
                            }
                        }
                    },
                    Err(e) => warn!("Failed to fetch artist data: {e}"),
                }
            }
        }

        // Fetch audio features
        if config.tag_enabled(SupportedTag::Key) {
            if let Some(track_id) = track.track_id.as_ref() {
                match self.audio_features(&TrackId::from_id(track_id)?) {
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

        Ok(())
    }
}

impl AutotaggerSource for Spotify {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Vec<TrackMatch>, Error> {
        // Try ISRC
        if let Some(isrc) = info.isrc.as_ref() {
            let query = format!("isrc:{isrc}");
            let results = self.search_tracks(&query, 20)?;
            let tracks: Vec<Track> = results.clone().into_iter().map(|t| full_track_to_track(t)).collect();
            if let Some(track) = tracks.first() {
                return Ok(vec![TrackMatch::new_isrc(track.clone())]);
            }
        };

        // Normal search
        let query = format!("{} {}", info.artist()?, MatchingUtils::clean_title(info.title()?));
        let results = self.search_tracks(&query, 20)?;
        let tracks = results.clone().into_iter().map(|t| full_track_to_track(t)).collect();
        Ok(MatchingUtils::match_track(info, &tracks, config, true))
    }

    fn extend_track(&mut self, track: &mut Track, config: &TaggerConfig) -> Result<(), Error> {
        Self::extend_track_spotify(&self, track, config)?;
        Ok(())
    }

    
   
}

/// Convert rspotify FullTrack into 1T Track
fn full_track_to_track(track: FullTrack) -> Track {
    Track {
        platform: "spotify".to_string(),
        title: track.name,
        version: None,
        custom: [("artist_ids".to_string(), track.artists.iter().filter_map(|a| a.id.as_ref().map(|a| a.to_string())).collect::<Vec<_>>().join("\0"))].into_iter().collect(),
        artists: track.artists.into_iter().map(|a| a.name).collect(),
        album_artists: track.album.artists.into_iter().map(|a| a.name).collect(),
        album: Some(track.album.name),
        art: track.album.images.first().map(|i| i.url.to_string()),
        url: format!("https://open.spotify.com/track/{}", track.id.as_ref().map(|i| i.id()).unwrap_or("")),
        track_id: track.id.map(|i| i.id().to_string()),
        release_id: track.album.id.map(|i| i.id().to_string()),
        duration: track.duration.to_std().unwrap().into(),
        track_number: Some(TrackNumber::Number(track.track_number as i32)),
        isrc: track.external_ids.into_iter().find(|(k, _)| k == "isrc").map(|(_, v)| v.to_string()),
        release_year: track.album.release_date.as_ref().map(|d| if d.len() > 4 { d[0..4].to_string().parse().ok() } else { None }).flatten(),
        release_date: track.album.release_date.as_ref().map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()).flatten(),
        explicit: Some(track.explicit),
        thumbnail: track.album.images.iter().min_by(|a, b| a.width.unwrap_or(1000).partial_cmp(&b.width.unwrap_or(1000)).unwrap()).map(|i| i.url.to_string()),
        ..Default::default()
    }
}

/// For creating instance of Spotify AT plugin
#[derive(Debug, Clone)]
pub struct SpotifyBuilder;

impl AutotaggerSourceBuilder for SpotifyBuilder {
    fn new() -> SpotifyBuilder {
        SpotifyBuilder
    }

    fn get_source(&mut self, config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Error> {
        let config = config.spotify.clone().ok_or(anyhow!("Missing Spotify config!"))?;
        let spotify = Spotify::try_cached_token(&config.client_id, &config.client_secret).ok_or(anyhow!("Spotify not authorized!"))?;
        Ok(Box::new(spotify))
    }

    fn info(&self) -> PlatformInfo {
        PlatformInfo {
            id: "spotify".to_string(),
            name: "Spotify".to_string(),
            description: "Requires a free account, can match using ISRC".to_string(),
            icon: include_bytes!("../assets/spotify.png"),
            max_threads: 1,
            version: "1.0.0".to_string(),
            custom_options: Default::default(),
            requires_auth: true,
            supported_tags: supported_tags!(Title, Artist, AlbumArtist, Album, AlbumArt, URL, TrackId, ReleaseId, Duration, TrackNumber, ISRC, ReleaseDate, Genre, Key, Label, Explicit)
        }
    }
}