#[macro_use] extern crate log;

use std::error::Error;
use std::cmp::Ordering;
use std::time::Duration;
use chrono::NaiveDate;
use regex::Regex;
use serde::{Serialize, Deserialize};
use onetagger_tag::{TagSeparators, FrameName, AudioFileFormat};
use strsim::normalized_levenshtein;
use unidecode::unidecode;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MusicPlatform {
    Beatport,
    Traxsource,
    Discogs,
    JunoDownload,
    ITunes,
    MusicBrainz,
    Beatsource,
    Spotify,

    // For default
    None
}

impl Default for MusicPlatform {
    fn default() -> Self {
        MusicPlatform::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub isrc: bool,
    // 1T meta tags
    pub meta_tags: bool,

    // Advanced
    pub separators: TagSeparators,
    pub id3v24: bool,
    pub overwrite: bool,
    pub threads: u16,
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
    pub styles_custom_tag: Option<FrameName>,
    pub track_number_leading_zeroes: usize,
    pub enable_shazam: bool,
    pub force_shazam: bool,
    pub skip_tagged: bool,

    // Platform specific
    pub beatport: BeatportConfig,
    pub discogs: DiscogsConfig,
    pub beatsource: BeatsourceConfig,
    pub spotify: Option<SpotifyConfig>,
}


impl Default for TaggerConfig {
    // Suffering, but threads has to be 16 by default, strictness >0 etc and Default::default() caused stack overflow
    fn default() -> Self {
        Self {
            platforms: vec![MusicPlatform::Beatport], threads: 16, strictness: 0.7, path: None,
            title: false, artist: false, album: false, key: false, bpm: false, genre: false,
            style: false, label: false, release_date: false, publish_date: false, album_art: false,
            other_tags: false, catalog_number: false, url: false, track_id: false, release_id: false,
            version: false, duration: false, album_artist: false, remixer: false, track_number: false,
            isrc: false, meta_tags: false, separators: TagSeparators::default(), id3v24: false,
            overwrite: false, merge_genres: false, album_art_file: false, camelot: false,
            parse_filename: false, filename_template: None, short_title: false, match_duration: false,
            max_duration_difference: 30, match_by_id: false, multiple_matches: MultipleMatchesSort::Default,
            post_command: None, styles_options: StylesOptions::Default, styles_custom_tag: None,
            track_number_leading_zeroes: 0, enable_shazam: false, force_shazam: false, skip_tagged: false,
            beatport: Default::default(), discogs: Default::default(), beatsource: Default::default(), spotify: None,
        }
    }
}

// Beatport specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatportConfig {
    pub art_resolution: u32,
    pub max_pages: i64
}

impl Default for BeatportConfig {
    fn default() -> Self {
        Self { art_resolution: 500, max_pages: 1 }
    }
}

// Discogs specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscogsConfig {
    pub token: Option<String>,
    pub max_results: i16,
    pub track_number_int: bool,
    #[serde(skip)]
    pub rate_limit_override: Option<i16>,
}

impl Default for DiscogsConfig {
    fn default() -> Self {
        Self { token: None, max_results: 4, track_number_int: false, rate_limit_override: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyConfig {
    pub client_id: String,
    pub client_secret: String
}

/// Beatsource specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatsourceConfig {
    pub art_resolution: u32
}

impl Default for BeatsourceConfig {
    fn default() -> Self {
        Self { art_resolution: 1400 }
    }
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
    pub other: Vec<(FrameName, String)>,
    pub track_id: Option<String>,
    pub release_id: String,
    pub duration: Duration,
    pub remixers: Vec<String>,
    pub track_number: Option<TrackNumber>,
    pub isrc: Option<String>,
    
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
pub struct AudioFileInfo {
    pub title: Option<String>,
    pub artists: Vec<String>,
    pub format: AudioFileFormat,
    pub path: String,
    pub isrc: Option<String>,
    pub duration: Option<Duration>,
    pub track_number: Option<u16>,
    pub ids: AudioFileIDs,
    pub was_tagged: bool,
}

impl AudioFileInfo {
    // Get title
    pub fn title(&self) -> Result<&str, Box<dyn Error>> {
        if self.title.is_none() {
            error!("Track is missing title tag. {}", self.path);
            return Err("Missing title tag!".into());
        }
        Ok(self.title.as_ref().unwrap().as_str())
    }

    // Get first artist
    pub fn artist(&self) -> Result<&str, Box<dyn Error>> {
        if self.artists.is_empty() {
            error!("Track is missing artist tag. {}", self.path);
            return Err("Missing artist tag!".into());
        }
        Ok(self.artists.first().unwrap().as_str())
    }
    
}

/// IDs from various platforms
#[derive(Debug, Clone, Default)]
pub struct AudioFileIDs {
    pub discogs_release_id: Option<i64>,
    pub beatport_track_id: Option<i64>,
}

impl AudioFileIDs {
    /// Clean tag data and parse as int ID
    pub fn try_parse_int(input: &Vec<String>) -> Option<i64> {
        if input.is_empty() {
            return None;
        }
        input[0].trim().replace("\0", "").parse().ok()
    }

    // If all values are missing
    pub fn is_empty(&self) -> bool {
        self.discogs_release_id.is_none() && self.beatport_track_id.is_none()
    }
}

/// For all the platforms
pub trait AutotaggerSource {
    /// Create new instance
    fn new(config: &TaggerConfig) -> Result<Self, Box<dyn Error>> where Self: Sized;

    /// Returns (accuracy, track)
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Option<(f64, Track)>, Box<dyn Error>>;
}

pub struct MatchingUtils;
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
        unidecode(out.trim())
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
}