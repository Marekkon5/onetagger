use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread::sleep;
use rspotify::blocking::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::blocking::util::get_token_by_code;
use rspotify::blocking::client;
use rspotify::model::album::FullAlbum;
use rspotify::model::artist::FullArtist;
use rspotify::senum::SearchType;
use rspotify::model::search::SearchResult;
use rspotify::model::track::FullTrack;
use rspotify::model::audio::AudioFeatures;
use rspotify::client::ApiError;
use url::Url;
use rouille::{Server, router};
use onetagger_shared::Settings;
use onetagger_tagger::{AutotaggerSource, Track, TaggerConfig, AudioFileInfo, MusicPlatform, MatchingUtils, TrackNumber, AutotaggerSourceBuilder, PlatformInfo, SpotifyConfig};

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
    pub spotify: client::Spotify
}

impl Spotify {
    // Create OAuth with parameters
    fn create_oauth(client_id: &str, client_secret: &str) -> SpotifyOAuth {
        SpotifyOAuth::default()
            .cache_path(Settings::get_folder().unwrap().join("spotify_token_cache.json"))
            .client_id(client_id)
            .client_secret(client_secret)
            .scope("user-read-private")
            .redirect_uri(&format!("http://127.0.0.1:{}/spotify", CALLBACK_PORT))
            .build()
    }

    // Generate authorization URL
    pub fn generate_auth_url(client_id: &str, client_secret: &str) -> (String, SpotifyOAuth) {
        let oauth = Spotify::create_oauth(client_id, client_secret);
        (oauth.get_authorize_url(None, None), oauth)
    }

    // Authentication server for callback from spotify
    pub fn auth_server(oauth: &mut SpotifyOAuth, expose: bool) -> Result<Spotify, Box<dyn Error>> {
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
                    if let Some(code) = request.get_param("code") {
                        let mut t = token_clone.lock().unwrap();
                        *t = Some(code);
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
        // Create client
        let token_info = get_token_by_code(oauth, token).ok_or("Invalid token")?;
        let credentials = SpotifyClientCredentials::default()
            .token_info(token_info)
            .build();
        let spotify = client::Spotify::default()
            .client_credentials_manager(credentials)
            .build();

        Ok(Spotify {
            spotify
        })
    }

    /// Parse auth token code from URL
    pub fn auth_token_code(oauth: &mut SpotifyOAuth, url: &str) -> Result<Spotify, Box<dyn Error>> {
        let url = Url::parse(url)?;
        let (_, code) = url.query_pairs().find(|(q, _)| q == "code").ok_or("Missing code parameter")?;
        let token_info = get_token_by_code(oauth, &code.to_string()).ok_or("Invalid token")?;
        // Create client
        let credentials = SpotifyClientCredentials::default()
            .token_info(token_info)
            .build();
        let spotify = client::Spotify::default()
            .client_credentials_manager(credentials)
            .build();

        Ok(Spotify {
            spotify
        })
    }

    // Try to authorize Spotify from cached token
    pub fn try_cached_token(client_id: &str, client_secret: &str) -> Option<Spotify> {
        let mut oauth = Spotify::create_oauth(client_id, client_secret);
        let token = oauth.get_cached_token()?;
        let credentials = SpotifyClientCredentials::default()
            .token_info(token)
            .build();
        let spotify = client::Spotify::default()
            .client_credentials_manager(credentials)
            .build();
        Some(Spotify { spotify })
    }

    // Handle error and sleep if rate limit
    fn handle_rspotify_error(&self, error: failure::Error) -> Result<(), Box<dyn Error>> {
        let err: Result<ApiError, failure::Error> = error.downcast();

        // For some reason the fucking downcasting always fails no matter what I try, even as_fail and the other methods fail, so idk what next, very dirty
        let err_string = format!("{:?}", err);
        if err_string.starts_with("Err(RateLimited(") {
            // Extract delay
            let val = &err_string[16..err_string.find(")").ok_or("Invalid error message")?];
            let mut delay = 1;
            if val.starts_with("Some(") {
                delay = val[5..].parse().unwrap_or(1);
            }
            warn!("Spotify rate limit, waiting {}s", delay);
            // Add 100ms for safety
            sleep(Duration::from_millis(delay*1000 + 100));
            return Ok(());
        }

        Err(err.unwrap_err().into())
    }

    // Search tracks by query
    pub fn search_tracks(&self, query: &str, limit: u32) -> Result<Vec<FullTrack>, Box<dyn Error>> {
        // rspotify doesn't url encode for some reason
        let q = urlencoding::encode(query);
        match self.spotify.search(&q, SearchType::Track, limit, 0, None, None) {
            Ok(results) => {
                let mut tracks = vec![];
                if let SearchResult::Tracks(tracks_page) = results {
                    tracks = tracks_page.items;
                }
                Ok(tracks)
            },
            Err(e) => {
                // Handle error and retry on rate limit
                self.handle_rspotify_error(e)?;
                self.search_tracks(query, limit)
            }
        }
    }

    /// Fetch audio features for track id
    pub fn audio_features(&self, id: &str) -> Result<AudioFeatures, Box<dyn Error>> {
        match self.spotify.audio_features(id) {
            Ok(f) => Ok(f),
            Err(e) => {
                // Handle error and retry on rate limit
                self.handle_rspotify_error(e)?;
                self.audio_features(id)
            }
        }
    }

    /// Fetch full album
    pub fn album(&self, id: &str) -> Result<FullAlbum, Box<dyn Error>> {
        match self.spotify.album(id) {
            Ok(a) => Ok(a),
            Err(e) => {
                // Handle error and retry on rate limit
                self.handle_rspotify_error(e)?;
                self.album(id)
            }
        }
    }

    /// Fetch full artist
    pub fn artist(&self, id: &str) -> Result<FullArtist, Box<dyn Error>> {
        match self.spotify.artist(id) {
            Ok(a) => Ok(a),
            Err(e) => {
                // Handle error and retry on rate limit
                self.handle_rspotify_error(e)?;
                self.artist(id)
            }
        }
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
                match self.album(&track.release_id) {
                    Ok(album) => {
                        track.label = album.label;
                    }
                    Err(e) => warn!("Failed to fetch album data: {}", e),
                }
            }
            // Fetch artist
            if config.genre {
                // Get artist id
                let t = results.iter().find(|t| t.id == track.track_id).unwrap();
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
                let t = results.iter().find(|t| t.id == track.track_id).unwrap();
                if let Some(track_id) = &t.id {
                    match self.audio_features(track_id) {
                        Ok(features) => {
                            if features.key < 0 || features.key >= 12 {
                                warn!("Spotify returned unkown key!");
                            } else {
                                match features.mode.round() as i8 {
                                    1 => track.key = Some(PITCH_CLASS_MAJOR[features.key as usize].to_string()),
                                    0 => track.key = Some(PITCH_CLASS_MINOR[features.key as usize].to_string()),
                                    v => warn!("Invalid audio features mode: {}", v)
                                }
                            }
                        },
                        Err(e) => warn!("Failed to fetch audio features: {}", e)
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
        platform: MusicPlatform::Spotify,
        title: track.name,
        version: None,
        artists: track.artists.into_iter().map(|a| a.name).collect(),
        album_artists: track.album.artists.into_iter().map(|a| a.name).collect(),
        album: Some(track.album.name),
        art: track.album.images.first().map(|i| i.url.to_string()),
        url: format!("https://open.spotify.com/track/{}", track.id.clone().unwrap_or(String::new())),
        track_id: track.id,
        release_id: track.album.id.unwrap_or(String::new()),
        duration: Duration::from_millis(track.duration_ms as u64),
        track_number: Some(TrackNumber::Number(track.track_number as i32)),
        isrc: track.external_ids.into_iter().find(|(k, _)| k == "isrc").map(|(_, v)| v.to_string()),
        release_year: track.album.release_date.map(|d| if d.len() > 4 { d[0..4].to_string().parse().ok() } else { None }).flatten(),
        ..Default::default()
    }
}

/// For creating instance of Spotify AT plugin
pub struct SpotifyBuilder {
    config: Option<SpotifyConfig>
}

impl AutotaggerSourceBuilder for SpotifyBuilder {
    fn new(config: &TaggerConfig) -> SpotifyBuilder {
        SpotifyBuilder {
            config: config.spotify.clone()
        }
    }

    fn get_source(&mut self) -> Result<Box<dyn AutotaggerSource>, Box<dyn Error>> {
        let config = self.config.take().ok_or("Missing Spotify config!")?;
        let spotify = Spotify::try_cached_token(&config.client_id, &config.client_secret).ok_or("Spotify not authorized!")?;
        Ok(Box::new(spotify))
    }

    fn info(&self) -> PlatformInfo {
        todo!()
    }
}