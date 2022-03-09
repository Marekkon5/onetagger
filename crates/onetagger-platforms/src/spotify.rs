use std::error::Error;
use std::sync::{Arc, Mutex};
use rspotify::clients::{BaseClient, OAuthClient};
use rspotify::model::{SearchType, TrackId, Id, AlbumId, ArtistId, Modality, Market, Country};
use rspotify::{Credentials, Config, AuthCodeSpotify, OAuth, scopes};
use rspotify::model::album::FullAlbum;
use rspotify::model::artist::FullArtist;
use rspotify::model::search::SearchResult;
use rspotify::model::track::FullTrack;
use rspotify::model::audio::AudioFeatures;
use rouille::{Server, router};
use onetagger_shared::Settings;
use onetagger_tagger::{AutotaggerSource, Track, TaggerConfig, AudioFileInfo, MatchingUtils, TrackNumber, AutotaggerSourceBuilder, PlatformInfo};

/// Reexport, beacause the rspotify dependency is git
pub use rspotify;

static CALLBACK_PORT: u16 = 36914;
static CALLBACK_HTML: &'static str = "
<html>
    <head><script>window.close();</script></head>
    <body>
        <h1>Spotify authorized successfully, you can close this window.</h1>
    </body>
</html>
";

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
        oauth.redirect_uri = format!("http://127.0.0.1:{}/spotify", CALLBACK_PORT);
        let client = AuthCodeSpotify::with_config(credentials, oauth, config);
        client
    }

    /// Generate OAuth authorization URL
    pub fn generate_auth_url(client_id: &str, client_secret: &str) -> Result<(String, AuthCodeSpotify), Box<dyn Error>> {
        let client = Self::create_client(client_id, client_secret);
        Ok((client.get_authorize_url(false)?, client ))
    }

    /// Try to authorize spotify from cached token
    pub fn try_cached_token(client_id: &str, client_secret: &str) -> Option<Spotify> {
        let mut client = Self::create_client(client_id, client_secret);
        let token = client.read_token_cache(true).ok()??;
        *client.token.lock().unwrap() = Some(token);
        client.refresh_token().ok()?;
        client.auto_reauth().ok()?;
        Some(Spotify { spotify: client })
    }

     /// Authentication server for callback from spotify
     pub fn auth_server(mut spotify: AuthCodeSpotify, expose: bool) -> Result<Spotify, Box<dyn Error>> {
        // Prepare server
        let token: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let token_clone = token.clone();

        let host = match expose {
            true => "0.0.0.0",
            false => "127.0.0.1"
        };
        let server = Server::new(&format!("{}:{}", host, CALLBACK_PORT), move |request| {
            router!(request, 
                (GET) (/spotify) => {
                    // Get token
                    if request.get_param("code").is_some() {
                        let mut t = token_clone.lock().unwrap();
                        *t = Some(format!("http://127.0.0.1:{}{}", CALLBACK_PORT, request.raw_url()));
                    }
                },
                _ => {}
            );
            // Navigate back
            rouille::Response::html(CALLBACK_HTML)
        }).unwrap();
        // Run server
        loop {
            if token.lock().unwrap().is_some() {
                break;
            }
            server.poll();
        }
        let token_lock = token.lock().unwrap();
        let token = token_lock.as_ref().unwrap();

        // Auth
        let code = spotify.parse_response_code(token.trim()).ok_or("Invalid token url!")?;
        spotify.request_token(&code)?;
        spotify.auto_reauth()?;
        spotify.write_token_cache()?;
        Ok(Spotify {
            spotify
        })
    }

    /// Authorize from URL
    pub fn auth_token_code(mut spotify: AuthCodeSpotify, url: &str) -> Result<Spotify, Box<dyn Error>> {
        let code = spotify.parse_response_code(url).ok_or("Invalid token url!")?;
        spotify.request_token(&code)?;
        spotify.auto_reauth()?;
        spotify.write_token_cache()?;
        Ok(Spotify {
            spotify
        })
    }

    /// Search tracks by query
    pub fn search_tracks(&self, query: &str, limit: u32) -> Result<Vec<FullTrack>, Box<dyn Error>> {
        let results = self.spotify.search(query, &SearchType::Track, Some(&Market::Country(Country::UnitedKingdom)), None, Some(limit), None)?;
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
        let spotify = Spotify::try_cached_token(&config.client_id, &config.client_secret).ok_or("Spotify not authorized!")?;
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