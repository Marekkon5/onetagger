use clap::{Parser, Subcommand, ArgEnum};

use crate::VERSION;
use crate::tagger::{MusicPlatform, TaggerConfig, Tagger};
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
            Actions::Autotagger { path, .. } => {
                let config = action.generate_at_config();
                let files = Tagger::get_file_list(&path);
                let rx = Tagger::tag_files(&config, files);
                let start = timestamp!();
                for status in rx {
                    debug!("{status:?}");
                }
                info!("Tagging finished, took: {} seconds.", (timestamp!() - start) / 1000);
            },
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
}

#[derive(Subcommand, Debug, Clone)]
enum Actions {
    /// Start autotagger in CLI mode
    Autotagger {
        #[clap(short, long, parse(try_from_str = ConfigTagsWrap::parse_arg))]
        tags: ConfigTagsWrap,

        /// List of music platforms to fetch data from
        #[clap(short = 'P', long, parse(try_from_str = MusicPlatformsArgWrap::parse_arg))]
        platforms: MusicPlatformsArgWrap,

        /// Use ID3v2.4 for MP3 and AIFF files
        #[clap(long)]
        id3v24: bool,

        /// How many tracks to tag concurrently (not supported on every platform)
        #[clap(long, default_value_t = 8)]
        threads: u16,

        /// How strict should the matching process be (0% to 100%)
        #[clap(long, default_value_t = 80)]
        strictness: u8,

        /// Path to music files
        #[clap(short, long)]
        path: String,
    }
}

impl Actions {
    /// Generate autotagger config
    pub fn generate_at_config(&self) -> TaggerConfig {
        match self {
            Actions::Autotagger { tags, platforms, id3v24, threads, strictness, path } => {
                if *strictness > 100 {
                    panic!("Invalid strictness!");
                }

                TaggerConfig {
                    platforms: platforms.0.iter().map(|p| (*p).into()).collect(),
                    path: Some(path.to_string()),
                    title: tags.0.contains(&ConfigTags::Title),
                    artist: tags.0.contains(&ConfigTags::Artist),
                    album: tags.0.contains(&ConfigTags::Album),
                    key: tags.0.contains(&ConfigTags::Key),
                    bpm: tags.0.contains(&ConfigTags::BPM),
                    genre: tags.0.contains(&ConfigTags::Genre),
                    style: tags.0.contains(&ConfigTags::Style),
                    label: tags.0.contains(&ConfigTags::Label),
                    release_date: tags.0.contains(&ConfigTags::ReleaseDate),
                    publish_date: tags.0.contains(&ConfigTags::PublishDate),
                    album_art: tags.0.contains(&ConfigTags::AlbumArt),
                    other_tags: tags.0.contains(&ConfigTags::Other),
                    catalog_number: tags.0.contains(&ConfigTags::CatalogNumber),
                    url: tags.0.contains(&ConfigTags::URL),
                    track_id: tags.0.contains(&ConfigTags::TrackID),
                    release_id: tags.0.contains(&ConfigTags::ReleaseID),
                    version: tags.0.contains(&ConfigTags::Version),
                    duration: tags.0.contains(&ConfigTags::Duration),
                    album_artist: tags.0.contains(&ConfigTags::AlbumArtist),
                    remixer: tags.0.contains(&ConfigTags::Remixer),
                    track_number: tags.0.contains(&ConfigTags::TrackNumber),
                    isrc: tags.0.contains(&ConfigTags::ISRC),
                    meta_tags: tags.0.contains(&ConfigTags::Meta),
                    id3v24: *id3v24,
                    threads: *threads,
                    strictness: *strictness as f64 / 100.0,
                    ..Default::default()
                }
            },
            // Should be handled earlier on
            // _ => unreachable!()
        }
    }
}


/// Wrapper so it can be used as argument
#[derive(Debug, Clone, Copy, ArgEnum)]
enum MusicPlatformsArg {
    Beatport,
    Traxsource,
    Discogs,
    Junodownload,
    Itunes,
    Musicbrainz,
    Beatsource,
    Spotify,
}

impl Into<MusicPlatform> for MusicPlatformsArg {
    fn into(self) -> MusicPlatform {
        match self {
            MusicPlatformsArg::Beatport => MusicPlatform::Beatport,
            MusicPlatformsArg::Traxsource => MusicPlatform::Traxsource,
            MusicPlatformsArg::Discogs => MusicPlatform::Discogs,
            MusicPlatformsArg::Junodownload => MusicPlatform::JunoDownload,
            MusicPlatformsArg::Itunes => MusicPlatform::ITunes,
            MusicPlatformsArg::Musicbrainz => MusicPlatform::MusicBrainz,
            MusicPlatformsArg::Beatsource => MusicPlatform::Beatsource,
            MusicPlatformsArg::Spotify => MusicPlatform::Spotify,
        }
    }
}

/// Wrapper for parsing arguments
#[derive(Debug, Clone)]
struct MusicPlatformsArgWrap(Vec<MusicPlatformsArg>);

impl MusicPlatformsArgWrap {
    /// Parse comma separated argument
    pub fn parse_arg(input: &str) -> Result<MusicPlatformsArgWrap, &'static str> {
        Ok(MusicPlatformsArgWrap(
            input.split(",")
                .filter_map(|i| match MusicPlatformsArg::from_str(i, true).ok() {
                    Some(i) => Some(i),
                    None => {
                        warn!("Invalid platform: {i}");
                        None
                    }
                }).collect::<Vec<_>>()))
    }
}


/// Wrapper for parsing arguments
#[derive(Debug, Clone, ArgEnum, PartialEq, Eq, Copy)]
enum ConfigTags {
    Title, Artist, Album, Key, BPM, Genre, Style, Label, ReleaseDate, PublishDate,
    AlbumArt, Other, CatalogNumber, URL, TrackID, ReleaseID, Version, Duration, 
    AlbumArtist, Remixer, TrackNumber, ISRC, Meta
}

/// Wrapper for parsing arguments
#[derive(Debug, Clone)]
struct ConfigTagsWrap(Vec<ConfigTags>);

impl ConfigTagsWrap {
    /// Parse comma separated argument
    pub fn parse_arg(input: &str) -> Result<ConfigTagsWrap, &'static str> {
        Ok(ConfigTagsWrap(
            input.split(",")
                .filter_map(|i| match ConfigTags::from_str(i, true).ok() {
                    Some(i) => Some(i),
                    None => {
                        warn!("Invalid tag: {i}");
                        None
                    }
                }).collect::<Vec<_>>()))
    }
}