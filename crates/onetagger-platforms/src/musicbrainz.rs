use anyhow::Error;
use std::time::Duration;
use chrono::NaiveDate;
use rand::Rng;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use onetagger_tagger::{Track, AutotaggerSource, AudioFileInfo, TaggerConfig, MatchingUtils, TrackNumber, AutotaggerSourceBuilder, PlatformInfo, supported_tags, TrackMatch};

pub struct MusicBrainz {
    client: Client
}

impl MusicBrainz {
    pub fn new() -> MusicBrainz {
        MusicBrainz {
            client: Client::builder()
                .user_agent("OneTagger/1.0")
                .build()
                .unwrap()
        }
    }

    /// Make GET request to MusicBrainz, rate limit inlcuded
    fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<T, Error> {
        let mut new_query = query.to_owned();
        new_query.push(("fmt", "json"));
        debug!("MusicBrainz GET: {} {:?}", path, new_query);

        let response = self.client.get(&format!("https://musicbrainz.org/ws/2{}", path))
            .query(&new_query)
            .send()?;
        if response.status() == StatusCode::SERVICE_UNAVAILABLE {
            warn!("MusicBrainz rate limit hit! Waiting...");
            // Use random rate limit delay because threading
            std::thread::sleep(Duration::from_millis(rand::thread_rng().gen_range(1000..3000)));
            return self.get(path, &query);
        }
        Ok(response.error_for_status()?.json()?)
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
        if let Some(release) = releases.releases.first() {
            // Add cover
            if release.cover_art_archive.back || release.cover_art_archive.front {
                track.art = Some(format!("https://coverartarchive.org/release/{}/{}", release.id, match release.cover_art_archive.front {
                    true => "front",
                    false => "back"
                }));
            }
            track.album = Some(release.title.to_string());
            track.release_id = release.id.to_string();
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
                |m| m.tracks.iter().find(|t| &t.recording.id == track.track_id.as_ref().unwrap())
            ).next() {
                track.track_number = Some(TrackNumber::Number(mb_track.position as i32));
            }

            // Genres
            track.genres = release.genres.iter().map(|g| g.name.to_string()).collect();
        }
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
        let releases = self.full_release(track.track_id.as_ref().unwrap())?;
        MusicBrainz::extend_track(track, releases);
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
        let mut release = None;
        if let Some(releases) = self.releases.as_ref() {
            for r in releases {
                if !r.release_group.secondary_types.as_ref().unwrap_or(&vec![]).contains(&"compilation".to_string()) {
                    release = Some(r);
                    break;
                }
            }
            if release.is_none() {
                release = releases.first();
            }
        }

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
            release_id: release.map(|r| r.id.to_string()).unwrap_or(String::new()),
            duration: self.length.map(|l| Duration::from_millis(l)).unwrap_or(Duration::ZERO),
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

pub struct MusicBrainzBuilder;

impl AutotaggerSourceBuilder for MusicBrainzBuilder {
    fn new() -> MusicBrainzBuilder {
        MusicBrainzBuilder
    }

    fn get_source(&mut self, _config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Error> {
        Ok(Box::new(MusicBrainz::new()))
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
    pub fn test_search_serialization() {
        let m = super::MusicBrainz::new();
        m.search("marting garrix animals").expect("Search 1 failed");
        m.search("illenium needed you").expect("Search 2 failed");
        m.search("michael jackson").expect("Search 3 failed");
        m.search("test").expect("Search 4 failed");
    }

    #[test]
    pub fn text_full_release_serialization() {
        let m = super::MusicBrainz::new();
        let results = m.search("illenium needed you").expect("Search failed!");
        for r in results.recordings {
            println!("ID: {}", r.id);
            m.full_release(&r.id).expect("Failed getting full release info");
        }
    }
}