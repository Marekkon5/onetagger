#[macro_use] extern crate log;
#[macro_use] extern crate anyhow;

use std::path::{Path, PathBuf};
use std::io::BufReader;
use std::fs::File;
use std::process::{Command, Stdio};
use anyhow::Error;
use onetagger_shared::Settings;
use onetagger_tagger::{PlatformInfo, AutotaggerSourceBuilder, TaggerConfig, AutotaggerSource, AudioFileInfo, Track, TrackMatch, ConfigCallbackResponse};
use pyembed::MainPythonInterpreter;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use subprocess::{SubprocessWrap, PythonResponse, PythonRequest};

mod module;
mod subprocess;

/// Re-Export python process
pub use subprocess::python_process;
use tempdir::TempDir;

/// Python Standard Library ZIP
const PYTHON_STDLIB: &[u8] = include_bytes!("../pyembedded/stdlib.zip");
const PYTHON_VERSION: &'static str = include_str!("../pyembedded/VERSION");
const PIP_PYZ: &[u8] = include_bytes!("../pyembedded/pip.pyz");

/// Windows specific libraries that have to be unzipped
#[cfg(target_os = "windows")]
const PYTHON_LIBS: &[u8] = include_bytes!("../pyembedded/lib.zip");

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

    // Unzip Windows libs
    #[cfg(target_os = "windows")]
    {
        use zip::read::ZipArchive;
        use std::io::Cursor;
        let mut zip = ZipArchive::new(Cursor::new(PYTHON_LIBS))?;
        std::fs::create_dir_all(dir.join("lib"))?;
        zip.extract(dir.join("lib"))?;
    }

    // Setup pyo3
    module::setup();

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

    // Check version and dependencies
    let installed_version = match std::fs::read_to_string(path.as_ref().join(".version")) {
        Ok(version) => version,
        Err(_) => String::new(),
    };
    if installed_version != info.info.version {
        // Install pip dependencies
        info!("Running pip install {:?}", info.requirements);
        let mut wrap = spawn_python_child()?;
        wrap.send(&PythonRequest::PipInstall { 
            path: path.as_ref().join(".python"), 
            requirements: info.requirements.clone()
        })?;
        wrap.recv()?;
    }

    // Save installed version
    std::fs::write(path.as_ref().join(".version"), &info.info.version)?;

    Ok(PythonPlatformBuilder { info, path: dunce::canonicalize(path)? })
}

/// Spawn python child process
fn spawn_python_child() -> Result<SubprocessWrap, Error> {
    let child = Command::new(std::env::current_exe()?)
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .stderr(Stdio::inherit())
        .arg("--python-subprocess")
        .spawn()?;
    let wrap = SubprocessWrap::new(child);
    Ok(wrap)
}

/// Generate python module docs
pub fn generate_docs() -> Result<PathBuf, Error> {
    let temp = TempDir::new("1tdocs")?;

    // Pip
    debug!("Pip install");
    let mut wrap = spawn_python_child()?;
    wrap.send(&PythonRequest::PipInstall { 
        path: temp.path().to_owned(), 
        requirements: vec!["pdoc3".into(), "requests".into(), "beautifulsoup4".into(), "lxml".into()]
    })?;
    wrap.recv()?;
    drop(wrap);

    // Generate
    debug!("Generate docs");
    let mut wrap = spawn_python_child()?;
    let path = Settings::get_folder()?.join("onetagger-python.html");
    wrap.send(&PythonRequest::GenerateDocs { python_path: temp.path().to_owned(), output: path.to_owned() })?;
    wrap.recv()?;
    Ok(path)
}

/// Get python interpreter for platform
/// WARNING: Exits the process, can be only called once
pub fn python_interpreter(platform: &str) -> Result<(), Error> {
    module::setup();
    let folder = Settings::get_folder()?;
    let mut config = module::pyoxidizer_config(folder.join("platforms").join(platform).join(".python"))?;
    config.interpreter_config.module_search_paths.as_mut().unwrap().push(folder.join("pip.pyz"));
    let interpreter = MainPythonInterpreter::new(config)?;
    // Enable modern shell
    interpreter.with_gil(|py| { py.import("readline").ok(); });
    std::process::exit(interpreter.run());
}

/// Check the args, if it has python (non-1T related ones, start Python)
pub fn python_hook() {
    // 1T Python subprocess
    let args = std::env::args().collect::<Vec<_>>();
    if args.iter().any(|a| a == "--python-subprocess") {
        python_process();
        std::process::exit(0);
    }

    // Python subprocess spawned
    if std::env::var("_1T_PY_HOME").is_ok() {
        let mut config = module::pyoxidizer_config(std::env::var("_1T_PY_HOME").unwrap()).unwrap();
        config.argv = Some(args.into_iter().map(|v| v.into()).collect());
        std::process::exit(MainPythonInterpreter::new(config).unwrap().py_runmain());
    }
}

pub struct PythonPlatformBuilder {
    pub info: PythonPlatformInfo,
    path: PathBuf
}

impl PythonPlatformBuilder {
    /// Initialize subprocess
    fn init(&self) -> Result<SubprocessWrap, Error> {
        // Load code
        let code = std::fs::read_to_string(self.path.join(&self.info.main))?;
        // Spawn subprocess
        let mut wrap = spawn_python_child()?;
        wrap.send(&PythonRequest::Init { path: self.path.join(".python"), code })?;
        // Receive init ok or error
        wrap.recv()?;
        Ok(wrap)
    }
}

impl AutotaggerSourceBuilder for PythonPlatformBuilder {
    fn new() -> Self where Self: Sized {
        panic!("Not used / Python platforms should be loaded externally");
    }

    fn get_source(&mut self, _config: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Error> {
        Ok(Box::new(PythonPlatform { subprocess: self.init()? }))
    }

    fn info(&self) -> PlatformInfo {
        self.info.info.clone()
    }

    fn config_callback(&mut self, name: &str, config: Value) -> ConfigCallbackResponse {
        // Call subprocess
        let f = || -> Result<ConfigCallbackResponse, Error> {
            let mut subprocess = self.init()?;
            subprocess.send(&PythonRequest::ConfigCallback { name: name.to_string(), config })?;
            if let PythonResponse::ConfigCallback { result } = subprocess.recv()? {
                return Ok(result);
            }
            return Err(anyhow!("Invalid subprocess response"));
        };
       
        match f() {
            Ok(r) => r,
            Err(e) => ConfigCallbackResponse::Error { error: e.to_string() },
        }
    }
    
}

pub struct PythonPlatform {
    subprocess: SubprocessWrap
}

impl AutotaggerSource for PythonPlatform {
    fn match_track(&mut self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<Vec<TrackMatch>, Error> {
        self.subprocess.send(&PythonRequest::MatchTrack { info: info.clone(), config: config.clone() })?;
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

impl Drop for PythonPlatform {
    fn drop(&mut self) {
        self.subprocess.send(&PythonRequest::Exit).ok();
        self.subprocess.recv().ok();
    }
}