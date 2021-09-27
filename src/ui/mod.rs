use directories::ProjectDirs;
use rouille::{router, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::thread;

pub mod audiofeatures;
pub mod player;
pub mod quicktag;
pub mod socket;
pub mod tageditor;

// UI
static INDEX_HTML: &'static str = include_str!("../../client/dist/dist.html");
static BG_PNG: &'static [u8] = include_bytes!("../../client/dist/bg.png");
static FAVICON_PNG: &'static [u8] = include_bytes!("../../client/dist/favicon.png");

// Onetagger settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    ui: Value,
    version: Option<i32>,
}

impl Settings {
    // Create settings from UI json
    pub fn from_ui(ui: &Value) -> Settings {
        Settings {
            ui: ui.to_owned(),
            version: Some(2),
        }
    }

    // Load settings from file
    pub fn load() -> Result<Settings, Box<dyn Error>> {
        let path = Settings::get_path()?;
        let settings: Settings = serde_json::from_reader(File::open(&path)?)?;

        // v1.0 are not compatible with 1.1, create backup
        if settings.version.unwrap_or(1) == 1 {
            let new_path = format!("{}-1.0.bak", &path);
            fs::copy(&path, &new_path)?;
            info!("Backup of settings created: {}", new_path);
            fs::remove_file(&path)?;
            return Settings::load();
        }

        Ok(settings)
    }
    // Save settings to file
    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let path = Settings::get_path()?;
        let mut file = File::create(path)?;
        file.write_all(serde_json::to_string_pretty(self)?.as_bytes())?;
        Ok(())
    }

    // Get app data folder
    pub fn get_folder() -> Result<PathBuf, Box<dyn Error>> {
        let root =
            ProjectDirs::from("com", "OneTagger", "OneTagger").ok_or("Error getting dir!")?;
        if !root.preference_dir().exists() {
            fs::create_dir_all(root.preference_dir())?;
        }
        Ok(root.preference_dir().to_owned())
    }

    // Get settings path
    fn get_path() -> Result<String, Box<dyn Error>> {
        let path = Settings::get_folder()?.join("settings.json");
        Ok(path
            .to_str()
            .ok_or("Error converting path to string!")?
            .to_string())
    }
}

// Should have data from arguments and other flags (eg. port / host in future)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartContext {
    pub server_mode: bool,
    pub start_path: Option<String>,
    pub expose: bool,
}

// Start webview window
#[cfg(not(target_os = "windows"))]
pub fn start_webview() {
    // Normal webview
    let webview = web_view::builder()
        .invoke_handler(|_, __| Ok(()))
        .content(web_view::Content::Url("http://127.0.0.1:36913"))
        .user_data(())
        .title("One Tagger")
        .size(1280, 750)
        .min_size(1150, 550)
        .resizable(true)
        .debug(true)
        .build()
        .unwrap();
    webview.run().unwrap();
}

// Start WebSocket server
pub fn start_socket_thread(context: StartContext) {
    thread::spawn(move || {
        socket::start_socket_server(context);
    });
}

// Start webserver for hosting static index.html
pub fn start_webserver_thread(context: &StartContext) {
    let host = match context.expose {
        true => "0.0.0.0:36913",
        false => "127.0.0.1:36913",
    };

    thread::spawn(move || {
        rouille::start_server(host, move |request| {
            router!(request,
                (GET) ["/"] => {
                    Response::html(INDEX_HTML)
                },
                (GET) ["/bg.png"] => {
                    Response::from_data("image/png", BG_PNG)
                },
                (GET) ["/favicon.png"] => {
                    Response::from_data("image/png", FAVICON_PNG)
                },
                // Get thumbnail of image from tag by path
                (GET) ["/thumb"] => {
                    match request.get_param("path") {
                        Some(path) => {
                            match quicktag::QuickTagFile::get_art(&path) {
                                Ok(art) => Response::from_data("image/jpeg", art),
                                Err(e) => {
                                    warn!("Error loading album art: {} File: {}", e, path);
                                    Response::empty_404()
                                }
                            }
                        },
                        None => Response::empty_404()
                    }
                },
                _ => Response::empty_404()
            )
        });
    });
}

// Start everything
pub fn start_all(context: StartContext) {
    match context.expose {
        true => {
            info!("Starting server on http://0.0.0.0:36913 ws://0.0.0.0:36912");
            warn!("Server is exposed to public!");
        }
        false => info!("Starting server on http://127.0.0.1:36913 ws://127.0.0.1:36912"),
    }

    // Server mode
    if context.server_mode {
        start_webserver_thread(&context);
        socket::start_socket_server(context);
        return;
    }

    start_webserver_thread(&context);
    start_socket_thread(context);
    start_webview();
}

// Windows webview
#[cfg(target_os = "windows")]
pub fn start_webview() {
    use once_cell::sync::OnceCell;
    use serde_json::json;
    use std::mem;
    use std::path::Path;
    use std::rc::Rc;
    use urlencoding::decode;
    use webview2::Environment;
    use winapi::shared::windef::{HWND, RECT};
    use winapi::um::winuser::GetClientRect;
    use winit::dpi::Size;
    use winit::event::{Event, WindowEvent};
    use winit::event_loop::{ControlFlow, EventLoop};
    use winit::platform::windows::WindowExtWindows;
    use winit::window::{Icon, WindowBuilder};

    // Install webview2 runtime
    bootstrap_webview2_wrap();
    // winit
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("One Tagger")
        .with_inner_size(Size::Logical((1280, 750).into()))
        .with_min_inner_size(Size::Logical((1150, 550).into()))
        .with_window_icon(Some(
            Icon::from_rgba(include_bytes!("../../assets/64x64.bin").to_vec(), 64, 64).unwrap(),
        ))
        .build(&event_loop)
        .unwrap();
    // webview2
    let controller = Rc::new(OnceCell::new());
    {
        let controller_clone = controller.clone();
        let hwnd = window.hwnd() as HWND;
        let data_dir = Settings::get_folder().unwrap().join("webview2");

        // Build webview2
        Environment::builder()
            .with_user_data_folder(data_dir.as_path())
            .build(move |env| {
                env.unwrap().create_controller(hwnd, move |controller| {
                    let controller = controller?;
                    let w = controller.get_webview()?;

                    w.get_settings().map(|settings| {
                        settings.put_is_status_bar_enabled(false).ok();
                        settings.put_are_default_context_menus_enabled(false).ok();
                        settings.put_is_zoom_control_enabled(false).ok();
                    })?;

                    unsafe {
                        let mut rect = mem::zeroed();
                        GetClientRect(hwnd, &mut rect);
                        controller.put_bounds(rect)?;
                    }

                    // Start webview
                    w.navigate("http://127.0.0.1:36913")?;
                    w.add_new_window_requested(|w, a| {
                        let uri = a.get_uri().unwrap();
                        if uri.starts_with("file://") {
                            // Windowsify
                            let uri = uri.replace("file:///", "");
                            let decoded = decode(&uri).unwrap().replace("/", "\\");
                            let path = Path::new(&decoded);
                            if path.exists() && path.is_dir() {
                                // Send to UI
                                w.post_web_message_as_string(
                                    &json!({
                                        "action": "browse",
                                        "path": decoded
                                    })
                                    .to_string(),
                                )?;
                            }
                        }

                        // Drag and drop don't create new window
                        a.put_new_window(w)
                    })?;
                    w.add_navigation_starting(|_w, n| {
                        let uri = n.get_uri()?;
                        // Cancel redirect on dropping a file
                        if uri.starts_with("file://") {
                            n.put_cancel(true)?;
                        }
                        Ok(())
                    })?;

                    controller_clone.set(controller).unwrap();
                    Ok(())
                })
            })
    }
    .unwrap();

    // winit EventLoop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    if let Some(webview) = controller.get() {
                        webview.close().unwrap();
                    }
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::Moved(_) => {
                    if let Some(webview) = controller.get() {
                        webview.notify_parent_window_position_changed().ok();
                    }
                }
                WindowEvent::Resized(new_size) => {
                    if let Some(webview) = controller.get() {
                        let r = RECT {
                            left: 0,
                            top: 0,
                            right: new_size.width as i32,
                            bottom: new_size.height as i32,
                        };
                        webview.put_bounds(r).ok();
                    }
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                // Updates here
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {}
            _ => {}
        }
    });
}

// Wrapper for exitting and logging
#[cfg(target_os = "windows")]
pub fn bootstrap_webview2_wrap() {
    use std::process::exit;
    match bootstrap_webview2() {
        Ok(r) => match r {
            true => {}
            false => {
                error!("webview2 bootstrap installation was successful, however webview2 failed to detect it.");
                exit(2);
            }
        },
        Err(e) => {
            error!("Failed bootstrapping webview2: {}", e);
            exit(1);
        }
    }
}

// Install evergreen webview2 for Windows
#[cfg(target_os = "windows")]
fn bootstrap_webview2() -> Result<bool, Box<dyn Error>> {
    use std::process::Command;
    use tempfile::tempdir;
    // Already installed
    if webview2::get_available_browser_version_string(None).is_ok() {
        return Ok(true);
    }

    info!("Bootstrapping webview2...");
    // Download
    let dir = tempdir()?;
    let path = dir.path().join("evergreen.exe");
    {
        let mut file = File::create(&path)?;
        let mut res = reqwest::blocking::get("https://go.microsoft.com/fwlink/p/?LinkId=2124703")?;
        std::io::copy(&mut res, &mut file)?;
    }

    // Run
    Command::new(path.to_str().ok_or("Invalid path")?).status()?;
    dir.close().ok();

    // Verify
    Ok(webview2::get_available_browser_version_string(None).is_ok())
}

// OneTagger Error, meant for UI
#[derive(Debug, Clone)]
pub struct OTError {
    message: String,
}
impl OTError {
    pub fn new(msg: &str) -> OTError {
        OTError {
            message: msg.to_owned(),
        }
    }
}
impl Error for OTError {}
impl fmt::Display for OTError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
