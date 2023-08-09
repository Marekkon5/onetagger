#![windows_subsystem = "windows"]

#[macro_use] extern crate log;
#[macro_use] extern crate include_dir;
#[macro_use] extern crate onetagger_shared;

use std::error::Error;
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

    info!("\n\nStarting OneTagger v{VERSION} Commit: {COMMIT} OS: {}\n\n", std::env::consts::OS);
    
    // MacOS
    if !cli.server {
        old_macos_warning().ok();
    }
            
    // Start
    let context = StartContext {
        start_path: cli.path, 
        server_mode: cli.server, 
        expose: cli.expose,
        browser: cli.browser
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

    /// Open in browser
    #[clap(long)]
    browser: bool,
}

/// Show warning for old macOS
#[cfg(target_os = "macos")]
fn old_macos_warning() -> Result<(), Box<dyn Error>> {
    use std::process::Command;
    use native_dialog::MessageDialog;

    // Get version
    let output = Command::new("sw_vers")
        .arg("-productVersion")
        .output()?
        .stdout;
    let version = String::from_utf8(output)?;
    // Show warning
    if version.starts_with("10.") && !version.contains("10.15") {
        let server_version = MessageDialog::new()
            .set_title("Unsupported version")
            .set_text("In order to use One Tagger on older macOS, install a more recent browser like Google Chrome and set it as default browser. Click Yes to restart One Tagger into server mode and open in browser, No to open normally.")
            .show_confirm()?;

        if server_version {
            Command::new("osascript")
                .arg("-e")
                .arg(format!(
                    "tell application \"Terminal\" to do script \"{} --server --browser\"",
                    std::env::args().next().unwrap()
                ))
                .output()
                .ok();
            std::process::exit(0);
        }
    }
    Ok(())

}

/// Show warning for old macOS
#[cfg(not(target_os = "macos"))]
fn old_macos_warning() -> Result<(), Box<dyn Error>> { Ok(()) }