#[macro_use] extern crate log;
#[macro_use] extern crate onetagger_shared;

use anyhow::Error;
use onetagger_ui::StartContext;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use clap::{Parser, Subcommand};
use convert_case::{Casing, Case};
use onetagger_platforms::spotify::Spotify;
use onetagger_renamer::{RenamerConfig, Renamer, TemplateParser};
use onetagger_shared::{VERSION, COMMIT};
use onetagger_autotag::audiofeatures::{AudioFeaturesConfig, AudioFeatures};
use onetagger_autotag::{Tagger, TaggerConfigExt, AudioFileInfoImpl};
use onetagger_tagger::{TaggerConfig, AudioFileInfo, SupportedTag};

fn input_files(path: &Path, subfolders: bool) -> Result<Vec<PathBuf>, Error> {
    if path.is_dir() { return Ok(AudioFileInfo::get_file_list(path, subfolders)); }
    anyhow::ensure!(path.is_file(), "Input file or directory does not exist: {}", path.display());
    if onetagger_playlist::is_audio_file(path) { return Ok(vec![path.to_owned()]); }
    onetagger_playlist::get_files_from_playlist_file(path)
}

fn input_or_exit(path: &Path, subfolders: bool) -> Vec<PathBuf> {
    input_files(path, subfolders).unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(2);
    })
}

fn main() {
    let cli = Cli::parse();

    // Default configs
    if cli.autotagger_config {
        let config = serde_json::to_string_pretty(&TaggerConfig::custom_default()).expect("Failed serializing default config!");
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

    // Setup logging
    onetagger_shared::setup();
    info!("\n\nStarting OneTagger v{VERSION} Commit: {COMMIT} OS: {}\n\n", std::env::consts::OS);


    let action = cli.action.unwrap();
    match &action {
        Actions::Autotagger { path, report, selection_manifest, .. } => {
            let config = action.get_at_config().expect("Failed loading config file!");
            debug!("{:?}", config);

            // Get files
            let selection = selection_manifest.as_ref().map(|path| {
                let data = std::fs::read_to_string(path)?;
                onetagger_autotag::report::Selection::parse(&data)
            }).transpose().expect("Invalid Helper selection manifest");
            let files = match &selection {
                Some(selection) => selection.files(),
                None => input_or_exit(path, config.include_subfolders),
            };

            let finished = Arc::new(Mutex::new(None));
            let rx = if *report || selection.is_some() {
                Tagger::tag_files_report(&config, files, finished.clone(), selection).expect("Invalid report settings")
            } else { Tagger::tag_files(&config, files, finished.clone()) };
            let start = timestamp!();
            for status in rx {
                debug!("{status:?}");
            }
            info!("Tagging finished, took: {} seconds.", (timestamp!() - start) / 1000);
            let finished = finished.lock().unwrap();
            if let Some(result) = finished.as_ref() {
                if let Some(path) = &result.report_file { println!("Results JSON: {path}"); }
                if let Some(error) = &result.report_error { eprintln!("{error}"); std::process::exit(1); }
            }
            match finished.as_ref() {
                Some(result) if result.move_errors.is_empty() => {},
                Some(result) => {
                    for error in &result.move_errors { eprintln!("File move failed: {error}"); }
                    std::process::exit(1);
                },
                None => {
                    eprintln!("Tagging ended without a complete result report; check the application log.");
                    std::process::exit(1);
                },
            }
        },
        Actions::Audiofeatures { path, config, client_id, client_secret, no_subfolders } => {
            let file = File::open(config).expect("Failed reading config file!");
            let config: AudioFeaturesConfig = serde_json::from_reader(&file).expect("Failed parsing config file!");
            // Cli subfolders override
            let mut subfolders = config.include_subfolders;
            if *no_subfolders {
                subfolders = false;
            }
            // Auth spotify
            let spotify = Spotify::try_cached_token(client_id, client_secret)
                .expect("Spotify unauthorized, please run the authorize-spotify option or login to Spotify in UI at least once!");

            // Get files
            let files = input_or_exit(path, subfolders);

            let rx = AudioFeatures::start_tagging(config, spotify, files);
            let start = timestamp!();
            for status in rx {
                debug!("{status:?}");
            }
            info!("Tagging finished, took: {} seconds.", (timestamp!() - start) / 1000);
        },
        // Spotify OAuth flow
        Actions::AuthorizeSpotify { client_id, client_secret, prompt, expose } => {
            let (auth_url, client) = Spotify::generate_auth_url(&client_id, &client_secret).expect("Failed generating auth URL!");
            println!("\nPlease go to the following URL and authorize 1T:\n{auth_url}");
            // should cache the token
            match prompt {
                true => {
                    println!("\nEnter the URL you were redirected to and press enter: ");
                    let mut url = String::new();
                    std::io::stdin().read_line(&mut url).expect("Couldn't read from stdin!");
                    let _spotify = Spotify::auth_token_code(client, url.trim()).expect("Spotify authentication failed!");
                },
                false => {
                    let expose = *expose;
                    std::thread::spawn(move || {
                        onetagger_ui::start_all(StartContext::new(true, None, expose, false)).expect("Failed starting server!");
                    });
                    let _spotify = Spotify::auth_server(client).expect("Spotify authentication failed!");
                }
            }
            info!("Succesfully authorized Spotify!");
            // Exit because of webserver
            std::process::exit(0);
        },
        // Renamer
        Actions::Renamer { path, output, template, copy, no_subfolders, preview, overwrite, separator, keep_subfolders } => {
            let config = RenamerConfig {
                path: path.to_owned(),
                out_dir: output.to_owned(),
                template: template.to_string(),
                copy: *copy,
                subfolders: !*no_subfolders,
                overwrite: *overwrite,
                separator: separator.to_string(),
                keep_subfolders: *keep_subfolders,
            };
            let mut renamer = Renamer::new(TemplateParser::parse(&template));
            let files = AudioFileInfo::load_files_iter(&config.path, config.subfolders, None, None);
            let names = renamer.generate(files, &config).expect("Failed generating filenames!");

            // Only preview
            if *preview {
                for (i, (from, to)) in names.iter().enumerate() {
                    println!("{}. {:?} -> {:?}", i + 1, from, to);
                }
                return;
            }

            let report = renamer.rename(&names, &config).expect("Failed renaming!");
            println!("{} completed, {} unchanged, {} failed/conflicted", report.succeeded, report.skipped, report.failed);
            for result in &report.results {
                if let Some(message) = &result.message {
                    eprintln!("{} -> {}: {}", result.from.display(), result.to.display(), message);
                }
            }
            if report.failed > 0 { std::process::exit(1); }
        },
        // Server mode
        Actions::Server { expose, path, browser } => {
            onetagger_ui::start_all(StartContext::new(true, path.clone().map(String::from), *expose, *browser)).expect("Failed starting the server");
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
        /// Audio file, music directory or M3U playlist (overrides config)
        #[clap(short, long)]
        path: PathBuf,

        /// Save a JSON report with actual before/after tags and provider evidence
        #[clap(long)]
        report: bool,

        /// Helper JSON selection; overrides the input path and implies --report
        #[clap(long)]
        selection_manifest: Option<PathBuf>,

        /// Specify a path to config file
        #[clap(short, long)]
        config: Option<PathBuf>,

        /// Comma separated list of platforms to use. For custom platforms use the library filename
        #[clap(short = 'P', long)]
        platforms: Option<String>,

        /// Comma separated list of tags to use
        #[clap(short, long)]
        tags: Option<String>,

        /// Use ID3v2.4 instead of IDv2.3 for MP3/AIFF files
        #[clap(long)]
        id3v24: bool,

        /// Overwrite the existing tags in the track
        #[clap(long)]
        overwrite: bool,

        /// How many threads to use for the searching & matching process
        #[clap(long)]
        threads: Option<u16>,

        /// How strict should the matching be? Use: 0 - 100, Default: 80 (%).
        #[clap(long)]
        strictness: Option<u8>,

        /// Writes a cover.jpg into the folder
        #[clap(long)]
        album_art_file: bool,

        /// Merge new genres with existing ones
        #[clap(long)]
        merge_genres: bool,

        /// Write the key tag in CAMELOT format
        #[clap(long)]
        camelot: bool,

        /// Write title tag without version (ie. remix)
        #[clap(long)]
        short_title: bool,

        /// Match the song duration as well (WARNING: very strict)
        #[clap(long)]
        match_duration: bool,

        /// If duration matching is enabled, how big the difference in durations can be (in seconds)
        #[clap(long)]
        max_duration_difference: Option<u64>,

        /// Use platform specific ID tags to get exact matches
        #[clap(long)]
        match_by_id: bool,

        /// Try to indentify the track on Shazam if title & artist tags are missing
        #[clap(long)]
        enable_shazam: bool,

        /// Always try to indentify the track on Shazam
        #[clap(long)]
        force_shazam: bool,

        /// Skip tracks that have 1T_TAGGEDDATE tag
        #[clap(long)]
        skip_tagged: bool,

        /// Try to get title & artist from filename if the tags are missing
        #[clap(long)]
        parse_filename: bool,

        /// Template for parse_filename option. Example: `%track$. %artists% - %title%`
        #[clap(long)]
        filename_template: Option<String>,

        /// Don't include subfolders
        #[clap(long)]
        no_subfolders: bool,

        /// Write only year instead of full date
        #[clap(long)]
        only_year: bool,

        /// Tag on multiple platforms instead of the default fallback mode
        #[clap(long)]
        multiplatform: bool,
    },
    /// Start Audio Features in CLI mode
    Audiofeatures {
        /// Audio file, music directory or M3U playlist (overrides config)
        #[clap(short, long)]
        path: PathBuf,

        /// Specify a path to config file
        #[clap(short, long)]
        config: String,

        /// Spotify Client ID
        #[clap(long)]
        client_id: String,

        /// Spotify Client Secret
        #[clap(long)]
        client_secret: String,

        /// Don't include subfolders
        #[clap(long)]
        no_subfolders: bool,
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
    },
    Renamer {
        /// Path to input files
        #[clap(long, short)]
        path: PathBuf,

        /// Output directory
        #[clap(long, short)]
        output: Option<PathBuf>,

        /// New filename template
        #[clap(long, short)]
        template: String,

        /// Copy files instead of moving
        #[clap(long)]
        copy: bool,

        /// Exclude subfolders 
        #[clap(long)]
        no_subfolders: bool,

        /// Don't actually affect files, only generate new names
        #[clap(long)]
        preview: bool,

        /// Overwrite files
        #[clap(long)]
        overwrite: bool,

        /// Multiple values separator
        #[clap(long, default_value = ", ")]
        separator: String,

        /// Keep original subfolders
        #[clap(long)]
        keep_subfolders: bool,
    },
    /// Start OneTagger server mode
    Server {
        /// Expose the internal servers (WARNING: Unsecure)
        #[clap(long, short)]
        expose: bool,
        /// Specify initial path to use in UI
        #[clap(long, short)]
        path: Option<String>,
        /// Open web browser
        #[clap(long, short)]
        browser: bool,
    }
}

/// For easily generating CLI -> config
macro_rules! config_option {
    ($target:expr, $t:tt) => {
        if *$t {
            $target.$t = *$t;
        }
    };
    ($target:expr, $($t:tt),+) => {
        $(config_option!($target, $t);)+
    }
}

impl Actions {
    //. Create tagger config
    pub fn get_at_config(&self) -> Result<TaggerConfig, Error> {
        match self {
            Actions::Autotagger { path, config, platforms, tags, id3v24, 
                overwrite, threads, strictness, album_art_file, merge_genres, camelot, 
                short_title, match_duration, max_duration_difference, match_by_id, enable_shazam, force_shazam, 
                skip_tagged, parse_filename, filename_template, no_subfolders, only_year, multiplatform, .. } => {

                // Load config
                let mut config = if let Some(config_path) = config {
                    let config = serde_json::from_reader(&File::open(config_path)?)?;
                    config
                } else {
                    TaggerConfig::custom_default()
                };

                // Overrides
                config.path = Some(path.to_owned());
                if let Some(platforms) = platforms {
                    config.platforms = platforms.split(",").map(String::from).collect();
                }
                // Tags
                if let Some(tags) = tags {
                    let tags: Vec<SupportedTag> = tags
                        .split(",")
                        .filter_map(|t| {
                            match serde_json::from_str(&format!("\"{}\"", t.to_case(Case::Camel))) {
                                Ok(tag) => Some(tag),
                                Err(_) => {
                                    warn!("Invalid tag: {t}");
                                    None
                                },
                            }
                        })
                        .collect();
                    config.tags = tags;
                }
                // Boolean options
                config_option!(config, id3v24, overwrite, album_art_file, merge_genres, camelot, short_title, match_duration,
                    match_by_id, enable_shazam, force_shazam, skip_tagged, parse_filename, only_year, multiplatform);
                // Remaining options
                if let Some(threads) = threads {
                    config.threads = *threads;
                }
                if let Some(strictness) = strictness {
                    if *strictness > 100 {
                        warn!("Invalid stricness!");
                    } else {
                        config.strictness = *strictness as f64 / 100.0;
                    }
                }
                if let Some(mdd) = max_duration_difference {
                    config.max_duration_difference = *mdd;
                }
                if let Some(template) = filename_template {
                    config.filename_template = Some(template.to_string());
                }
                if *no_subfolders {
                    config.include_subfolders = false;
                }
                return Ok(config);
            },
            _ => unreachable!()
        }
    }
}

#[cfg(test)]
mod input_tests {
    use super::*;

    #[test]
    fn direct_audio_playlist_and_directory_resolve_the_expected_paths() {
        let root = tempfile::tempdir().unwrap();
        let literal = root.path().join("Mix%20One.MP3");
        let space = root.path().join("Mix One.mp3");
        std::fs::write(&literal, b"fixture").unwrap();
        std::fs::write(&space, b"other").unwrap();
        let playlist = root.path().join("crate.M3U8");
        std::fs::write(&playlist, "Mix%20One.MP3\n").unwrap();
        assert_eq!(input_files(&literal, false).unwrap(), vec![literal.clone()]);
        assert_eq!(input_files(&playlist, false).unwrap(), vec![literal.clone()]);
        let directory = input_files(root.path(), false).unwrap();
        assert!(directory.contains(&literal) && directory.contains(&space));
        assert!(input_files(&root.path().join("missing.mp3"), false).is_err());
        let unsupported = root.path().join("readme.txt");
        std::fs::write(&unsupported, b"notes").unwrap();
        assert!(input_files(&unsupported, false).is_err());
    }
}
