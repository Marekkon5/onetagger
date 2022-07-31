use onetagger_player::AudioSources;
use rouille::{router, Response};
use serde::{Serialize, Deserialize};

use crate::quicktag::QuickTagFile;

// TODO: Load full folder
// static BG_PNG: &'static [u8] = include_bytes!("../../../client/dist/bg.png");
// static INDEX_HTML: &'static str = include_str!("../../../client/dist/dist.html");
// static FAVICON_PNG: &'static [u8] = include_bytes!("../../../client/dist/favicon.png");


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
            router!(request, 
                //TODO: Serve static
                // (GET) ["/"] => {
                //     Response::html(INDEX_HTML)
                // },
                // (GET) ["/bg.png"] => {
                //     Response::from_data("image/png", BG_PNG)
                // },
                // (GET) ["/favicon.png"] => {
                //     Response::from_data("image/png", FAVICON_PNG)
                // },
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

    // Server mode
    if context.server_mode {
        start_webserver_thread(&context);
        crate::socket::start_socket_server(context);
        return;
    }

    start_webserver_thread(&context);
    start_socket_thread(context);
    start_webview();
}


// Windows webview
#[cfg(target_os = "windows")]
pub fn start_webview() {
    use std::mem;
    use std::rc::Rc;
    use std::path::Path;
    use once_cell::sync::OnceCell;
    use winit::event_loop::{ControlFlow, EventLoop};
    use winit::event::{Event, WindowEvent};
    use winit::dpi::Size;
    use winit::window::{WindowBuilder, Icon};
    use winit::platform::windows::WindowExtWindows;
    use winapi::shared::windef::{HWND, RECT};
    use winapi::um::winuser::GetClientRect;
    use webview2::Environment;
    use serde_json::json;
    use urlencoding::decode;
    use onetagger_shared::Settings;

    // Install webview2 runtime
    bootstrap_webview2_wrap();
    
    // winit
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("One Tagger")
        .with_inner_size(Size::Logical((1280, 750).into()))
        .with_min_inner_size(Size::Logical((1150, 550).into()))
        .with_window_icon(Some(Icon::from_rgba(include_bytes!("../../../assets/64x64.bin").to_vec(), 64, 64).unwrap()))
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
                                w.post_web_message_as_string(&json!({
                                    "action": "browse",
                                    "path": decoded
                                }).to_string())?;
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
                }
            )
        })
    }.unwrap();

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
                            bottom: new_size.height as i32
                        };
                        webview.put_bounds(r).ok();
                    }
                }
                _ => {}
            }
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
fn bootstrap_webview2() -> Result<bool, Box<dyn std::error::Error>> {
    use tempfile::tempdir;
    use std::process::Command;
    use std::fs::File;
    
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
    Command::new(path.to_str().ok_or("Invalid path")?)
        .status()?;
    dir.close().ok();

    // Verify
    Ok(webview2::get_available_browser_version_string(None).is_ok())
}