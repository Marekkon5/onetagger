use std::path::Path;
use anyhow::Error;
use onetagger_tagger::{Track, AudioFileInfo, TaggerConfig, SpotifyConfig, MultipleMatchesSort, 
    TrackMatch, MatchReason, StylesOptions, FileTaggedStatus, SupportedTag, TagSeparators, 
    AudioFileFormat, FrameName, Lyrics, LyricsLine, LyricsLinePart};
use pyembed::OxidizedPythonInterpreterConfig;
use pyo3::prelude::*;

/// Get pyoxidizer config
pub fn pyoxidizer_config<'a>(dir: impl AsRef<Path>, stdlib: impl AsRef<Path>) -> Result<OxidizedPythonInterpreterConfig<'a>, Error> {
    mod pyoxidizer_config {
        include!("../pyembedded/config.rs");
    }
    let mut config = pyoxidizer_config::default_python_config();

    config.interpreter_config.filesystem_encoding = Some("utf-8".to_string());
    config.tcl_library = None;
    config.packed_resources = vec![];
    config.filesystem_importer = true;
    config.oxidized_importer = false;
    config.interpreter_config.isolated = Some(true);
    config.interpreter_config.use_environment = Some(false);
    config.interpreter_config.home = Some(dunce::canonicalize(dir)?);
    config.interpreter_config.module_search_paths = Some(vec![dunce::canonicalize(stdlib)?]);
    config.interpreter_config.run_filename = None;
    config.interpreter_config.argv = Some(vec![]);

    Ok(config)
}

/// Register modules
pub fn setup() {
    pyo3::append_to_inittab!(onetagger);
}

/// 1t Python Module
#[pymodule]
fn onetagger(_py: Python<'_>, module: &PyModule) -> PyResult<()> {
    #[pyfn(module)]
    fn info(s: &str) {
        info!("{s}");
    }
    #[pyfn(module)]
    fn warn(s: &str) {
        warn!("{s}");
    }
    #[pyfn(module)]
    fn error(s: &str) {
        error!("{s}");
    }
    #[pyfn(module)]
    fn debug(s: &str) {
        debug!("{s}");
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

    Ok(())
}

