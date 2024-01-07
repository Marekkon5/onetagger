#[macro_use] extern crate log;
#[macro_use] extern crate anyhow;
#[macro_use] extern crate include_dir;
#[macro_use] extern crate onetagger_shared;

use std::time::Duration;
use include_dir::Dir;
use onetagger_player::AudioSources;
use rouille::{router, Response};
use serde::{Serialize, Deserialize};
use quicktag::QuickTagFile;

pub mod socket;
pub mod browser;
pub mod quicktag;
pub mod tageditor;

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
}

