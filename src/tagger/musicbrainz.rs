use chrono::NaiveDate;
use rand::Rng;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::error::Error;
use std::time::Duration;

use crate::tagger::matcher::Matcher;
use crate::tagger::{MusicPlatform, TaggerConfig, Track, TrackMatcher};

pub struct MusicBrainz {
    client: Client,
}

impl MusicBrainz {
    pub fn new() -> MusicBrainz {
        MusicBrainz {
            client: Client::builder()
                .user_agent("OneTagger/1.0")
                .build()
                .unwrap(),
        }
    }

    /// Make GET request to MusicBrainz, rate limit inlcuded
    fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, Box<dyn Error>> {
        let mut new_query = query.to_owned();
        new_query.push(("fmt", "json"));
        debug!("MusicBrainz GET: {} {:?}", path, new_query);

        let response = self
            .client
            .get(&format!("https://musicbrainz.org/ws/2{}", path))
            .query(&new_query)
            .send()?;
        if response.status() == StatusCode::SERVICE_UNAVAILABLE {
            warn!("MusicBrainz rate limit hit! Waiting...");
            // Use random rate limit delay because threading
            std::thread::sleep(Duration::from_millis(
                rand::thread_rng().gen_range(1000..3000),
            ));
            return self.get(path, &query);
        }
        Ok(response.error_for_status()?.json()?)
    }

    /// Search tracks (recordings)
    pub fn search(&self, query: &str) -> Result<RecordingSearchResults, Box<dyn Error>> {
        let results: RecordingSearchResults =
            self.get("/recording", &[("query", query), ("limit", "100")])?;
        Ok(results)
    }

    /// Get full release for recording
    pub fn full_release(&self, recording_id: &str) -> Result<BrowseReleases, Box<dyn Error>> {
        let results: BrowseReleases = self.get(
            "/release",
            &[
                ("recording", recording_id),
                ("inc", "labels isrcs recordings genres tags"),
            ],
        )?;
        Ok(results)
    }

    /// Add info from release to track
    pub fn extend_track(track: &mut Track, releases: BrowseReleases) {
        if let Some(release) = releases.releases.first() {
            // Add cover
            if release.cover_art_archive.back || release.cover_art_archive.front {
                track.artwork_url = Some(format!(
                    "https://coverartarchive.org/release/{}/{}",
                    release.id,
                    match release.cover_art_archive.front {
                        true => "front",
                        false => "back",
                    }
                ));
            }
            track.album = Some(release.title.to_string());
            //track.release_id = release.id.to_string();
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
            // Genres
            track.genres = Some(release.genres.iter().map(|g| g.name.to_string()).collect());
        }
    }
}

impl TrackMatcher for MusicBrainz {
    fn match_track(
        &self,
        local: &Track,
        config: &TaggerConfig,
    ) -> Result<Option<(f64, Track)>, Box<dyn Error>> {
        let query = format!(
            "{} {}~",
            local.artist.unwrap_or_default(),
            local.title.unwrap_or_default()
        );
        match self.search(&query) {
            Ok(results) => {
                let tracks: Vec<Track> = results.recordings.into_iter().map(|r| r.into()).collect();
                if let Some((accuracy, mut track)) = Matcher::match_track(&local, &tracks, &config)
                {
                    match self
                        .full_release(track.musicbrainz.unwrap().track_id.to_string().as_str())
                    {
                        Ok(releases) => MusicBrainz::extend_track(&mut track, releases),
                        Err(e) => {
                            warn!("Failed extending MusicBrainz track! {}", e);
                        }
                    }
                    return Ok(Some((accuracy, track)));
                }
            }
            Err(e) => {
                error!("MusicBrainz search failed. Query: {}. {}", query, e);
                return Err(e);
            }
        }
        Ok(None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSearchResults {
    pub count: usize,
    pub offset: usize,
    pub recordings: Vec<Recording>,
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
    pub isrcs: Option<Vec<String>>,
}

impl Into<Track> for Recording {
    fn into(self) -> Track {
        Track {
            platform: Some(MusicPlatform::MusicBrainz),
            title: Some(self.title),
            artists: Some(
                self.artist_credit
                    .unwrap_or(Vec::new())
                    .into_iter()
                    .map(|a| a.name)
                    .collect(),
            ),
            album_artists: Some(
                self.releases
                    .as_ref()
                    .unwrap_or(&Vec::new())
                    .first()
                    .map(|r| {
                        r.artist_credit.as_ref().map(|a| {
                            a.into_iter()
                                .map(|artist| artist.name.to_string())
                                .collect()
                        })
                    })
                    .flatten()
                    .unwrap_or(vec![]),
            ),
            album: self
                .releases
                .as_ref()
                .unwrap_or(&Vec::new())
                .first()
                .map(|a| a.title.to_string()),
            //url: format!("https://musicbrainz.org/recording/{}", self.id),
            //track_id: Some(self.id),
            /*
            release_id: self
                .releases
                .unwrap_or(vec![])
                .first()
                .map(|r| r.id.to_string())
                .unwrap_or(String::new()),
            */
            duration: Some(
                self.length
                    .map(|l| Duration::from_millis(l))
                    .unwrap_or(Duration::ZERO),
            ),
            release_year: self
                .first_release_date
                .clone()
                .map(|d| (d.len() >= 4).then(|| d[0..4].parse().ok()).flatten())
                .flatten(),
            release_date: self
                .first_release_date
                .map(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
                .flatten(),
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
    pub cover_art_archive: CoverArtArchive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CoverArtArchive {
    pub back: bool,
    pub front: bool,
    pub artwork: bool,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LabelInfoResult {
    Array(Vec<LabelInfo>),
    Single(LabelInfo),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ReleaseMedia {
    pub tracks: Vec<MusicBrainzTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MusicBrainzTrack {
    pub id: String,
    pub position: usize,
    pub recording: Recording,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Genre {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ArtistCredit {
    pub name: String,
    pub artist: Artist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Artist {
    pub name: String,
    pub id: String,
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
    pub label: Option<Label>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Label {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BrowseReleases {
    pub release_offset: usize,
    pub release_count: usize,
    pub releases: Vec<Release>,
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
            m.full_release(&r.id)
                .expect("Failed getting full release info");
        }
    }
}
