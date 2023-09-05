#[macro_use] extern crate log;
#[macro_use] extern crate anyhow;

use std::path::{Path, PathBuf};
use std::io::BufReader;
use std::fs::File;
use std::process::{Command, Stdio};
use anyhow::Error;
use onetagger_shared::Settings;
use onetagger_tagger::{PlatformInfo, AutotaggerSourceBuilder, TaggerConfig, AutotaggerSource, AudioFileInfo, Track, TrackMatch};
use pyembed::{MainPythonInterpreter, OxidizedPythonInterpreterConfig};
use serde::{Serialize, Deserialize};
use subprocess::{SubprocessWrap, PythonResponse, PythonRequest};

mod module;
mod subprocess;

/// Re-Export python process
pub use subprocess::python_process;

/// Python Standard Library ZIP
const PYTHON_STDLIB: &[u8] = include_bytes!("../pyembedded/stdlib.zip");
const PYTHON_VERSION: &'static str = include_str!("../pyembedded/VERSION");
const PIP_PYZ: &[u8] = include_bytes!("../pyembedded/pip.pyz");

/// Setup Python
pub fn setup() -> Result<(), Error> {
    let dir = Settings::get_folder()?;

    // Check python version
    let version_file = dir.join("PYTHON_VERSION");
    let version = match version_file.exists() {
        true => {
            std::fs::read_to_string(&version_file)?
        },
        false => {
            std::fs::write(version_file, PYTHON_VERSION)?;
            String::new()
        }
    };
    // Write stdlib & pip
    if version != PYTHON_VERSION {
        info!("Writing python stdlib for {}", PYTHON_VERSION);
        std::fs::write(dir.join("python_stdlib.zip"), PYTHON_STDLIB)?;
        std::fs::write(dir.join("pip.pyz"), PIP_PYZ)?;
    }

    Ok(())
}

/// Get standard library path
fn stdlib_path() -> Result<PathBuf, Error> {
    Ok(dunce::canonicalize(Settings::get_folder()?.join("python_stdlib.zip"))?)
}


/// pip install packages
fn pip_install(mut config: OxidizedPythonInterpreterConfig, requirements: &[String]) -> Result<(), Error> {
    let pip_path = dunce::canonicalize(Settings::get_folder()?.join("pip.pyz"))?;
    // Params
    config.interpreter_config.run_filename = Some(pip_path.clone());
    config.interpreter_config.argv = Some(vec![
        "pip.pyz".into(),
        "pip.pyz".into(),
        "install".into(),
        "pip".into(),
        "setuptools".into(),
        "wheel".into()
    ]);
    config.interpreter_config.argv.as_mut().unwrap().extend(requirements.iter().map(|r| r.into()));

    // Run
    let interpreter = MainPythonInterpreter::new(config)?;
    let r = interpreter.py_runmain();
    if r != 0 {
        return Err(anyhow!("pip install failed with code: {r}"));
    }
    Ok(())
}

/// Platform info for Python
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonPlatformInfo {
    /// Python requirements
    pub requirements: Vec<String>,

    /// Main python file
    pub main: String,

    #[serde(flatten)]
    pub info: PlatformInfo,
}

/// Load python platform from path
pub fn load_python_platform(path: impl AsRef<Path>) -> Result<PythonPlatformBuilder, Error> {
    let info: PythonPlatformInfo = serde_json::from_reader(BufReader::new(File::open(path.as_ref().join("info.json"))?))?;
    // Create python dir
    std::fs::create_dir_all(path.as_ref().join(".python"))?;
    Ok(PythonPlatformBuilder { info, path: dunce::canonicalize(path)? })
}

pub struct PythonPlatformBuilder {
    pub info: PythonPlatformInfo,
    path: PathBuf
}

impl AutotaggerSourceBuilder for PythonPlatformBuilder {
    fn new() -> Self where Self: Sized {
        panic!("Not used / Python platforms should be loaded externally");
    }

    fn get_source(&mut self, _config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Error> {
        // Install packages
        let config = module::pyoxidizer_config(&self.path.join(".python"), stdlib_path()?)?;
        info!("Running pip install {:?}", self.info.requirements);
        pip_install(config.clone(), &self.info.requirements)?;

        // Load code
        let code = std::fs::read_to_string(self.path.join(&self.info.main))?;

        // Spawn subprocess
        let child = Command::new(std::env::current_exe()?)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::inherit())
            .arg("--python-subprocess")
            .spawn()?;
        let mut wrap = SubprocessWrap::new(child);
        debug!("Pre init");
        wrap.send(&PythonRequest::Init { path: self.path.join(".python"), code })?;
        debug!("Post init");
        // Receive init ok or error
        wrap.recv()?;
        debug!("init ok");

        Ok(Box::new(PythonPlatform { subprocess: wrap }))
    }

    fn info(&self) -> PlatformInfo {
        // Cap thread count
        let mut info = self.info.info.clone();
        info.max_threads = 1;
        info
    }
}

pub struct PythonPlatform {
    subprocess: SubprocessWrap
}

impl AutotaggerSource for PythonPlatform {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Vec<TrackMatch>, Error> {
        debug!("Pre match");
        self.subprocess.send(&PythonRequest::MatchTrack { info: info.clone(), config: config.clone() })?;
        debug!("Post match");
        if let PythonResponse::MatchTrack { result } = self.subprocess.recv()? {
            return result.map_err(|e| anyhow!("{e}"));
        }
        unreachable!()
    }

    fn extend_track(&mut self, track: &mut Track, config: &TaggerConfig) -> Result<(), Error> {
        self.subprocess.send(&PythonRequest::ExtendTrack { track: track.to_owned(), config: config.clone() })?;
        if let PythonResponse::ExtendTrack { result } = self.subprocess.recv()? {
            *track = result.map_err(|e| anyhow!("{e}"))?;
        }
        Ok(())
    }
}