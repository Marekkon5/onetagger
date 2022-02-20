#[macro_use] extern crate log;
#[macro_use] extern crate onetagger_shared;

use std::fs::File;
use clap::{Parser, Subcommand};
use onetagger_platforms::spotify::Spotify;
use onetagger_shared::{VERSION, COMMIT};
use onetagger_tagger::TaggerConfig;
use onetagger_autotag::audiofeatures::{AudioFeaturesConfig, AudioFeatures};
use onetagger_autotag::Tagger;


fn main() {
    onetagger_shared::setup();
    let cli = Cli::parse();

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

    if cli.action.is_none() {
        println!("No action. Use onetagger-cli --help to get print help.");
        return;
    }

    info!("\n\nStarting OneTagger v{VERSION} Commit: {COMMIT} OS: {}\n\n", std::env::consts::OS);


    match &cli.action.unwrap() {
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
        Actions::AuthorizeSpotify { client_id, client_secret, expose, prompt } => {
            let (auth_url, mut oauth) = Spotify::generate_auth_url(&client_id, &client_secret);
            println!("\nPlease go to the following URL and authorize 1T:\n{auth_url}");
            // should cache the token
            match prompt {
                true => {
                    println!("\nEnter the URL you were redirected to and press enter: ");
                    let mut url = String::new();
                    std::io::stdin().read_line(&mut url).expect("Couldn't read from stdin!");
                    let _spotify = Spotify::auth_token_code(&mut oauth, url.trim()).expect("Spotify authentication failed!");
                },
                false => {
                    let _spotify = Spotify::auth_server(&mut oauth, *expose).expect("Spotify authentication failed!");
                }
            }
            info!("Succesfully authorized Spotify!");
        }
    
    }
}


#[derive(Parser, Debug, Clone)]
#[clap(version)]
struct Cli {
    /// What should OneTagger do
    #[clap(subcommand)]
    action: Option<Actions>,
    
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
        config: String,
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
        expose: bool,

        /// Don't start server, prompt for the redirected URL 
        #[clap(long)]
        prompt: bool
    }
}