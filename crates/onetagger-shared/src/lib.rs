#[macro_use] extern crate log;

use std::io::{Write, BufReader, BufWriter};
use std::error::Error;
use std::path::PathBuf;
use std::fs::File;
use backtrace::Backtrace;
use chrono::Local;
use directories::ProjectDirs;
use fern::Dispatch;
use fern::colors::{ColoredLevelConfig, Color};
use log::LevelFilter;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use crossterm::style::Stylize;

/// Get timestamp macro
#[macro_export]
macro_rules! timestamp {
    () => {
        std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_millis()
    };
}

/// Current onetagger version
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
/// Hash of commit used to build this version
pub const COMMIT: &'static str = env!("COMMIT");

/// One-off error type
#[derive(Debug, Clone)]
pub struct OTError {
    message: String
}
impl OTError {
    pub fn new(msg: &str) -> OTError {
        OTError {
            message: msg.to_owned()
        }
    }
}

impl Error for OTError {}
impl std::fmt::Display for OTError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Setup onetagger logging and panic hooks
pub fn setup() {
    // Fern logger setup
    let colors = ColoredLevelConfig::new()
        .trace(Color::White)
        .debug(Color::Blue)
        .info(Color::Green)
        .warn(Color::Yellow)
        .error(Color::Red);
    let level = if cfg!(debug_assertions) { LevelFilter::Debug } else { LevelFilter::Info };

    Dispatch::new()
        .level(LevelFilter::Warn)
        .level_for("onetagger_shared", level)
        .level_for("onetagger_autotag", level)
        .level_for("onetagger_platforms", level)
        .level_for("onetagger_player", level)
        .level_for("onetagger_playlist", level)
        .level_for("onetagger_renamer", level)
        .level_for("onetagger_tag", level)
        .level_for("onetagger_cli", level)
        .level_for("onetagger_tagger", level)
        .level_for("onetagger_ui", level)
        .level_for("onetagger", level)
        // Custom platforms
        .level_for("onetagger_custom_platform", level)
        // Colored
        .chain(
            Dispatch::new()
                .format(move |out, message, record| {
                    out.finish(format_args!(
                        "{} [{}] {}: {}",
                        Local::now().format("%Y-%m-%d %H:%M:%S").to_string().bold(),
                        colors.color(record.level()),
                        record.target().bold(),
                        message
                    ))
                })
                .chain(std::io::stdout())
        )
        // Log file
        .chain(
            Dispatch::new()
                .format(move |out, message, record| {
                    out.finish(format_args!(
                        "{} [{}] {}: {}",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        record.level(),
                        record.target(),
                        message
                    ))
                })
                .chain(fern::log_file(Settings::get_folder().unwrap().join("onetagger.log")).unwrap())
        )
        .apply()
        .unwrap();

    // Panic hook
    std::panic::set_hook(Box::new(|p| {
        let bt = Backtrace::new();
        error!("PANIC: {}", p);
        if let Some(location) = p.location() {
            error!("LOCATION: File: {}, Line: {}", location.file(), location.line());
        }
        // Show backtrace
        if std::env::var_os("RUST_BACKTRACE").is_some() {
            debug!("BACKTRACE:\n{:?}", bt);
        }
    }));
}

/// Onetagger settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub ui: Value,
    pub version: Option<i32>
}

impl Settings {
    // Create settings from UI json
    pub fn from_ui(ui: &Value) -> Settings {
        Settings {
            ui: ui.to_owned(),
            version: Some(2)
        }
    }

    /// Load settings from file
    pub fn load() -> Result<Settings, Box<dyn Error>> {
        let path = Settings::get_path()?;
        let settings: Settings = serde_json::from_reader(BufReader::new(File::open(&path)?))?;

        // v1.0 are not compatible with 1.1, create backup
        if settings.version.unwrap_or(1) == 1 {
            let new_path = format!("{}-1.0.bak", &path);
            std::fs::copy(&path, &new_path)?;
            info!("Backup of settings created: {}", new_path);
            std::fs::remove_file(&path)?;
            return Settings::load();
        }

        Ok(settings)
    }
    
    /// Save settings to file
    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let path = Settings::get_path()?;
        let mut file = BufWriter::new(File::create(path)?);
        file.write_all(serde_json::to_string_pretty(self)?.as_bytes())?;
        Ok(())
    }

    /// Get app data folder
    pub fn get_folder() -> Result<PathBuf, Box<dyn Error>> {
        // Android data dir override
        #[cfg(target_os = "android")]
        if let Ok(dir) = std::env::var("__ANDROID_DATA_DIR") {
            return Ok(PathBuf::from(dir));
        }

        let root = ProjectDirs::from("com", "OneTagger", "OneTagger").ok_or("Error getting dir!")?;
        if !root.preference_dir().exists() {
            std::fs::create_dir_all(root.preference_dir())?;
        }
        Ok(root.preference_dir().to_owned())
    }

    /// Get settings path
    fn get_path() -> Result<String, Box<dyn Error>> {
        let path = Settings::get_folder()?.join("settings.json");
        Ok(path.to_str().ok_or("Error converting path to string!")?.to_string())
    }
}


/// Capitalize every word
/// https://stackoverflow.com/questions/38406793/why-is-capitalizing-the-first-letter-of-a-string-so-convoluted-in-rust/38406885#38406885
pub fn capitalize(input: &str) -> String {
    input.split(" ").map(|w| {
        let mut c = w.trim().chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str()
        }
    }).collect::<Vec<_>>().join(" ")
}