use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;
use log::{Metadata, Log, Level, Record, LevelFilter};
use serde::Serialize;
use crossbeam_channel::{unbounded, Sender, Receiver};

use onetagger_autotag::{Tagger, TaggingStatusWrap, AudioFileInfoImpl, TaggerConfigExt, AutotaggerPlatforms};
use onetagger_platforms::spotify::Spotify;
use onetagger_shared::{COMMIT, VERSION};
use onetagger_tagger::{TaggerConfig, AudioFileInfo};

lazy_static::lazy_static! {
    static ref STATUS_CHANNEL: (Sender<TaggingStatusWrap>, Receiver<TaggingStatusWrap>) = unbounded();
    static ref LOG: Arc<Mutex<Vec<LogMessage>>> = Arc::new(Mutex::new(vec![]));
    static ref IS_DONE: AtomicBool = AtomicBool::new(false);
}

static LOGGER: CustomLogger = CustomLogger;

/// Custom logger to forward messages to Flutter
struct CustomLogger;

impl Log for CustomLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if !record.target().contains("onetagger") {
            return;
        }

        // Send to logcat
        android_logger::log(record);

        let message = LogMessage {
            level: record.level().into(),
            message: record.args().to_string(),
            module: record.target().to_string(),
            time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        };
        LOG.lock().unwrap().push(message);
    }

    fn flush(&self) {
        // do nothing
    }
}

#[derive(Debug, Clone)]
pub struct LogMessage {
    pub level: LogLevel,
    pub message: String,
    pub module: String,
    pub time: u64,
}

/// Wrap log::Level so it gets generated for Flutter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum LogLevel {
    Trace, Debug, Warn, Info, Error
}

impl From<Level> for LogLevel {
    fn from(level: Level) -> Self {
        match level {
            Level::Error => LogLevel::Error,
            Level::Warn => LogLevel::Warn,
            Level::Info => LogLevel::Info,
            Level::Debug => LogLevel::Debug,
            Level::Trace => LogLevel::Trace,
        }
    }
}


/// Initialize the library
pub fn init(data_dir: String) {
    // Set the data dir for Android
    std::env::set_var("__ANDROID_DATA_DIR", data_dir);

    // Ignore the error because logger might be already set
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Debug))
        .ok();
}

/// Get logs
pub fn logs() -> Vec<LogMessage> {
    LOG.lock().unwrap().clone()
}

/// Start autotagger
pub fn start_at(path: String, config_json: String) -> Result<()> {
    IS_DONE.store(false, Ordering::SeqCst);
    let config: TaggerConfig = serde_json::from_str(&config_json)?;
    let files = AudioFileInfo::get_file_list(&path, config.include_subfolders);
    std::thread::spawn(move || {
        let rx = Tagger::tag_files(&config, files, Arc::new(Mutex::new(None)));
        for status in rx {
            STATUS_CHANNEL.0.send(status).ok();
        }
        IS_DONE.store(true, Ordering::SeqCst);
    });
    Ok(())
}

/// Get latest statuses
pub fn get_statuses() -> Vec<String> {
    STATUS_CHANNEL.1
        .try_iter()
        .map(|s| serde_json::to_string(&s).unwrap())
        .collect()
}

/// Is tagging done
pub fn is_done() -> bool {
    IS_DONE.load(Ordering::SeqCst)
}

/// Get default custom platform options
pub fn custom_default() -> String {
    serde_json::to_string(&TaggerConfig::custom_default().custom).unwrap()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AutoTaggerPlatforms(&'static AutotaggerPlatforms);

/// Get platforms list
pub fn platforms() -> String {
    serde_json::to_string(&AutoTaggerPlatforms(&onetagger_autotag::AUTOTAGGER_PLATFORMS)).unwrap()
}

/// Authorize spotify, None if authorized, otherwise string with auth url
pub fn authorize_spotify(client_id: String, client_secret: String) -> Result<Option<String>> {
    match Spotify::try_cached_token(&client_id, &client_secret) {
        // Generate URL
        None => {
            let (auth_url, client) = Spotify::generate_auth_url(&client_id, &client_secret).map_err(|e|
                anyhow::anyhow!("Failed generating Spotify auth URL {e:?}")
            )?;
            std::thread::spawn(|| {
                info!("Spotify auth server successful: {}", Spotify::auth_server(client, false).is_ok());
            });
            Ok(Some(auth_url))
        }
        Some(_) => Ok(None)
    }
}

#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version: String,
    pub commit: String
}

/// Get the current version and commit
pub fn version() -> VersionInfo {
    VersionInfo { version: VERSION.to_string(), commit: COMMIT.to_string() }
}