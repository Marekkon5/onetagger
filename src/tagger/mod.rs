use crate::tag::{
    AudioFileFormat, CoverType, Field, Tag, TagDate, TagImpl, TagSeparators, UITag, EXTENSIONS,
};
use crate::ui::{OTError, Settings};
use chrono::{Datelike, NaiveDate};
use execute::Execute;
use regex::Regex;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use threadpool::ThreadPool;
use walkdir::WalkDir;

pub mod helpers;
pub mod matcher;

pub mod beatport;
pub mod discogs;
pub mod itunes;
pub mod junodownload;
pub mod musicbrainz;
pub mod spotify;
pub mod traxsource;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MusicPlatform {
    Beatport,
    Traxsource,
    Discogs,
    JunoDownload,
    ITunes,
    MusicBrainz,
    Spotify,
    None,
}

impl Default for MusicPlatform {
    fn default() -> Self {
        MusicPlatform::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TaggerConfig {
    // Global
    pub platforms: Vec<MusicPlatform>,
    pub path: Option<String>,

    // Tags
    pub title: bool,
    pub artist: bool,
    pub album: bool,
    pub key: bool,
    pub bpm: bool,
    pub genre: bool,
    pub style: bool,
    pub label: bool,
    pub release_date: bool,
    pub publish_date: bool,
    pub album_art: bool,
    pub other_tags: bool,
    pub catalog_number: bool,
    pub urls: bool,
    pub ids: bool,
    pub version: bool,
    pub duration: bool,
    pub album_artist: bool,
    // 1T meta tags
    pub meta_tags: bool,

    // Advanced
    pub separators: TagSeparators,
    pub id3v24: bool,
    pub overwrite: bool,
    pub threads: i16,
    // From 0 to 1
    pub strictness: f64,
    pub merge_genres: bool,
    pub album_art_file: bool,
    pub camelot: bool,
    pub parse_filename: bool,
    pub filename_template: Option<String>,
    pub short_title: bool,
    pub match_duration: bool,
    // In seconds
    pub max_duration_difference: u64,
    pub match_by_id: bool,
    pub multiple_matches: MultipleMatchesSort,
    pub post_command: Option<String>,
    pub styles_options: StylesOptions,
    // Option to prevent update errors
    pub styles_custom_tag: Option<UITag>,

    // Platform specific
    pub beatport: BeatportConfig,
    pub discogs: DiscogsConfig,
}

// Beatport specific settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BeatportConfig {
    pub art_resolution: i64,
    pub max_pages: i64,
}

// Discogs specific settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiscogsConfig {
    pub token: Option<String>,
    pub max_results: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StylesOptions {
    Default,
    OnlyGenres,
    OnlyStyles,
    MergeToGenres,
    MergeToStyles,
    StylesToGenre,
    GenresToStyle,
    CustomTag,
}

impl Default for StylesOptions {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MultipleMatchesSort {
    Default,
    Oldest,
    Newest,
}

impl Default for MultipleMatchesSort {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Track {
    pub platform: Option<MusicPlatform>,
    pub path: Option<String>,
    pub format: Option<AudioFileFormat>,

    pub title: Option<String>,
    pub name: Option<String>,

    pub version: Option<String>,
    pub mix: Option<String>,
    pub remixer: Option<String>,
    pub remixers: Option<Vec<String>>,

    pub artist: Option<String>,
    pub artists: Option<Vec<String>>,
    pub main_artists: Option<Vec<String>>,
    pub feat_artists: Option<Vec<String>>,
    pub involved_artists: Option<Vec<String>>, // main + feat + remixer

    pub album: Option<String>,
    pub artwork_url: Option<String>,
    pub album_artist: Option<String>,
    pub album_artists: Option<Vec<String>>,

    pub bpm: Option<i64>,
    pub key: Option<String>,
    pub duration: Option<Duration>,
    pub explicitness: Option<Explicitness>,

    pub genre: Option<String>,
    pub genres: Option<Vec<String>>,
    pub styles: Option<Vec<String>>,

    pub track_number: Option<i64>,
    pub track_count: Option<i64>,
    pub disc_number: Option<i64>,
    pub disc_count: Option<i64>,

    pub release_date: Option<NaiveDate>,
    pub release_year: Option<i64>,
    pub publish_date: Option<NaiveDate>,
    pub publish_year: Option<i64>,

    pub isrc: Option<String>,
    pub label: Option<String>,
    pub catalog_number: Option<String>,
    pub copyright: Option<String>,

    pub beatport: Option<BeatportID>,
    pub discogs: Option<DiscogsID>,
    pub itunes: Option<ITunesID>,
    pub junodownload: Option<JunoDownloadID>,
    pub musicbrainz: Option<MusicBrainzID>,
    pub spotify: Option<SpotifyID>,
    pub traxsource: Option<TraxsourceID>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Explicitness {
    Explicit,
    Cleanned,
    NotExplicit,
}
impl Default for Explicitness {
    fn default() -> Self {
        Explicitness::NotExplicit
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BeatportID {
    pub track_id: i64,
    pub release_id: i64,

    pub track_url: String,
    pub release_url: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DiscogsID {
    pub track_id: i64,
    pub release_id: i64,

    pub track_url: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ITunesID {
    pub track_id: i64,
    pub artist_id: i64,
    pub release_id: i64,

    pub preview_url: String,
    pub track_url: String,
    pub artist_url: String,
    pub release_url: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct JunoDownloadID {
    pub track_id: i64,
    pub release_id: i64,

    pub track_url: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MusicBrainzID {
    pub track_id: i64,
    pub release_id: i64,

    pub track_url: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpotifyID {
    pub track_id: i64,
    pub release_id: i64,

    pub track_url: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TraxsourceID {
    pub track_id: i64,
    pub release_id: i64,

    pub track_url: String,
}

/*
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Track {
    pub platform: MusicPlatform,
    // Short title
    pub title: String,
    pub version: Option<String>,
    pub artists: Vec<String>,
    pub album_artists: Vec<String>,
    pub album: Option<String>,
    pub key: Option<String>,
    pub bpm: Option<i64>,
    pub genres: Vec<String>,
    pub styles: Vec<String>,
    pub art: Option<String>,
    pub url: String,
    pub label: Option<String>,
    pub catalog_number: Option<String>,
    //  Tag name, Value
    pub other: Vec<(String, String)>,
    pub track_id: Option<String>,
    pub release_id: String,
    pub duration: Duration,
    // Only year OR date should be available
    pub release_year: Option<i64>,
    pub release_date: Option<NaiveDate>,
    pub publish_year: Option<i64>,
    pub publish_date: Option<NaiveDate>
}
*/

const CAMELOT_NOTES: [(&str, &str); 35] = [
    ("Abm", "1A"),
    ("G#m", "1A"),
    ("B", "1B"),
    ("D#m", "2A"),
    ("Ebm", "2A"),
    ("Gb", "2B"),
    ("F#", "2B"),
    ("A#m", "3A"),
    ("Bbm", "3A"),
    ("C#", "3B"),
    ("Db", "3B"),
    ("Dd", "3B"),
    ("Fm", "4A"),
    ("G#", "4B"),
    ("Ab", "4B"),
    ("Cm", "5A"),
    ("D#", "5B"),
    ("Eb", "5B"),
    ("Gm", "6A"),
    ("A#", "6B"),
    ("Bb", "6B"),
    ("Dm", "7A"),
    ("F", "7B"),
    ("Am", "8A"),
    ("C", "8B"),
    ("Em", "9A"),
    ("G", "9B"),
    ("Bm", "10A"),
    ("D", "10B"),
    ("Gbm", "11A"),
    ("F#m", "11A"),
    ("A", "11B"),
    ("C#m", "12A"),
    ("Dbm", "12A"),
    ("E", "12B"),
];

impl Track {
    pub fn read_tags(path: &str, template: Option<Regex>) -> Result<Track, Box<dyn Error>> {
        let tag_wrap = Tag::load_file(&path, true)?;
        let tags = tag_wrap.tag();

        // Get title and artist from tag
        let mut title = tags
            .get_field(Field::Title)
            .map(|t| t.first().map(|t| t.to_owned()))
            .flatten();
        let mut artist = tags
            .get_field(Field::Artist)
            .map(|a| a.first().map(|a| a.to_owned()));

        // Parse filename
        if (title.is_none() || artist.is_none()) && template.is_some() {
            let p = Path::new(path);
            let filename = p.file_name().ok_or("Missing filename!")?.to_str().unwrap();

            if let Some(captures) = template.unwrap().captures(filename) {
                // Title
                if title.is_none() {
                    if let Some(m) = captures.name("title") {
                        title = Some(m.as_str().trim().to_string());
                    }
                }
                // Artists
                if artist.is_none() {
                    if let Some(m) = captures.name("artists") {
                        // artist = Some(Helpers::parse_artist(vec![m.as_str().trim()]));
                    }
                }
            }
        }

        // Platform IDs
        /*
        let ids = ExternalIDs::load(&tag);
        if (title.is_none() || artist.is_none()) && ids.is_empty() {
            return Err(OTError::new("Missing track metadata (title/artist or id)").into());
        }
        */

        // Track number
        let track_number = tags
            .get_field(Field::TrackNumber)
            .unwrap_or(vec![String::new()])[0]
            .parse()
            .ok();
        Ok(Track {
            format: Some(tag_wrap.format()),
            title,
            artist: Some(artist.flatten().unwrap()),
            path: Some(path.to_owned()),
            isrc: tags
                .get_field(Field::ISRC)
                .unwrap_or(vec![])
                .first()
                .map(String::from),
            duration: None,
            track_number,
            ..Default::default()
        })
    }

    fn fill_tags(&mut self) {
        //Title
        if self.title.is_none() && self.name.is_some() {
            self.title = Some(helpers::Helpers::title(self.name.unwrap(), self.mix, None))
        }
        // Name
        if self.name.is_none() {
            self.name = Some(helpers::Helpers::name(&self.title.unwrap()))
        }
        //Mix
        if self.mix.is_none() {
            self.mix = Some(helpers::Helpers::mix(&self.title.unwrap()))
        }
        //Remixer
        if self.remixer.is_none() {
            if self.remixers.is_some() {
                self.remixer = Some(helpers::Helpers::join_artists(&self.remixers.unwrap()))
            } else {
                self.remixer = Some(helpers::Helpers::remixer(&self.title.unwrap()))
            }
        }
        if self.remixers.is_none() && self.remixer.is_some() {
            self.remixers = Some(helpers::Helpers::parse_artist(&self.remixer.unwrap()))
        }
        //Artists
        if self.artist.is_none() && self.artists.is_some() {
            self.artist = Some(helpers::Helpers::join_artists(&self.artists.unwrap()))
        }
        if self.artists.is_none() && self.artist.is_some() {
            self.artists = Some(helpers::Helpers::parse_artist(&self.artist.unwrap()))
        }
        //Album artists
        if self.album_artist.is_none() && self.album_artists.is_some() {
            self.album_artist = Some(helpers::Helpers::join_artists(&self.album_artists.unwrap()))
        }
        if self.album_artists.is_none() && self.album_artist.is_some() {
            self.album_artists = Some(helpers::Helpers::parse_artist(&self.album_artist.unwrap()))
        }
        //Genre
        if self.genre.is_none() && self.genres.is_some() {
            self.genre = Some(helpers::Helpers::join_genres(&self.genres.unwrap()))
        }
        if self.genres.is_none() && self.genre.is_some() {
            self.genres = Some(helpers::Helpers::parse_genre(&self.genre.unwrap()))
        }
        //Dates
        /*
        track.release_date = release_date;
        track.release_year = release_year;
        track.publish_date = publish_date;
        track.publish_year = publish_year;
        */
    }

    pub fn write_tags(&self, api: &Track, config: &TaggerConfig) -> Result<(), Box<dyn Error>> {
        let mut tag_wrap = Tag::load_file(&self.path.unwrap(), true)?;
        let format = tag_wrap.format();
        //Settings
        tag_wrap.set_separators(&config.separators);
        if let Tag::ID3(t) = &mut tag_wrap {
            t.set_id3v24(config.id3v24);
        }
        if let Tag::MP4(mp4) = &mut tag_wrap {
            if (config.overwrite || mp4.get_art().is_empty())
                && config.album_art
                && api.artwork_url.is_some()
            {
                mp4.remove_all_artworks();
            }
        }

        //Tags
        let tags = tag_wrap.tag_mut();
        if config.title {
            match config.short_title {
                true => tags.set_field(Field::Title, vec![api.name.unwrap()], config.overwrite),
                false => tags.set_field(Field::Title, vec![api.title.unwrap()], config.overwrite),
            }
        }

        /*
        if config.mix && api.mix.is_some() {
            tags.set_field(
                Field::Mix,
                vec![api.mix.unwrap().as_ref().unwrap().to_string()],
                config.overwrite,
            );
        }
        if config.version && api.version.is_some() {
            tags.set_field(
                Field::Version,
                vec![api.version.unwrap().as_ref().unwrap().to_string()],
                config.overwrite,
            );
        }
        */

        if config.artist {
            tags.set_field(Field::Artist, api.artists.unwrap(), config.overwrite);
        }
        /*
        if config.remixer {
            tags.set_field(Field::Remixer, api.remixers.unwrap(), config.overwrite);
        }
        */
        if config.album_artist && !api.album_artists.unwrap().is_empty() {
            tags.set_field(
                Field::AlbumArtist,
                api.album_artists.unwrap(),
                config.overwrite,
            );
        }
        if config.album && api.album.is_some() {
            tags.set_field(
                Field::Album,
                vec![api.album.as_ref().unwrap().to_string()],
                config.overwrite,
            );
        }
        if config.album_art
            && api.artwork_url.is_some()
            && (config.overwrite || tags.get_art().is_empty())
        {
            info!("Downloading art: {:?}", api.artwork_url);
            match self.download_art(self.artwork_url.as_ref().unwrap()) {
                Ok(data) => {
                    match data {
                        Some(data) => {
                            tags.set_art(
                                CoverType::CoverFront,
                                "image/jpeg",
                                Some("Cover"),
                                data.clone(),
                            );
                            // Save to file
                            if config.album_art_file {
                                let path = Path::new(&self.path.unwrap())
                                    .parent()
                                    .unwrap()
                                    .join("cover.jpg");
                                if !path.exists() {
                                    if let Ok(mut file) = File::create(path) {
                                        file.write_all(&data).ok();
                                    }
                                }
                            }
                        }
                        None => warn!("Invalid album art!"),
                    }
                }
                Err(e) => warn!("Error downloading album art! {}", e),
            }
        }
        if config.key && api.key.is_some() {
            let mut value = api.key.as_ref().unwrap().to_string();
            // Convert to camelot
            if config.camelot {
                if let Some((_, c)) = CAMELOT_NOTES.iter().find(|(o, _)| o == &value) {
                    value = c.to_string();
                }
            }
            tags.set_field(Field::Key, vec![value], config.overwrite);
        }
        if config.bpm && api.bpm.is_some() {
            tags.set_field(
                Field::BPM,
                vec![api.bpm.unwrap().to_string()],
                config.overwrite,
            );
        }
        if config.label && api.label.is_some() {
            tags.set_field(
                Field::Label,
                vec![api.label.as_ref().unwrap().to_string()],
                config.overwrite,
            );
        }
        if config.genre && !api.genres.unwrap().is_empty() {
            if config.merge_genres {
                // Merge with existing ones
                let mut current: Vec<String> = tags
                    .get_field(Field::Genre)
                    .unwrap_or(vec![])
                    .iter()
                    .map(|g| g.to_lowercase())
                    .collect();
                let mut genres = api
                    .genres
                    .unwrap_or(vec![])
                    .iter()
                    .filter(|g| !current.iter().any(|i| i == &g.to_lowercase().to_owned()))
                    .map(|g| g.to_owned())
                    .collect();
                current.append(&mut genres);
                tags.set_field(Field::Genre, current, config.overwrite);
            } else {
                tags.set_field(Field::Genre, self.genres.unwrap().clone(), config.overwrite);
            }
        }
        /*
        if config.style && !api.styles.unwrap().is_empty() {
            if config.discogs.styles == DiscogsStyles::CustomTag
                && config.discogs.styles_custom_tag.is_some()
            {
                // Custom style tag
                let ui_tag = config.discogs.styles_custom_tag.as_ref().unwrap();
                tags.set_raw(
                    &ui_tag.by_format(&format),
                    self.styles.clone(),
                    config.overwrite,
                );
            } else if config.merge_genres {
                // Merge with existing ones
                let mut current: Vec<String> = tag
                    .get_field(Field::Style)
                    .unwrap_or(vec![])
                    .iter()
                    .map(|s| s.to_lowercase())
                    .collect();
                let mut styles = self
                    .styles
                    .clone()
                    .into_iter()
                    .filter(|s| !current.iter().any(|i| i == &s.to_lowercase()))
                    .collect();
                current.append(&mut styles);
                tags.set_field(Field::Style, current, config.overwrite);
            } else {
                // Default write to style
                tags.set_field(Field::Style, self.styles.clone(), config.overwrite);
            }
        }
        */
        if config.release_date {
            if let Some(date) = api.release_date {
                tags.set_date(
                    &TagDate {
                        year: date.year() as i32,
                        month: Some(date.month() as u8),
                        day: Some(date.day() as u8),
                    },
                    config.overwrite,
                );
            } else if let Some(year) = api.release_year {
                tags.set_date(
                    &TagDate {
                        year: year as i32,
                        month: None,
                        day: None,
                    },
                    config.overwrite,
                );
            }
        }
        if config.publish_date {
            if let Some(date) = api.publish_date {
                tags.set_publish_date(
                    &TagDate {
                        year: date.year() as i32,
                        month: Some(date.month() as u8),
                        day: Some(date.day() as u8),
                    },
                    config.overwrite,
                );
            } else if let Some(year) = api.publish_year {
                tags.set_publish_date(
                    &TagDate {
                        year: year as i32,
                        month: None,
                        day: None,
                    },
                    config.overwrite,
                );
            }
        }
        if config.catalog_number && api.catalog_number.is_some() {
            tags.set_field(
                Field::CatalogNumber,
                vec![api.catalog_number.as_ref().unwrap().to_string()],
                config.overwrite,
            );
        }
        if config.duration && api.duration.unwrap().as_secs() > 0 {
            tags.set_field(
                Field::Duration,
                vec![api.duration.unwrap().as_secs().to_string()],
                config.overwrite,
            );
        }
        if config.urls {
            if api.beatport.is_some() {
                tags.set_raw(
                    "WBEATPORT_TRACK",
                    vec![api.beatport.unwrap().track_url],
                    config.overwrite,
                );
                tags.set_raw(
                    "WBEATPORT_RELEASE",
                    vec![api.beatport.unwrap().track_url],
                    config.overwrite,
                );
            }
            if api.discogs.is_some() {
                tags.set_raw(
                    "WDISCOGS_TRACK",
                    vec![api.discogs.unwrap().track_url],
                    config.overwrite,
                );
                tags.set_raw(
                    "WDISCOGS_RELEASE",
                    vec![api.discogs.unwrap().track_url],
                    config.overwrite,
                );
            }
            if api.itunes.is_some() {
                tags.set_raw(
                    "WITUNES_TRACK_PREVIEW",
                    vec![api.itunes.unwrap().preview_url],
                    config.overwrite,
                );
                tags.set_raw(
                    "WITUNES_TRACK",
                    vec![api.itunes.unwrap().track_url],
                    config.overwrite,
                );
                tags.set_raw(
                    "WITUNES_ARTIST",
                    vec![api.itunes.unwrap().artist_url],
                    config.overwrite,
                );
                tags.set_raw(
                    "WITUNES_RELEASE",
                    vec![api.itunes.unwrap().release_url],
                    config.overwrite,
                );
            }
            if api.junodownload.is_some() {
                tags.set_raw(
                    "WJUNO_TRACK",
                    vec![api.junodownload.unwrap().track_url],
                    config.overwrite,
                );
            }
            if api.spotify.is_some() {
                tags.set_raw(
                    "WSPOTIFY_TRACK",
                    vec![api.spotify.unwrap().track_url],
                    config.overwrite,
                );
            }
            if api.traxsource.is_some() {
                tags.set_raw(
                    "WTRAX_TRACK",
                    vec![api.traxsource.unwrap().track_url],
                    config.overwrite,
                );
            }
        }
        // IDs
        if config.ids {
            if api.beatport.is_some() {
                tags.set_raw(
                    "TBEATPORT_TRACK_ID",
                    vec![api.beatport.unwrap().track_id.to_string()],
                    config.overwrite,
                );
                tags.set_raw(
                    "TBEATPORT_RELEASE_ID",
                    vec![api.beatport.unwrap().release_id.to_string()],
                    config.overwrite,
                );
            }
            if api.discogs.is_some() {
                tags.set_raw(
                    "TDISCOGS_TRACK_ID",
                    vec![api.discogs.unwrap().track_id.to_string()],
                    config.overwrite,
                );
                tags.set_raw(
                    "TDISCOGS_RELEASE_ID",
                    vec![api.discogs.unwrap().release_id.to_string()],
                    config.overwrite,
                );
            }
            if api.itunes.is_some() {
                tags.set_raw(
                    "TITUNES_TRACK_ID",
                    vec![api.traxsource.unwrap().track_id.to_string()],
                    config.overwrite,
                );
                tags.set_raw(
                    "TITUNES_ARTIST_ID",
                    vec![api.itunes.unwrap().artist_id.to_string()],
                    config.overwrite,
                );
                tags.set_raw(
                    "TITUNES_RELEASE_ID",
                    vec![api.itunes.unwrap().release_id.to_string()],
                    config.overwrite,
                );
            }
            if api.junodownload.is_some() {
                tags.set_raw(
                    "TJUNO_TRACK_ID",
                    vec![api.junodownload.unwrap().track_id.to_string()],
                    config.overwrite,
                );
                tags.set_raw(
                    "TJUNO_RELEASE_ID",
                    vec![api.junodownload.unwrap().release_id.to_string()],
                    config.overwrite,
                );
            }

            if api.spotify.is_some() {
                tags.set_raw(
                    "TSPOTIFY_TRACK_ID",
                    vec![api.spotify.unwrap().track_id.to_string()],
                    config.overwrite,
                );
                tags.set_raw(
                    "TSPOTIFY_RELEASE_ID",
                    vec![api.spotify.unwrap().release_id.to_string()],
                    config.overwrite,
                );
            }
            if api.traxsource.is_some() {
                tags.set_raw(
                    "TTRAX_TRACK_ID",
                    vec![api.traxsource.unwrap().track_id.to_string()],
                    config.overwrite,
                );
                tags.set_raw(
                    "TTRAX_RELEASE_ID",
                    vec![api.traxsource.unwrap().release_id.to_string()],
                    config.overwrite,
                );
            }
        }

        /*
        // Other tags
        if config.other_tags {
            for (t, value) in &api.other {
                tags.set_raw(t.as_str(), vec![value.to_string()], config.overwrite);
            }
        }
        */

        // Save
        tags.save_file(&self.path.unwrap())?;
        Ok(())
    }

    fn download_art(&self, url: &str) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        let response = reqwest::blocking::get(url)?;
        if response.status() != StatusCode::OK {
            return Ok(None);
        }
        // Too small, most likely a text response
        if let Some(cl) = response.content_length() {
            if cl < 2048 {
                return Ok(None);
            }
        }
        // Content-type needs image
        let headers = response.headers();
        if let Some(ct) = headers.get("content-type") {
            if !ct.to_str()?.contains("image") {
                return Ok(None);
            }
        }
        Ok(Some(response.bytes()?.to_vec()))
    }

    // Convert template into a regex
    pub fn parse_template(template: &str) -> Option<Regex> {
        // Regex reserved
        let reserved = ".?+*$^()[]/|";
        let mut template = template.to_string();
        for c in reserved.chars() {
            template = template.replace(c, &format!("\\{}", c));
        }
        // Replace variables
        template = template
            .replace("%title%", "(?P<title>.+)")
            .replace("%artist%", "(?P<artists>.+)")
            .replace("%artists%", "(?P<artists>.+)");
        // Remove all remaining variables
        let re = Regex::new("%[a-zA-Z0-9 ]+%").unwrap();
        template = re.replace(&template, "(.+)").to_string();
        // Extension
        template = format!("{}\\.[a-zA-Z0-9]{{2,4}}$", template);
        // Final regex
        Regex::new(&template).ok()
    }
}

/// IDs from various platforms
#[derive(Debug, Clone, Default)]
pub struct AudioFileIDs {
    pub discogs_release_id: Option<i64>,
    pub beatport_track_id: Option<i64>,
}

impl AudioFileIDs {
    // Load IDs from file
    pub fn load(tag: &Box<&dyn TagImpl>) -> AudioFileIDs {
        AudioFileIDs {
            discogs_release_id: tag
                .get_raw("DISCOGS_RELEASE_ID")
                .map(|v| v[0].parse().ok())
                .flatten(),
            beatport_track_id: tag
                .get_raw("BEATPORT_TRACK_ID")
                .map(|v| v[0].parse().ok())
                .flatten(),
        }
    }

    // If all values are missing
    pub fn is_empty(&self) -> bool {
        self.discogs_release_id.is_none() && self.beatport_track_id.is_none()
    }
}

// Parse duration from String
pub fn parse_duration(input: &str) -> Result<Duration, Box<dyn Error>> {
    let clean = input.replace("(", "").replace(")", "");
    let mut parts = clean.trim().split(":").collect::<Vec<&str>>();
    parts.reverse();
    let mut seconds: u64 = parts.first().ok_or("Invalid timestamp!")?.parse()?;
    if parts.len() > 1 {
        seconds += parts[1].parse::<u64>()? * 60;
    }
    if parts.len() > 2 {
        seconds += parts[2].parse::<u64>()? * 3600;
    }
    Ok(Duration::from_secs(seconds))
}

// For all the platforms
pub trait TrackMatcher {
    // Returns (accuracy, track)
    fn match_track(
        &self,
        local: &Track,
        config: &TaggerConfig,
    ) -> Result<Option<(f64, Track)>, Box<dyn Error>>;
}

// Single threaded, mutable
pub trait TrackMatcherST {
    // Returns (accuracy, track)
    fn match_track(
        &mut self,
        local: &Track,
        config: &TaggerConfig,
    ) -> Result<Option<(f64, Track)>, Box<dyn Error>>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TaggingState {
    Ok,
    Error,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggingStatus {
    pub status: TaggingState,
    pub path: String,
    pub message: Option<String>,
    pub accuracy: Option<f64>,
}

// Wrap for sending into UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggingStatusWrap {
    pub status: TaggingStatus,
    pub platform: MusicPlatform,
    pub progress: f64,
}
impl TaggingStatusWrap {
    // pi = platform index, pl = platforms length, p = processed, total = total tracks in this platform
    pub fn wrap(
        platform: MusicPlatform,
        status: &TaggingStatus,
        pi: usize,
        pl: usize,
        p: i64,
        total: usize,
    ) -> TaggingStatusWrap {
        TaggingStatusWrap {
            platform,
            status: status.to_owned(),
            progress: (pi as f64 / pl as f64) + ((p as f64 / total as f64) / pl as f64),
        }
    }
}

pub struct Tagger {}
impl Tagger {
    // Returtns progress receiver, and file count
    pub fn tag_files(cfg: &TaggerConfig, mut files: Vec<String>) -> Receiver<TaggingStatusWrap> {
        let original_files = files.clone();
        let total_files = files.len();
        info!("Starting tagger with: {} files!", total_files);

        // Create thread
        let (tx, rx) = channel();
        let config = cfg.clone();
        thread::spawn(move || {
            // Tag
            for (platform_index, platform) in config.platforms.iter().enumerate() {
                // For progress
                let mut processed = 0;
                let total = files.len();
                // No more files
                if files.is_empty() {
                    info!("All tagged succesfully!");
                    break;
                }
                match platform {
                    // Discogs
                    MusicPlatform::Discogs => {
                        // Auth discogs
                        let mut discogs = discogs::Discogs::new();
                        if config.discogs.token.as_ref().is_none() {
                            error!("Missing Discogs token! Skipping Discogs...");
                            continue;
                        }
                        discogs.set_auth_token(config.discogs.token.as_ref().unwrap());
                        if !discogs.validate_token() {
                            error!("Invalid Discogs token! Skipping Discogs...");
                            continue;
                        }
                        // Remove rate limit for small batches
                        if files.len() <= 35 {
                            discogs.set_rate_limit(150);
                        }
                        if files.len() <= 20 {
                            discogs.set_rate_limit(1000);
                        }
                        // Tag
                        let rx = Tagger::tag_dir_single_thread(&files, discogs, &config);
                        info!("Starting Discogs");
                        for status in rx {
                            info!(
                                "[{:?}] State: {:?}, Accuracy: {:?}, Path: {}",
                                MusicPlatform::Discogs,
                                status.status,
                                status.accuracy,
                                status.path
                            );
                            processed += 1;
                            // Send to UI
                            tx.send(TaggingStatusWrap::wrap(
                                MusicPlatform::Discogs,
                                &status,
                                platform_index,
                                config.platforms.len(),
                                processed,
                                total,
                            ))
                            .ok();
                            // Fallback
                            if status.status == TaggingState::Ok {
                                files.remove(files.iter().position(|f| f == &status.path).unwrap());
                            }
                        }
                    }
                    // iTunes
                    MusicPlatform::ITunes => {
                        let itunes = itunes::ITunes::new();
                        let rx = Tagger::tag_dir_single_thread(&files, itunes, &config);
                        info!("Starting iTunes");
                        for status in rx {
                            info!(
                                "[{:?}] State: {:?}, Accuracy: {:?}, Path: {}",
                                MusicPlatform::ITunes,
                                status.status,
                                status.accuracy,
                                status.path
                            );
                            processed += 1;
                            // Send to UI
                            tx.send(TaggingStatusWrap::wrap(
                                MusicPlatform::ITunes,
                                &status,
                                platform_index,
                                config.platforms.len(),
                                processed,
                                total,
                            ))
                            .ok();
                            // Fallback
                            if status.status == TaggingState::Ok {
                                files.remove(files.iter().position(|f| f == &status.path).unwrap());
                            }
                        }
                    }
                    platform => {
                        // No config platforms
                        let tagger: Box<dyn TrackMatcher + Send + Sync + 'static> = match platform {
                            MusicPlatform::Beatport => Box::new(beatport::Beatport::new()),
                            MusicPlatform::Traxsource => Box::new(traxsource::Traxsource::new()),
                            MusicPlatform::JunoDownload => {
                                Box::new(junodownload::JunoDownload::new())
                            }
                            MusicPlatform::MusicBrainz => Box::new(musicbrainz::MusicBrainz::new()),
                            _ => unreachable!(),
                        };
                        info!("Starting {:?}", platform);

                        let rx = if platform == &MusicPlatform::JunoDownload
                            || platform == &MusicPlatform::MusicBrainz
                        {
                            // Cap max threads due to rate limiting
                            let mut config = config.clone();
                            if config.threads > 4 {
                                config.threads = 4;
                            }
                            Tagger::tag_dir_multi_thread(&files, tagger, &config)
                        } else {
                            Tagger::tag_dir_multi_thread(&files, tagger, &config)
                        };
                        // Get statuses
                        for status in rx {
                            info!(
                                "[{:?}] State: {:?}, Accuracy: {:?}, Path: {}",
                                platform, status.status, status.accuracy, status.path
                            );
                            processed += 1;
                            // Send to UI
                            tx.send(TaggingStatusWrap::wrap(
                                platform.to_owned(),
                                &status,
                                platform_index,
                                (&config.platforms).len(),
                                processed,
                                total,
                            ))
                            .ok();
                            // Fallback
                            if status.status == TaggingState::Ok {
                                files.remove(files.iter().position(|f| f == &status.path).unwrap());
                            }
                        }
                    }
                }
            }

            // Tagging ended, save lists of files
            let write_result = || -> Result<(String, String), Box<dyn Error>> {
                let time = timestamp!();
                let folder = PathBuf::from(Settings::get_folder()?.to_str().unwrap().to_string())
                    .join("runs");
                if !folder.exists() {
                    fs::create_dir_all(&folder)?;
                }
                let failed_file = folder.join(format!("failed-{}.m3u", time));
                let success_file = folder.join(format!("success-{}.m3u", time));
                {
                    let mut file = File::create(&failed_file)?;
                    file.write_all(files.join("\r\n").as_bytes())?;
                }
                {
                    let mut file = File::create(&success_file)?;
                    let files: Vec<String> = original_files
                        .into_iter()
                        .filter(|i| !files.contains(i))
                        .collect();
                    file.write_all(files.join("\r\n").as_bytes())?;
                }
                // Run command
                let (failed_file, success_file) = (
                    failed_file.to_str().unwrap().to_string(),
                    success_file.to_str().unwrap().to_string(),
                );
                if let Some(command) = &config.post_command {
                    if !command.trim().is_empty() {
                        let command = command
                            .replace("$failed", &failed_file)
                            .replace("$success", &success_file);
                        thread::spawn(|| {
                            info!("Executing command: {}", command);
                            let mut command = execute::shell(command);
                            let result = command.execute().ok().flatten();
                            info!("Command finished with: {:?}", result);
                        });
                    }
                }

                Ok((failed_file, success_file))
            };
            match write_result() {
                Ok((failed, success)) => info!(
                    "Written failed songs to: {}, successful to: {}",
                    failed, success
                ),
                Err(e) => warn!("Failed writing failed songs to file! {}", e),
            };
        });
        rx
    }

    // Tag single track
    pub fn tag_track(
        path: &str,
        tagger_mt: Option<&dyn TrackMatcher>,
        tagger_st: Option<&mut dyn TrackMatcherST>,
        config: &TaggerConfig,
    ) -> TaggingStatus {
        // Output
        let mut out = TaggingStatus {
            status: TaggingState::Error,
            path: path.to_owned(),
            accuracy: None,
            message: None,
        };

        // Filename template
        let mut template = None;
        if config.parse_filename {
            if let Some(t) = &config.filename_template {
                template = Track::parse_template(&t);
            }
        }

        match Track::read_tags(path, template) {
            Ok(mut local) => {
                // Match track
                let result = if let Some(tagger) = tagger_mt {
                    tagger.match_track(&local, &config)
                } else if let Some(tagger) = tagger_st {
                    tagger.match_track(&local, &config)
                } else {
                    out.message = Some("No tagger!".to_owned());
                    return out;
                };
                match result {
                    Ok(o) => {
                        match o {
                            Some((acc, track)) => {
                                // Save to file
                                match &local.write_tags(&track, &config) {
                                    Ok(_) => {
                                        out.accuracy = Some(acc);
                                        out.status = TaggingState::Ok;
                                    }
                                    Err(e) => {
                                        out.message =
                                            Some(format!("Failed writing tags to file: {}", e))
                                    }
                                }
                            }
                            None => out.message = Some("No match!".to_owned()),
                        }
                    }
                    // Failed matching track
                    Err(e) => {
                        error!("Matching error: {} ({})", e, path);
                        out.message = Some(format!("Error marching track: {}", e));
                    }
                }
            }
            // Failed loading file
            Err(e) => {
                out.status = TaggingState::Skipped;
                warn!("Error loading file: {}", e);
                out.message = Some(format!("Error loading file: {}", e));
            }
        }
        out
    }

    // Get list of all files in with supported extensions
    pub fn get_file_list(path: &str) -> Vec<String> {
        if path.is_empty() {
            return vec![];
        }
        let files: Vec<String> = WalkDir::new(path)
            .into_iter()
            .filter(|e| {
                e.is_ok()
                    && EXTENSIONS.iter().any(|&i| {
                        e.as_ref()
                            .unwrap()
                            .path()
                            .to_str()
                            .unwrap()
                            .to_lowercase()
                            .ends_with(i)
                    })
            })
            .map(|e| e.unwrap().path().to_str().unwrap().to_owned())
            .collect();
        files
    }

    // Tag all files with threads specified in config
    pub fn tag_dir_multi_thread(
        files: &Vec<String>,
        tagger: Box<(dyn TrackMatcher + Send + Sync + 'static)>,
        config: &TaggerConfig,
    ) -> Receiver<TaggingStatus> {
        info!(
            "Starting tagging: {} files, {} threads!",
            files.len(),
            config.threads
        );
        // Create threadpool
        let pool = ThreadPool::new(config.threads as usize);
        let (tx, rx) = channel();
        let tagger_arc = Arc::new(tagger);
        for file in files {
            let tx = tx.clone();
            let config = config.clone();
            let t = tagger_arc.clone();
            let f = file.to_owned();
            pool.execute(move || {
                let res: TaggingStatus = Tagger::tag_track(&f, Some(&**t), None, &config);
                tx.send(res).ok();
            });
        }
        rx
    }

    // Tag all files with single thread
    pub fn tag_dir_single_thread(
        files: &Vec<String>,
        mut tagger: (impl TrackMatcherST + Send + 'static),
        config: &TaggerConfig,
    ) -> Receiver<TaggingStatus> {
        info!("Starting single threaded tagging of {} files!", files.len());
        // Spawn thread
        let (tx, rx) = channel();
        let c = config.clone();
        let f = files.clone();
        thread::spawn(move || {
            for file in f {
                tx.send(Tagger::tag_track(&file, None, Some(&mut tagger), &c))
                    .ok();
            }
        });
        rx
    }
}
