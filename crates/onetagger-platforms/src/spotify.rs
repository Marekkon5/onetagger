use crate::cooldown::{retry_delay, ProviderCooldown};
use crate::request_cache::RequestCache;
use crate::request_pacer::RequestPacer;
use anyhow::Error;
use chrono::NaiveDate;
use std::time::Duration;
use rspotify::clients::{BaseClient, OAuthClient};
use rspotify::model::{TrackId, Id, AlbumId, ArtistId, Modality, Page, SimplifiedArtist};
use rspotify::{Credentials, Config, AuthCodeSpotify, OAuth, scopes, ClientError, ClientResult};
use rspotify::model::album::FullAlbum;
use rspotify::model::artist::FullArtist;
use serde::{Deserialize, de::DeserializeOwned};
use rspotify::model::track::FullTrack;
use rspotify::model::audio::AudioFeatures;
use onetagger_shared::{Settings, WEBSERVER_CALLBACKS, PORT};
use onetagger_tagger::{AutotaggerSource, Track, TaggerConfig, AudioFileInfo, MatchingUtils, TrackNumber, AutotaggerSourceBuilder, PlatformInfo, supported_tags, SupportedTag, TrackMatch};

static COOLDOWN: ProviderCooldown = ProviderCooldown::new();
static PACER: RequestPacer = RequestPacer::new();

/// Spotify account and API types shared with the Audio Features tool.
pub use rspotify;


static PITCH_CLASS_MAJOR: [&'static str; 12] = ["C", "C#",   "D",  "D#",  "E",  "F",  "F#",  "G",  "G#",  "A",  "A#",  "B" ];
static PITCH_CLASS_MINOR: [&'static str; 12] = ["Cm", "Dbm", "Dm", "Ebm", "Em", "Fm", "Gbm", "Gm", "Abm", "Am", "Bbm", "Bm"];

// Development Mode accepts up to ten candidates in one search request.
const SEARCH_PAGE_SIZE: u32 = 10;
// Before pagination this public method could request at most 50 candidates in total.
const MAX_SEARCH_CANDIDATES: u32 = 50;

#[derive(Deserialize)]
#[serde(bound(deserialize = "T: DeserializeOwned"))]
struct TrackSearch<T: DeserializeOwned> {
    tracks: Page<T>,
}

fn collect_search_pages<T: DeserializeOwned>(
    limit: u32,
    mut fetch: impl FnMut(u32, u32) -> Result<String, Error>,
) -> Result<Vec<T>, Error> {
    anyhow::ensure!(limit <= MAX_SEARCH_CANDIDATES, "Spotify search supports up to {MAX_SEARCH_CANDIDATES} candidates");
    let mut items = Vec::new();
    let mut offset = 0;
    while offset < limit {
        let count = (limit - offset).min(SEARCH_PAGE_SIZE);
        let body = fetch(offset, count)?;
        let page = serde_json::from_str::<TrackSearch<T>>(&body)?.tracks;
        items.extend(page.items.into_iter().take(count as usize));
        if page.next.is_none() {
            break;
        }
        offset += count;
    }
    Ok(items)
}

/// The SDK defaults a missing popularity to zero. File tags need to distinguish the two.
#[derive(Clone, Deserialize)]
pub struct AudioFeaturesTrack {
    pub id: Option<TrackId<'static>>,
    pub name: String,
    pub artists: Vec<SimplifiedArtist>,
    pub popularity: Option<u32>,
}

#[derive(Clone)]
pub struct Spotify {
    pub spotify: AuthCodeSpotify,
    slowdown: u16,
    album_cache: RequestCache<String, FullAlbum>,
    artist_cache: RequestCache<String, FullArtist>,
}

impl Spotify {
    fn with_client(spotify: AuthCodeSpotify) -> Self {
        Self { spotify, slowdown: 2, album_cache: RequestCache::default(), artist_cache: RequestCache::default() }
    }
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
        Some(Spotify::with_client(client))
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
        Ok(Spotify::with_client(spotify))
    }

    /// Authorize from URL
    pub fn auth_token_code(spotify: AuthCodeSpotify, url: &str) -> Result<Spotify, Error> {
        let code = spotify.parse_response_code(url).ok_or(anyhow!("Invalid token url!"))?;
        spotify.request_token(&code)?;
        spotify.auto_reauth()?;
        spotify.write_token_cache()?;
        Ok(Spotify::with_client(spotify))
    }

    /// Wrapper for rate limit 
    fn rate_limit_wrap<F, R>(&self, f: F) -> Result<R, Error>
    where
        F: Fn(&Spotify) -> ClientResult<R>
    {
        COOLDOWN.check()?;
        PACER.wait_scaled(Duration::from_millis(250), self.slowdown);
        COOLDOWN.check()?;
        match f(self) {
            Ok(r) => {
                COOLDOWN.record_success();
                Ok(r)
            },
            Err(ClientError::Http(http)) => {
                match *http {
                    rspotify::http::HttpError::StatusCode(r) => {
                        // Rate limit
                        if matches!(r.status(), 429 | 503) {
                            let fallback = Duration::from_secs(15);
                            let delay = r.header("Retry-After")
                                .map(|header| retry_delay(Some(header.as_ref()), fallback))
                                .unwrap_or(fallback);
                            return Err(COOLDOWN.pause(delay).into());
                        }
                        return Err(anyhow!("Unknown Spotify status code: {}", r.status()));
                    },
                    e => return Err(e.into())
                }
            },
            Err(e) => Err(e.into()),
        }
    }

    /// Search tracks by query.
    pub fn search_tracks(&self, query: &str, limit: u32) -> Result<Vec<FullTrack>, Error> {
        self.search_pages(query, limit)
    }

    pub fn search_audio_features_tracks(&self, query: &str, limit: u32) -> Result<Vec<AudioFeaturesTrack>, Error> {
        self.search_pages(query, limit)
    }

    fn search_pages<T: DeserializeOwned>(&self, query: &str, limit: u32) -> Result<Vec<T>, Error> {
        collect_search_pages(limit, |offset, count| {
            let count = count.to_string();
            let offset = offset.to_string();
            let params = [
                ("q", query), ("type", "track"), ("limit", &count), ("offset", &offset),
            ].into_iter().collect();
            self.rate_limit_wrap(|s| s.spotify.api_get("search", &params))
        })
    }

    /// Fetch audio features for track id
    pub fn audio_features(&self, _id: &TrackId) -> Result<AudioFeatures, Error> {
        // self.rate_limit_wrap(|s| s.spotify.track_features(id.to_owned()))
        return Err(anyhow!("Spotify deprecated AudioFeatures"))
    }

    /// Fetch full album
    pub fn album(&self, id: &AlbumId) -> Result<FullAlbum, Error> {
        self.album_cache.get_or_try_init(id.id().to_owned(), || {
            let album = self.rate_limit_wrap(|s| s.spotify.album(id.to_owned(), None))?;
            anyhow::ensure!(album.id.id() == id.id(), "Spotify returned a different album");
            Ok(album)
        })
    }

    /// Fetch full artist
    pub fn artist(&self, id: &ArtistId) -> Result<FullArtist, Error> {
        self.artist_cache.get_or_try_init(id.id().to_owned(), || {
            let artist = self.rate_limit_wrap(|s| s.spotify.artist(id.to_owned()))?;
            anyhow::ensure!(artist.id.id() == id.id(), "Spotify returned a different artist");
            Ok(artist)
        })
    }


    /// Extend track for autotagger
    #[allow(deprecated, reason = "Eligible apps can still return the album label and artist genres")]
    fn extend_track_spotify(&self, track: &mut Track, config: &TaggerConfig) -> Result<(), Error> {
        // Fetch album
        let want_album = config.tag_enabled(SupportedTag::Label) || config.tag_enabled(SupportedTag::Genre);
        if let Some(release_id) = track.release_id.as_deref().filter(|_| want_album) {
            match self.album(&AlbumId::from_id(release_id)?) {
                Ok(album) => {
                    if let Some(label) = album.label {
                        track.label = Some(label);
                    }
                    track.genres.extend(album.genres);
                }
                Err(e) => warn!("Failed to fetch album data: {}", e),
            }
        }

        // Fetch genres from artist if album has them unavailable
        if config.tag_enabled(SupportedTag::Genre) && track.custom.get("artist_ids").is_some() {
            for id in track.custom.get("artist_ids").unwrap().split('\0').filter(|id| !id.is_empty()) {
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
            if let Some(track) = self.search_tracks(&query, 1)?.into_iter().next() {
                return Ok(vec![TrackMatch::new_isrc(full_track_to_track(track))]);
            }
        };

        // Normal search
        let query = format!("{} {}", info.artist()?, MatchingUtils::clean_title(info.title()?));
        let results = self.search_tracks(&query, 20)?;
        let tracks = results.into_iter().map(full_track_to_track).collect();
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
        let slowdown = config.provider_slowdown();
        let config = config.spotify.clone().ok_or(anyhow!("Missing Spotify config!"))?;
        let mut spotify = Spotify::try_cached_token(&config.client_id, &config.client_secret).ok_or(anyhow!("Spotify not authorized!"))?;
        spotify.slowdown = slowdown;
        Ok(Box::new(spotify))
    }

    fn info(&self) -> PlatformInfo {
        PlatformInfo {
            id: "spotify".to_string(),
            name: "Spotify".to_string(),
            description: "Matches using ISRC or artist and title. Development Mode requires Spotify Premium for the app owner.".to_string(),
            icon: include_bytes!("../assets/spotify.png"),
            max_threads: 1,
            version: "1.0.0".to_string(),
            custom_options: Default::default(),
            requires_auth: true,
            supported_tags: supported_tags!(Title, Artist, AlbumArtist, Album, AlbumArt, URL, TrackId, ReleaseId, Duration, TrackNumber, ISRC, ReleaseDate, Genre, Key, Label, Explicit)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn track(number: u32) -> Value {
        json!({
            "name": format!("Track {number}"), "album": {
                "name": "Album", "artists": [], "images": [], "external_urls": {}
            }, "artists": [], "disc_number": 1, "duration_ms": 1000, "explicit": false,
            "external_ids": {}, "external_urls": {}, "is_local": false,
            "track_number": number, "type": "track"
        })
    }

    #[test]
    fn searches_in_ten_result_pages_and_reads_tracks_without_popularity() {
        let mut requests = Vec::new();
        let matches: Vec<FullTrack> = collect_search_pages(20, |offset, count| {
            requests.push((offset, count));
            let end = (offset + count).min(14);
            let items: Vec<_> = (offset..end).map(track).collect();
            let body = json!({"tracks": {
                "href": "", "items": items, "limit": count, "next": (end < 14).then_some("next page"),
                "offset": offset, "previous": null, "total": 14
            }});
            Ok(body.to_string())
        }).unwrap();
        assert_eq!(requests, [(0, 10), (10, 10)]);
        assert_eq!(matches.len(), 14);
        assert_eq!(matches[10].name, "Track 10");
        assert!(collect_search_pages::<FullTrack>(51, |_, _| panic!("oversized search made a request")).is_err());
    }

    #[test]
    #[allow(deprecated, reason = "Verify responses both with and without legacy artist genres")]
    fn absent_popularity_is_distinct_from_zero_and_artist_genres_are_optional() {
        let mut item = track(1);
        assert_eq!(serde_json::from_value::<AudioFeaturesTrack>(item.clone()).unwrap().popularity, None);
        item["popularity"] = json!(0);
        assert_eq!(serde_json::from_value::<AudioFeaturesTrack>(item).unwrap().popularity, Some(0));

        let mut artist = json!({"id": "abc", "name": "Artist", "external_urls": {}, "href": "", "images": []});
        assert!(serde_json::from_value::<FullArtist>(artist.clone()).unwrap().genres.is_empty());
        artist["genres"] = json!(["House"]);
        assert_eq!(serde_json::from_value::<FullArtist>(artist).unwrap().genres, ["House"]);
    }

    #[test]
    fn missing_album_and_artist_ids_do_not_discard_the_search_match() {
        let provider = Spotify::with_client(AuthCodeSpotify::default());
        let mut track = full_track_to_track(serde_json::from_value(track(1)).unwrap());
        let config = TaggerConfig {
            tags: vec![SupportedTag::Genre, SupportedTag::Label],
            ..Default::default()
        };
        provider.extend_track_spotify(&mut track, &config).unwrap();
        assert_eq!(track.title, "Track 1");
        assert!(track.label.is_none());
        assert!(track.genres.is_empty());
    }
}
