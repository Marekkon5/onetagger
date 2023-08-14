use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;
use include_dir::Dir;
use onetagger_player::AudioSources;
use onetagger_shared::Settings;
use rouille::{router, Response};
use serde::{Serialize, Deserialize};
use wry::application::dpi::{Size, PhysicalSize};
use wry::application::event::{StartCause, Event, WindowEvent};
use wry::application::event_loop::{ControlFlow, EventLoopBuilder};
use wry::application::window::{WindowBuilder, Theme, Icon};
use wry::webview::{WebViewBuilder, FileDropEvent, WebContext};

use crate::quicktag::QuickTagFile;

static CLIENT_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../client/dist");

// Should have data from arguments and other flags (eg. port / host in future)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartContext {
    pub server_mode: bool,
    pub start_path: Option<String>,
    pub expose: bool,
    pub browser: bool,
}

/// Start webview window
pub fn start_webview() -> Result<(), Box<dyn Error>> {
    // Setup wry
    let event_loop = EventLoopBuilder::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let window = WindowBuilder::new()
        .with_title("One Tagger")
        .with_min_inner_size(Size::Physical(PhysicalSize::new(1150, 550)))
        .with_inner_size(Size::Physical(PhysicalSize::new(1280, 720)))
        .with_resizable(true)
        .with_window_icon(Some(Icon::from_rgba(include_bytes!("../../../assets/64x64.bin").to_vec(), 64, 64).unwrap()))
        .with_theme(Some(Theme::Dark))
        .build(&event_loop)?;
    window.set_inner_size(Size::Physical(PhysicalSize::new(1280, 720)));
    let mut context = WebContext::new(Some(Settings::get_folder()?.join("webview")));
    let mut webview = WebViewBuilder::new(window)?
        .with_url("http://127.0.0.1:36913")?
        .with_web_context(&mut context);

    // Windows webview2 does NOT support custom DnD, janky workaround
    if cfg!(target_os = "windows") {
        // Handler
        let proxy = proxy.clone();
        let handle_url = move |url: String| -> bool {
            debug!("Navigation/NewWindow to: {url}");
            if url.starts_with("file://") {
                let url = url.replace("file:///", "");
                let path = urlencoding::decode(&url).map(|r| r.to_string()).unwrap_or(url).replace("/", "\\");
                proxy.send_event(CustomWindowEvent::DropFolder(path.into())).ok();
                return false;
            }
            true
        };
        
        // Register
        webview = webview.with_navigation_handler(handle_url.clone());
        webview = webview.with_new_window_req_handler(handle_url);
    }

    // Handle dropped folders (for all other than Windows)
    if cfg!(not(target_os = "windows")) {
        webview = webview.with_file_drop_handler(move |_window, event| {
            match event {
                FileDropEvent::Dropped { mut paths, .. } => {
                    if paths.len() > 1 || paths.is_empty() {
                        warn!("Drop only 1 path!");
                        return true;
                    }
                    let path = paths.remove(0);
                    if path.is_dir() {
                        proxy.send_event(CustomWindowEvent::DropFolder(path)).ok();
                        return true;
                    }
                    if path.is_file() {
                        return false;
                    }
                },
                _ => {}
            }

            true
        });
    }

    // Create webview
    let webview = webview.build()?;

    // Event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => debug!("Started webview!"),
            // Check for unsaved progress
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                match webview.evaluate_script("window.onWebviewEvent({\"action\": \"exit\"})") {
                    Ok(_) => {},
                    Err(e) => {
                        warn!("Failed to ask for exit: {e}");
                        *control_flow = ControlFlow::Exit;
                    }
                }

            },
            // Drop folder to client
            Event::UserEvent(CustomWindowEvent::DropFolder(path)) => {
                match webview.evaluate_script(&format!("window.onWebviewEvent({{\"action\": \"browse\", \"path\": \"{}\"}})", path.to_string_lossy().replace("\\", "\\\\").replace("\"", "\\\""))) {
                    Ok(_) => {},
                    Err(e) => error!("Failed executing JS on webview: {e}"),
                }
            }
            _ => ()
        }

    });
}

enum CustomWindowEvent {
    DropFolder(PathBuf)
}

// Start WebSocket server
pub fn start_socket_thread(context: StartContext) {
    std::thread::spawn(move || {
        crate::socket::start_socket_server(context);
    });
}

// Start webserver for hosting static index.html
pub fn start_webserver_thread(context: &StartContext) {
    let host = match context.expose {
        true => "0.0.0.0:36913",
        false => "127.0.0.1:36913"
    };

    std::thread::spawn(move || {
        rouille::start_server(host, move |request| {
            // Path to static file
            let mut path = request.url();
            if path == "/" {
                path = "/index.html".to_string();
            }
            path = path[1..].to_string();

            // Static files
            if let Some(file) = CLIENT_DIR.get_file(&path) {
                let mime = mime_guess::from_path(&path).first().unwrap_or(mime::APPLICATION_OCTET_STREAM);
                return Response::from_data(mime.to_string(), file.contents());
            }

            router!(request, 
                // Get thumbnail of image from tag by path
                (GET) ["/thumb"] => {
                    match request.get_param("path") {
                        Some(path) => {
                            match QuickTagFile::get_art(&path) {
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
                // Audio stream
                (GET) ["/audio"] => {
                    match request.get_param("path") {
                        Some(path) => {
                            match AudioSources::from_path(&path).map(|s| s.generate_wav()) {
                                Ok(Ok(wav)) => {
                                    Response::from_data("audio/wav", wav)
                                },
                                Ok(Err(e)) => {
                                    warn!("Failed generating wav: {e}");
                                    Response::empty_404()
                                },
                                Err(e) => {
                                    warn!("Failed opening audio file {path}: {e}");
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
        },
        false => info!("Starting server on http://127.0.0.1:36913 ws://127.0.0.1:36912")
    }

    // Open in browser with 1s delay to allow the srever to load
    if context.browser {
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(1));
            webbrowser::open("http://127.0.0.1:36913").ok();
        });
    }

    // Server mode
    if context.server_mode {
        start_webserver_thread(&context);
        crate::socket::start_socket_server(context);
        return;
    }

    start_webserver_thread(&context);
    start_socket_thread(context);
    start_webview().expect("Failed to start webview");
}

