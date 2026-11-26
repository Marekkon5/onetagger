#[macro_use] extern crate log;
#[macro_use] extern crate anyhow;

use anyhow::Error;
use std::collections::HashMap;
use std::any::Any;
use std::cmp::Ordering;
use std::ops::Deref;
use std::path::PathBuf;
use std::time::Duration;
use chrono::NaiveDate;
use regex::Regex;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;
use serde_json::Value;
use strsim::normalized_levenshtein;
use unidecode::unidecode;

pub mod custom;

const ATTRIBUTES_TO_REMOVE: [&'static str; 23] = ["(intro)", "(clean)", "(intro clean)", "(dirty)", "(intro dirty)", "(clean extended)",
    "(intro outro)", "(extended)", "(instrumental)", "(quick hit)", "(club version)", "(radio version)", "(club)", "(radio)", "(main)", 
    "(radio edit)", "(ck cut)", "(super cut)", "(mega cutz)", "(snip hitz)", "(jd live cut)", "(djcity intro)", "(vdj jd edit)"];

// Re-export
pub use onetagger_tag::{TagSeparators, FrameName, AudioFileFormat, Field, Lyrics, LyricsLine, LyricsLinePart};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggerConfig {
    // Global
    pub platforms: Vec<String>,
    pub path: Option<PathBuf>,

    pub tags: Vec<SupportedTag>,

    /// Advanced
    pub separators: TagSeparators,
    pub id3v24: bool,

    /// Overwrite all tags
    pub overwrite: bool,
    /// Which tags to overwrite
    pub overwrite_tags: Vec<SupportedTag>,

    pub threads: u16,
    /// Multiply provider request intervals and divide worker limits by this factor.
    /// Old profiles inherit the conservative default instead of bypassing pacing.
    #[serde(default = "default_provider_slowdown")]
    pub provider_slowdown: u16,
    /// From 0 to 1
    pub strictness: f64,
    pub merge_genres: bool,
    pub album_art_file: bool,
    pub camelot: bool,
    pub parse_filename: bool,
    pub filename_template: Option<String>,
    pub short_title: bool,
    pub match_duration: bool,
    /// In seconds
    pub max_duration_difference: u64,
    pub match_by_id: bool,
    pub multiple_matches: MultipleMatchesSort,
    pub post_command: Option<String>,
    pub styles_options: StylesOptions,
    // Option to prevent update errors
    pub styles_custom_tag: Option<FrameName>,
    pub track_number_leading_zeroes: usize,
    pub enable_shazam: bool,
    pub force_shazam: bool,
    pub skip_tagged: bool,
    pub include_subfolders: bool,
    pub only_year: bool,
    pub title_regex: Option<String>,
    pub move_success: bool,
    pub move_success_path: Option<String>,
    pub move_failed: bool,
    pub move_failed_path: Option<String>,
    pub write_lrc: bool,
    pub enhanced_lrc: bool,
    pub capitalize_genres: bool,
    pub id3_comm_lang: Option<String>,
    pub remove_all_covers: bool,
    /// Tag the same track on multiple platforms
    pub multiplatform: bool,
    /// Fetch all results instead of the most likely ones (used for (future) manual tag)
    pub fetch_all_results: bool,

    pub album_tagging: bool,
    /// % of tracks that have to be from one album to be considered as the correct
    pub album_tagging_ratio: f32,
    /// Renamer template
    pub cover_filename: Option<String>,

    /// Platform specific. Format: `{ platform: { custom_option: value }}`
    pub custom: PlatformTaggerConfig,
    pub spotify: Option<SpotifyConfig>,
}

fn default_provider_slowdown() -> u16 { 2 }

#[test]
fn old_profiles_inherit_gentler_pacing_without_allowing_zero_or_unbounded_factors() {
    let mut value = serde_json::to_value(TaggerConfig::default()).unwrap();
    value.as_object_mut().unwrap().remove("providerSlowdown");
    let mut config: TaggerConfig = serde_json::from_value(value).unwrap();
    assert_eq!(config.provider_slowdown(), 2);
    assert_eq!(config.worker_limit(0), 8);
    assert_eq!(config.worker_limit(4), 2);
    assert_eq!(config.worker_limit(1), 1);
    config.provider_slowdown = 0;
    assert_eq!(config.worker_limit(0), 16);
    config.provider_slowdown = u16::MAX;
    assert_eq!(config.provider_slowdown(), 10);
    config.threads = 0;
    assert_eq!(config.worker_limit(0), 1);
}

impl TaggerConfig {
    pub fn provider_slowdown(&self) -> u16 { self.provider_slowdown.clamp(1, 10) }

    /// Keep at least one worker, including for providers that allow only one.
    pub fn worker_limit(&self, provider_max: u16) -> u16 {
        let threads = self.threads.max(1);
        let limit = if provider_max == 0 { threads } else { threads.min(provider_max) };
        limit.div_ceil(self.provider_slowdown())
    }

    /// Get platform's custom config
    pub fn get_custom<T: DeserializeOwned>(&self, platform_id: &str) -> Result<T, Error> {
        let config = self.custom.get(platform_id).ok_or(anyhow!("Missing {platform_id} custom config!"))?;
        Ok(serde_json::from_value(config.to_owned())?)
    }

    /// Is tag enabled
    pub fn tag_enabled(&self, tag: SupportedTag) -> bool {
        self.tags.contains(&tag)
    }

    /// Is any of the following tags enabled
    pub fn any_tag_enabled(&self, tags: &[SupportedTag]) -> bool {
        tags.iter().any(|t| self.tags.contains(t))
    }

    /// Should this track be overwritten
    pub fn overwrite_tag(&self, tag: SupportedTag) -> bool {
        if self.overwrite {
            return true;
        }
        self.overwrite_tags.contains(&tag)
    }
}


impl Default for TaggerConfig {
    fn default() -> Self {
        Self {
            platforms: vec!["beatport".to_string()], 
            threads: 16, 
            provider_slowdown: default_provider_slowdown(),
            strictness: 0.7, 
            path: None, 
            tags: vec![
                SupportedTag::Genre, SupportedTag::BPM, SupportedTag::Style, SupportedTag::Label, SupportedTag::ReleaseDate, 
            ],
            separators: TagSeparators::default(), 
            id3v24: true, 
            only_year: false,
            overwrite: true, 
            overwrite_tags: vec![],
            merge_genres: false, 
            album_art_file: false, 
            camelot: false, 
            styles_options: StylesOptions::Default,
            parse_filename: false, 
            filename_template: Some("%artists% - %title%".to_string()), 
            short_title: false, 
            match_duration: false, 
            multiplatform: false,
            max_duration_difference: 30, 
            match_by_id: false, 
            multiple_matches: MultipleMatchesSort::Default, 
            title_regex: None,
            post_command: None, 
            styles_custom_tag: Some(FrameName::same("STYLE")), 
            spotify: None, 
            custom: Default::default(), 
            include_subfolders: true,
            track_number_leading_zeroes: 0, 
            enable_shazam: false, 
            force_shazam: false, 
            skip_tagged: false, 
            move_success: false, 
            move_success_path: None, 
            move_failed: false, 
            move_failed_path: None,
            write_lrc: false,
            enhanced_lrc: false,
            capitalize_genres: false,
            remove_all_covers: false,
            id3_comm_lang: None,
            fetch_all_results: false,
            album_tagging: false,
            album_tagging_ratio: 0.5,
            cover_filename: None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[repr(transparent)]
pub struct PlatformTaggerConfig(pub HashMap<String, Value>);

impl Deref for PlatformTaggerConfig {
    type Target = HashMap<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<HashMap<String, Value>> for PlatformTaggerConfig {
    fn from(value: HashMap<String, Value>) -> Self {
        Self(value)
    }
}

impl Into<HashMap<String, Value>> for PlatformTaggerConfig {
    fn into(self) -> HashMap<String, Value> {
        self.0
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyConfig {
    pub client_id: String,
    pub client_secret: String
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

impl From<String> for MultipleMatchesSort {
    fn from(value: String) -> Self {
        match value.as_str() {
            "default" => MultipleMatchesSort::Default,
            "oldest" => MultipleMatchesSort::Oldest,
            "newest" => MultipleMatchesSort::Newest,
            _ => MultipleMatchesSort::Default
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct Track {
    /// Use platform id
    pub platform: String,
    /// Short title
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
    /// Tag name, Value
    pub other: Vec<(FrameName, Vec<String>)>,
    pub track_id: Option<String>,
    pub release_id: Option<String>,
    pub duration: Duration,
    pub remixers: Vec<String>,
    pub track_number: Option<TrackNumber>,
    pub track_total: Option<u16>,
    pub disc_number: Option<u16>,
    pub isrc: Option<String>,
    pub mood: Option<String>,
    /// None = unknown
    pub explicit: Option<bool>,

    pub lyrics: Option<Lyrics>,
    
    // Only year OR date should be available
    pub release_year: Option<i16>,
    pub release_date: Option<NaiveDate>,
    pub publish_year: Option<i16>,
    pub publish_date: Option<NaiveDate>,

    /// URL to cover thumbnail
    pub thumbnail: Option<String>,

    /// Custom fields for passing into extend_track
    pub custom: HashMap<String, String>
}

impl Track {
    /// Get title with version
    pub fn full_title(&self) -> String {
        if let Some(v) = self.version.as_ref() {
            if v.trim().is_empty() {
                self.title.to_string()
            } else {
                let v = v.trim();
                if v.starts_with("(") || v.starts_with("[") || v.starts_with("{") {
                    format!("{} {}", self.title, v)
                } else {
                    format!("{} ({})", self.title, v)                    
                }
            }
        } else {
            self.title.to_string()
        }
    }
}

impl Track {
    /// Merge with other track
    pub fn merge(mut self, other: Track) -> Track {
        /// Merge 2 arrays
        fn merge_array<T: PartialEq>(a: &mut Vec<T>, b: Vec<T>) {
            let x = b.into_iter().filter(|i| !a.contains(i)).collect::<Vec<_>>();
            a.extend(x);
        }

        // Generate self.value = self.value.or(other.value)
        macro_rules! gen_track_merge_option {
            ($($a: ident),*) => {
                $( self.$a = self.$a.or(other.$a); )*
            }
        }
        gen_track_merge_option!(version, album, key, bpm, art, label, catalog_number, track_number, 
                track_total, disc_number, isrc, mood, explicit, lyrics, release_date, release_year, 
                publish_date, publish_year, thumbnail);

        // Generate merge_array(&mut self.value, other.value)
        macro_rules! gen_track_merge_array {
            ($($a: ident),*) => {
                $( merge_array(&mut self.$a, other.$a); )*
            }
        }
        gen_track_merge_array!(artists, album_artists, genres, styles, other, remixers);

        self
    }
}


/// Matched track
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[repr(C)]
pub struct TrackMatch {
    pub accuracy: f64,
    pub track: Track,
    pub reason: MatchReason
}

impl TrackMatch {
    /// Create new instance
    pub fn new(accuracy: f64, track: Track) -> TrackMatch {
        TrackMatch { accuracy, track, reason: MatchReason::Fuzzy }
    }

    /// Create new ISRC matched match
    pub fn new_isrc(track: Track) -> TrackMatch {
        TrackMatch { accuracy: 1.0, track, reason: MatchReason::ISRC }
    }

    /// Create new ID matched match
    pub fn new_id(track: Track) -> TrackMatch {
        TrackMatch { accuracy: 1.0, track, reason: MatchReason::ID }
    }
}

impl PartialOrd for TrackMatch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.accuracy.partial_cmp(&other.accuracy)
    }
}

/// Why was this track matched
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Ord, PartialOrd, Copy)]
#[serde(rename_all = "camelCase")]
pub enum MatchReason {
    Fuzzy,
    #[serde(rename = "isrc")]
    ISRC,
    #[serde(rename = "id")]
    ID,
    Album
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[repr(C)]
#[serde(untagged)]
pub enum TrackNumber {
    Number(i32),
    /// Custom format (Discogs)
    Custom(String)
}

impl TrackNumber {
    pub fn to_string_with_zeroes(&self, leading_zeroes: usize) -> String {
        match self {
            TrackNumber::Number(n) => format!("{n:00$}", leading_zeroes),
            TrackNumber::Custom(c) => c.to_owned(),
        }
    }
}

impl From<i32> for TrackNumber {
    fn from(i: i32) -> Self {
        TrackNumber::Number(i)
    }
}

impl ToString for TrackNumber {
    fn to_string(&self) -> String {
        match self {
            TrackNumber::Number(n) => n.to_string(),
            TrackNumber::Custom(c) => c.to_string(),
        }
    }
}


/// For Discogs & Beatport
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[repr(C)]
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

impl From<String> for StylesOptions {
    fn from(value: String) -> Self {
        match value.as_str() {
            "default" => StylesOptions::Default,
            "onlyGenres" => StylesOptions::OnlyGenres,
            "onlyStyles" => StylesOptions::OnlyStyles,
            "mergeToGenres" => StylesOptions::MergeToGenres,
            "mergeToStyles" => StylesOptions::MergeToStyles,
            "stylesToGenre" => StylesOptions::StylesToGenre,
            "genresToStyle" => StylesOptions::GenresToStyle,
            "customTag" => StylesOptions::CustomTag,
            _ => StylesOptions::Default
        }
    }
}

pub const CAMELOT_NOTES: [(&str, &str); 35] = [
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

/// Convert to camelot or return original
pub fn to_camelot(key: &str) -> &str {
    if let Some((_, v)) = CAMELOT_NOTES.iter().find(|(k, _)| *k == key.trim()) {
        return *v;
    }
    key
}

/// Convert from camelot to normal key
pub fn from_camelot(key: &str) -> &str {
    if let Some((v, _)) = CAMELOT_NOTES.iter().find(|(_, k)| *k == key.trim()) {
        return *v;
    }
    key
}


pub trait LyricsExt {
    /// Generate LRC data
    /// If meta is present, will be written
    /// None if are unsynced
    fn generate_lrc(&self, meta: Option<&Track>, enhanced: bool) -> Option<String>;
}

/// Format LRC timestamp
fn format_lrc_ts(ts: Duration) -> String {
    format!("{:02}:{}{:.2}", ts.as_secs() / 60, if (ts.as_secs() % 60) < 10 { "0" } else { "" }, ts.as_secs_f32() % 60.0)
}

impl LyricsExt for Lyrics {
    fn generate_lrc(&self, meta: Option<&Track>, enhanced: bool) -> Option<String> {
        let mut output = String::new();
        // Add meta
        if let Some(track) = meta {
            if !track.title.trim().is_empty() {
                output.push_str(&format!("[ti:{}]\n", track.title));
                if let Some(album) = track.album.as_ref() {
                    output.push_str(&format!("[al:{album}]\n"));
                }
                if let Some(artist) = track.artists.first() {
                    output.push_str(&format!("[ar:{artist}]\n"));
                }
                if track.duration != Duration::ZERO {
                    output.push_str(&format!("[length: {}:{:02}]\n", track.duration.as_secs() / 60, track.duration.as_secs() % 60));
                }
                output.push('\n');
            }
        }
        // Write lines
        let mut written = false;
        for line in self.iter_lines() {
            if let Some(start) = line.start {
                // Write normal
                if !enhanced || line.parts.is_empty() {
                    output.push_str(&format!("[{}]{}\n", format_lrc_ts(start), line.text));
                } else {
                    // Write enhanced
                    output.push_str(&format!("[{}]", format_lrc_ts(start)));
                    for part in &line.parts {
                        if let Some(start) = part.start {
                            output.push_str(&format!(" <{}> {}", format_lrc_ts(start), part.text));
                        } else {
                            output.push_str(&format!(" {}", part.text));
                        }
                    }
                    output.push('\n');
                }
                written = true;
            }
        }

        // No lyrics if aren't synced
        match written {
            true => Some(output),
            false => None
        }
    }

}

/// Representation of an Album / Release. 
/// Very slim, since `Track` should have all the relevant information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(C)]
pub struct Album {
    pub id: String,
    pub name: String,

    pub tracks: Vec<Track>
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(C)]
pub struct AudioFileInfo {
    pub title: Option<String>,
    pub artists: Vec<String>,
    pub format: AudioFileFormat,
    pub path: PathBuf,
    pub isrc: Option<String>,
    pub duration: Option<Duration>,
    pub track_number: Option<u16>,
    pub tagged: FileTaggedStatus,
    pub tags: HashMap<String, Vec<String>>
}

impl AudioFileInfo {
    /// Get title (or error shorthand)
    pub fn title(&self) -> Result<&str, Error> {
        if self.title.is_none() {
            error!("Track is missing title tag. {:?}", self.path);
            return Err(anyhow!("Missing title tag!"));
        }
        Ok(self.title.as_ref().unwrap().as_str())
    }

    /// Get first artist (or error shorthand)
    pub fn artist(&self) -> Result<&str, Error> {
        if self.artists.is_empty() {
            error!("Track is missing artist tag. {:?}", self.path);
            return Err(anyhow!("Missing artist tag!"));
        }
        Ok(self.artists.first().unwrap().as_str())
    }

}

impl AudioFileInfo {
    // Try to split artist string with common separators
    pub fn parse_artist_tag(input: Vec<&str>) -> Vec<String> {
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

/// If the file was already tagged with OneTagger
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Copy)]
pub enum FileTaggedStatus {
    /// Not tagged with 1T
    Untagged,
    /// Tagged using 1T AudioFeatures
    AudioFeatures,
    /// Tagged using 1T AutoTagger
    AutoTagger,
    /// Tagged using older version of 1T (can be either AT or AF)
    Tagged
}

impl FileTaggedStatus {
    /// Was tagged with AutoTagger
    pub fn at(&self) -> bool {
        self == &FileTaggedStatus::AutoTagger || self == &FileTaggedStatus::Tagged
    }

    /// Was tagged with AudioFeatures
    pub fn af(&self) -> bool {
        self == &FileTaggedStatus::AudioFeatures || self == &FileTaggedStatus::Tagged
    }
}

/// For generating `AutotaggerSource`
pub trait AutotaggerSourceBuilder: Any + Send + Sync {
    /// Constructor so creation can be generalized
    fn new() -> Self where Self: Sized;

    /// Get AutotaggerSource for tagging
    fn get_source(&mut self, config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Error>;

    /// Get info about this platform
    fn info(&self) -> PlatformInfo;

    /// Callback from config
    /// Use Value as data type for the sake of object safety, FFI, simplicity, and it's used in JS anyway
    fn config_callback(&mut self, _name: &str, _config: Value) -> ConfigCallbackResponse {
        ConfigCallbackResponse::Empty
    }
}

/// For all the platforms
pub trait AutotaggerSource: Any + Send + Sync {
    /// Returns (accuracy, track)
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Vec<TrackMatch>, Error>;

    /// Extend track with extra metadata (match track should be as fast as possible)
    fn extend_track(&mut self, track: &mut Track, config: &TaggerConfig) -> Result<(), Error>;

    /// Get an album by ID (used for album tagging)
    #[allow(unused_variables)]
    fn get_album(&mut self, id: &str, config: &TaggerConfig) -> Result<Option<Album>, Error> {
        warn!("Album tagging not supported on this platform!");
        Ok(None)
    }
}

/// Response from Config callback
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
#[repr(C)]
pub enum ConfigCallbackResponse {
    UpdateConfig { config: Value },
    Error { error: String },
    Empty
}

/// Platform info for GUI platform selector
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub struct PlatformInfo {
    /// Should be unique
    pub id: String,
    /// Shown in UI
    pub name: String,
    /// Shown only in UI, can use HTML
    pub description: String,
    /// Version of this platform (use SemVer)
    pub version: String,
    /// Image bytes, use 1:1 aspect ratio, PNG for transparency recommended
    #[serde(skip)]
    pub icon: &'static [u8],
    /// Max amounts of threads this tagger can use (use 0 for any user defined amount)
    pub max_threads: u16,
    /// For showing custom options in UI
    pub custom_options: PlatformCustomOptions,
    /// Which fields does this platform support
    pub supported_tags: Vec<SupportedTag>,
    /// Does this platform require authentication
    pub requires_auth: bool,
}

/// All the different tags a platform can support
#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub enum SupportedTag {
    Title, Artist, Album, Key, Genre, Style, ReleaseDate, PublishDate,
    AlbumArt, OtherTags, CatalogNumber, TrackId, ReleaseId, Version,
    Duration, AlbumArtist, Remixer, TrackNumber, TrackTotal, DiscNumber,
    Mood, SyncedLyrics, UnsyncedLyrics, Label, Explicit, MetaTags,
    #[serde(rename = "bpm")]
    BPM,
    #[serde(rename = "url")]
    URL,
    #[serde(rename = "isrc")]
    ISRC
} 

/// Generate supported tags list quickly
/// Use: `supported_tags!(Title, Artist, Album...)`
#[macro_export]
macro_rules! supported_tags {
    ($($a:tt),*) => {
        vec![$(::onetagger_tagger::SupportedTag::$a, )*]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub enum PlatformCustomOptionValue {
    /// Switch
    Boolean { value: bool },
    /// Slider
    Number { min: i32, max: i32, step: i32, value: i32 },
    /// Input field
    String { value: String, hidden: Option<bool> },
    /// Custom tag picker
    Tag { value: FrameName },
    /// Select / dropdown
    Option { values: Vec<String>, value: String },
    /// Callback button
    Button,
}

impl PlatformCustomOptionValue {
    /// Get JSON value 
    pub fn json_value(&self) -> Value {
        match self {
            PlatformCustomOptionValue::Boolean { value } => Value::from(*value),
            PlatformCustomOptionValue::Number { value, .. } => Value::from(*value),
            PlatformCustomOptionValue::String { value, .. } => Value::from(value.clone()),
            PlatformCustomOptionValue::Tag { value } => serde_json::to_value(&value).unwrap(),
            PlatformCustomOptionValue::Option { value, .. } => Value::from(value.to_string()),
            PlatformCustomOptionValue::Button => Value::Null,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub struct PlatformCustomOption {
    pub id: String,
    pub label: String,
    pub tooltip: Option<String>,
    pub value: PlatformCustomOptionValue
}

impl PlatformCustomOption {
    /// Create new custom option
    pub fn new(id: &str, label: &str, value: PlatformCustomOptionValue) -> PlatformCustomOption {
        PlatformCustomOption {
            id: id.to_string(),
            label: label.to_string(),
            tooltip: None,
            value,
        }
    }

    /// Add tooltip
    pub fn tooltip(mut self, tooltip: &str) -> PlatformCustomOption {
        self.tooltip = Some(tooltip.to_string());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[repr(C)]
pub struct PlatformCustomOptions {
    pub options: Vec<PlatformCustomOption>
}

impl PlatformCustomOptions {
    /// Create new empty instance
    pub fn new() -> PlatformCustomOptions {
        PlatformCustomOptions {
            options: vec![]
        }
    }

    /// Add new option
    pub fn add(mut self, id: &str, label: &str, value: PlatformCustomOptionValue) -> PlatformCustomOptions {
        self.options.push(PlatformCustomOption::new(id, label, value));
        self
    }

    /// Add new option with tooltip
    pub fn add_tooltip(mut self, id: &str, label: &str, tooltip: &str, value: PlatformCustomOptionValue) -> PlatformCustomOptions {
        self.options.push(PlatformCustomOption::new(id, label, value).tooltip(tooltip));
        self
    }

    /// Get the default config values
    pub fn get_defaults(&self) -> Value {
        let mut out = HashMap::new();
        for option in &self.options {
            out.insert(option.id.to_owned(), option.value.json_value());
        }
        serde_json::to_value(&out).unwrap()
    }
}

pub struct MatchingUtils;
impl MatchingUtils {
    /// Clean title for searching
    pub fn clean_title(input: &str) -> String {
        let input = Self::clean_title_step1(input);
        let input = Self::clean_title_step2(&input);
        let input = Self::clean_title_step3(&input);
        let input = Self::clean_title_step4(&input);
        // Trim and clean again
        let input = Self::clean_title_step1(&input);
        let input = Self::clean_title_step5(&input);
        // Trim and clean again
        Self::clean_title_step1(&input)
    }

    /// Step 1: lowercase,remove dashes and double spaces because of search engines
    fn clean_title_step1(input: &str) -> String {
        let mut input = input.to_lowercase().replace("-", " ");
        while input.contains("  ") {
            input = input.replace("  ", " ");
        }
        input.trim().to_string()
    }

    /// Step 2: Remove initial a/an/the
    fn clean_title_step2(input: &str) -> String {
        let re = Regex::new(r"^( (a|an|the) )").unwrap();
        re.replace(input, "").into_owned()
    }

    /// Step 3: Remove original mix/edit/version
    fn clean_title_step3(input: &str) -> String {
        let re = Regex::new(r"((\(|\[)*)original( (mix|version|edit))*((\)|\])*)$").unwrap();
        re.replace(input, "").into_owned()
    }

    /// Step 4: Remove attributes
    fn clean_title_step4(input: &str) -> String {
        let mut input = input.to_string();
        for t in &ATTRIBUTES_TO_REMOVE {
            input = input.replace(t, "");
        }
        input
    }

    /// Step 5: Remove feat/ft
    fn clean_title_step5(input: &str) -> String {
        let re = Regex::new(r" (\(|\[)?(feat|ft)\.? .+?(\)|\]|\(|$)").unwrap();
        re.replace(input, "").into_owned()
    }

    /// Step 6: Remove edit
    fn clean_title_step6(input: &str) -> String {
        input.replace("edit", "")
    }

    /// Step 7: Remove special characters
    fn clean_title_step7(input: &str) -> String {
        Self::remove_special(input)
    }

    /// Remove spacial characters
    pub fn remove_special(input: &str) -> String {
        let special = ".,()[]&_\"'-/\\^";
        let mut out = input.to_string();
        for c in special.chars() {
            out = out.replace(c, "");
        }
        out = out.replace("  ", " ");
        unidecode(out.trim())
    }

    /// Clean list of artists
    pub fn clean_artists(input: &Vec<String>) -> Vec<String> {
        let mut clean: Vec<String> = input.into_iter().map(
            |a| MatchingUtils::remove_special(&a.to_lowercase()).trim().to_string()
        ).collect();
        clean.sort();
        clean
    }

    /// Clean title for matching, removes special characters etc
    pub fn clean_title_matching(input: &str) -> String {
        let input = MatchingUtils::clean_title(input);
        let input = Self::clean_title_step6(&input);
        Self::clean_title_step7(&input)
    }

    /// Clean artist for searching on platforms
    pub fn clean_artist_searching(input: &str) -> String {
        let out = Self::clean_title_step1(&input.to_lowercase());
        // Remove potential feat.
        let out = Self::clean_title_step5(&out);
        out.trim().to_string()
    }

    /// Match atleast 1 artist
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

    // Each candidate appears once at the earliest cleaning step that matched.
    fn exact_indices(info: &AudioFileInfo, tracks: &[Track], config: &TaggerConfig, match_artist: bool) -> Vec<usize> {
        let Ok(title) = info.title() else { return Vec::new() };
        let steps = [Self::clean_title_step1, Self::clean_title_step2, Self::clean_title_step3,
            Self::clean_title_step4, Self::clean_title_step5, Self::clean_title_step6];
        let mut titles = vec![title.to_owned()];
        for clean in steps { titles.push(clean(titles.last().unwrap())); }

        let mut matches = Vec::new();
        for (index, track) in tracks.iter().enumerate() {
            if !Self::match_duration(info, track, config) { continue; }
            let mut candidate = track.full_title();
            for (level, title) in titles.iter().enumerate() {
                if *title == candidate {
                    if !match_artist || Self::match_artist(&info.artists, &track.artists, config.strictness) {
                        matches.push((level, index));
                    }
                    break;
                }
                if let Some(clean) = steps.get(level) { candidate = clean(&candidate); }
            }
        }
        matches.sort_unstable();
        matches.into_iter().map(|(_, index)| index).collect()
    }

    pub fn match_track_exact_fallback(info: &AudioFileInfo, tracks: &Vec<Track>, config: &TaggerConfig, match_artist: bool) -> Vec<Track> {
        Self::exact_indices(info, tracks, config, match_artist).into_iter().map(|index| tracks[index].clone()).collect()
    }

    pub fn match_track(info: &AudioFileInfo, tracks: &Vec<Track>, config: &TaggerConfig, match_artist: bool) -> Vec<TrackMatch> {
        let exact = Self::exact_indices(info, tracks, config, match_artist);
        let mut output: Vec<_> = exact.iter().map(|&i| TrackMatch::new(1.0, tracks[i].clone())).collect();
        if !config.fetch_all_results && !output.is_empty() { return output; }
        let Ok(title) = info.title() else { return output };
        let title = Self::clean_title_matching(title);
        let mut matched = vec![false; tracks.len()];
        for index in exact { matched[index] = true; }
        for (index, track) in tracks.iter().enumerate() {
            if matched[index] || !Self::match_duration(info, track, config) { continue; }
            if match_artist && !Self::match_artist(&info.artists, &track.artists, config.strictness) { continue; }
            let score = normalized_levenshtein(&Self::clean_title_matching(&track.full_title()), &title);
            if score >= config.strictness { output.push(TrackMatch::new(score, track.clone())); }
        }
        output
    }

    /// Folder consensus establishes a release, not which recording belongs to a file.
    pub fn match_album_track(info: &AudioFileInfo, tracks: &[Track], config: &TaggerConfig) -> Option<TrackMatch> {
        let eligible = |track: &&Track| Self::match_duration(info, track, config);
        let mut identified = Vec::new();
        let mut candidates = Vec::new();
        let local_artists = Self::clean_artists(&info.artists);
        let local_title = info.title.as_deref().map(str::trim).filter(|t| !t.is_empty());
        let clean_title = local_title.map(Self::clean_title_matching);
        for track in tracks.iter().filter(eligible) {
            if let Some(local_id) = info.tags.get(&format!("{}_TRACK_ID", track.platform.to_uppercase()))
                .and_then(|ids| ids.first()).filter(|id| !id.trim().is_empty()) {
                if track.track_id.as_deref() == Some(local_id.trim()) {
                    identified.push((2, track));
                }
                continue;
            }
            if let (Some(local), Some(remote)) = (&info.isrc, &track.isrc) {
                if !local.trim().is_empty() && !remote.trim().is_empty() {
                    if local.trim().eq_ignore_ascii_case(remote.trim()) { identified.push((1, track)); }
                    continue;
                }
            }
            let Some(title) = local_title else { continue };
            let full_title = track.full_title();
            let exact = full_title.trim().eq_ignore_ascii_case(title);
            let remote_artists = Self::clean_artists(&track.artists);
            let have_artists = local_artists.iter().any(|a| !a.is_empty())
                && remote_artists.iter().any(|a| !a.is_empty());
            if have_artists {
                let compatible = local_artists.iter().filter(|a| !a.is_empty()).any(|a|
                    remote_artists.iter().filter(|b| !b.is_empty()).any(|b| {
                        let similarity = normalized_levenshtein(a, b);
                        similarity > 0.0 && similarity >= config.strictness
                    }));
                if !compatible { continue; }
            } else if !exact { continue; }
            let score = if exact { 1.0 } else {
                let local = clean_title.as_deref().unwrap_or("");
                let remote = Self::clean_title_matching(&full_title);
                if local.is_empty() || remote.is_empty() { continue; }
                normalized_levenshtein(local, &remote)
            };
            if score > 0.0 && score >= config.strictness { candidates.push((exact, score, track)); }
        }
        if !identified.is_empty() {
            let strongest = identified.iter().map(|(rank, _)| *rank).max()?;
            let mut hits = identified.into_iter().filter(|(rank, _)| *rank == strongest);
            let (_, track) = hits.next()?;
            return hits.next().is_none().then(|| TrackMatch::new_id(track.clone()));
        }
        candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.total_cmp(&a.1)));
        let &(exact, score, track) = candidates.first()?;
        if candidates.get(1).is_some_and(|other| other.0 == exact && other.1 == score) { return None; }
        Some(TrackMatch::new(score, track.clone()))
    }

    /// A provider can return the same recording on several editions.
    pub fn deduplicate_matches(matches: &mut Vec<TrackMatch>) {
        matches.sort_by(|a, b| b.accuracy.total_cmp(&a.accuracy));
        let mut ids = std::collections::HashSet::new();
        let mut anonymous = Vec::new();
        matches.retain(|m| {
            if let Some(id) = m.track.track_id.as_ref().filter(|id| !id.is_empty()) {
                ids.insert((m.track.platform.clone(), id.clone(), m.track.release_id.clone()))
            } else if anonymous.contains(&m.track) { false }
            else { anonymous.push(m.track.clone()); true }
        });
    }

    pub fn sort_tracks(tracks: &mut [TrackMatch], config: &TaggerConfig) {
        use chrono::Datelike;
        let date = |t: &Track| t.release_date.map(|d| (d.year(), Some(d)))
            .or_else(|| t.release_year.map(|y| (i32::from(y), None)));
        tracks.sort_by(|a, b| {
            let order = match (&config.multiple_matches, date(&a.track), date(&b.track)) {
                (MultipleMatchesSort::Default, _, _) => Ordering::Equal,
                (_, None, Some(_)) => Ordering::Greater,
                (_, Some(_), None) => Ordering::Less,
                (MultipleMatchesSort::Oldest, Some(a), Some(b)) => a.cmp(&b),
                (MultipleMatchesSort::Newest, Some(a), Some(b)) => b.cmp(&a),
                (_, None, None) => Ordering::Equal,
            };
            order.then_with(|| b.accuracy.total_cmp(&a.accuracy))
        });
    }

    /// Match duration
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

    /// Parse duration from String
    pub fn parse_duration(input: &str) -> Result<Duration, Error> {
        let clean = input.replace("(", "").replace(")", "");
        let mut parts = clean.trim().split(":").collect::<Vec<&str>>();
        parts.reverse();
        let mut seconds: u64 = parts.first().ok_or(anyhow!("Invalid timestamp!"))?.parse()?;
        if parts.len() > 1 {
            seconds += parts[1].parse::<u64>()? * 60;
        }
        if parts.len() > 2 {
            seconds += parts[2].parse::<u64>()? * 3600;
        }
        Ok(Duration::from_secs(seconds))
    }
}


#[cfg(test)]
mod duration_regression_tests {
    use super::*;

    fn info(duration: Option<Duration>) -> AudioFileInfo {
        AudioFileInfo {
            title: Some("Night Drive".into()), artists: vec!["Artist".into()],
            format: AudioFileFormat::MP3, path: "track.mp3".into(), isrc: None,
            duration, track_number: None, tagged: FileTaggedStatus::Untagged, tags: HashMap::new(),
        }
    }

    #[test]
    fn manual_matches_preserve_editions_once_and_keep_the_best_provider_result() {
        let cfg = TaggerConfig { fetch_all_results: true, ..Default::default() };
        let editions: Vec<_> = ["A", "B"].into_iter().map(|id| Track {
            platform: "fixture".into(), track_id: Some("recording".into()), release_id: Some(id.into()),
            title: "Night Drive".into(), artists: vec!["Artist".into()], ..Default::default()
        }).collect();
        let mut choices = MatchingUtils::match_track(&info(None), &editions, &cfg, true);
        assert_eq!(choices.len(), 2);
        choices.insert(0, TrackMatch::new(0.6, editions[0].clone()));
        MatchingUtils::deduplicate_matches(&mut choices);
        assert_eq!(choices.len(), 2);
        assert_eq!(choices[0].accuracy, 1.0);
        assert_eq!(choices[1].track.release_id.as_deref(), Some("B"));
    }

    #[test]
    fn date_preferences_rank_dated_tracks_across_undated_ones() {
        for (preference, expect) in [(MultipleMatchesSort::Oldest, 1990), (MultipleMatchesSort::Newest, 2020)] {
            let mut tracks: Vec<_> = [Some(1990), None, Some(2020)].into_iter().map(|y|
                TrackMatch::new(1.0, Track { release_year: y, ..Default::default() })).collect();
            MatchingUtils::sort_tracks(&mut tracks, &TaggerConfig { multiple_matches: preference, ..Default::default() });
            assert_eq!(tracks[0].track.release_year, Some(expect));
            assert_eq!(tracks[2].track.release_year, None);
        }
    }

    #[test]
    fn album_identity_rejects_conflicts_ties_and_duration_mismatch() {
        let cfg = TaggerConfig { strictness: 1.0, match_duration: true, max_duration_difference: 0, ..Default::default() };
        let mut file = info(Some(Duration::from_secs(180)));
        let original = Track { platform: "shop".into(), title: "Night Drive".into(), artists: vec!["Artist".into()],
            track_id: Some("A".into()), duration: Duration::from_secs(180), ..Default::default() };
        let mut other = original.clone();
        other.track_id = Some("B".into());
        let album = vec![original.clone(), other.clone()];
        assert!(MatchingUtils::match_album_track(&file, &album, &cfg).is_none());
        file.tags.insert("SHOP_TRACK_ID".into(), vec!["B".into()]);
        let hit = MatchingUtils::match_album_track(&file, &album, &cfg).unwrap();
        assert_eq!(hit.track.track_id.as_deref(), Some("B"));
        file.tags.insert("SHOP_TRACK_ID".into(), vec!["other".into()]);
        assert!(MatchingUtils::match_album_track(&file, &album, &cfg).is_none());
        file.tags.clear();
        other.duration = Duration::from_secs(90);
        assert_eq!(MatchingUtils::match_album_track(&file, &[original.clone(), other], &cfg).unwrap().track.track_id, original.track_id);
        file.artists = vec!["Different artist".into()];
        assert!(MatchingUtils::match_album_track(&file, &[original], &cfg).is_none());
    }

    #[test]
    fn fuzzy_fallback_cannot_reintroduce_duration_rejected_track() {
        let config = TaggerConfig { match_duration: true, max_duration_difference: 10, ..Default::default() };
        let track = Track { title: "Night Drive".into(), artists: vec!["Artist".into()], duration: Duration::from_secs(420), ..Default::default() };
        let file = info(Some(Duration::from_secs(180)));
        assert!(MatchingUtils::match_track(&file, &vec![track.clone()], &config, true).is_empty());
        let all = TaggerConfig { fetch_all_results: true, ..config.clone() };
        assert!(MatchingUtils::match_track(&file, &vec![track], &all, true).is_empty());
    }

    #[test]
    fn fuzzy_matches_respect_boundary_and_keep_optional_duration_behavior() {
        let config = TaggerConfig { match_duration: true, max_duration_difference: 10, ..Default::default() };
        let mut track = Track { title: "Night Drives".into(), artists: vec!["Artist".into()], duration: Duration::from_secs(190), ..Default::default() };
        let file = info(Some(Duration::from_secs(180)));
        assert!(!MatchingUtils::match_track(&file, &vec![track.clone()], &config, true).is_empty());
        track.duration = Duration::from_secs(191);
        assert!(MatchingUtils::match_track(&file, &vec![track.clone()], &config, true).is_empty());
        assert!(!MatchingUtils::match_track(&info(None), &vec![track.clone()], &config, true).is_empty());
        let disabled = TaggerConfig { match_duration: false, ..config };
        assert!(!MatchingUtils::match_track(&file, &vec![track], &disabled, true).is_empty());
    }
}
