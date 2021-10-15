use std::cmp::Ordering;
use std::error::Error;
use std::thread;
use std::fs;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::time::Duration;
use std::default::Default;
use std::io::prelude::*;
use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver};
use chrono::Local;
use execute::Execute;
use regex::Regex;
use reqwest::StatusCode;
use walkdir::WalkDir;
use threadpool::ThreadPool;
use strsim::normalized_levenshtein;
use chrono::{NaiveDate, Datelike};
use serde::{Serialize, Deserialize};
use crate::tag::{AudioFileFormat, Tag, Field, TagDate, CoverType, TagImpl, UITag, TagSeparators, EXTENSIONS};
use crate::ui::{OTError, Settings};
use crate::ui::player::AudioSources;

pub mod beatport;
pub mod traxsource;
pub mod discogs;
pub mod junodownload;
pub mod spotify;
pub mod itunes;
pub mod musicbrainz;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MusicPlatform {
    Beatport,
    Traxsource,
    Discogs,
    JunoDownload,
    ITunes,
    MusicBrainz,

    // Currently only used in Audio Features
    Spotify,
    // For default
    None
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
    pub url: bool,
    pub track_id: bool,
    pub release_id: bool,
    pub version: bool,
    pub duration: bool,
    pub album_artist: bool,
    pub remixer: bool,
    pub track_number: bool,
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
    pub track_number_leading_zeroes: usize,

    // Platform specific
    pub beatport: BeatportConfig,
    pub discogs: DiscogsConfig
}

// Beatport specific settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BeatportConfig {
    pub art_resolution: i64,
    pub max_pages: i64
}

// Discogs specific settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiscogsConfig {
    pub token: Option<String>,
    pub max_results: i16,
    pub track_number_int: bool
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
    CustomTag
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
    Newest
}

impl Default for MultipleMatchesSort {
    fn default() -> Self {
        Self::Default
    }
}

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
    pub remixers: Vec<String>,
    pub track_number: Option<TrackNumber>,
    
    // Only year OR date should be available
    pub release_year: Option<i64>,
    pub release_date: Option<NaiveDate>,
    pub publish_year: Option<i64>,
    pub publish_date: Option<NaiveDate>
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrackNumber {
    Number(i32),
    /// Custom format (Discogs)
    Custom(String)
}

impl From<i32> for TrackNumber {
    fn from(i: i32) -> Self {
        TrackNumber::Number(i)
    }
}

const CAMELOT_NOTES: [(&str, &str); 35] = [
    ("Abm", "1A"),
    ("G#m", "1A"),
    ("B",   "1B"),
    ("D#m", "2A"),
    ("Ebm", "2A"),
    ("Gb",  "2B"),
    ("F#",  "2B"),
    ("A#m", "3A"),
    ("Bbm", "3A"),
    ("C#",  "3B"),
    ("Db",  "3B"),
    ("Dd",  "3B"),
    ("Fm",  "4A"),
    ("G#",  "4B"),
    ("Ab",  "4B"),
    ("Cm",  "5A"),
    ("D#",  "5B"),
    ("Eb",  "5B"),
    ("Gm",  "6A"),
    ("A#",  "6B"),
    ("Bb",  "6B"),
    ("Dm",  "7A"),
    ("F",   "7B"),
    ("Am",  "8A"),
    ("C",   "8B"),
    ("Em",  "9A"),
    ("G",   "9B"),
    ("Bm",  "10A"),
    ("D",   "10B"),
    ("Gbm", "11A"),
    ("F#m", "11A"),
    ("A",   "11B"),
    ("C#m", "12A"),
    ("Dbm", "12A"),
    ("E",   "12B"),
];

impl Track {
    // Write tags to file
    pub fn write_to_file(&self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<(), Box<dyn Error>> {        
        // Get tag
        let mut tag_wrap = Tag::load_file(&info.path, true)?;
        tag_wrap.set_separators(&config.separators);
        let format = tag_wrap.format();
        // Configure format specific
        if let Tag::ID3(t) = &mut tag_wrap {
            t.set_id3v24(config.id3v24);
        }
        // MP4 Album art override
        if let Tag::MP4(mp4) = &mut tag_wrap {
            // Has art
            if (config.overwrite || mp4.get_art().is_empty()) && self.art.is_some() && config.album_art {
                mp4.remove_all_artworks();
            }
        }

        let tag = tag_wrap.tag_mut();
        // Set tags
        if config.title {
            match config.short_title {
                true => tag.set_field(Field::Title, vec![self.title.to_string()], config.overwrite),
                false => tag.set_field(Field::Title, vec![self.full_title()], config.overwrite)
            }
        }
        // Version
        if config.version && self.version.is_some() {
            tag.set_field(Field::Version, vec![self.version.as_ref().unwrap().to_string()], config.overwrite);
        }
        if config.artist {
            tag.set_field(Field::Artist, self.artists.clone(), config.overwrite);
        }
        if config.album_artist && !self.album_artists.is_empty() {
            tag.set_field(Field::AlbumArtist, self.album_artists.clone(), config.overwrite);
        }
        if self.album.is_some() && config.album  {
            tag.set_field(Field::Album, vec![self.album.as_ref().unwrap().to_string()], config.overwrite);
        }
        if config.key && self.key.is_some() {
            let mut value = self.key.as_ref().unwrap().to_string();
            // Convert to camelot
            if config.camelot {
                if let Some((_, c)) = CAMELOT_NOTES.iter().find(|(o, _)| o == &value) {
                    value = c.to_string();
                }
            }
            tag.set_field(Field::Key, vec![value], config.overwrite);
        }
        if config.bpm && self.bpm.is_some() {
            tag.set_field(Field::BPM, vec![self.bpm.unwrap().to_string()], config.overwrite);
        }
        if config.label && self.label.is_some() {
            tag.set_field(Field::Label, vec![self.label.as_ref().unwrap().to_string()], config.overwrite);
        }
        if config.genre && !self.genres.is_empty() {
            if config.merge_genres {
                // Merge with existing ones
                let mut current: Vec<String> = tag.get_field(Field::Genre).unwrap_or(vec![]).iter().map(|g| g.to_lowercase()).collect();
                let mut genres = self.genres.clone().into_iter().filter(|g| !current.iter().any(|i| i == &g.to_lowercase())).collect();
                current.append(&mut genres);
                tag.set_field(Field::Genre, current, config.overwrite); 
            } else {
                tag.set_field(Field::Genre, self.genres.clone(), config.overwrite);
            }
        }
        if config.style && !self.styles.is_empty() {
            if config.styles_options == StylesOptions::CustomTag && config.styles_custom_tag.is_some() {
                // Custom style tag
                let ui_tag = config.styles_custom_tag.as_ref().unwrap();
                tag.set_raw(&ui_tag.by_format(&format), self.styles.clone(), config.overwrite);

            } else if config.merge_genres {
                // Merge with existing ones
                let mut current: Vec<String> = tag.get_field(Field::Style).unwrap_or(vec![]).iter().map(|s| s.to_lowercase()).collect();
                let mut styles = self.styles.clone().into_iter().filter(|s| !current.iter().any(|i| i == &s.to_lowercase())).collect();
                current.append(&mut styles);
                tag.set_field(Field::Style, current, config.overwrite); 

            } else {
                // Default write to style
                tag.set_field(Field::Style, self.styles.clone(), config.overwrite);
            }
        }
        // Release dates
        if config.release_date {
            if let Some(date) = self.release_date {
                tag.set_date(&TagDate {
                    year: date.year() as i32,
                    month: Some(date.month() as u8),
                    day: Some(date.day() as u8)
                }, config.overwrite);
            } else if let Some(year) = self.release_year {
                tag.set_date(&TagDate {
                    year: year as i32,
                    month: None,
                    day: None
                }, config.overwrite);
            }
        }
        // Publish date
        if config.publish_date {
            if let Some(date) = self.publish_date {
                tag.set_publish_date(&TagDate {
                    year: date.year() as i32,
                    month: Some(date.month() as u8),
                    day: Some(date.day() as u8)
                }, config.overwrite);
            } else if let Some(year) = self.publish_year {
                tag.set_publish_date(&TagDate {
                    year: year as i32,
                    month: None,
                    day: None
                }, config.overwrite);
            }
        }
        // URL
        if config.url {
            tag.set_raw("WWWAUDIOFILE", vec![self.url.to_string()], config.overwrite);
        }
        // Other tags
        if config.other_tags {
            for (t, value) in &self.other {
                tag.set_raw(t.as_str(), vec![value.to_string()], config.overwrite);
            }
        }
        // IDs
        if config.track_id && self.track_id.is_some() {
            let t = format!("{}_TRACK_ID", serde_json::to_value(self.platform.clone()).unwrap().as_str().unwrap().to_uppercase());
            tag.set_raw(&t, vec![self.track_id.as_ref().unwrap().to_string()], config.overwrite);
        }
        if config.release_id {
            let t = format!("{}_RELEASE_ID", serde_json::to_value(self.platform.clone()).unwrap().as_str().unwrap().to_uppercase());
            tag.set_raw(&t, vec![self.release_id.to_string()], config.overwrite);
        }
        // Catalog number
        if config.catalog_number && self.catalog_number.is_some() {
            tag.set_field(Field::CatalogNumber, vec![self.catalog_number.as_ref().unwrap().to_string()], config.overwrite);
        }
        // Duration
        if config.duration && self.duration.as_secs() > 0 {
            tag.set_field(Field::Duration, vec![self.duration.as_secs().to_string()], config.overwrite);
        }
        // Remixers
        if config.remixer && !self.remixers.is_empty() {
            tag.set_field(Field::Remixer, self.remixers.clone(), config.overwrite);
        }
        // Track number
        if config.track_number && self.track_number.is_some() {
            match self.track_number.as_ref().unwrap() {
                TrackNumber::Number(n) => tag.set_field(Field::TrackNumber, vec![format!("{:0width$}", n, width = config.track_number_leading_zeroes)], config.overwrite),
                TrackNumber::Custom(n) => tag.set_field(Field::TrackNumber, vec![n.to_string()], config.overwrite),
            }
        }
        // Album art
        if (config.overwrite || tag.get_art().is_empty()) && self.art.is_some() && config.album_art {
            info!("Downloading art: {:?}", self.art);
            match self.download_art(self.art.as_ref().unwrap()) {
                Ok(data) => {
                    match data {
                        Some(data) => {
                            tag.set_art(CoverType::CoverFront, "image/jpeg", Some("Cover"), data.clone());
                            // Save to file
                            if config.album_art_file {
                                let path = Path::new(&info.path).parent().unwrap().join("cover.jpg");
                                if !path.exists() {
                                    if let Ok(mut file) = File::create(path) {
                                        file.write_all(&data).ok();
                                    }
                                }
                            }
                        },
                        None => warn!("Invalid album art!")
                    } 
                },
                Err(e) => warn!("Error downloading album art! {}", e)
            }
        }

        // Meta tags (date / success)
        if config.meta_tags {
            let time = Local::now();
            tag.set_raw("1T_TAGGEDDATE", vec![time.format("%Y-%m-%d %H:%M:%S").to_string()], true);
        }

        // Save
        tag.save_file(&info.path)?;
        Ok(())
    }

    // Download album art, None if invalid album art
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

    // Get title with version
    pub fn full_title(&self) -> String {
        if let Some(v) = self.version.as_ref() {
            if v.trim().is_empty() {
                self.title.to_string()
            } else {
                format!("{} ({})", self.title, v.trim())
            }
        } else {
            self.title.to_string()
        }
    }
}

#[derive(Debug, Clone)]
pub struct AudioFileInfo {
    pub title: Option<String>,
    pub artists: Vec<String>,
    pub format: AudioFileFormat,
    pub path: String,
    pub isrc: Option<String>,
    pub duration: Option<Duration>,
    pub track_number: Option<u16>,
    pub ids: AudioFileIDs
}

impl AudioFileInfo {
    // Load audio file info from path
    pub fn load_file(path: &str, filename_template: Option<Regex>) -> Result<AudioFileInfo, Box<dyn Error>> {
        let tag_wrap = Tag::load_file(&path, true)?;
        let tag = tag_wrap.tag();
        // Get title artist from tag
        let mut title = tag.get_field(Field::Title).map(|t| t.first().map(|t| t.to_owned())).flatten();
        let mut artists = tag.get_field(Field::Artist)
            .map(|a| AudioFileInfo::parse_artist_tag(a.iter().map(|a| a.as_str()).collect()));

        // Parse filename
        if (title.is_none() || artists.is_none()) && filename_template.is_some() {
            let p = Path::new(path);
            let filename = p.file_name().ok_or("Missing filename!")?.to_str().ok_or("Missing filename")?;

            if let Some(captures) = filename_template.unwrap().captures(filename) {
                // Title
                if title.is_none() {
                    if let Some(m) = captures.name("title") {
                        title = Some(m.as_str().trim().to_string());
                    }
                }
                // Artists
                if artists.is_none() {
                    if let Some(m) = captures.name("artists") {
                        artists = Some(AudioFileInfo::parse_artist_tag(vec![m.as_str().trim()]));
                    }
                }
            }
        }

        // Platform IDs
        let ids = AudioFileIDs::load(&tag);
        if (title.is_none() || artists.is_none()) && ids.is_empty() {
            return Err(OTError::new("Missing track metadata (title/artist or id)").into());
        }

        // Track number
        let track_number = tag.get_field(Field::TrackNumber).unwrap_or(vec![String::new()])[0].parse().ok();
        Ok(AudioFileInfo {
            format: tag_wrap.format(),
            title,
            artists: artists.unwrap_or(vec![]),
            path: path.to_owned(),
            isrc: tag.get_field(Field::ISRC).unwrap_or(vec![]).first().map(String::from),
            duration: None,
            track_number,
            ids
        })
    }

    // Get title
    pub fn title(&self) -> Result<&str, Box<dyn Error>> {
        if self.title.is_none() {
            error!("Track is missing title tag, skipping. {}", self.path);
            return Err(OTError::new("Missing title tag!").into());
        }
        Ok(self.title.as_ref().unwrap().as_str())
    }

    // Get first artist
    pub fn artist(&self) -> Result<&str, Box<dyn Error>> {
        if self.artists.is_empty() {
            error!("Track is missing artist tag, skipping. {}", self.path);
            return Err(OTError::new("Missing artist tag!").into());
        }
        Ok(self.artists.first().unwrap().as_str())
    }

    // Load duration from file
    pub fn load_duration(&mut self) {
        // Mark as loaded
        self.duration = Some(Duration::ZERO);
        if let Ok(source) = AudioSources::from_path(&self.path) {
            self.duration = Some(Duration::from_millis(source.duration() as u64))
        } else {
            warn!("Failed loading duration from file! {}", self.path);
        }
    }

    // Convert template into a regex
    pub fn parse_template(template: &str) -> Option<Regex> {
        // Regex reserved
        let reserved = ".?+*$^()[]/|";
        let mut template = template.to_string();
        for c in reserved.chars() {
            template = template.replace(c, &format!("\\{}", c));
        };
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

    // Try to split artist string with common separators
    fn parse_artist_tag(input: Vec<&str>) -> Vec<String> {
        // Already an array
        if input.len() > 1 {
            return input.into_iter().map(|v| v.to_owned()).collect();
        }
        let src = input.first().unwrap();

        if src.contains(';') {
            return src.split(';').collect::<Vec<&str>>().into_iter().map(|v| v.to_owned()).collect();
        }
        if src.contains(',') {
            return src.split(',').collect::<Vec<&str>>().into_iter().map(|v| v.to_owned()).collect();
        }
        if src.contains('/') {
            return src.split('/').collect::<Vec<&str>>().into_iter().map(|v| v.to_owned()).collect();
        }
        vec![src.to_owned().to_owned()]
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
            discogs_release_id: tag.get_raw("DISCOGS_RELEASE_ID").map(|v| v[0].parse().ok()).flatten(),
            beatport_track_id: tag.get_raw("BEATPORT_TRACK_ID").map(|v| v[0].parse().ok()).flatten()
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
    fn match_track(&self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>>;
}

// Single threaded, mutable
pub trait TrackMatcherST {
    // Returns (accuracy, track)
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>>;
}

pub struct MatchingUtils {}
impl MatchingUtils {
    // Clean title for searching
    pub fn clean_title(input: &str) -> String {
        let step1 = input.to_lowercase()
            // Remove - because search engines
            .replace("-", " ")
            .replace("  ", " ");
        let step2 = step1.trim();
        // Remove original mix
        let mut re = Regex::new(r"((\(|\[)*)original( (mix|version|edit))*((\)|\])*)$").unwrap();
        let step3 = re.replace(&step2, "");
        // Remove initial a/an/the
        re = Regex::new(r"^((a|an|the) )").unwrap();
        let step4 = re.replace(&step3, "");
        // Remove attributes
        let step5 = step4
            .replace("(intro)", "")
            .replace("(clean)", "");
        // Remove - and trim
        let step6 = step5.replace("-", "").replace("  ", " ");
        // Remove feat.
        re = Regex::new(r"(\(|\[)?(feat|ft)\.?.+?(\)|\]|\(|$)").unwrap();
        let out = re.replace(&step6, "");
        out.trim().to_string()
    }

    // Remove spacial characters
    pub fn remove_special(input: &str) -> String {
        let special = ".,()[]&_\"'-/\\^";
        let mut out = input.to_string();
        for c in special.chars() {
            out = out.replace(c, "");
        }
        out = out.replace("  ", " ");
        out.trim().to_string()
    }

    // Clean list of artists
    pub fn clean_artists(input: &Vec<String>) -> Vec<String> {
        let mut clean: Vec<String> = input.into_iter().map(
            |a| MatchingUtils::remove_special(&a.to_lowercase()).trim().to_string()
        ).collect();
        clean.sort();
        clean
    }

    // Clean title for matching, removes special characters etc
    pub fn clean_title_matching(input: &str) -> String {
        let title = MatchingUtils::clean_title(input);
        // Remove edit, specials
        let step1 = title.replace("edit", "");
        let step2 = MatchingUtils::remove_special(&step1);
        step2.to_string()
    }

    // Match atleast 1 artist
    pub fn match_artist(a: &Vec<String>, b: &Vec<String>, strictness: f64) -> bool {
        // Exact match atleast 1 artist
        let clean_a = MatchingUtils::clean_artists(a);
        let clean_b = MatchingUtils::clean_artists(b);
        for artist in &clean_a {
            if clean_b.contains(&artist) {
                return true;
            }
        }

        // String exact match (for separator problems)
        let clean_a_joined = clean_a.join(" ");
        for artist in &clean_b {
            if clean_a_joined.contains(artist) {
                return true;
            }
        }
        let clean_b_joined = clean_b.join(" ");
        for artist in &clean_a {
            if clean_b_joined.contains(artist) {
                return true;
            }
        }

        // Fuzzy
        let acc = normalized_levenshtein(&clean_a.join(" "), &clean_b.join(", "));
        if acc >= strictness {
            return true;
        }

        false
    }

    // Default track matching
    pub fn match_track(info: &AudioFileInfo, tracks: &Vec<Track>, config: &TaggerConfig, match_artist: bool) -> Option<(f64, Track)> {
        let clean_title = MatchingUtils::clean_title_matching(info.title().ok()?);
        // Exact match
        let mut exact_matches = vec![];
        for track in tracks {
            if !MatchingUtils::match_duration(info, track, config) {
                continue;
            }
            if clean_title == MatchingUtils::clean_title_matching(&track.full_title()) {
                if match_artist {
                    if MatchingUtils::match_artist(&info.artists, &track.artists, config.strictness) {
                        exact_matches.push((1.0, track));
                    }
                } else {
                    exact_matches.push((1.0, track));
                }
            }
        }

        // Use exact match
        if !exact_matches.is_empty() {
            MatchingUtils::sort_tracks(&mut exact_matches, &config);
            return Some((1.0, exact_matches[0].1.to_owned()));
        }

        // Fuzzy match - value, track
        let mut fuzz: Vec<(f64, &Track)> = vec![];
        for track in tracks {
            // Artist
            if match_artist {
                if !MatchingUtils::match_artist(&info.artists, &track.artists, config.strictness) {
                    continue;
                }
            }
            // Match title
            let clean = MatchingUtils::clean_title_matching(&track.full_title());
            let l = normalized_levenshtein(&clean, &clean_title);
            if l >= config.strictness {
                fuzz.push((l, track));
            }
        }
        // Empty array
        if fuzz.is_empty() {
            return None;
        }
        // Sort
        fuzz.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        let best_acc = fuzz[0].0;
        let mut fuzz: Vec<(f64, &Track)> = fuzz.into_iter().filter(|(acc, _)| *acc >= best_acc).collect();
        MatchingUtils::sort_tracks(&mut fuzz, &config);
        Some((fuzz[0].0, fuzz[0].1.to_owned()))
    }

    /// Sort matched tracks by release dates
    fn sort_tracks(tracks: &mut Vec<(f64, &Track)>, config: &TaggerConfig) {
        match config.multiple_matches {
            MultipleMatchesSort::Default => {},
            MultipleMatchesSort::Oldest => tracks.sort_by(|a, b| {
                if a.1.release_date.is_none() || b.1.release_date.is_none() {
                    Ordering::Equal
                } else {
                    a.1.release_date.as_ref().unwrap().cmp(b.1.release_date.as_ref().unwrap())
                }
            }),
            MultipleMatchesSort::Newest => tracks.sort_by(|a, b| {
                if a.1.release_date.is_none() || b.1.release_date.is_none() {
                    Ordering::Equal
                } else {
                    b.1.release_date.as_ref().unwrap().cmp(a.1.release_date.as_ref().unwrap())
                }
            }),
        }
    }

    // Match duration
    pub fn match_duration(info: &AudioFileInfo, track: &Track, config: &TaggerConfig) -> bool {
        // Disabled
        if !config.match_duration || info.duration.is_none() {
            return true;
        }
        let duration = *info.duration.as_ref().unwrap();
        //  No duration available
        if duration == Duration::ZERO || track.duration == Duration::ZERO {
            return true;
        }
        let diff = (duration.as_secs() as i64 - track.duration.as_secs() as i64).abs() as u64;
        diff <= config.max_duration_difference
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TaggingState {
    Ok, Error, Skipped
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
    pub fn wrap(platform: MusicPlatform, status: &TaggingStatus, pi: usize, pl: usize, p: i64, total: usize) -> TaggingStatusWrap {
        TaggingStatusWrap {
            platform,
            status: status.to_owned(),
            progress: (pi as f64 / pl as f64) + ((p as f64 / total as f64) / pl as f64)
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
                            info!("[{:?}] State: {:?}, Accuracy: {:?}, Path: {}", MusicPlatform::Discogs, status.status, status.accuracy, status.path);
                            processed += 1;
                            // Send to UI
                            tx.send(TaggingStatusWrap::wrap(MusicPlatform::Discogs, &status, 
                                platform_index, config.platforms.len(), processed, total
                            )).ok();
                            // Fallback
                            if status.status == TaggingState::Ok {
                                files.remove(files.iter().position(|f| f == &status.path).unwrap());
                            }
                        }
                    },
                    // iTunes
                    MusicPlatform::ITunes => {
                        let itunes = itunes::ITunes::new();
                        let rx = Tagger::tag_dir_single_thread(&files, itunes, &config);
                        info!("Starting iTunes");
                        for status in rx {
                            info!("[{:?}] State: {:?}, Accuracy: {:?}, Path: {}", MusicPlatform::ITunes, status.status, status.accuracy, status.path);
                            processed += 1;
                            // Send to UI
                            tx.send(TaggingStatusWrap::wrap(MusicPlatform::ITunes, &status, 
                                platform_index, config.platforms.len(), processed, total
                            )).ok();
                            // Fallback
                            if status.status == TaggingState::Ok {
                                files.remove(files.iter().position(|f| f == &status.path).unwrap());
                            }
                        }
                    },
                    platform => {
                        // No config platforms
                        let tagger: Box<dyn TrackMatcher + Send + Sync + 'static> = match platform {
                            MusicPlatform::Beatport => Box::new(beatport::Beatport::new()),
                            MusicPlatform::Traxsource => Box::new(traxsource::Traxsource::new()),
                            MusicPlatform::JunoDownload => Box::new(junodownload::JunoDownload::new()),
                            MusicPlatform::MusicBrainz => Box::new(musicbrainz::MusicBrainz::new()),
                            _ => unreachable!()
                        };
                        info!("Starting {:?}", platform);
                        
                        let rx = if platform == &MusicPlatform::JunoDownload || platform == &MusicPlatform::MusicBrainz {
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
                            info!("[{:?}] State: {:?}, Accuracy: {:?}, Path: {}", platform, status.status, status.accuracy, status.path);
                            processed += 1;
                            // Send to UI
                            tx.send(TaggingStatusWrap::wrap(platform.to_owned(), &status, 
                                platform_index, (&config.platforms).len(), processed, total
                            )).ok();
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
                let folder = PathBuf::from(Settings::get_folder()?.to_str().unwrap().to_string()).join("runs");
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
                    let files: Vec<String> = original_files.into_iter().filter(|i| !files.contains(i)).collect();
                    file.write_all(files.join("\r\n").as_bytes())?;
                }
                
                // Run command
                let (failed_file, success_file) = (failed_file.to_str().unwrap().to_string(), success_file.to_str().unwrap().to_string());
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
                Ok((failed, success)) => info!("Written failed songs to: {}, successful to: {}", failed, success),
                Err(e) => warn!("Failed writing failed songs to file! {}", e)
            };
            

        });
        
        rx
    }

    // Tag single track
    pub fn tag_track(path: &str, tagger_mt: Option<&dyn TrackMatcher>, tagger_st: Option<&mut dyn TrackMatcherST>, config: &TaggerConfig) -> TaggingStatus {
        // Output
        let mut out = TaggingStatus {
            status: TaggingState::Error,
            path: path.to_owned(),
            accuracy: None,
            message: None
        };

        // Filename template
        let mut template = None;
        if config.parse_filename {
            if let Some(t) = &config.filename_template {
                template = AudioFileInfo::parse_template(&t);
            }
        }

        match AudioFileInfo::load_file(path, template) {
            Ok(mut info) => {
                //  Load duration for matching
                if config.match_duration {
                    info.load_duration();
                }
                // Match track
                let result = if let Some(tagger) = tagger_mt {
                    tagger.match_track(&info, &config)
                } else if let Some(tagger) = tagger_st {
                    tagger.match_track(&info, &config)
                } else {
                    out.message = Some("No tagger!".to_owned());
                    return out;
                };
                match result {
                    Ok(o) => {
                        match o {
                            Some((acc, track)) => {
                                // Save to file
                                match track.write_to_file(&info, &config) {
                                    Ok(_) => {
                                        out.accuracy = Some(acc);
                                        out.status = TaggingState::Ok;
                                    },
                                    Err(e) => out.message = Some(format!("Failed writing tags to file: {}", e))
                                }
                            },
                            None => out.message = Some("No match!".to_owned())
                        }
                    },
                    // Failed matching track
                    Err(e) => {
                        error!("Matching error: {} ({})", e, path);
                        out.message = Some(format!("Error marching track: {}", e));
                    }
                }
            },
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
        let files: Vec<String> = WalkDir::new(path).into_iter().filter(
            |e| e.is_ok() && 
            EXTENSIONS.iter().any(|&i| e.as_ref().unwrap().path().to_str().unwrap().to_lowercase().ends_with(i))
        ).map(|e| e.unwrap().path().to_str().unwrap().to_owned()).collect();
        files
    }

    // Tag all files with threads specified in config
    pub fn tag_dir_multi_thread(files: &Vec<String>, tagger: Box<(dyn TrackMatcher + Send + Sync + 'static)>, config: &TaggerConfig) -> Receiver<TaggingStatus> {
        info!("Starting tagging: {} files, {} threads!", files.len(), config.threads);
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
    pub fn tag_dir_single_thread(files: &Vec<String>, mut tagger: (impl TrackMatcherST + Send + 'static), config: &TaggerConfig) -> Receiver<TaggingStatus> {
        info!("Starting single threaded tagging of {} files!", files.len());
        // Spawn thread
        let (tx, rx) = channel();
        let c = config.clone();
        let f = files.clone();
        thread::spawn(move || {
            for file in f {
                tx.send(Tagger::tag_track(&file, None, Some(&mut tagger), &c)).ok();
            }
        });
        rx
    }
}