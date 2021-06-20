use std::error::Error;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::path::Path;
use tungstenite::server::accept;
use tungstenite::{Message, WebSocket};
use serde_json::{Value, json};
use directories::UserDirs;
use dunce::canonicalize;
use serde::{Serialize, Deserialize};

use crate::tag::TagChanges;
use crate::tagger::{TaggerConfig, Tagger};
use crate::tagger::spotify::Spotify;
use crate::ui::{OTError, Settings, StartContext};
use crate::ui::player::{AudioSources, AudioPlayer};
use crate::ui::quicktag::{QuickTag, QuickTagFile};
use crate::ui::audiofeatures::{AudioFeaturesConfig, AudioFeatures};
use crate::ui::tageditor::TagEditor;
use crate::playlist::UIPlaylist;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

//Wrap of tagger config, so playlists can be passed too
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaggerConfigWrap {
    config: TaggerConfigs,
    playlist: Option<UIPlaylist>
}

//loadQuickTag message
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QuickTagLoad {
    path: Option<String>,
    playlist: Option<UIPlaylist>,
    recursive: Option<bool>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
enum TaggerConfigs {
    AutoTagger(TaggerConfig), 
    AudioFeatures(AudioFeaturesConfig)
}

//Shared variables in socket
struct SocketContext {
    player: AudioPlayer,
    spotify: Option<Spotify>,
    start_context: StartContext
} 

impl SocketContext {
    pub fn new(start_context: StartContext) -> SocketContext {
        SocketContext {
            player: AudioPlayer::new(),
            spotify: None,
            start_context
        }
    }
}

//Start WebSocket UI server
pub fn start_socket_server(context: StartContext) {
    let server = TcpListener::bind("127.0.0.1:36912").unwrap();
    for stream in server.incoming() {
        let context = context.clone();
        thread::spawn(move || {
            //Create shared
            let mut context = SocketContext::new(context);

            //Websocket loop
            let mut websocket = accept(stream.unwrap()).unwrap();
            loop {
                match websocket.read_message() {
                    Ok(msg) => {
                        if msg.is_text() {
                            let text = msg.to_text().unwrap();
                            match handle_message(text, &mut websocket, &mut context) {
                                Ok(_) => {},
                                Err(err) => {
                                    //Send error to UI
                                    error!("Websocket: {:?}, Data: {}", err, text);
                                    websocket.write_message(Message::from(json!({
                                        "action": "error",
                                        "message": &format!("{}", err)
                                    }).to_string())).ok();
                                }
                            }
                        }
                    },
                    Err(e) => {
                        //Connection closed
                        if !websocket.can_read() || !websocket.can_write() {
                            warn!("{} - Websocket can't read or write, closing connection!", e);
                            break;
                        }
                        warn!("Invalid websocket message: {}", e);
                    }
                }
            }
        });
    }
}


fn handle_message(text: &str, websocket: &mut WebSocket<TcpStream>, context: &mut SocketContext) -> Result<(), Box<dyn Error>> {
    //Parse JSON
    let json: Value = serde_json::from_str(text)?;
    match json["action"].as_str().ok_or("Missing action!")? {
        //Get initialization info
        "init" => {
            websocket.write_message(Message::from(json!({
                "action": "init",
                "version": VERSION,
                "startContext": context.start_context
            }).to_string())).ok();
        },
        //Save, load settings from UI
        "saveSettings" => {
            let settings = Settings::from_ui(&json["settings"]);
            settings.save()?;
        },
        "loadSettings" => {
            //Ignore settings load error, might be first try
            match Settings::load() {
                Ok(settings) => {
                    websocket.write_message(Message::from(json!({
                        "action": "loadSettings",
                        "settings": settings.ui
                    }).to_string())).ok();
                },
                Err(e) => {
                    error!("Failed loading settings, using defaults. {}", e);
                }
            }
        },
        //Browse folder
        "browse" => {
            let mut initial = json["path"].as_str().unwrap_or(".");
            if initial.is_empty() {
                initial = ".";
            }
            if let Some(path) = tinyfiledialogs::select_folder_dialog("Select path", initial) {
                websocket.write_message(Message::from(json!({
                    "action": "browse",
                    "path": path,
                    "context": json["context"]
                }).to_string())).ok();
            }
        },
        //Open URL in external browser
        "browser" => {
            if let Some(url) = json["url"].as_str() {
                webbrowser::open(url)?;
            }
        },
        //Open folder with settings and log
        "openSettingsFolder" => {
            opener::open(Settings::get_folder()?.to_str().unwrap())?;
        },
        //Start tagger
        "startTagging" => {
            //Exctract hidden properties manually
            let tagger_type = json["config"]["type"].clone();
            let path = json["config"]["path"].as_str().unwrap_or("").to_string();
            let wrap: TaggerConfigWrap = serde_json::from_value(json)?;
            //Get files
            let files = if let Some(playlist) = wrap.playlist {
                playlist.get_files()?
            } else {
                if path.is_empty() {
                    return Err(OTError::new("Invalid path!").into());
                }
                Tagger::get_file_list(&path)
            };
            let file_count = files.len();

            //Get tagger
            let rx = match wrap.config {
                TaggerConfigs::AutoTagger(config) => Tagger::tag_files(&config, files),
                TaggerConfigs::AudioFeatures(config) => {
                    let spotify = context.spotify.as_ref().ok_or("Spotify unauthorized!")?.to_owned().to_owned();
                    AudioFeatures::start_tagging(config.clone(), spotify, files)
                }
            };

            //Start
            websocket.write_message(Message::from(json!({
                "action": "startTagging",
                "files": file_count,
                "type": tagger_type
            }).to_string())).ok();

            let start = timestamp!();
            for status in rx {
                websocket.write_message(Message::from(json!({
                    "action": "taggingProgress",
                    "status": status
                }).to_string())).ok();
            }
            info!("Tagging finished, took: {} seconds.", (timestamp!() - start) / 1000);
            //Done
            websocket.write_message(Message::from(json!({
                "action": "taggingDone"
            }).to_string())).ok();
        },
        //Generate waveform, should be run from separate connection
        "waveform" => {
            let path = json["path"].as_str().unwrap();
            let source = AudioSources::from_path(path)?;
            let (waveform_rx, cancel_tx) = source.generate_waveform(180)?;
            //Streamed
            for wave in waveform_rx {
                websocket.write_message(Message::from(json!({
                    "action": "waveformWave",
                    "wave": wave
                }).to_string())).ok();
                //Check reply
                websocket.read_message().ok();
                if !websocket.can_write() {
                    cancel_tx.send(true).ok();
                }
            }
            //Done
            websocket.write_message(Message::from(json!({
                "action": "waveformDone",
            }).to_string())).ok();

        },
        //Load player file
        "playerLoad" => {
            let path = json["path"].as_str().ok_or("Missing path!")?;
            let source = AudioSources::from_path(path)?;
            //Send to UI
            websocket.write_message(Message::from(json!({
                "action": "playerLoad",
                "duration": source.duration() as u64
            }).to_string())).ok();
            //Load
            context.player.load_file(source);
        },
        //Player controls
        "playerPlay" => {
            context.player.play();
        },
        "playerPause" => {
            context.player.pause();
        },
        "playerSeek" => {
            let playing = context.player.seek(json["pos"].as_i64().ok_or("Missing position!")? as u64);
            //Sync
            websocket.write_message(Message::from(json!({
                "action": "playerSync",
                "playing": playing
            }).to_string())).ok();
        },
        "playerVolume" => {
            let volume = json["volume"].as_f64().ok_or("Missing volume!")? as f32;
            context.player.volume(volume);
        }
        //Quicktag
        "quicktagLoad" => {
            let msg: QuickTagLoad = serde_json::from_value(json)?;
            let mut files = vec![];
            //Playlist
            if let Some(playlist) = msg.playlist {
                files = QuickTag::load_files_playlist(&playlist)?;
            }
            //Path
            if let Some(path) = msg.path {
                files = QuickTag::load_files_path(&path, msg.recursive.unwrap_or(false))?;
            }
            websocket.write_message(Message::from(json!({
                "action": "quicktagLoad",
                "data": files
            }).to_string())).ok();
        },
        //Save quicktag
        "quicktagSave" => {
            let changes: TagChanges = serde_json::from_value(json["changes"].clone())?;
            let tag = changes.commit()?;
            websocket.write_message(Message::from(json!({
                "action": "quicktagSaved",
                "path": &changes.path,
                "file": QuickTagFile::from_tag(&changes.path, &tag).ok_or("Failed loading tags")?
            }).to_string())).ok();
        },
        //Authorize spotify
        "spotifyAuthorize" => {
            //Get vars
            let client_id = json["clientId"].as_str().ok_or("Missing clientId")?;
            let client_secret = json["clientSecret"].as_str().ok_or("Missing clientSecret")?;
            //Authorize cached
            if let Some(spotify) = Spotify::try_cached_token(client_id, client_secret) {
                context.spotify = Some(spotify);
            //Authorize new
            } else {
                let (auth_url, mut oauth) = Spotify::generate_auth_url(client_id, client_secret);
                webbrowser::open(&auth_url)?;
                let spotify = Spotify::auth_server(&mut oauth)?;
                context.spotify = Some(spotify);
            }
            websocket.write_message(Message::from(json!({
                "action": "spotifyAuthorized",
                "value": true
            }).to_string())).ok();
        },
        //Check if authorized
        "spotifyAuthorized" => {
            websocket.write_message(Message::from(json!({
                "action": "spotifyAuthorized",
                "value": context.spotify.is_some()
            }).to_string())).ok();
        },
        //Tag editor
        "tagEditorFolder" => {
            let recursive = json["recursive"].as_bool().unwrap_or(false);
            let user_dirs = UserDirs::new().ok_or("Invalid home dir!")?;
            let path_raw = json["path"].as_str().unwrap_or(
                user_dirs.audio_dir().ok_or("Missing path!")?.to_str().ok_or("Invalid path!")?
            );
            //Get parent
            let subdir = json["subdir"].as_str().unwrap_or("");
            let path = Path::new(path_raw);
            //Override for playlists
            let path = if !path.is_dir() {
                if subdir == ".." {
                    path.parent().ok_or("Invalid playlist parent!")?.to_owned()
                } else {
                    path.to_owned()
                }
            } else {
                canonicalize(Path::new(path_raw).join(subdir))?
            };
            //Load
            let path = path.to_str().unwrap();
            let files = match recursive {
                true => TagEditor::list_dir_recursive(path)?,
                false => TagEditor::list_dir(path)?
            };
            websocket.write_message(Message::from(json!({
                "action": "tagEditorFolder",
                "files": files,
                "path": path,
                //Stateless
                "recursive": recursive
            }).to_string())).ok();
        },
        //Load playlist from data
        "tagEditorPlaylist" => {
            let playlist: UIPlaylist = serde_json::from_value(json)?;
            let files = playlist.get_files()?;
            //Keep only existing files, and clean path
            let files: Vec<String> = files.iter().filter_map(|f| {
                let path = Path::new(f);
                match path.exists() {
                    true => match canonicalize(path) {
                        Ok(p) => Some(p.to_str().unwrap().to_string()),
                        Err(_) => None
                    }
                    false => None
                }
            }).collect();

            websocket.write_message(Message::from(json!({
                "action": "tagEditorFolder",
                "files": files,
                "path": "",
                //To add to custom list
                "recursive": true
            }).to_string())).ok();

        },
        "tagEditorLoad" => {
            let path = Path::new(json["path"].as_str().ok_or("Missing path!")?);
            let data = TagEditor::load_file(path.to_str().unwrap())?;
            websocket.write_message(Message::from(json!({
                "action": "tagEditorLoad",
                "data": data
            }).to_string())).ok();
        },
        "tagEditorSave" => {
            let changes: TagChanges = serde_json::from_value(json["changes"].clone())?;
            let _tag = changes.commit()?;
            websocket.write_message(Message::from(json!({
                "action": "tagEditorSave"
            }).to_string())).ok();
        }
        _ => {}
    };
    Ok(())
}