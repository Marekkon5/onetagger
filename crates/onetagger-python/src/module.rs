use std::path::Path;
use anyhow::Error;
use onetagger_shared::Settings;
use onetagger_tagger::{Track, AudioFileInfo, TaggerConfig, SpotifyConfig, MultipleMatchesSort, 
    TrackMatch, MatchReason, StylesOptions, FileTaggedStatus, SupportedTag, TagSeparators, 
    AudioFileFormat, FrameName, Lyrics, LyricsLine, LyricsLinePart, MatchingUtils, OTDuration};
use pyembed::OxidizedPythonInterpreterConfig;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Get pyoxidizer config
pub fn pyoxidizer_config<'a>(dir: impl AsRef<Path>) -> Result<OxidizedPythonInterpreterConfig<'a>, Error> {
    mod pyoxidizer_config {
        include!("../pyembedded/config.rs");
    }
    let folder = Settings::get_folder()?;
    
    let mut config = pyoxidizer_config::default_python_config();
    config.interpreter_config.filesystem_encoding = Some("utf-8".to_string());
    config.tcl_library = None;
    config.packed_resources = vec![];
    config.filesystem_importer = true;
    config.oxidized_importer = false;
    config.interpreter_config.isolated = Some(true);
    config.interpreter_config.use_environment = Some(false);
    config.interpreter_config.home = Some(dunce::canonicalize(dir)?);
    config.interpreter_config.module_search_paths = Some(vec![
        dunce::canonicalize(folder.join("python_stdlib.zip"))?,
        dunce::canonicalize(folder.join("pip.pyz"))?,
    ]);
    config.interpreter_config.run_filename = None;
    config.interpreter_config.argv = Some(vec![]);

    // Set 1T python home env variable for subprocesses
    std::env::set_var("_1T_PY_HOME", config.interpreter_config.home.as_ref().map(|p| p.as_os_str()).unwrap());

    Ok(config)
}

/// Register modules
pub fn setup() {
    pyo3::append_to_inittab!(onetagger);
}

/// OneTagger Python Module
/// 
/// ## How to implement your own module:
/// 
/// ```
/// from onetagger import AudioFileInfo, TaggerConfig, TrackMatch, Track, new_track, match_tracks
/// 
/// # This function should search on your platform and return list of relevant / matched tracks
/// def match_track(info: AudioFileInfo, config: TaggerConfig) -> list[TrackMatch]:
///     # Here implement searching in your API
///     ...
///     # Now convert your tracks into 1T Tracks
///     track = new_track(
///         platform = "your_platform",
///         title = "Track title",
///         artists = ["Artist"]
///     )
///     # Match your tracks
///     matches = match_tracks(info, [track], config, True)
///     return matches
/// 
/// # This function will be called later on matched track, so here you can fetch additional metadata
/// def extend_track(track: Track, config: TaggerConfig) -> Track:
///     track.album_artists = ["Example"]
///     return track
/// ```
/// 
/// The Python `match_track` and `extend_track` are same as the Rust API, so you can get more info
/// by looking at the Rust OneTagger documentation (`cargo doc --open`) or source code.
/// 
#[pymodule]
fn onetagger(_py: Python<'_>, module: &PyModule) -> PyResult<()> {
    /// Log info
    #[pyfn(module)]
    fn info(s: &str) {
        info!("{s}");
    }
    /// Log warning
    #[pyfn(module)]
    fn warn(s: &str) {
        warn!("{s}");
    }
    /// Log error
    #[pyfn(module)]
    fn error(s: &str) {
        error!("{s}");
    }
    /// Log debug
    #[pyfn(module)]
    fn debug(s: &str) {
        debug!("{s}");
    }

    // Matching functions

    /// Clean title for searching
    #[pyfn(module)]
    #[pyo3(signature = (input))]
    fn clean_title(input: &str) -> String {
        MatchingUtils::clean_title(input)
    }
    
    /// Remove special characters
    #[pyfn(module)]
    #[pyo3(signature = (input))]
    fn remove_special(input: &str) -> String {
        MatchingUtils::remove_special(input)
    }

    /// Clean list of artists (artists: list[str])
    #[pyfn(module)]
    #[pyo3(signature = (artists))]
    fn clean_artists(artists: Vec<String>) -> Vec<String> {
        MatchingUtils::clean_artists(&artists)
    }

    /// Clean title for matching, removes special characters etc
    #[pyfn(module)]
    #[pyo3(signature = (input))]
    fn clean_title_matching(input: &str) -> String {
        MatchingUtils::clean_title_matching(input)
    }

    /// Clean artist (str) for searching on platforms
    #[pyfn(module)]
    #[pyo3(signature = (input))]
    fn clean_artist_searching(input: &str) -> String {
        MatchingUtils::clean_artist_searching(input)
    }

    /// Match atleast 1 artist
    #[pyfn(module)]
    #[pyo3(signature = (a, b, strictness))]
    fn match_artist(a: Vec<String>, b: Vec<String>, strictness: f64) -> bool {
        MatchingUtils::match_artist(&a, &b, strictness)
    }

    /// Default track matching algo (v2 with exact match fallabck)
    /// NOTE: Output is unsorted, sorted later in AT
    #[pyfn(module)]
    #[pyo3(signature = (info, tracks, config, match_artist = true))]
    fn match_tracks(info: &AudioFileInfo, tracks: Vec<Track>, config: &TaggerConfig, match_artist: bool) -> Vec<TrackMatch> {
        MatchingUtils::match_track(info, &tracks, config, match_artist)
    }

    /// Do exact matches on each step of track cleaning
    #[pyfn(module)]
    #[pyo3(signature = (info, tracks, config, match_artist = true))]
    fn match_tracks_exact_fallback(info: &AudioFileInfo, tracks: Vec<Track>, config: &TaggerConfig, match_artist: bool) -> Vec<Track> {
        MatchingUtils::match_track_exact_fallback(info, &tracks, config, match_artist)
    }

    /// Sort matched tracks by accuracy or release dates
    #[pyfn(module)]
    #[pyo3(signature = (tracks, config))]
    fn sort_tracks(mut tracks: Vec<TrackMatch>, config: &TaggerConfig) -> Vec<TrackMatch> {
        MatchingUtils::sort_tracks(&mut tracks, config);
        tracks
    }

    /// Match track duration
    #[pyfn(module)]
    #[pyo3(signature = (info, track, config))]
    pub fn match_duration(info: &AudioFileInfo, track: &Track, config: &TaggerConfig) -> bool {
        MatchingUtils::match_duration(info, track, config)
    }

    /// Parse duration from String
    #[pyfn(module)]
    #[pyo3(signature = (input))]
    pub fn parse_duration(input: &str) -> Result<OTDuration, Error> {
        MatchingUtils::parse_duration(input).map(|d| d.into())
    }

    module.add_class::<Track>()?;
    module.add_class::<AudioFileInfo>()?;
    module.add_class::<TaggerConfig>()?;
    module.add_class::<SpotifyConfig>()?;
    module.add_class::<MultipleMatchesSort>()?;
    module.add_class::<TrackMatch>()?;
    module.add_class::<MatchReason>()?;
    module.add_class::<StylesOptions>()?;
    module.add_class::<FileTaggedStatus>()?;
    module.add_class::<SupportedTag>()?;
    module.add_class::<TagSeparators>()?;
    module.add_class::<AudioFileFormat>()?;
    module.add_class::<FrameName>()?;
    module.add_class::<Lyrics>()?;
    module.add_class::<LyricsLine>()?;
    module.add_class::<LyricsLinePart>()?;

    module.add_function(wrap_pyfunction!(new_track, module)?)?;
    module.add_function(wrap_pyfunction!(new_track_match, module)?)?;

    Ok(())
}

/// Create new track, all fields are optional via kwargs  
/// WARNING: title and platform fields are required!  
/// Example:  
/// ```
/// import onetagger
/// onetagger.new_track(
///     platform = "my_platform",
///     title = "Title",
///     artists = ["Artist"],
/// )
/// ```
/// 
/// For list of all fields check documentation of Track object
/// 
#[pyfunction]
#[pyo3(signature = (**kwargs))]
fn new_track(kwargs: Option<&PyDict>) -> Result<Track, Error> {
    let kwargs = match kwargs {
        Some(k) => k,
        None => return Ok(Track::default())
    };

    // Set some fields to default value
    macro_rules! set_default {
        ($field: tt, $value: expr) => {
            if !kwargs.hasattr($field).unwrap_or(false) {
                kwargs.set_item($field, $value)?;
            }
        }
    }
    set_default!("album_artists", Vec::<()>::new());
    set_default!("genres", Vec::<()>::new());
    set_default!("styles", Vec::<()>::new());
    set_default!("other", Vec::<()>::new());
    set_default!("remixers", Vec::<()>::new());
    set_default!("duration", 0.0f64);
    set_default!("release_id", String::new());
    set_default!("url", String::new());

    Ok(pythonize::depythonize(kwargs)?)
}

/// Create new TrackMatch object from track
/// Required as return type in match_track
#[pyfunction]
#[pyo3(signature = (track, accuracy=1.0, reason=None))]
fn new_track_match(track: Track, accuracy: f64, reason: Option<MatchReason>) -> TrackMatch {
    TrackMatch { accuracy, track, reason: reason.unwrap_or(MatchReason::Fuzzy) }
}