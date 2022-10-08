#[macro_use] extern crate log;

use std::error::Error;
use std::collections::HashMap;
use std::any::Any;
use std::cmp::Ordering;
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
pub use onetagger_tag::{TagSeparators, FrameName, AudioFileFormat, Field};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggerConfig {
    // Global
    pub platforms: Vec<String>,
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
    pub track_total: bool,
    pub disc_number: bool,
    pub isrc: bool,
    pub mood: bool,
    /// 1T meta tags
    pub meta_tags: bool,

    /// Advanced
    pub separators: TagSeparators,
    pub id3v24: bool,
    pub overwrite: bool,
    pub threads: u16,
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
    /// Tag the same track on multiple platforms
    pub multiplatform: bool,

    /// Platform specific. Format: `{ platform: { custom_option: value }}`
    pub custom: HashMap<String, Value>,
    pub spotify: Option<SpotifyConfig>,
}

impl TaggerConfig {
    /// Get platform's custom config
    pub fn get_custom<T: DeserializeOwned>(&self, platform_id: &str) -> Result<T, Box<dyn Error>> {
        let config = self.custom.get(platform_id).ok_or(format!("Missing {platform_id} custom config!"))?;
        Ok(serde_json::from_value(config.to_owned())?)
    }
}

impl Default for TaggerConfig {
    fn default() -> Self {
        Self {
            platforms: vec!["beatport".to_string()], 
            threads: 16, 
            strictness: 0.7, 
            path: None, 
            track_total: false,
            title: false, 
            artist: false, 
            album: false, 
            key: false, 
            bpm: true, 
            genre: true, 
            mood: false,
            style: true, 
            label: true, 
            release_date: true, 
            publish_date: false, 
            album_art: false, 
            disc_number: false,
            other_tags: false, 
            catalog_number: false, 
            url: false, 
            track_id: false, 
            release_id: false,
            version: false, 
            duration: false, 
            album_artist: false, 
            remixer: false, 
            track_number: false,
            isrc: false, 
            meta_tags: false, 
            separators: TagSeparators::default(), 
            id3v24: true, 
            only_year: false,
            overwrite: true, 
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
            custom: HashMap::new(), 
            include_subfolders: true,
            track_number_leading_zeroes: 0, 
            enable_shazam: false, 
            force_shazam: false, 
            skip_tagged: false, 
            move_success: false, 
            move_success_path: None, 
            move_failed: false, 
            move_failed_path: None
        }
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

#[derive(Debug, Clone, PartialEq, Default)]
#[repr(C)]
pub struct Track {
    // Use platform id
    pub platform: String,
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
    pub other: Vec<(FrameName, String)>,
    pub track_id: Option<String>,
    pub release_id: String,
    pub duration: Duration,
    pub remixers: Vec<String>,
    pub track_number: Option<TrackNumber>,
    pub track_total: Option<u16>,
    pub disc_number: Option<u16>,
    pub isrc: Option<String>,
    pub mood: Option<String>,
    
    // Only year OR date should be available
    pub release_year: Option<i64>,
    pub release_date: Option<NaiveDate>,
    pub publish_year: Option<i64>,
    pub publish_date: Option<NaiveDate>
}

impl Track {
    /// Get title with version
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

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
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

#[derive(Debug, Clone)]
#[repr(C)]
pub struct AudioFileInfo {
    pub title: Option<String>,
    pub artists: Vec<String>,
    pub format: AudioFileFormat,
    pub path: String,
    pub isrc: Option<String>,
    pub duration: Option<Duration>,
    pub track_number: Option<u16>,
    pub tagged: FileTaggedStatus,
    pub tags: HashMap<String, Vec<String>>
}

impl AudioFileInfo {
    /// Get title (or error shorthand)
    pub fn title(&self) -> Result<&str, Box<dyn Error>> {
        if self.title.is_none() {
            error!("Track is missing title tag. {}", self.path);
            return Err("Missing title tag!".into());
        }
        Ok(self.title.as_ref().unwrap().as_str())
    }

    /// Get first artist (or error shorthand)
    pub fn artist(&self) -> Result<&str, Box<dyn Error>> {
        if self.artists.is_empty() {
            error!("Track is missing artist tag. {}", self.path);
            return Err("Missing artist tag!".into());
        }
        Ok(self.artists.first().unwrap().as_str())
    }

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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    fn get_source(&mut self, config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Box<dyn Error>>;

    /// Get info about this platform
    fn info(&self) -> PlatformInfo;
}

/// For all the platforms
pub trait AutotaggerSource: Any + Send + Sync {
    /// Returns (accuracy, track)
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>>;
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
    Option { values: Vec<String>, value: String }
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
}


pub struct MatchingUtils;
impl MatchingUtils {
    /// Clean title for searching
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
        let mut step5 = step4.to_string();
        for t in &ATTRIBUTES_TO_REMOVE {
            step5 = step5.replace(t, "");
        }
        // Remove - and trim
        let step6 = step5.replace("-", "").replace("  ", " ");
        // Remove feat.
        re = Regex::new(r"(\(|\[)?(feat|ft)\.?.+?(\)|\]|\(|$)").unwrap();
        let out = re.replace(&step6, "");
        out.trim().to_string()
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
        let title = MatchingUtils::clean_title(input);
        // Remove edit, specials
        let step1 = title.replace("edit", "");
        let step2 = MatchingUtils::remove_special(&step1);
        step2.to_string()
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

    /// Default track matching
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
}

