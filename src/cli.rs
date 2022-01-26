use std::fs::File;
use clap::{Parser, Subcommand};

use crate::VERSION;
use crate::tagger::spotify::Spotify;
use crate::tagger::{TaggerConfig, Tagger};
use crate::ui::audiofeatures::{AudioFeaturesConfig, AudioFeatures};
use crate::ui::{self, StartContext};

/// Parse args and continue
pub fn parse_args() {
    let cli = Cli::parse();

    // Windows Webview2 Bootstrap
    if cli.bootstrap_webview2 {
        #[cfg(target_os = "windows")]
        {
            ui::bootstrap_webview2_wrap();
            return;
        }
        panic!("Windows only install option!");
    }

    // Default configs
    if cli.autotagger_config {
        let config = serde_json::to_string_pretty(&TaggerConfig::default()).expect("Failed serializing default config!");
        println!("{config}");
        return;
    }
    if cli.audiofeatures_config {
        let config = serde_json::to_string_pretty(&AudioFeaturesConfig::default()).expect("Failed serializing config!");
        println!("{config}");
        return;
    }

    info!("\n\nStarting OneTagger v{VERSION} Commit: {} OS: {}\n\n", env!("COMMIT"), std::env::consts::OS);

    // Start GUI version
    if cli.action.is_none() {
        // Start
        let context = StartContext {
            start_path: cli.path, 
            server_mode: cli.server, 
            expose: cli.expose
        };
        ui::start_all(context);
    }

    // CLI
    if let Some(action) = cli.action {
        match &action {
            Actions::Autotagger { path, config } => {
                let file = File::open(config).expect("Failed reading config file!");
                let config = serde_json::from_reader(&file).expect("Failed parsing config file!");
                let files = Tagger::get_file_list(&path);
                let rx = Tagger::tag_files(&config, files);
                let start = timestamp!();
                for status in rx {
                    debug!("{status:?}");
                }
                info!("Tagging finished, took: {} seconds.", (timestamp!() - start) / 1000);
            },
            Actions::Audiofeatures { path, config, client_id, client_secret } => {
                let file = File::open(config).expect("Failed reading config file!");
                let config = serde_json::from_reader(&file).expect("Failed parsing config file!");
                // Auth spotify
                let spotify = Spotify::try_cached_token(client_id, client_secret)
                    .expect("Spotify unauthorized, please run the authorize-spotify option or login to Spotify in UI at least once!");
                
                let files = Tagger::get_file_list(&path);
                let rx = AudioFeatures::start_tagging(config, spotify, files);
                let start = timestamp!();
                for status in rx {
                    debug!("{status:?}");
                }
                info!("Tagging finished, took: {} seconds.", (timestamp!() - start) / 1000);
            },
            // Spotify OAuth flow
            Actions::AuthorizeSpotify { client_id, client_secret, expose } => {
                let (auth_url, mut oauth) = Spotify::generate_auth_url(&client_id, &client_secret);
                println!("Please go to the following URL and authorize 1T:\n{auth_url}");
                // should cache the token
                let _spotify = Spotify::auth_server(&mut oauth, *expose).expect("Spotify authentication failed!");
            }
        }
    }

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

    #[clap(subcommand)]
    action: Option<Actions>,
    
    /// Windows only installer option
    #[clap(long)]
    bootstrap_webview2: bool,

    /// Prints the default Autotagger config and exits
    #[clap(long)]
    autotagger_config: bool,

    /// Prints the default Audio Features config and exits
    #[clap(long)]
    audiofeatures_config: bool,
}

#[derive(Subcommand, Debug, Clone)]
enum Actions {
    /// Start Autotagger in CLI mode
    Autotagger {
        /// Path to music files (overrides config)
        #[clap(short, long)]
        path: String,

        /// Specify a path to config file
        #[clap(short, long)]
        config: String
    },
    /// Start Audio Features in CLI mode
    Audiofeatures {
        /// Path to music files (overrides config)
        #[clap(short, long)]
        path: String,

        /// Specify a path to config file
        #[clap(short, long)]
        config: String,

        /// Spotify Client ID
        #[clap(long)]
        client_id: String,

        /// Spotify Client Secret
        #[clap(long)]
        client_secret: String
    },
    /// Authorize Spotify and cache the token
    AuthorizeSpotify {
        /// Spotify Client ID
        #[clap(long)]
        client_id: String,
        
        /// Spotify Client Secret
        #[clap(long)]
        client_secret: String,

        /// Run Spotify authentication callback server on `0.0.0.0`
        #[clap(long)]
        expose: bool
    }
}
