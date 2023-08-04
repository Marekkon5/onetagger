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
pub use onetagger_tag::{TagSeparators, FrameName, AudioFileFormat, Field, Lyrics, LyricsLine, LyricsLinePart};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggerConfig {
    // Global
    pub platforms: Vec<String>,
    pub path: Option<String>,

    pub tags: Vec<SupportedTag>,

    /// Advanced
    pub separators: TagSeparators,
    pub id3v24: bool,

    /// Overwrite all tags
    pub overwrite: bool,
    /// Which tags to overwrite
    pub overwrite_tags: Vec<SupportedTag>,

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
    pub write_lrc: bool,
    pub enhanced_lrc: bool,
    pub capitalize_genres: bool,
    pub id3_comm_lang: Option<String>,
    pub remove_all_covers: bool,
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
            custom: HashMap::new(), 
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
    // Tag name, Value
    pub other: Vec<(FrameName, Vec<String>)>,
    pub track_id: Option<String>,
    pub release_id: String,
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
        let re = Regex::new(r"(\(|\[)?(feat|ft)\.? .+?(\)|\]|\(|$)").unwrap();
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

    /// Do exact matches on each step of track cleaning
    pub fn match_track_exact_fallback(info: &AudioFileInfo, tracks: &Vec<Track>, config: &TaggerConfig, match_artist: bool) -> Option<Track> {
        let cleaning_steps = [
            Self::clean_title_step1, Self::clean_title_step2, Self::clean_title_step3, Self::clean_title_step4,
            Self::clean_title_step5, Self::clean_title_step6, Self::clean_title_step7
        ];

        // Execute cleaning steps in order
        let clean_steps = |steps: usize, input: &str| -> String {
            let mut input = input.to_string();
            for i in 0..steps {
                input = cleaning_steps[i](&input);
            }
            input
        };

        // Match
        for step_count in 0..cleaning_steps.len() {
            let clean_title = clean_steps(step_count, info.title().ok()?);
            for track in tracks {
                // Duration
                if !MatchingUtils::match_duration(info, track, config) {
                    continue;
                }
                // Exact matches
                if clean_title == clean_steps(step_count, &track.full_title()) {
                    // Match artist
                    if match_artist {
                        if MatchingUtils::match_artist(&info.artists, &track.artists, config.strictness) {
                            return Some(track.to_owned())
                        } else {
                            continue;
                        }
                    }
                    return Some(track.to_owned())
                }
            }
        }

        None
    }

    /// Default track matching algo (v2 with exact match fallabck)
    pub fn match_track(info: &AudioFileInfo, tracks: &Vec<Track>, config: &TaggerConfig, match_artist: bool) -> Option<(f64, Track)> {
        // Exact fallback match
        if let Some(track) = MatchingUtils::match_track_exact_fallback(info, tracks, config, match_artist) {
            return Some((1.0, track));
        }

        let clean_title = MatchingUtils::clean_title_matching(info.title().ok()?);

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

    /// Default (old) track matching
    pub fn match_track_v1(info: &AudioFileInfo, tracks: &Vec<Track>, config: &TaggerConfig, match_artist: bool) -> Option<(f64, Track)> {
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
