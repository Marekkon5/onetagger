use crate::cooldown::{retry_delay, ProviderCooldown};
use crate::request_cache::RequestCache;
use crate::request_pacer::RequestPacer;
use anyhow::Error;
use std::time::Duration;
use chrono::NaiveDate;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use onetagger_tagger::{Track, AutotaggerSource, AudioFileInfo, TaggerConfig, MatchingUtils, TrackNumber, AutotaggerSourceBuilder, PlatformInfo, supported_tags, TrackMatch};

static COOLDOWN: ProviderCooldown = ProviderCooldown::new();
static PACER: RequestPacer = RequestPacer::new();

pub struct MusicBrainz {
    client: Client,
    release_cache: RequestCache<String, Release>,
}

impl MusicBrainz {
    pub fn new() -> MusicBrainz {
        MusicBrainz {
            client: Client::builder()
                .user_agent("OneTagger (+https://github.com/fantuz/onetagger)")
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            release_cache: RequestCache::default(),
        }
    }

    /// Make GET request to MusicBrainz, rate limit inlcuded
    fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<T, Error> {
        COOLDOWN.check()?;
        let mut new_query = query.to_owned();
        new_query.push(("fmt", "json"));
        debug!("MusicBrainz GET: {} {:?}", path, new_query);
        PACER.wait(Duration::from_secs(1));
        COOLDOWN.check()?;
        let response = self.client.get(&format!("https://musicbrainz.org/ws/2{}", path))
            .query(&new_query)
            .send()?;
        if matches!(response.status(), StatusCode::SERVICE_UNAVAILABLE | StatusCode::TOO_MANY_REQUESTS) {
            let delay = retry_delay(response.headers().get(reqwest::header::RETRY_AFTER)
                .and_then(|header| header.to_str().ok()), Duration::from_secs(2));
            return Err(COOLDOWN.pause(delay).into());
        }
        Ok(response.error_for_status()?.json()?)
    }

    fn release(&self, id: &str) -> Result<Release, Error> {
        self.release_with(id, || self.get(&format!("/release/{id}"), &[
            ("inc", "artist-credits labels recordings genres"),
        ]))
    }

    fn release_with(&self, id: &str, fetch: impl FnOnce() -> Result<Release, Error>) -> Result<Release, Error> {
        self.release_cache.get_or_try_init(id.to_owned(), || {
            let release = fetch()?;
            anyhow::ensure!(release.id == id, "MusicBrainz returned a different release");
            Ok(release)
        })
    }

    /// Search tracks (recordings)
    pub fn search(&self, query: &str) -> Result<RecordingSearchResults, Error> {
        let results: RecordingSearchResults = self.get("/recording", &[
            ("query", query),
            ("limit", "100")
        ])?;
        Ok(results)
    }

    /// Get full release for recording
    pub fn full_release(&self, recording_id: &str) -> Result<BrowseReleases, Error> {
        let results: BrowseReleases = self.get("/release", &[
            ("recording", recording_id),
            ("inc", "labels isrcs recordings genres tags")
        ])?;
        Ok(results)
    }

    /// Add info from release to track
    pub fn extend_track(track: &mut Track, releases: BrowseReleases) {
        let release = match &track.release_id {
            Some(id) => releases.releases.iter().find(|release| release.id == *id),
            None if releases.releases.len() == 1 => releases.releases.first(),
            None => None,
        };
        if let Some(release) = release { Self::apply_release(track, release); }
    }

    fn apply_release(track: &mut Track, release: &Release) {
        if let Some(credits) = &release.artist_credit {
            track.album_artists = credits.iter().map(|artist| artist.name.clone()).collect();
        }
        // Add cover
        if release.cover_art_archive.back || release.cover_art_archive.front {
            track.art = Some(format!("https://coverartarchive.org/release/{}/{}", release.id, match release.cover_art_archive.front {
                true => "front",
                false => "back"
            }));
        }
        track.album = Some(release.title.to_string());
        track.release_id = Some(release.id.to_string());
        // Label
        if let Some(label_info) = match &release.label_info {
            LabelInfoResult::Array(labels) => labels.first(),
            LabelInfoResult::Single(label) => Some(label),
        } {
            if let Some(label) = &label_info.label {
                track.label = Some(label.name.to_string());
            }
            track.catalog_number = label_info.catalog_number.clone();
        }
        // Get track for track number
        if let Some(mb_track) = release.media.iter().filter_map(
            |m| m.tracks.iter().find(|t| Some(&t.recording.id) == track.track_id.as_ref())
        ).next() {
            track.track_number = Some(TrackNumber::Number(mb_track.position as i32));
        }

        // Genres
        track.genres = release.genres.iter().map(|g| g.name.to_string()).collect();
    }
}

impl AutotaggerSource for MusicBrainz {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Vec<TrackMatch>, Error> {
        let query = format!("{} {}~", info.artist()?, MatchingUtils::clean_title(info.title()?));
        match self.search(&query) {
            Ok(results) => {
                let tracks: Vec<Track> = results.recordings.into_iter().map(|r| r.into()).collect();
                return Ok(MatchingUtils::match_track(&info, &tracks, &config, true));
            }
            Err(e) => {
                error!("MusicBrainz search failed. Query: {}. {}", query, e);
                return Err(e);
            }
        }

    }

    fn extend_track(&mut self, track: &mut Track, _config: &TaggerConfig) -> Result<(), Error> {
        if let Some(id) = &track.release_id {
            let release = self.release(id)?;
            Self::apply_release(track, &release);
        } else {
            let id = track.track_id.as_deref().ok_or_else(|| anyhow!("Missing recording ID"))?;
            let releases = self.full_release(id)?;
            MusicBrainz::extend_track(track, releases);
        }
        Ok(())
    }

    
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSearchResults {
    pub count: usize,
    pub offset: usize,
    pub recordings: Vec<Recording>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Recording {
    pub id: String,
    pub title: String,
    pub length: Option<u64>,
    pub artist_credit: Option<Vec<ArtistCredit>>,
    pub first_release_date: Option<String>,
    pub releases: Option<Vec<ReleaseSmall>>,
    pub isrcs: Option<Vec<String>>
}

impl Into<Track> for Recording {
    fn into(self) -> Track {
        // Find release with priority to not use compilations
        let release = self.releases.as_ref().and_then(|releases| {
            releases.iter().find(|r| !r.release_group.secondary_types.iter().flatten()
                .any(|kind| kind.eq_ignore_ascii_case("compilation"))).or_else(|| releases.first())
        });

        Track {
            platform: "musicbrainz".to_string(),
            title: self.title,
            version: None,
            artists: self.artist_credit.unwrap_or(Vec::new()).into_iter().map(|a| a.name).collect(),
            album_artists: release
                .map(|r| r.artist_credit.as_ref().map(|a| a.into_iter().map(|artist| artist.name.to_string()).collect()))
                .flatten().unwrap_or(vec![]),
            album: release.map(|a| a.title.to_string()),
            url: format!("https://musicbrainz.org/recording/{}", self.id),
            track_id: Some(self.id),
            release_id: release.map(|r| r.id.to_string()),
            duration: self.length.map(|l| Duration::from_millis(l)).unwrap_or(Duration::ZERO).into(),
            release_year: self.first_release_date.clone().map(|d| (d.len() >= 4).then(|| d[0..4].parse().ok()).flatten()).flatten(),
            release_date: self.first_release_date.map(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()).flatten(),
            isrc: self.isrcs.map(|v| v.first().map(String::from)).flatten(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ReleaseSmall {
    pub id: String,
    pub title: String,
    pub artist_credit: Option<Vec<ArtistCredit>>,
    pub release_group: ReleaseGroup,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Release {
    pub id: String,
    pub title: String,
    pub artist_credit: Option<Vec<ArtistCredit>>,
    pub date: Option<String>,
    pub barcode: Option<String>,
    pub genres: Vec<Genre>,
    pub label_info: LabelInfoResult,
    pub media: Vec<ReleaseMedia>,
    pub cover_art_archive: CoverArtArchive
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CoverArtArchive {
    pub back: bool,
    pub front: bool,
    pub artwork: bool,
    pub count: usize
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LabelInfoResult {
    Array(Vec<LabelInfo>),
    Single(LabelInfo)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ReleaseMedia {
    pub tracks: Vec<MusicBrainzTrack>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MusicBrainzTrack {
    pub id: String,
    pub position: usize,
    pub recording: Recording
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Genre {
    pub id: String,
    pub name: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ArtistCredit {
    pub name: String,
    pub artist: Artist
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Artist {
    pub name: String,
    pub id: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ReleaseGroup {
    pub id: String,
    pub title: String,
    pub secondary_types: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LabelInfo {
    pub catalog_number: Option<String>,
    pub label: Option<Label>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Label {
    pub name: String,
    pub id: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BrowseReleases {
    pub release_offset: usize,
    pub release_count: usize,
    pub releases: Vec<Release>
}

#[derive(Debug, Clone)]
pub struct MusicBrainzBuilder {
    release_cache: RequestCache<String, Release>,
}

impl AutotaggerSourceBuilder for MusicBrainzBuilder {
    fn new() -> MusicBrainzBuilder {
        MusicBrainzBuilder { release_cache: RequestCache::default() }
    }

    fn get_source(&mut self, _config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Error> {
        let mut source = MusicBrainz::new();
        source.release_cache = self.release_cache.clone();
        Ok(Box::new(source))
    }

    fn info(&self) -> PlatformInfo {
        PlatformInfo {
            id: "musicbrainz".to_string(),
            name: "MusicBrainz".to_string(),
            description: "Published & unpublished, western & non-western".to_string(),
            icon: include_bytes!("../assets/musicbrainz.png"),
            max_threads: 4,
            version: "1.0.0".to_string(),
            custom_options: Default::default(),
            requires_auth: false,
            supported_tags: supported_tags!(Title, Artist, AlbumArtist, Album, URL, ReleaseId, TrackId, Duration, ISRC, Label, CatalogNumber, TrackNumber, Genre),
        }
    }
}


/// Test if API works properly
mod tests {
    #[test]
    fn selected_release_reuse_keeps_per_track_numbers_and_rejects_wrong_editions() {
        use super::*;
        use serde_json::json;
        let source = MusicBrainz::new();
        let release = |id: &str| serde_json::from_value(json!({"id":id, "title":id,
            "artist-credit":[{"name":"Album artist","artist":{"id":"artist","name":"Album artist"}}],
            "label-info":[], "genres":[], "media":[{"tracks":[
                {"id":"medium-one","position":1,"recording":{"id":"song-one","title":"One"}},
                {"id":"medium-two","position":2,"recording":{"id":"song-two","title":"Two"}}
            ]}], "cover-art-archive":{"back":false,"front":true,"artwork":true,"count":1}})).unwrap();
        assert!(source.release_with("edition-a", || Ok(release("edition-b"))).is_err());
        let selected = source.release_with("edition-a", || Ok(release("edition-a"))).unwrap();
        let reused = source.release_with("edition-a", || -> Result<Release, Error> { panic!("duplicate HTTP fetch") }).unwrap();
        for (recording, number, details) in [("song-one", 1, &selected), ("song-two", 2, &reused)] {
            let mut track = Track { track_id: Some(recording.into()), ..Default::default() };
            MusicBrainz::apply_release(&mut track, details);
            assert_eq!(track.release_id.as_deref(), Some("edition-a"));
            assert_eq!(track.track_number, Some(TrackNumber::Number(number)));
            assert_eq!(track.album_artists, vec!["Album artist"]);
        }
    }

    #[test]
    fn enrichment_keeps_the_selected_edition_across_browse_order_and_missing_pages() {
        use super::*;
        use serde_json::json;
        let small = |id: &str, kinds: &[&str]| json!({"id":id, "title":id, "release-group":{
            "id":id, "title":id, "secondary-types":kinds}});
        let full = |id: &str| json!({"id":id, "title":id, "artist-credit":[{"name":id,"artist":{"id":id,"name":id}}],
            "label-info":[{"label":{"id":id,"name":id}}], "media":[], "genres":[],
            "cover-art-archive":{"back":false,"front":true,"artwork":true,"count":1}});
        let rec: Recording = serde_json::from_value(json!({"id":"song","title":"Song",
            "releases":[small("compilation",&["Compilation"]),small("original",&[])]})).unwrap();
        let mut selected: Track = rec.into();
        assert_eq!(selected.release_id.as_deref(), Some("original"));
        let browse = |releases| serde_json::from_value(json!({"release-count":2,"release-offset":0,"releases":releases})).unwrap();
        MusicBrainz::extend_track(&mut selected, browse(vec![full("compilation")]));
        assert_eq!(selected.label, None);
        MusicBrainz::extend_track(&mut selected, browse(vec![full("compilation"),full("original")]));
        assert_eq!(selected.release_id.as_deref(), Some("original"));
        assert_eq!(selected.label.as_deref(), Some("original"));
        assert_eq!(selected.album_artists, vec!["original"]);
        assert!(selected.art.unwrap().contains("/original/"));
    }

    #[test]
#[ignore = "Live provider integration test; run explicitly with --ignored"]
    pub fn test_search_serialization() {
        let m = super::MusicBrainz::new();
        m.search("marting garrix animals").expect("Search 1 failed");
        m.search("illenium needed you").expect("Search 2 failed");
        m.search("michael jackson").expect("Search 3 failed");
        m.search("test").expect("Search 4 failed");
    }

    #[test]
#[ignore = "Live provider integration test; run explicitly with --ignored"]
    pub fn text_full_release_serialization() {
        let m = super::MusicBrainz::new();
        let results = m.search("illenium needed you").expect("Search failed!");
        for r in results.recordings {
            println!("ID: {}", r.id);
            m.full_release(&r.id).expect("Failed getting full release info");
        }
    }
}
