#[macro_use] extern crate log;
#[macro_use] extern crate anyhow;
#[macro_use] extern crate include_dir;
#[macro_use] extern crate onetagger_shared;

use std::net::SocketAddr;
use std::net::TcpListener as StdListener;
use anyhow::Error;
use axum::body::Body;
use axum::extract::{Query, WebSocketUpgrade, State, Request};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::Router;
use axum::routing::{get, post};
use axum::middleware;
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
pub mod session;
mod messages;

static CLIENT_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../client/dist");

// Should have data from arguments and other flags (eg. port / host in future)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartContext {
    pub server_mode: bool,
    pub start_path: Option<String>,
    pub expose: bool,
    pub browser: bool,
    #[serde(skip)]
    pub session: session::SessionSecret,
}

impl StartContext {
    pub fn new(server_mode: bool, start_path: Option<String>, expose: bool, browser: bool) -> Self {
        Self { server_mode, start_path, expose, browser, session: Default::default() }
    }
}

/// Bind in the caller before handing the launch capability to any UI.
pub fn prepare_server(context: StartContext) -> Result<impl FnOnce() -> Result<(), Error>, Error> {
    let host = if context.expose { "0.0.0.0" } else { "127.0.0.1" };
    let listener = StdListener::bind((host, PORT as u16))
        .map_err(|error| anyhow!("Cannot start OneTagger on {host}:{PORT}: {error}"))?;
    listener.set_nonblocking(true)?;
    let runtime = Builder::new_multi_thread().enable_all().build()?;
    if context.expose { warn!("Server is exposed to public!"); }
    let launch_url = context.session.launch_url();
    println!("Open OneTagger: {launch_url}");
    Ok(move || runtime.block_on(async move {
        let listener = TcpListener::from_std(listener)?;
        if context.browser {
            std::thread::spawn(move || { webbrowser::open(&launch_url).ok(); });
        }
        info!("Listening on http://{host}:{PORT}");
        axum::serve(listener, router(context).into_make_service_with_connect_info::<SocketAddr>()).await?;
        Ok(())
    }))
}

// Keep the production route boundary directly testable without binding a port or audio device.
pub(crate) fn router(context: StartContext) -> Router {
    Router::new()
        .route("/thumb", get(get_thumb))
        .route("/audio", get(get_audio))
        .route("/ws", get(get_ws))
        .route("/session", get(|| async { StatusCode::NO_CONTENT }))
        .route_layer(middleware::from_fn_with_state(context.clone(), session::require_session))
        .route("/session", post(session::establish))
        .route("/spotify", get(get_spotify_callback))
        .route("/{*path}", get(get_static_file))
        .route("/", get(get_static_file))
        .with_state(context)
}

/// Serve assets file
async fn get_static_file(request: Request<Body>) -> impl IntoResponse {
    let mut path = request.uri().path().to_string();
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
    ws.max_message_size(16 * 1024 * 1024).max_frame_size(16 * 1024 * 1024).on_upgrade(move |socket| {
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

pub fn start_all(context: StartContext) -> Result<(), Error> {
    prepare_server(context)?()
}
