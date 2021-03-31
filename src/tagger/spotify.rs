use std::error::Error;
use rspotify::client::SpotifyBuilder;
use rspotify::oauth2::{CredentialsBuilder, OAuthBuilder};
use rspotify::scopes;
use rspotify::client;
use rspotify::model::search::SearchResult;
use rspotify::model::track::FullTrack;
use rspotify::model::audio::AudioFeatures;
use rspotify::model::enums::types::SearchType;
use rspotify::model::idtypes::Id;

static CALLBACK_PORT: u16 = 36914;

#[derive(Clone)]
pub struct Spotify {
    pub spotify: client::Spotify
}

impl Spotify {
    //Create Spotify client
    pub fn create_client(client_id: &str, client_secret: &str) -> Result<Spotify, Box<dyn Error>> {
        let creds = CredentialsBuilder::default()
            .id(client_id)
            .secret(client_secret)
            .build()?;
        let oauth = OAuthBuilder::default()
            .scope(scopes!("user-read-private"))
            .redirect_uri(&format!("http://localhost:{}/spotify", CALLBACK_PORT))
            .build()?;
        let client = SpotifyBuilder::default()
            .oauth(oauth)
            .credentials(creds)
            .build()?;
        Ok(Spotify {
            spotify: client
        })
    }

    //Authenticate Spotify
    pub fn authenticate(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(self.spotify.request_client_token_without_cache()?)
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

    //Get audio features with rate limit
    pub fn audio_features(&self, id: &str) -> Result<AudioFeatures, Box<dyn Error>> {
        Ok(self.spotify.track_features(&Id::from_id(id)?)?)
    }

}