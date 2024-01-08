#[macro_use] extern crate log;
#[macro_use] extern crate anyhow;
#[macro_use] extern crate include_dir;
#[macro_use] extern crate onetagger_shared;

use std::net::SocketAddr;
use std::time::Duration;
use anyhow::Error;
use axum::body::Body;
use axum::extract::{Query, WebSocketUpgrade, State, Request, Path};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::Router;
use axum::routing::get;
use include_dir::Dir;
use onetagger_player::AudioSources;
use serde::{Serialize, Deserialize};
use quicktag::QuickTagFile;
use tokio::runtime::Builder;
use tokio::net::TcpListener;
use onetagger_shared::{PORT, WEBSERVER_CALLBACKS};

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

fn start_async_thread(context: StartContext) -> Result<(), Error> {
    let expose = context.expose;
    Builder::new_multi_thread().enable_all().build()?.block_on(async move {
        // Register routes
        let app = Router::new()
            .route("/thumb", get(get_thumb))
            .route("/audio", get(get_audio))
            .route("/ws", get(get_ws))
            .route("/spotify", get(get_spotify_callback))
            .route("/*path", get(get_static_file))
            .with_state(context);

        // Start http server
        let host = match expose {
            true => format!("0.0.0.0:{PORT}"),
            false => format!("127.0.0.1:{PORT}")
        };
        info!("Starting web server on: http://{host}");
        let listener = TcpListener::bind(host).await?;
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;

        Ok::<(), Error>(())
    })?;
    Ok(())
}

/// Serve assets file
async fn get_static_file(Path(mut path): Path<String>) -> impl IntoResponse {
    // Index HTML
    if path == "/" {
        path = "/index.html".to_string();
    }
    path = path[1..].to_string();

    // Static files
    if let Some(file) = CLIENT_DIR.get_file(&path) {
        let mime = mime_guess::from_path(&path).first().unwrap_or(mime::APPLICATION_OCTET_STREAM);
        return (StatusCode::OK, [(CONTENT_TYPE, mime.to_string())], file.contents().to_vec());
    }

    (StatusCode::NOT_FOUND, [(CONTENT_TYPE, "text/plain".to_string())], "Not found".as_bytes().to_vec())
}

#[derive(Debug, Clone, Deserialize)]
struct GetQueryPath {
    path: String
}

/// Serve thumbnail
async fn get_thumb(Query(GetQueryPath { path }): Query<GetQueryPath>) -> impl IntoResponse {
    match QuickTagFile::get_art(&path) {
        Ok(art) => (StatusCode::OK, [(CONTENT_TYPE, "image/jpeg".to_string())], art),
        Err(e) => {
            warn!("Error loading album art: {} File: {}", e, path);
            (StatusCode::NOT_FOUND, [(CONTENT_TYPE, "text/plain".to_string())], format!("Error loading album art: {} File: {}", e, path).into_bytes())
        }
    }
}

/// Serve audio
async fn get_audio(Query(GetQueryPath { path }): Query<GetQueryPath>) -> impl IntoResponse {
    let data = tokio::task::spawn_blocking(move || {
        match AudioSources::from_path(&path).map(|s| s.generate_wav()) {
            Ok(Ok(wav)) => wav,
            Ok(Err(e)) => {
                warn!("Failed generating wav: {e}");
                vec![]
            },
            Err(e) => {
                warn!("Failed opening audio file {path}: {e}");
                vec![]
            }
        }
    }).await.unwrap_or(vec![]);

    // Empty 404 on error
    if data.is_empty() {
        return (StatusCode::NOT_FOUND, [(CONTENT_TYPE, "text/plain")], vec![]);
    }
    (StatusCode::OK, [(CONTENT_TYPE, "audio/wav")], data)
}

/// WS connection
async fn get_ws(ws: WebSocketUpgrade, State(context): State<StartContext>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        debug!("WS Connected!");
        async move {
            match socket::handle_ws_connection(socket, context).await {
                Ok(_) => {},
                Err(e) => warn!("WS connection error: {e}"),
            }
            debug!("WS Disconnected!");
        }
    })
}

/// Spotify token callback
async fn get_spotify_callback(request: Request<Body>) -> impl IntoResponse {
    info!("Got Spotify token from callback");
    WEBSERVER_CALLBACKS.lock().unwrap().insert("spotify".to_string(), request.uri().to_string());
    (StatusCode::OK, [(CONTENT_TYPE, "text/html")], include_str!("../../../assets/spotify_callback.html"))
}

// Start everything
pub fn start_all(context: StartContext) -> Result<(), Error> {
    if context.expose {
        warn!("Server is exposed to public!");
    }

    // Open in browser with 1s delay to allow the srever to load
    if context.browser {
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(1));
            webbrowser::open(&format!("http://127.0.0.1:{PORT}")).ok();
        });
    }

    start_async_thread(context.clone())?;
    Ok(())
}

