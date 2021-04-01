use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread::sleep;
use rspotify::blocking::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::blocking::util::get_token_by_code;
use rspotify::blocking::client;
use rspotify::senum::SearchType;
use rspotify::model::search::SearchResult;
use rspotify::model::track::FullTrack;
use rspotify::model::audio::AudioFeatures;
use rspotify::client::ApiError;
use rouille::{Server, router};

static CALLBACK_PORT: u16 = 36914;
static CALLBACK_HTML: &'static str = "<script>window.close();</script>";

#[derive(Clone)]
pub struct Spotify {
    pub spotify: client::Spotify
}

impl Spotify {
    //Generate authorization URL
    pub fn generate_auth_url(client_id: &str, client_secret: &str) -> (String, SpotifyOAuth) {
        let oauth = SpotifyOAuth::default()
            .client_id(client_id)
            .client_secret(client_secret)
            .scope("user-read-private")
            .redirect_uri(&format!("http://localhost:{}/spotify", CALLBACK_PORT))
            .build();
        (oauth.get_authorize_url(None, None), oauth)
    }

    //Authentication server for callback from spotify
    pub fn auth_server(oauth: &mut SpotifyOAuth) -> Result<Spotify, Box<dyn Error>> {
        //Prepare server
        let token: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let token_clone = token.clone();

        let server = Server::new(&format!("127.0.0.1:{}", CALLBACK_PORT), move |request| {
            router!(request, 
                (GET) (/spotify) => {
                    //Get token
                    if let Some(code) = request.get_param("code") {
                        let mut t = token_clone.lock().unwrap();
                        *t = Some(code);
                    }
                },
                _ => {}
            );
            //Navigate back
            rouille::Response::html(CALLBACK_HTML)
        }).unwrap();
        //Run server
        loop {
            if token.lock().unwrap().is_some() {
                break;
            }
            server.poll();
        }
        let token_lock = token.lock().unwrap();
        let token = token_lock.as_ref().unwrap();
        //Create client
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

    //Handle error and sleep if rate limit
    fn handle_rspotify_error(&self, error: failure::Error) -> Result<(), Box<dyn Error>> {
        let err: Result<ApiError, failure::Error> = error.downcast();

        //For some reason the fucking downcasting always fails no matter what I try, even as_fail and the other methods fail, so idk what next, very dirty
        let err_string = format!("{:?}", err);
        if err_string.starts_with("Err(RateLimited(") {
            //Extract delay
            let val = &err_string[16..err_string.find(")").ok_or("Invalid error message")?];
            let mut delay = 1;
            if val.starts_with("Some(") {
                delay = val[5..].parse().unwrap_or(1);
            }
            warn!("Spotify rate limit, waiting {}s", delay);
            //Add 100ms for safety
            sleep(Duration::from_millis(delay*1000 + 100));
            return Ok(());
        }

        Err(err.unwrap_err().into())
    }

    //Search tracks by query
    pub fn search_tracks(&self, query: &str, limit: u32) -> Result<Vec<FullTrack>, Box<dyn Error>> {
        match self.spotify.search(query, SearchType::Track, limit, 0, None, None) {
            Ok(results) => {
                let mut tracks = vec![];
                if let SearchResult::Tracks(tracks_page) = results {
                    tracks = tracks_page.items;
                }
                Ok(tracks)
            },
            Err(e) => {
                //Handle error and retry on rate limit
                self.handle_rspotify_error(e)?;
                self.search_tracks(query, limit)
            }
        }
    }

    pub fn audio_features(&self, id: &str) -> Result<AudioFeatures, Box<dyn Error>> {
        match self.spotify.audio_features(id) {
            Ok(f) => Ok(f),
            Err(e) => {
                //Handle error and retry on rate limit
                self.handle_rspotify_error(e)?;
                self.audio_features(id)
            }
        }
    }

}