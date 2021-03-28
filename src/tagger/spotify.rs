use std::error::Error;
use std::sync::{Arc, Mutex};
use rspotify::blocking::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::blocking::util::get_token_by_code;
use rspotify::blocking::client;
use rspotify::senum::SearchType;
use rspotify::model::search::SearchResult;
use rspotify::model::track::FullTrack;
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

    //Search tracks by query
    pub fn search_tracks(&self, query: &str, limit: u32) -> Result<Vec<FullTrack>, Box<dyn Error>> {
        let results = self.spotify.search(query, SearchType::Track, limit, 0, None, None)?;
        let mut tracks = vec![];
        if let SearchResult::Tracks(tracks_page) = results {
            tracks = tracks_page.items;
        }
        Ok(tracks)
    }

}