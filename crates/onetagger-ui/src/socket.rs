use std::collections::HashMap;
use anyhow::Error;
use axum::extract::ws::{WebSocket, Message};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use onetagger_renamer::ac::Autocomplete;
use onetagger_renamer::docs::FullDocs;
use onetagger_renamer::{Renamer, TemplateParser, RenamerConfig};
use serde_json::{Value, json};
use serde::{Serialize, Deserialize};
use dunce::canonicalize;
use onetagger_tag::{TagChanges, TagSeparators, Tag, Field};
use onetagger_tagger::{TaggerConfig, AudioFileInfo, TrackMatch};
use onetagger_autotag::{RetryPlan, TaggerFinishedData, Tagger, AudioFileInfoImpl, TaggerConfigExt, AUTOTAGGER_PLATFORMS};
use onetagger_autotag::audiofeatures::{AudioFeaturesConfig, AudioFeatures};
use onetagger_platforms::spotify::Spotify;
use onetagger_player::{AudioSources, AudioPlayer};
use onetagger_shared::{Settings, COMMIT};
use onetagger_playlist::UIPlaylist;

use crate::StartContext;
use crate::quicktag::{QuickTagFile, QuickTagData};
use crate::tageditor::TagEditor;
use crate::browser::{FileBrowser, FolderBrowser};
use crate::messages::{localized, request_error, UiMessage, UserError};

// STOP_TAGGING is shared by the engines. A second browser tab must not reset
// that flag while another run is stopping or finishing its result files.
static TAGGING_ACTIVE: AtomicBool = AtomicBool::new(false);
struct TaggingLease<'a>(&'a AtomicBool);
impl<'a> TaggingLease<'a> {
    fn acquire(active: &'a AtomicBool) -> Result<Self, Error> {
        active.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| UserError::TaggingAlreadyActive)?;
        Ok(Self(active))
    }
}
impl Drop for TaggingLease<'_> {
    fn drop(&mut self) { self.0.store(false, Ordering::SeqCst); }
}


/// Keep only the current run's live summary and its most recent continuation.
#[derive(Debug)]
pub(crate) struct RunStatus {
    id: u64,
    kind: &'static str,
    total: usize,
    progress: f64,
    completion: Option<Value>,
    retry: Option<Arc<RetryPlan>>,
}

impl RunStatus {
    fn snapshot(&self) -> Value {
        let mut snapshot = json!({ "action": "taggingState", "runId": self.id, "type": self.kind,
            "files": self.total, "progress": self.progress, "completion": self.completion });
        if let Some(retry) = &self.retry {
            snapshot["retryCount"] = json!(retry.file_count());
            snapshot["retryAfterMs"] = json!(retry.remaining().as_millis().min(u64::MAX as u128) as u64);
        }
        snapshot
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
enum Action {
    Init,
    Exit,
    SaveSettings { settings: Value },
    LoadSettings,
    DefaultCustomPlatformSettings,
    Browse { path: Option<String>, context: Option<String> },
    Browser { url: String },
    OpenSettingsFolder,
    OpenFolder { path: PathBuf },
    OpenFile { path: PathBuf },
    DeleteFiles { paths: Vec<String> },
    GetLog,
    GeneratePlaylist { paths: Vec<String> },

    LoadPlatforms,
    StartTagging { config: TaggerConfigs, playlist: Option<UIPlaylist>,
        #[serde(default)] report: bool, selection: Option<onetagger_autotag::report::Selection>,
        #[serde(default, rename = "rotateProviders")] rotate: bool },
    #[serde(rename_all = "camelCase")]
    RetryTagging { run_id: u64 },
    TaggingState,
    StopTagging,
    ConfigCallback { config: Value, platform: String, id: String },
    RepoManifest,
    #[serde(rename_all = "camelCase")]
    InstallPlatform { id: String, version: String, is_native: bool },

    #[serde(rename_all = "camelCase")]
    Waveform { path: PathBuf, request_id: Option<String> },
    #[serde(rename_all = "camelCase")]
    PlayerLoad { path: PathBuf, request_id: Option<String>, #[serde(default)] client_side: bool },
    PlayerState,
    PlayerPlay, 
    PlayerPause,
    PlayerSeek { pos: u64 },
    PlayerVolume { volume: f32 },
    PlayerStop,

    #[serde(rename_all = "camelCase")]
    QuickTagLoad { path: Option<String>, playlist: Option<UIPlaylist>, recursive: Option<bool>,
        separators: TagSeparators, limit: Option<bool>, request_id: Option<String>, #[serde(default)] progressive: bool },
    #[serde(rename_all = "camelCase")]
    QuickTagSave { changes: TagChanges, request_id: Option<String> },
    QuickTagFolder { path: Option<String>, subdir: Option<String> },

    #[serde(rename_all = "camelCase")]
    SpotifyAuthorize { client_id: String, client_secret: String },
    SpotifyAuthorized,

    TagEditorFolder { path: Option<String>, subdir: Option<String>, recursive: Option<bool>  },
    TagEditorLoad { path: PathBuf },
    #[serde(rename_all = "camelCase")]
    TagEditorSave { changes: TagChanges, request_id: Option<String> },

    RenamerSyntaxHighlight { template: String },
    RenamerAutocomplete { template: String },
    RenamerPreview { config: RenamerConfig },
    RenamerStart { config: RenamerConfig },

    FolderBrowser { path: PathBuf, child: String, base: bool },

    ManualTag { config: TaggerConfig, path: PathBuf },
    ManualTagApply { matches: Vec<TrackMatch>, path: PathBuf, config: TaggerConfig },
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
enum TaggerConfigs {
    AutoTagger(TaggerConfig), 
    AudioFeatures(AudioFeaturesConfig)
}

impl TaggerConfigs {
    // Print to log for later easier debug
    pub fn debug_print(&self) {
        match self {
            TaggerConfigs::AutoTagger(c) => {
                let mut c = c.clone();
                // don't leak secrets
                c.custom = HashMap::new().into();
                c.spotify = None;
                info!("AutoTagger config: {:?}", c);
            },
            TaggerConfigs::AudioFeatures(c) => {
                info!("AudioFeatures Config: {:?}", c);
            }
        }
    }
}

// Shared variables in socket
struct SocketContext {
    player: Option<AudioPlayer>,
    volume: f32,
    spotify: Option<Spotify>,
    start_context: StartContext
} 

impl SocketContext {
    pub fn new(start_context: StartContext) -> SocketContext {
        SocketContext {
            player: None,
            volume: 0.5,
            spotify: None,
            start_context
        }
    }
}


/// Reply to init call
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InitData {
    action: &'static str,
    version: &'static str,
    os: &'static str,
    arch: &'static str,
    custom_platform_compat: i32,
    start_context: StartContext,
    renamer_docs: FullDocs,
    commit: &'static str,
    work_dir: PathBuf,
    data_dir: PathBuf
}

impl InitData {
    /// Create new default instance
    pub fn new(start_context: StartContext) -> InitData {
        InitData {
            action: "init",
            version: onetagger_shared::VERSION,
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            custom_platform_compat: onetagger_tagger::custom::CUSTOM_PLATFORM_COMPATIBILITY,
            start_context,
            renamer_docs: FullDocs::get().html(),
            commit: COMMIT,
            work_dir: std::env::current_dir().unwrap_or_default(),
            data_dir: Settings::get_folder().unwrap_or_default(),
        }
    }
}

pub(crate) async fn handle_ws_connection(mut websocket: WebSocket, context: StartContext) -> Result<(), Error> {
    let mut context = SocketContext::new(context);
    
    while let Some(message) = websocket.recv().await {
        match message? {
            Message::Text(text) => {
                if let Err(error) = handle_message(&text, &mut websocket, &mut context).await {
                    error!("WebSocket command failed: {error:#}");
                    send_socket(&mut websocket, request_error(&error)).await?;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    Ok(())
}

/// Serialize and send to socket with warning intercept
async fn send_socket<D: Serialize>(ws: &mut WebSocket, json: D) -> Result<(), Error> {
    match send_socket_inner(ws, json).await {
        Ok(_) => Ok(()),
        Err(e) => {
            warn!("Failed sending to socket: {e}");
            Err(e)
        },
    }
}

/// Serialize and send to socket
async fn send_socket_inner<D: Serialize>(ws: &mut WebSocket, json: D) -> Result<(), Error> {
    ws.send(Message::from(serde_json::to_string(&json)?)).await?;
    Ok(())
}

// Keep save acknowledgements correlated with the file and request, including
// failures. A legacy client still receives the generic error event it understands.
fn tag_editor_save_response(changes: &TagChanges, request_id: Option<String>) -> Value {
    let result = changes.commit();
    crate::library::invalidate(&changes.path);
    match result {
        Ok(_) => json!({ "action": "tagEditorSave", "status": "ok",
            "requestId": request_id, "path": changes.path }),
        Err(error) => {
            warn!("Tag editor save failed: {error:#}");
            localized(json!({ "action": if request_id.is_some() { "tagEditorSave" } else { "error" },
                "status": "error", "requestId": request_id, "path": changes.path,
                "message": format!("{error:#}") }),
                UiMessage::TagEditorSaveFailed { detail: format!("{error:#}") })
        }
    }
}

// Disk/decoder work must not occupy Tokio's network workers. The permit also
// bounds simultaneous work submitted by multiple browser tabs and job sockets.
static DISK_JOBS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);
pub(crate) async fn blocking<T, F>(work: F) -> Result<T, Error>
where T: Send + 'static, F: FnOnce() -> Result<T, Error> + Send + 'static {
    let permit = DISK_JOBS.acquire().await?;
    tokio::task::spawn_blocking(move || { let _permit = permit; work() }).await?
}

fn quick_tag_save_response(changes: &TagChanges, request_id: Option<String>) -> Value {
    let result = changes.commit().and_then(|tag| QuickTagFile::from_tag(&changes.path, &tag));
    // Invalidate even after a partial failure: a write may have reached disk.
    crate::library::invalidate(&changes.path);
    match result {
        Ok(file) => json!({"action": "quickTagSaved", "status": "ok", "requestId": request_id,
            "path": changes.path, "file": file}),
        Err(error) => {
            let detail = format!("{error:#}");
            warn!("QuickTag save failed for {}: {detail}", changes.path.display());
            localized(json!({"action": if request_id.is_some() { "quickTagSaved" } else { "error" },
                "status": "error", "requestId": request_id, "path": changes.path, "message": detail}),
                UiMessage::RequestFailed { detail })
        },
    }
}

async fn handle_message(text: &str, websocket: &mut WebSocket, context: &mut SocketContext) -> Result<(), Error> {
    // Parse JSON
    let action: Action = serde_json::from_str(text)?;
    match action {
        // Get initial info
        Action::Init => {
            info!("UI session initialized");
            send_socket(websocket, InitData::new(context.start_context.clone())).await.ok();
        },
        Action::Exit => std::process::exit(0),
        Action::SaveSettings { settings } => blocking(move || Settings::from_ui(&settings).save()).await?,
        Action::LoadSettings => match blocking(Settings::load).await {
            Ok(settings) => {
                send_socket(websocket, json!({
                    "action": "loadSettings",
                    "settings": settings.ui
                })).await.ok();
            }
            // Ignore settings if they don't exist (might be initial load)
            Err(e) => error!("Failed loading settings, using defaults. {}", e)
        },
        // Get the default custom platform options
        Action::DefaultCustomPlatformSettings => {
            send_socket(websocket, json!({
                "action": "defaultCustomPlatformSettings",
                "custom": TaggerConfig::custom_default().custom
            })).await.ok();
        }
        // Browse for folder
        Action::Browse { path, context } => {
            let selected = blocking(move || {
                let mut initial = path.unwrap_or_else(|| ".".into());
                if initial.is_empty() || !Path::new(&initial).exists() { initial = ".".into(); }
                Ok(tinyfiledialogs::select_folder_dialog("Select path", &initial))
            }).await?;
            if let Some(path) = selected { send_socket(websocket, json!({"action": "browse", "path": path, "context": context})).await?; }
        },
        // Get 1t Log
        Action::GetLog => {
            log::logger().flush();
            let log = blocking(|| Ok(std::fs::read_to_string(Settings::get_folder()?.join("onetagger.log"))?)).await?;
            send_socket(websocket, json!({
                "action": "log",
                "log": log
            })).await.ok();
        },
        // Open URL in external browser
        Action::Browser { url } => { webbrowser::open(&url)?; },
        Action::OpenSettingsFolder => opener::open(Settings::get_folder()?.to_str().unwrap())?,
        Action::OpenFolder { path } => { opener::open(&path).ok(); },
        Action::OpenFile { path } => { opener::open(&path).ok(); },
        Action::DeleteFiles { paths } => { blocking(move || Ok(trash::delete_all(&paths)?)).await?; },

        Action::GeneratePlaylist { paths } => {
            let playlist = onetagger_playlist::create_m3u_playlist(&paths.into_iter().map(|i| i.into()).collect::<Vec<_>>());
            if let Some(path) = tinyfiledialogs::save_file_dialog_with_filter(
                "Save playlist", 
                &std::env::current_dir()?.to_string_lossy().to_string(), 
                &["m3u", "m3u8"], 
                "Save playlist"
            ) {
                std::fs::write(&path, playlist)?;
                send_socket(websocket, localized(json!({
                    "action": "notify",
                    "message": format!("Playlist saved to: {path}")
                }), UiMessage::PlaylistSaved { path })).await.ok();
            }
        }

        Action::LoadPlatforms => {
            let platforms = tokio::task::spawn_blocking(|| {
                let mut platforms = AUTOTAGGER_PLATFORMS.lock().unwrap();
                platforms.load_all();
                platforms.platforms.iter().map(|p| p.info.clone()).collect::<Vec<_>>()
            }).await?;
            send_socket(websocket, json!({
                "action": "loadPlatforms",
                "platforms": platforms
            })).await.ok();
        },
        Action::ConfigCallback { config, platform, id } => {
            let platform_clone = platform.clone();
            let response = tokio::task::spawn_blocking(move || {
                if let Some(p) = AUTOTAGGER_PLATFORMS.lock().unwrap().get_builder(&platform) {
                    Some(p.config_callback(&id, config))
                } else {
                    None
                }
            }).await?;
            if let Some(r) = response {
                send_socket(websocket, json!({
                    "action": "configCallback",
                    "platform": platform_clone,
                    "response": r
                })).await.ok();
            }
        }
        Action::TaggingState => {
            let snapshot = context.start_context.runs.lock().unwrap().as_ref().map(RunStatus::snapshot);
            send_socket(websocket, snapshot.unwrap_or_else(|| json!({"action": "taggingState", "runId": null}))).await?;
        }
        action @ (Action::StartTagging { .. } | Action::RetryTagging { .. }) => {
            let _lease = TaggingLease::acquire(&TAGGING_ACTIVE)?;
            let runs = &context.start_context.runs;
            let tagger_finished = Arc::new(Mutex::new(None::<TaggerFinishedData>));
            let mut folder_path = None;
            let (tagger_type, file_count, rx) = match action {
                Action::RetryTagging { run_id } => {
                    let retry = runs.lock().unwrap().as_ref().filter(|run| run.id == run_id)
                        .and_then(|run| run.retry.clone()).ok_or_else(|| anyhow!("This lookup queue is no longer available"))?;
                    ("autoTagger", retry.file_count(), retry.start(tagger_finished.clone())?)
                }
                Action::StartTagging { config, playlist, report, selection, rotate } => {
                    config.debug_print();
                    let explicit = playlist.is_some() || selection.is_some();
                    let mut files = match playlist { Some(playlist) => playlist.get_files()?, None => vec![] };
                    match config {
                        TaggerConfigs::AutoTagger(c) => {
                            if let Some(manifest) = &selection { files = manifest.files(); }
                            if !explicit {
                                let path = c.path.clone().unwrap_or_default();
                                files = AudioFileInfo::get_file_list(&path, c.include_subfolders);
                                folder_path = Some(path);
                            }
                            ("autoTagger", files.len(), Tagger::start_ui(&c, files, tagger_finished.clone(), report, selection, rotate)?)
                        }
                        TaggerConfigs::AudioFeatures(c) => {
                            if report || selection.is_some() { return Err(UserError::ReportsRequireAutoTagger.into()); }
                            if !explicit {
                                let path = c.path.clone().unwrap_or_default();
                                files = AudioFileInfo::get_file_list(&path, c.include_subfolders);
                                folder_path = Some(path);
                            }
                            let spotify = context.spotify.as_ref().ok_or(UserError::SpotifyUnauthorized)?.clone();
                            ("audioFeatures", files.len(), AudioFeatures::start_tagging(c, spotify, files))
                        }
                    }
                }
                _ => unreachable!(),
            };
            let run_id = {
                let mut state = runs.lock().unwrap();
                let id = state.as_ref().map_or(1, |prev| prev.id + 1);
                *state = Some(RunStatus { id, kind: tagger_type, total: file_count, progress: 0.0, completion: None, retry: None });
                id
            };
            // Start
            let start = timestamp!();
            let mut connected = send_socket(websocket, json!({
                "action": "startTagging", "runId": run_id,
                "files": file_count,
                "type": tagger_type
            })).await.is_ok();
            // Receiving from crossbeam blocks. Keep it off the async runtime so
            // the separate Stop connection remains responsive on small machines.
            let (updates_tx, mut updates) = tokio::sync::mpsc::channel(64);
            tokio::task::spawn_blocking(move || {
                for status in rx {
                    if updates_tx.blocking_send(status).is_err() { break; }
                }
            });
            while let Some(status) = updates.recv().await {
                runs.lock().unwrap().as_mut().unwrap().progress = status.progress;
                if connected {
                    connected = send_socket(websocket, json!({"action": "taggingProgress", "status": status})).await.is_ok();
                }
            }
            info!("Tagging finished, took {} seconds", (timestamp!() - start) / 1000);
            let mut data = tagger_finished.lock().unwrap().take();
            let retry = data.as_mut().and_then(|finished| finished.retry.take());
            let completion = json!({"action": "taggingDone", "runId": run_id, "path": folder_path, "data": data,
                "retryCount": retry.as_ref().map_or(0, |plan| plan.file_count()),
                "retryAfterMs": retry.as_ref().map_or(0, |plan| plan.remaining().as_millis().min(u64::MAX as u128) as u64),
                "cancelled": onetagger_autotag::STOP_TAGGING.load(Ordering::SeqCst)});
            {
                let mut state = runs.lock().unwrap();
                let state = state.as_mut().unwrap();
                state.retry = retry;
                state.completion = Some(completion.clone());
            }
            if connected { send_socket(websocket, completion).await?; }
        },
        Action::StopTagging => {
            onetagger_autotag::STOP_TAGGING.store(true, Ordering::SeqCst);
            send_socket(websocket, json!({"action": "stopTagging", "accepted": true})).await?;
        },
        Action::Waveform { path, request_id } => {
            let cancelled = Arc::new(AtomicBool::new(false));
            let worker_cancelled = cancelled.clone();
            let reply_path = path.clone();
            let mut job = tokio::spawn(async move {
                blocking(move || crate::library::waveform(&path, &worker_cancelled)).await
            });
            let waves = loop {
                tokio::select! {
                    result = &mut job => break result??,
                    message = websocket.recv() => if !matches!(message, Some(Ok(Message::Ping(_) | Message::Pong(_)))) {
                        cancelled.store(true, Ordering::Relaxed); return Ok(());
                    },
                }
            };
            if request_id.is_some() {
                send_socket(websocket, json!({"action": "waveformDone", "requestId": request_id,
                    "path": reply_path, "waves": waves})).await?;
            } else {
                // Preserve the old per-bar acknowledgement protocol for older clients.
                for wave in waves {
                    send_socket(websocket, json!({"action": "waveformWave", "wave": wave})).await?;
                    if !matches!(websocket.recv().await, Some(Ok(Message::Text(_)))) { break; }
                }
                send_socket(websocket, json!({"action": "waveformDone"})).await?;
            }
        },
        // Load player file
        Action::PlayerLoad { path, request_id, client_side } => {
            let player = if client_side { None } else { Some(context.player.get_or_insert_with(AudioPlayer::new).clone()) };
            let volume = context.volume;
            let media = context.start_context.media.clone();
            let reply_id = request_id.clone();
            let reply_path = path.clone();
            let result = blocking(move || {
                let source = AudioSources::from_path(&path)?;
                let duration = source.duration() as u64;
                let tag = Tag::load_file(&path, false)?;
                let title = tag.tag().get_field(Field::Title).and_then(|values| values.first().cloned());
                let artists = tag.tag().get_field(Field::Artist).unwrap_or_default();
                if let Some(player) = player { player.volume(volume); player.load_file(source)?; }
                let media_id = client_side.then(|| media.grant(path.clone()));
                Ok(json!({"action": "playerLoad", "requestId": request_id, "path": path,
                    "title": title, "artists": artists, "duration": duration, "mediaId": media_id}))
            }).await;
            match result {
                Ok(response) => send_socket(websocket, response).await?,
                Err(error) => send_socket(websocket, localized(json!({"action": "playerLoad", "status": "error",
                    "requestId": reply_id, "path": reply_path, "message": error.to_string()}), UiMessage::RequestFailed { detail: error.to_string() })).await?,
            }
        },
        //  Controls
        Action::PlayerPlay => if let Some(player) = &context.player { player.play(); },
        Action::PlayerPause => if let Some(player) = &context.player { player.pause(); },
        Action::PlayerSeek { pos } => {
            let player = context.player.clone().ok_or(anyhow!("No player is loaded"))?;
            let state = blocking(move || player.seek(pos)).await?;
            send_socket(websocket, json!({"action": "playerSync", "position": state.position,
                "playing": state.playing, "ended": state.ended, "seek": true})).await?;
        },
        Action::PlayerState => {
            if let Some(player) = context.player.clone() {
                let state = blocking(move || player.state()).await?;
                send_socket(websocket, json!({"action": "playerSync", "position": state.position,
                    "playing": state.playing, "ended": state.ended})).await?;
            }
        },
        Action::PlayerVolume { volume } => {
            context.volume = volume;
            if let Some(player) = &context.player { player.volume(volume); }
        }
        Action::PlayerStop => if let Some(player) = &context.player { player.stop(); },
        // Load quicktag files or playlist
        Action::QuickTagLoad { path, playlist, recursive, separators, limit, request_id, progressive } => {
            let limit = if limit.unwrap_or(false) { 500 } else { usize::MAX };
            if progressive {
                let cancelled = Arc::new(AtomicBool::new(false));
                let worker_cancelled = cancelled.clone();
                let (sender, mut batches) = tokio::sync::mpsc::channel(2);
                let job = tokio::spawn(async move {
                    blocking(move || {
                        let limit = if playlist.is_some() || path.as_ref().is_some_and(|path| !Path::new(path).is_dir()) { usize::MAX } else { limit };
                        let files = crate::library::inputs(path, playlist, recursive.unwrap_or(false))?;
                        crate::library::scan(files, &separators, limit, &worker_cancelled,
                            |batch| sender.blocking_send(batch).map_err(|_| anyhow!("Library load cancelled")))
                    }).await
                });
                loop {
                    tokio::select! {
                        batch = batches.recv() => match batch {
                            Some(data) => if send_socket(websocket, json!({"action": "quickTagLoadBatch",
                                "requestId": request_id, "data": data})).await.is_err() {
                                cancelled.store(true, Ordering::Relaxed); break;
                            },
                            None => {
                                match job.await? {
                                    Ok(stats) => { send_socket(websocket, json!({"action": "quickTagLoadDone",
                                        "requestId": request_id, "stats": stats})).await?; },
                                    Err(error) => { send_socket(websocket, localized(json!({"action": "quickTagLoadDone",
                                        "requestId": request_id, "status": "error", "message": error.to_string()}),
                                        UiMessage::RequestFailed { detail: error.to_string() })).await?; },
                                }
                                break;
                            },
                        },
                        message = websocket.recv() => if !matches!(message, Some(Ok(Message::Ping(_) | Message::Pong(_)))) {
                            cancelled.store(true, Ordering::Relaxed); break;
                        },
                    }
                }
                cancelled.store(true, Ordering::Relaxed);
            } else {
                let data = blocking(move || {
                    let limit = if playlist.is_some() || path.as_ref().is_some_and(|path| !Path::new(path).is_dir()) { usize::MAX } else { limit };
                    let files = crate::library::inputs(path, playlist, recursive.unwrap_or(false))?;
                    let mut data = QuickTagData::default();
                    crate::library::scan(files, &separators, limit, &AtomicBool::new(false), |batch| {
                        data.files.extend(batch.files); data.failed.extend(batch.failed); Ok(())
                    })?;
                    Ok(data)
                }).await?;
                send_socket(websocket, json!({"action": "quickTagLoad", "data": data})).await?;
            }
        },
        // Save quicktag changes
        Action::QuickTagSave { changes, request_id } => {
            let response = blocking(move || Ok(quick_tag_save_response(&changes, request_id))).await?;
            send_socket(websocket, response).await?;
        },
        // List dir
        Action::QuickTagFolder { path, subdir } => {
            let (new_path, files) = blocking(move || FileBrowser::list_dir_or_default(path.map(PathBuf::from), subdir, true, false, false)).await?;
            send_socket(websocket, json!({
                "action": "quickTagFolder",
                "files": files,
                "path": new_path,
            })).await.ok();
        }
        Action::SpotifyAuthorize { client_id, client_secret } => {
            // Authorize cached
            if let Some(spotify) = Spotify::try_cached_token(&client_id, &client_secret) {
                context.spotify = Some(spotify);
            // Authorize new
            } else {
                let (auth_url, client) = Spotify::generate_auth_url(&client_id, &client_secret)?;
                webbrowser::open(&auth_url)?;
                let spotify = tokio::task::spawn_blocking(move || {
                    Spotify::auth_server(client)
                }).await??;
                context.spotify = Some(spotify);
            }
            send_socket(websocket, json!({
                "action": "spotifyAuthorized",
                "value": true
            })).await.ok();
            debug!("Spotify Authorized!");
        },
        // Check if authorized
        Action::SpotifyAuthorized => {
            send_socket(websocket, json!({
                "action": "spotifyAuthorized",
                "value": context.spotify.is_some()
            })).await.ok();
        },
        Action::TagEditorFolder { path, subdir, recursive } => {
            let recursive = recursive.unwrap_or(false);
            let (new_path, files) = blocking(move || FileBrowser::list_dir_or_default(path.map(PathBuf::from), subdir, true, true, recursive)).await?;
            send_socket(websocket, json!({
                "action": "tagEditorFolder",
                "files": files,
                "path": new_path,
                // Stateless
                "recursive": recursive
            })).await.ok();
        },
        // Load tags of file
        Action::TagEditorLoad { path } => {
            let data = blocking(move || TagEditor::load_file(&path)).await?;
            send_socket(websocket, json!({
                "action": "tagEditorLoad",
                "data": data
            })).await.ok();
        },
        // Save changes
        Action::TagEditorSave { changes, request_id } => {
            let response = blocking(move || Ok(tag_editor_save_response(&changes, request_id))).await?;
            send_socket(websocket, response).await?;
        },
        // Syntax highlight for renamer
        Action::RenamerSyntaxHighlight { template } => {
            let renamer = Renamer::new(TemplateParser::parse(&template));
            let html = renamer.generate_html(&template);
            send_socket(websocket, json!({
                "action": "renamerSyntaxHighlight",
                "html": html
            })).await.ok();
        },
        // Autocomplete data
        Action::RenamerAutocomplete { template } => {
            let ac = Autocomplete::parse(&template);
            let suggestions = ac.suggest_html();
            send_socket(websocket, json!({
                "action": "renamerAutocomplete",
                "suggestions": suggestions,
                "offset": ac.suggestion_offset()
            })).await.ok();
        },
        // Generate new names but don't rename
        Action::RenamerPreview { config } => {
            let files = blocking(move || {
                let mut renamer = Renamer::new(TemplateParser::parse(&config.template));
                let files = AudioFileInfo::load_files_iter(&config.path, config.subfolders, None, None);
                Ok(renamer.generate(files.take(3), &config).unwrap_or_default())
            }).await?;
            send_socket(websocket, json!({"action": "renamerPreview", "files": files})).await?;
        },
        // Start renamer
        Action::RenamerStart { config } => {
            let report = blocking(move || {
                let mut renamer = Renamer::new(TemplateParser::parse(&config.template));
                let files = AudioFileInfo::load_files_iter(&config.path, config.subfolders, None, None);
                let files = renamer.generate(files, &config)?;
                renamer.rename(&files, &config)
            }).await?;
            send_socket(websocket, json!({"action": "renamerDone", "report": report})).await?;
        },
        // File browser list dir
        Action::FolderBrowser { path, child, base } => {
            let (path, entry) = blocking(move || {
                let path = if cfg!(windows) && path.to_string_lossy() == "/" {
                    if child.is_empty() { PathBuf::from("/") } else { PathBuf::from(format!("{child}\\")) }
                } else { canonicalize(path.join(child))? };
                let entry = if base { FolderBrowser::generate_base(&path)? } else { FolderBrowser::list_dir(&path)? };
                Ok((path, entry))
            }).await?;
            send_socket(websocket, json!({"action": "folderBrowser", "entry": entry, "base": base, "path": path})).await?;
        },

        // Manually tag a file
        Action::ManualTag { config, path } => {
            // Log config
            info!("Manual tag starting for path: {path:?}");
            TaggerConfigs::AutoTagger(config.clone()).debug_print();

            let rx = tokio::task::spawn_blocking(move || {
                onetagger_autotag::manual_tagger(path, &config)
            }).await.unwrap()?;

            let (sender, mut results) = tokio::sync::mpsc::channel(8);
            tokio::task::spawn_blocking(move || {
                for result in rx { if sender.blocking_send(result).is_err() { break; } }
            });
            while let Some((platform, r)) = results.recv().await {
                match r {
                    Ok(matches) => {
                        send_socket(websocket, json!({
                            "action": "manualTag",
                            "platform": platform,
                            "status": "ok",
                            "matches": matches
                        })).await.ok();
                    },
                    Err(e) => {
                        send_socket(websocket, localized(json!({
                            "action": "manualTag",
                            "platform": platform,
                            "status": "error",
                            "error": e.to_string()
                        }), UiMessage::ManualTagFailed { detail: e.to_string() })).await.ok();
                    },
                }
            }

            // On done
            send_socket(websocket, json!({
                "action": "manualTagDone"
            })).await.ok();
        },
        // Apply the tags from manual tagger
        Action::ManualTagApply { matches, path, config } => {
            let result = blocking(move || {
                let result = onetagger_autotag::manual_tagger_apply(matches, &path, &config);
                crate::library::invalidate(&path);
                result
            }).await;
            match result {
                Ok(_) => {
                    send_socket(websocket, json!({
                        "action": "manualTagApplied",
                        "status": "ok"
                    })).await.ok();
                },
                Err(e) => {
                    error!("Failed applying manual tag: {e}");
                    send_socket(websocket, localized(json!({
                        "action": "manualTagApplied",
                        "status": "error",
                        "error": e.to_string()
                    }), UiMessage::ManualTagApplyFailed { detail: e.to_string() })).await.ok();
                },
            }
        },


        Action::RepoManifest => {
            send_socket(websocket, json!({
                "action": "repoManifest",
                "manifest": onetagger_autotag::repo::fetch_manifest_async().await?
            })).await.ok();
        },
        Action::InstallPlatform { id, version, is_native } => {
            let result = blocking(move || onetagger_autotag::repo::install_platform(&id, &version, is_native)).await;
            match result {
                Ok(_) => send_socket(websocket, json!({
                    "action": "installPlatform",
                    "status": "ok"
                })).await.ok(),
                Err(e) => {
                    error!("Failed installing platform: {e}");
                    send_socket(websocket, localized(json!({
                        "action": "installPlatform",
                        "status": "error",
                        "error": e.to_string()
                    }), UiMessage::PlatformInstallFailed { detail: e.to_string() })).await.ok()
                },
            };
        },

        
        
    }
   
    Ok(())
}

#[cfg(test)]
mod save_tests {
    use super::*;

    #[test]
    fn overlapping_runs_cannot_reset_cancellation_and_lease_releases() {
        let active = AtomicBool::new(false);
        let first = TaggingLease::acquire(&active).unwrap();
        let error = TaggingLease::acquire(&active).err().unwrap();
        assert!(error.downcast_ref::<UserError>().is_some());
        assert_eq!(request_error(&error)["messageKey"], "backend.taggingAlreadyActive");
        drop(first);
        assert!(TaggingLease::acquire(&active).is_ok());
        assert!(!active.load(Ordering::SeqCst));
    }

    fn request(path: &Path, request_id: Option<&str>) -> Action {
        let mut value = json!({"action": "tagEditorSave", "changes": {
            "path": path, "changes": [{"type": "raw", "tag": "TIT2", "value": ["Saved title"]}],
            "separators": {"id3": ", ", "vorbis": null, "mp4": ", "}, "id3v24": true
        }});
        if let Some(id) = request_id { value["requestId"] = json!(id); }
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn save_acknowledges_persisted_first_tag_with_request_and_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("untagged.mp3");
        std::fs::write(&path, [0u8; 64]).unwrap();
        let Action::TagEditorSave { changes, request_id } = request(&path, Some("save-1")) else { panic!() };
        let response = tag_editor_save_response(&changes, request_id);
        assert_eq!(response["status"], "ok");
        assert_eq!(response["requestId"], "save-1");
        assert_eq!(response["path"], json!(path));
        assert_eq!(Tag::load_file(&path, false).unwrap().tag().get_field(Field::Title).unwrap(), vec!["Saved title"]);
    }

    #[test]
    fn failed_save_is_correlated_and_legacy_errors_cannot_look_successful() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("corrupt.mp3");
        let original = b"ID3\x05\x00\x00\x00\x00\x00\x06broken";
        std::fs::write(&path, original).unwrap();
        for id in [Some("save-2"), None] {
            let Action::TagEditorSave { changes, request_id } = request(&path, id) else { panic!() };
            let response = tag_editor_save_response(&changes, request_id);
            assert_eq!(response["action"], if id.is_some() { "tagEditorSave" } else { "error" });
            assert_eq!(response["status"], "error");
            assert_eq!(response["requestId"], json!(id));
            assert_eq!(response["path"], json!(path));
            assert!(!response["message"].as_str().unwrap().is_empty());
            assert_eq!(response["messageKey"], "backend.tagEditorSaveFailed");
            assert_eq!(response["messageArgs"]["detail"], response["message"]);
            assert_eq!(std::fs::read(&path).unwrap(), original);
        }
    }
    #[test]
    fn quicktag_save_replies_settle_success_and_failure_with_the_request_and_path() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("track.mp3");
        std::fs::write(&path, [0u8; 64]).unwrap();
        let mut value = json!({"action": "quickTagSave", "requestId": "quick-1", "changes": {
            "path": path, "changes": [{"type": "raw", "tag": "TIT2", "value": ["Title"]},
            {"type": "raw", "tag": "TPE1", "value": ["Artist"]}],
            "separators": {"id3": ", ", "vorbis": null, "mp4": ", "}, "id3v24": true}});
        let Action::QuickTagSave { changes, request_id } = serde_json::from_value(value.clone()).unwrap() else { panic!() };
        let reply = quick_tag_save_response(&changes, request_id);
        assert_eq!(reply["status"], "ok"); assert_eq!(reply["requestId"], "quick-1");
        assert_eq!(reply["path"], json!(path)); assert_eq!(reply["file"]["title"], "Title");
        value["changes"]["path"] = json!(directory.path().join("missing").join("track.mp3"));
        let Action::QuickTagSave { changes, request_id } = serde_json::from_value(value).unwrap() else { panic!() };
        let reply = quick_tag_save_response(&changes, request_id);
        assert_eq!(reply["action"], "quickTagSaved"); assert_eq!(reply["status"], "error");
        assert_eq!(reply["requestId"], "quick-1"); assert_eq!(reply["path"], json!(changes.path));
        assert!(reply["message"].as_str().is_some_and(|message| !message.is_empty()));
    }

    #[test]
    fn quicktag_can_save_repeatedly_while_a_native_decoder_has_the_audio_open() {
        // Selection loads a decoder even before Play is pressed. Exercise the
        // real backend and tag writer, without requiring an audio output device.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("track.mp3");
        std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../onetagger-player/test-fixtures/tone.mp3"), &path).unwrap();
        let source = onetagger_player::AudioSources::from_path(&path).unwrap();
        let mut decoder = source.get_source().unwrap();
        for title in ["First edit", "Second edit"] {
            let value = json!({"action": "quickTagSave", "requestId": title, "changes": {
                "path": path, "changes": [{"type": "raw", "tag": "TIT2", "value": [title]},
                    {"type": "raw", "tag": "TPE1", "value": ["Fixture artist"]}],
                "separators": {"id3": ", ", "vorbis": null, "mp4": ", "}, "id3v24": true}});
            let Action::QuickTagSave { changes, request_id } = serde_json::from_value(value).unwrap() else { panic!() };
            let reply = quick_tag_save_response(&changes, request_id);
            assert_eq!(reply["status"], "ok", "{reply}");
            assert_eq!(reply["file"]["title"], title);
            assert_eq!(Tag::load_file(&path, false).unwrap().tag().get_field(Field::Title).unwrap(), vec![title]);
            assert!(decoder.next().is_some());
        }
    }

    #[cfg(windows)]
    #[test]
    fn quicktag_reports_the_windows_replacement_error_and_retains_the_original() {
        use std::os::windows::fs::OpenOptionsExt;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("locked.mp3");
        std::fs::write(&path, [0u8; 64]).unwrap();
        let Action::QuickTagSave { changes, .. } = serde_json::from_value(json!({"action":"quickTagSave", "changes": {
            "path":path, "changes":[{"type":"raw","tag":"TIT2","value":["Title"]},
                {"type":"raw","tag":"TPE1","value":["Artist"]}],
            "separators":{"id3":", ","vorbis":null,"mp4":", "},"id3v24":true}})).unwrap() else { panic!() };
        let original = std::fs::read(&path).unwrap();
        // Permit reading/copying, but simulate an external program denying deletion.
        let lock = std::fs::OpenOptions::new().read(true).share_mode(3).open(&path).unwrap();
        let reply = quick_tag_save_response(&changes, Some("locked-save".into()));
        assert_eq!(reply["status"], "error");
        assert_eq!(reply["requestId"], "locked-save");
        assert!(reply["message"].as_str().unwrap().contains("os error"), "{reply}");
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
        drop(lock);
        assert_eq!(quick_tag_save_response(&changes, Some("unlocked-save".into()))["status"], "ok");
    }

}
