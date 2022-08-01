#![windows_subsystem = "windows"]

#[macro_use] extern crate log;
#[macro_use] extern crate include_dir;
#[macro_use] extern crate onetagger_shared;

use clap::Parser;
use onetagger_shared::{VERSION, COMMIT};

use crate::ui::StartContext;

mod ui;
mod socket;
mod browser;
mod quicktag;
mod tageditor;

fn main() {
    // Setup 1T
    onetagger_shared::setup();
    let cli = Cli::parse();

    // Windows Webview2 Bootstrap
    if cli.bootstrap_webview2 {
        #[cfg(target_os = "windows")]
        {
            ui::bootstrap_webview2_wrap();
            return;
        }
        #[cfg(not(target_os = "windows"))]
        panic!("Windows only install option!");
    }

    info!("\n\nStarting OneTagger v{VERSION} Commit: {COMMIT} OS: {}\n\n", std::env::consts::OS);

    // Start
    let context = StartContext {
        start_path: cli.path, 
        server_mode: cli.server, 
        expose: cli.expose,
    };
    ui::start_all(context);
}

#[derive(Parser, Debug, Clone)]
#[clap(version)]
struct Cli {
    /// Start in server mode (no webview window)
    #[clap(short = 'S', long)]
    server: bool,

    /// Make the server listen on `0.0.0.0`
    #[clap(long)]
    expose: bool,

    /// Path to music files
    #[clap(short, long)]
    path: Option<String>,
    
    /// Windows only installer option
    #[clap(long)]
    bootstrap_webview2: bool,
}