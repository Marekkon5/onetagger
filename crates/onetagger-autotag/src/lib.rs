#[macro_use] extern crate log;
#[macro_use] extern crate onetagger_shared;

use std::collections::HashMap;
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::time::Duration;
use std::default::Default;
use std::io::prelude::*;
use chrono::Local;
use execute::Execute;
use onetagger_tagger::{FileTaggedStatus, LyricsExt, SupportedTag};
use regex::Regex;
use reqwest::StatusCode;
use walkdir::WalkDir;
use chrono::Datelike;
use serde::{Serialize, Deserialize};
use crossbeam_channel::{unbounded, Sender, Receiver};
use onetagger_tag::{AudioFileFormat, Tag, Field, TagDate, CoverType, TagImpl, EXTENSIONS};
use onetagger_shared::Settings;
use onetagger_player::AudioSources;
use onetagger_tagger::{Track, AudioFileInfo, TaggerConfig, StylesOptions, AutotaggerSource, AutotaggerSourceBuilder, CAMELOT_NOTES};

use crate::shazam::Shazam;
mod shazam;

pub mod platforms;
pub mod audiofeatures;

// Re-exports
pub use platforms::{AUTOTAGGER_PLATFORMS, AutotaggerPlatforms};

lazy_static::lazy_static! {
    /// Stop tagging global variable
    pub static ref STOP_TAGGING: AtomicBool = AtomicBool::new(false);
}

pub trait TaggerConfigExt {
    /// Add custom platform configs to the default config
    fn custom_default() -> TaggerConfig;
}
impl TaggerConfigExt for TaggerConfig {
    fn custom_default() -> TaggerConfig {
        let mut custom = HashMap::new();
        for platform in &AUTOTAGGER_PLATFORMS.0 {
            if !platform.platform.custom_options.options.is_empty() {
                let mut options = HashMap::new();
                for option in &platform.platform.custom_options.options {
                    options.insert(option.id.to_string(), option.value.json_value());
                }
                custom.insert(platform.platform.id.to_string(), serde_json::to_value(options).unwrap());
            }
        }
        let mut default = TaggerConfig::default();
        default.custom = custom;
        default
    }
} 


trait TrackImpl {
    fn write_to_file(&self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<(), Box<dyn Error>>;
    fn download_art(&self, url: &str) -> Result<Option<Vec<u8>>, Box<dyn Error>>;
    fn merge_styles(self, option: &StylesOptions) -> Self;
}

impl TrackImpl for Track {
    // Write tags to file
    fn write_to_file(&self, info: &AudioFileInfo, config: &TaggerConfig) -> Result<(), Box<dyn Error>> {        
        // Get tag
        let mut tag_wrap = Tag::load_file(&info.path, true)?;
        tag_wrap.set_separators(&config.separators);
        let format = tag_wrap.format();

        // Configure format specific
        if let Tag::ID3(t) = &mut tag_wrap {
            t.set_id3v24(config.id3v24);
            if let Some(lang) = config.id3_comm_lang.as_ref() {
                if !lang.is_empty() {
                    t.set_comm_lang(lang.to_string());
                }
            }
        }
        // MP4 Album art override
        if let Tag::MP4(mp4) = &mut tag_wrap {
            // Has art
            if (config.overwrite_tag(SupportedTag::AlbumArt) || mp4.get_art().is_empty()) && self.art.is_some() && config.tag_enabled(SupportedTag::AlbumArt) {
                mp4.remove_all_artworks();
            }
        }
        
        let tag = tag_wrap.tag_mut();
        // Set tags
        if config.tag_enabled(SupportedTag::Title) {
            match config.short_title {
                true => tag.set_field(Field::Title, vec![self.title.to_string()], config.overwrite_tag(SupportedTag::Title)),
                false => tag.set_field(Field::Title, vec![self.full_title()], config.overwrite_tag(SupportedTag::Title))
            }
        }
        // Version
        if config.tag_enabled(SupportedTag::Version) && self.version.is_some() {
            tag.set_field(Field::Version, vec![self.version.as_ref().unwrap().to_string()], config.overwrite_tag(SupportedTag::Version));
        }
        if config.tag_enabled(SupportedTag::Artist) {
            tag.set_field(Field::Artist, self.artists.clone(), config.overwrite_tag(SupportedTag::Artist));
        }
        if config.tag_enabled(SupportedTag::AlbumArtist) && !self.album_artists.is_empty() {
            tag.set_field(Field::AlbumArtist, self.album_artists.clone(), config.overwrite_tag(SupportedTag::AlbumArtist));
        }
        if self.album.is_some() && config.tag_enabled(SupportedTag::Album)  {
            tag.set_field(Field::Album, vec![self.album.as_ref().unwrap().to_string()], config.overwrite_tag(SupportedTag::Album));
        }
        if config.tag_enabled(SupportedTag::Key) && self.key.is_some() {
            let mut value = self.key.as_ref().unwrap().to_string();
            // Convert to camelot
            if config.camelot {
                if let Some((_, c)) = CAMELOT_NOTES.iter().find(|(o, _)| o == &value) {
                    value = c.to_string();
                }
            }
            tag.set_field(Field::Key, vec![value], config.overwrite_tag(SupportedTag::Key));
        }
        if config.tag_enabled(SupportedTag::BPM) && self.bpm.is_some() {
            tag.set_field(Field::BPM, vec![self.bpm.unwrap().to_string()], config.overwrite_tag(SupportedTag::BPM));
        }
        if config.tag_enabled(SupportedTag::Label) && self.label.is_some() {
            tag.set_field(Field::Label, vec![self.label.as_ref().unwrap().to_string()], config.overwrite_tag(SupportedTag::Label));
        }
        if config.tag_enabled(SupportedTag::Genre) && !self.genres.is_empty() {
            let mut genres = if config.merge_genres {
                // Merge with existing ones
                let mut current: Vec<String> = tag.get_field(Field::Genre).unwrap_or(vec![]).into_iter().filter(|i| !i.trim().is_empty()).collect::<Vec<_>>();
                let mut genres = self.genres.clone().into_iter().filter(|g| !current.iter().any(|i| i.to_lowercase() == g.to_lowercase())).collect();
                current.append(&mut genres);
                current
            } else {
                self.genres.clone()
            };

            // Capitalize genres
            if config.capitalize_genres {
                genres = genres.into_iter().map(|g| onetagger_shared::capitalize(&g)).collect();
            }

            tag.set_field(Field::Genre, genres, config.overwrite_tag(SupportedTag::Genre));
        }
        if config.tag_enabled(SupportedTag::Style) && !self.styles.is_empty() {
            if config.styles_options == StylesOptions::CustomTag && config.styles_custom_tag.is_some() {
                // Custom style tag
                let ui_tag = config.styles_custom_tag.as_ref().unwrap();
                tag.set_raw(&ui_tag.by_format(&format), self.styles.clone(), config.overwrite_tag(SupportedTag::Style));

            } else if config.merge_genres {
                // Merge with existing ones
                let mut current: Vec<String> = tag.get_field(Field::Style).unwrap_or(vec![]).into_iter().filter(|i| !i.trim().is_empty()).collect::<Vec<_>>();
                let mut styles = self.styles.clone().into_iter().filter(|s| !current.iter().any(|i| i.to_lowercase() == s.to_lowercase())).collect();
                current.append(&mut styles);
                tag.set_field(Field::Style, current, config.overwrite_tag(SupportedTag::Style)); 

            } else {
                // Default write to style
                tag.set_field(Field::Style, self.styles.clone(), config.overwrite_tag(SupportedTag::Style));
            }
        }
        // Release dates
        if config.tag_enabled(SupportedTag::ReleaseDate) {
            if let Some(date) = self.release_date {
                tag.set_date(&TagDate {
                    year: date.year() as i32,
                    month: match config.only_year {
                        true => None,
                        false => Some(date.month() as u8)
                    },
                    day: match config.only_year {
                        true => None,
                        false => Some(date.day() as u8)
                    }
                }, config.overwrite_tag(SupportedTag::ReleaseDate));
            } else if let Some(year) = self.release_year {
                tag.set_date(&TagDate {
                    year: year as i32,
                    month: None,
                    day: None
                }, config.overwrite_tag(SupportedTag::ReleaseDate));
            }
        }
        // Publish date
        if config.tag_enabled(SupportedTag::PublishDate) {
            if let Some(date) = self.publish_date {
                tag.set_publish_date(&TagDate {
                    year: date.year() as i32,
                    month: match config.only_year {
                        true => None,
                        false => Some(date.month() as u8)
                    },
                    day: match config.only_year {
                        true => None,
                        false => Some(date.day() as u8)
                    }
                }, config.overwrite_tag(SupportedTag::PublishDate));
            } else if let Some(year) = self.publish_year {
                tag.set_publish_date(&TagDate {
                    year: year as i32,
                    month: None,
                    day: None
                }, config.overwrite_tag(SupportedTag::PublishDate));
            }
        }
        // URL
        if config.tag_enabled(SupportedTag::URL) {
            tag.set_raw("WWWAUDIOFILE", vec![self.url.to_string()], config.overwrite_tag(SupportedTag::URL));
        }
        // Other tags
        if config.tag_enabled(SupportedTag::OtherTags) {
            for (t, value) in &self.other {
                tag.set_raw(&t.by_format(&format), value.to_owned(), config.overwrite_tag(SupportedTag::OtherTags));
            }
        }
        // IDs
        if config.tag_enabled(SupportedTag::TrackId) && self.track_id.is_some() {
            let t = format!("{}_TRACK_ID", serde_json::to_value(self.platform.clone()).unwrap().as_str().unwrap().to_uppercase());
            tag.set_raw(&t, vec![self.track_id.as_ref().unwrap().to_string()], config.overwrite_tag(SupportedTag::TrackId));
        }
        if config.tag_enabled(SupportedTag::ReleaseId) && !self.release_id.is_empty() {
            let t = format!("{}_RELEASE_ID", serde_json::to_value(self.platform.clone()).unwrap().as_str().unwrap().to_uppercase());
            tag.set_raw(&t, vec![self.release_id.to_string()], config.overwrite_tag(SupportedTag::ReleaseId));
        }
        // Catalog number
        if config.tag_enabled(SupportedTag::CatalogNumber) && self.catalog_number.is_some() {
            tag.set_field(Field::CatalogNumber, vec![self.catalog_number.as_ref().unwrap().to_string()], config.overwrite_tag(SupportedTag::CatalogNumber));
        }
        // Duration
        if config.tag_enabled(SupportedTag::Duration) && self.duration.as_secs() > 0 {
            tag.set_field(Field::Duration, vec![self.duration.as_secs().to_string()], config.overwrite_tag(SupportedTag::Duration));
        }
        // Remixers
        if config.tag_enabled(SupportedTag::Remixer) && !self.remixers.is_empty() {
            tag.set_field(Field::Remixer, self.remixers.clone(), config.overwrite_tag(SupportedTag::Remixer));
        }
        // ISRC
        if config.tag_enabled(SupportedTag::ISRC) && self.isrc.is_some() {
            tag.set_field(Field::ISRC, vec![self.isrc.clone().unwrap()], config.overwrite_tag(SupportedTag::ISRC));
        }
        // Mood
        if config.tag_enabled(SupportedTag::Mood) && self.mood.is_some() {
            tag.set_field(Field::Mood, vec![self.mood.clone().unwrap()], config.overwrite_tag(SupportedTag::Mood));
        }
        // Disc number
        if config.tag_enabled(SupportedTag::DiscNumber) && self.disc_number.is_some() {
            tag.set_field(Field::DiscNumber, vec![self.disc_number.clone().unwrap().to_string()], config.overwrite_tag(SupportedTag::DiscNumber));
        }
        // Track number
        if config.tag_enabled(SupportedTag::TrackNumber) && self.track_number.is_some() {
            match config.tag_enabled(SupportedTag::TrackTotal) {
                true => tag.set_track_number(&self.track_number.as_ref().unwrap().to_string_with_zeroes(config.track_number_leading_zeroes), self.track_total.clone(), config.overwrite_tag(SupportedTag::TrackNumber)),
                false => tag.set_track_number(&self.track_number.as_ref().unwrap().to_string_with_zeroes(config.track_number_leading_zeroes), None, config.overwrite_tag(SupportedTag::TrackNumber)),
            }
        }
        // Lyrics
        if config.tag_enabled(SupportedTag::SyncedLyrics) && self.lyrics.is_some() {
            tag.set_lyrics(self.lyrics.as_ref().unwrap(), true, config.overwrite_tag(SupportedTag::SyncedLyrics));
        }
        if config.tag_enabled(SupportedTag::UnsyncedLyrics) && self.lyrics.is_some() {
            tag.set_lyrics(self.lyrics.as_ref().unwrap(), false, config.overwrite_tag(SupportedTag::UnsyncedLyrics));
        }
        // Explicit
        if config.tag_enabled(SupportedTag::Explicit) && self.explicit.is_some() {
            tag.set_explicit(self.explicit.unwrap());
        }

        // Album art
        if (config.overwrite_tag(SupportedTag::AlbumArt) || tag.get_art().is_empty()) && self.art.is_some() && config.tag_enabled(SupportedTag::AlbumArt) {
            info!("Downloading art: {:?}", self.art);
            match self.download_art(self.art.as_ref().unwrap()) {
                Ok(data) => {
                    match data {
                        Some(data) => {
                            // Remove covers
                            if config.remove_all_covers {
                                for t in CoverType::types() {
                                    tag.remove_art(t);
                                }
                            }

                            tag.set_art(CoverType::CoverFront, "image/jpeg", Some("Cover"), data.clone());
                            // Save to file
                            if config.album_art_file {
                                let path = Path::new(&info.path).parent().unwrap().join("cover.jpg");
                                if !path.exists() {
                                    if let Ok(mut file) = File::create(path) {
                                        file.write_all(&data).ok();
                                    }
                                }
                            }
                        },
                        None => warn!("Invalid album art!")
                    } 
                },
                Err(e) => warn!("Error downloading album art! {}", e)
            }
        }

        // Meta tags (date / success)
        if config.tag_enabled(SupportedTag::MetaTags) {
            let time = Local::now();
            tag.set_raw("1T_TAGGEDDATE", vec![format!("{}_AT", time.format("%Y-%m-%d %H:%M:%S"))], true);
        }

        // LRC
        if config.write_lrc && self.lyrics.is_some() {
            let path = Path::new(&info.path).with_extension("lrc");
            if !path.exists() {
                if let Some(lrc) = self.lyrics.as_ref().unwrap().generate_lrc(Some(&self), config.enhanced_lrc) {
                    info!("Writing LRC");
                    match std::fs::write(&path, lrc) {
                        Ok(_) => {}
                        Err(e) => warn!("Failed writing .LRC file to {:?} {}", path, e),
                    }
                }
            }
        }

        // Save
        tag.save_file(&info.path)?;
        Ok(())
    }

    // Download album art, None if invalid album art
    fn download_art(&self, url: &str) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        let response = reqwest::blocking::get(url)?;
        if response.status() != StatusCode::OK {
            return Ok(None);
        }
        // Too small, most likely a text response
        if let Some(cl) = response.content_length() {
            if cl < 2048 {
                return Ok(None);
            }
        }
       
        Ok(Some(response.bytes()?.to_vec()))
    }

    /// Merge styles by config
    fn merge_styles(mut self, option: &StylesOptions) -> Self {
        let genres = self.genres.clone();
        let styles = self.styles.clone();
        match option {
            StylesOptions::OnlyGenres => self.styles = vec![],
            StylesOptions::OnlyStyles => self.genres = vec![],
            StylesOptions::MergeToGenres => {
                self.genres.extend(styles);
                self.styles = vec![];
            },
            StylesOptions::MergeToStyles => {
                self.styles.extend(genres);
                self.genres = vec![];
            },
            StylesOptions::StylesToGenre => {
                self.genres = styles;
                self.styles = vec![];
            },
            StylesOptions::GenresToStyle => {
                self.styles = genres;
                self.genres = vec![];
            },
            StylesOptions::Default => {},
            // Is written separately
            StylesOptions::CustomTag => {},
        }
        self
    }

}

pub trait AudioFileInfoImpl {
    /// Load audio file info from path
    fn load_file(path: &str, filename_template: Option<Regex>, title_regex: Option<Regex>) -> Result<AudioFileInfo, Box<dyn Error>>;
    /// Load duration from file
    fn load_duration(&mut self);
    /// Parse the filename template
    fn parse_template(template: &str) -> Option<Regex>;
    /// Load using shazam
    fn shazam(path: &str) -> Result<AudioFileInfo, Box<dyn Error>>;
    /// Get list of all files in with supported extensions
    fn get_file_list(path: &str, subfolders: bool) -> Vec<String>;
}

impl AudioFileInfoImpl for AudioFileInfo {
    fn load_file(path: &str, filename_template: Option<Regex>, title_regex: Option<Regex>) -> Result<AudioFileInfo, Box<dyn Error>> {
        let tag_wrap = Tag::load_file(&path, true)?;
        let tag = tag_wrap.tag();
        let separator = tag.get_separator().unwrap_or(" ".to_string());
        // Get title artist from tag
        let mut title = tag.get_field(Field::Title).map(|t| match t.is_empty() {
            true => None,
            false => Some(t.join(&separator))
        }).flatten();
        let mut artists = tag.get_field(Field::Artist)
            .map(|a| AudioFileInfo::parse_artist_tag(a.iter().map(|a| a.as_str()).collect()));

        // Parse filename
        if (title.is_none() || artists.is_none()) && filename_template.is_some() {
            let p = Path::new(path);
            let filename = p.file_name().ok_or("Missing filename!")?.to_str().ok_or("Missing filename")?;

            if let Some(captures) = filename_template.unwrap().captures(filename) {
                // Title
                if title.is_none() {
                    if let Some(m) = captures.name("title") {
                        title = Some(m.as_str().trim().to_string());
                    }
                }
                // Artists
                if artists.is_none() {
                    if let Some(m) = captures.name("artists") {
                        artists = Some(AudioFileInfo::parse_artist_tag(vec![m.as_str().trim()]));
                    }
                }
            }
        }

        // Get tagging status
        let tagged = match tag.get_raw("1T_TAGGEDDATE").map(|t| t.first().map(String::from)).flatten() {
            Some(val) => {
                if val.ends_with("_AT") {
                    FileTaggedStatus::AutoTagger
                } else if val.ends_with("_AF") {
                    FileTaggedStatus::AudioFeatures
                } else {
                    FileTaggedStatus::Tagged
                }
            },
            None => FileTaggedStatus::Untagged,
        };

        // Clean title
        if let Some(re) = title_regex {
            title = title.map(|t| re.replace_all(&t, "").to_string());
        }


        // Track number
        let track_number = tag.get_field(Field::TrackNumber).unwrap_or(vec![String::new()])[0].parse().ok();
        Ok(AudioFileInfo {
            format: tag_wrap.format(),
            title,
            artists: artists.ok_or("Missing artist tag!")?,
            path: path.to_owned(),
            isrc: tag.get_field(Field::ISRC).unwrap_or(vec![]).first().map(String::from),
            duration: None,
            track_number,
            tagged,
            tags: tag.all_tags()
        })
    }

    fn load_duration(&mut self) {
        // Mark as loaded
        self.duration = Some(Duration::ZERO);
        if let Ok(source) = AudioSources::from_path(&self.path) {
            self.duration = Some(Duration::from_millis(source.duration() as u64))
        } else {
            warn!("Failed loading duration from file! {}", self.path);
        }
    }

    // Convert template into a regex
    fn parse_template(template: &str) -> Option<Regex> {
        // Regex reserved
        let reserved = ".?+*$^()[]/|";
        let mut template = template.to_string();
        for c in reserved.chars() {
            template = template.replace(c, &format!("\\{}", c));
        };
        // Replace variables
        template = template
            .replace("%title%", "(?P<title>.+)")
            .replace("%artist%", "(?P<artists>.+)")
            .replace("%artists%", "(?P<artists>.+)");
        // Remove all remaining variables
        let re = Regex::new("%[a-zA-Z0-9 ]+%").unwrap();
        template = re.replace_all(&template, "(.+)").to_string();
        // Extension
        template = format!("{}\\.[a-zA-Z0-9]{{2,4}}$", template);
        // Final regex
        Regex::new(&template).ok()
    }

    // Recognize on Shazam
    fn shazam(path: &str) -> Result<AudioFileInfo, Box<dyn Error>> {
        info!("Recognizing on Shazam: {}", path);
        match Shazam::recognize_from_file(path) {
            Ok((shazam_track, duration)) => {
                info!("Recognized on Shazam: {}: {} - {}", path, shazam_track.title, shazam_track.subtitle);
                return Ok(AudioFileInfo {
                    title: Some(shazam_track.title),
                    artists: AudioFileInfo::parse_artist_tag(vec![&shazam_track.subtitle]),
                    format: AudioFileFormat::from_extension(path.split(".").last().unwrap()).unwrap(),
                    path: path.to_string(),
                    isrc: shazam_track.isrc,
                    duration: Some(Duration::from_millis(duration as u64)),
                    track_number: None,
                    tagged: FileTaggedStatus::Untagged,
                    tags: Default::default(),
                });
            },
            // Mark as failed
            Err(e) => {
                warn!("Shazam failed: {}", e);
                return Err(e);
            }
        }
    }

    // Get list of all files in with supported extensions
    fn get_file_list(path: &str, subfolders: bool) -> Vec<String> {
        if path.is_empty() {
            return vec![];
        }
        if subfolders {
            let files: Vec<String> = WalkDir::new(path).into_iter().filter(
                |e| e.is_ok() && 
                EXTENSIONS.iter().any(|&i| e.as_ref().unwrap().path().to_str().unwrap().to_lowercase().ends_with(i))
            ).map(|e| e.unwrap().path().to_str().unwrap().to_owned()).collect();
            files
        } else {
            // No subfolders
            match std::fs::read_dir(path) {
                Ok(readdir) => {
                    readdir
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .map(|e| e.path().to_str().unwrap().to_string())
                        .filter(|p| EXTENSIONS.iter().any(|i| p.to_lowercase().ends_with(i)))
                        .collect()
                },
                Err(e) => {
                    warn!("Failed loading folder: {e}");
                    vec![]
                }
            }
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TaggingState {
    Ok, Error, Skipped
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggingStatus {
    pub status: TaggingState,
    pub path: String,
    pub message: Option<String>,
    pub accuracy: Option<f64>,
    pub used_shazam: bool,
}

// Wrap for sending into UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggingStatusWrap {
    pub status: TaggingStatus,
    pub platform: String,
    pub progress: f64,
}
impl TaggingStatusWrap {
    // pi = platform index, pl = platforms length, p = processed, total = total tracks in this platform
    pub fn wrap(platform: &str, status: &TaggingStatus, pi: usize, pl: usize, p: i64, total: usize) -> TaggingStatusWrap {
        TaggingStatusWrap {
            platform: platform.to_string(),
            status: status.to_owned(),
            progress: (pi as f64 / pl as f64) + ((p as f64 / total as f64) / pl as f64)
        }
    }
}

pub struct Tagger {}
impl Tagger {

    // Returtns progress receiver, and file count
    pub fn tag_files(cfg: &TaggerConfig, mut files: Vec<String>, finished: Arc<Mutex<Option<TaggerFinishedData>>>) -> Receiver<TaggingStatusWrap> {
        STOP_TAGGING.store(false, Ordering::SeqCst);
        
        let original_files = files.clone();
        let mut succesful_files = vec![];
        let total_files = files.len();
        info!("Starting tagger with: {} files!", total_files);

        // Create thread
        let (tx, rx) = unbounded();
        let config = cfg.clone();
        std::thread::spawn(move || {
            // Tag
            for (platform_index, platform) in config.platforms.iter().enumerate() {
                // For progress
                let mut processed = 0;
                let total = files.len();
                // No more files
                if files.is_empty() {
                    info!("All tagged succesfully!");
                    break;
                }

                // Stop
                if STOP_TAGGING.load(Ordering::SeqCst) {
                    continue;
                }

                // Get tagger
                let mut tagger = match AUTOTAGGER_PLATFORMS.get_builder(platform) {
                    Some(tagger) => tagger,
                    None => {
                        error!("Invalid platform: {platform}");
                        continue;
                    }
                };
                let platform_info = tagger.info();
                let mut threads = config.threads;
                if platform_info.max_threads > 0 && platform_info.max_threads < config.threads {
                    threads = platform_info.max_threads;
                }
                let rx = match Tagger::tag_dir(&files, &mut tagger, &config, threads) {
                    Some(t) => t,
                    None => {
                        error!("Failed creating platform: {platform:?}, skipping...");
                        continue;
                    }
                };
                // Start tagging
                info!("Starting {platform}");
                for status in rx {
                    info!("[{platform}] State: {:?}, Accuracy: {:?}, Path: {}", status.status, status.accuracy, status.path);
                    processed += 1;
                    // Send to UI
                    tx.send(TaggingStatusWrap::wrap(&platform_info.name, &status, platform_index, config.platforms.len(), processed, total)).ok();

                    if status.status == TaggingState::Ok {
                        // Save good files
                        if !succesful_files.contains(&status.path) {
                            succesful_files.push(status.path.to_string());
                        }
                        // Fallback
                        if !config.multiplatform {
                            files.remove(files.iter().position(|f| f == &status.path).unwrap());
                        }
                    }
                }
            }

            // Move files
            let mut successful_paths = vec![];
            for file in &succesful_files {
                if config.move_success && config.move_success_path.is_some() {
                    match Self::move_file(file, &config.move_success_path.as_ref().unwrap()) {
                        Ok(p) => successful_paths.push(p),
                        Err(e) => warn!("Failed moving file: {file} {e}"),
                    }
                } else {
                    successful_paths.push(PathBuf::from(file));
                }
            }
            let mut failed_paths = vec![];
            for file in &original_files {
                if succesful_files.contains(file) {
                    continue;
                }
                if config.move_failed && config.move_failed_path.is_some() {
                    match Self::move_file(file, &config.move_failed_path.as_ref().unwrap()) {
                        Ok(p) => failed_paths.push(p),
                        Err(e) => warn!("Failed moving file: {file} {e}"),
                    }
                } else {
                    failed_paths.push(PathBuf::from(file))
                }
            }
            std::mem::drop(succesful_files);
            std::mem::drop(original_files);

            // Tagging ended, save lists of files
            let write_result = || -> Result<(String, String), Box<dyn Error>> {
                let time = timestamp!();
                let folder = PathBuf::from(Settings::get_folder()?.to_str().unwrap().to_string()).join("runs");
                if !folder.exists() {
                    std::fs::create_dir_all(&folder)?;
                }
                let failed_file = folder.join(format!("failed-{}.m3u", time));
                let success_file = folder.join(format!("success-{}.m3u", time));
                {
                    let mut file = File::create(&failed_file)?;
                    file.write_all(failed_paths
                        .iter()
                        .filter_map(|f| dunce::canonicalize(f).ok().map(|p| p.to_string_lossy().to_string()))
                        .collect::<Vec<_>>()
                        .join("\r\n")
                        .as_bytes()
                    )?;
                }
                {
                    let mut file = File::create(&success_file)?;
                    let files: Vec<String> = successful_paths
                        .iter()
                        .filter_map(|f| dunce::canonicalize(f).ok().map(|p| p.to_string_lossy().to_string()))
                        .collect();
                    file.write_all(files.join("\r\n").as_bytes())?;
                }
                
                // Run command
                let (failed_file, success_file) = (failed_file.to_str().unwrap().to_string(), success_file.to_str().unwrap().to_string());
                if let Some(command) = &config.post_command {
                    if !command.trim().is_empty() {
                        let command = command
                            .replace("$failed", &failed_file)
                            .replace("$success", &success_file);
                        std::thread::spawn(|| {
                            info!("Executing command: {}", command);
                            let mut command = execute::shell(command);
                            let result = command.execute().ok().flatten();
                            info!("Command finished with: {:?}", result);
                        });
                    }
                }

                Ok((failed_file, success_file))
            };
            match write_result() {
                Ok((failed, success)) => {
                    info!("Written failed songs to: {}, successful to: {}", failed, success);
                    *finished.lock().unwrap() = Some(TaggerFinishedData {
                        failed_file: failed, success_file: success
                    });
                },
                Err(e) => warn!("Failed writing failed songs to file! {}", e)
            };
            

        });
        
        rx
    }

    /// Load track, shazam, prepare output
    pub fn load_track(path: &str, config: &TaggerConfig) -> (Option<AudioFileInfo>, TaggingStatus) {
        // Output
        let mut out = TaggingStatus {
            status: TaggingState::Error,
            path: path.to_owned(),
            accuracy: None,
            message: None,
            used_shazam: false
        };

        // Filename template
        let mut template = None;
        if config.parse_filename {
            if let Some(t) = &config.filename_template {
                template = AudioFileInfo::parse_template(&t);
            }
        }

        // Title cleanup regex
        let title_regex = config.title_regex.as_ref().map(|r| Regex::new(&r).ok()).flatten();

        // Load audio file info by shazam or tags
        let mut info = if config.enable_shazam && config.force_shazam {
            match AudioFileInfo::shazam(path) {
                Ok(i) => {
                    out.used_shazam = true;
                    i
                },
                Err(e) => {
                    out.status = TaggingState::Skipped;
                    out.message = Some(format!("Error Shazaming file: {}", e));
                    return (None, out);
                }
            }
        } else {
            match AudioFileInfo::load_file(path, template, title_regex) {
                Ok(info) => info,
                Err(e) => {
                    // Try shazam if enabled
                    if config.enable_shazam {
                        match AudioFileInfo::shazam(path) {
                            Ok(info) => {
                                out.used_shazam = true;
                                info
                            },
                            // Mark as failed
                            Err(e) => {
                                out.status = TaggingState::Skipped;
                                out.message = Some(format!("Error loading file: {}", e));
                                return (None, out);
                            }
                        }
                    } else {
                        out.status = TaggingState::Skipped;
                        warn!("Error loading file: {}", e);
                        out.message = Some(format!("Error loading file: {}", e));
                        return (None, out);
                    }
                }
            }
        };

        // Skip tagged
        if config.skip_tagged && info.tagged.at() {
            info!("Skipping (already tagged): {path}");
            out.status = TaggingState::Skipped;
            out.message = Some("Already tagged".to_string());
            return (None, out);
        }

        // Load duration for matching
        if config.match_duration {
            info.load_duration();
        }

        (Some(info), out)
    }

    /// Tag single track
    pub fn tag_track<T>(path: &str, tagger: &mut Box<T>, config: &TaggerConfig) -> TaggingStatus 
    where T: AutotaggerSource + ?Sized
    {
        info!("Tagging: {}", path);
        // Load track
        let (info, mut out) = Self::load_track(path, config);
        let info = match info {
            Some(info) => info,
            None => return out,
        };
       
        // Match track
        let result = tagger.match_track(&info, &config);
        match result {
            Ok(o) => {
                match o {
                    Some((acc, track)) => {
                        // Save to file
                        match track.merge_styles(&config.styles_options).write_to_file(&info, &config) {
                            Ok(_) => {
                                out.accuracy = Some(acc);
                                out.status = TaggingState::Ok;
                            },
                            Err(e) => {
                                error!("Failed writing tags to file: {e}");
                                out.message = Some(format!("Failed writing tags to file: {}", e));
                            }
                        }
                    },
                    None => out.message = Some("No match!".to_owned())
                }
            },
            // Failed matching track
            Err(e) => {
                error!("Matching error: {} ({})", e, path);
                out.message = Some(format!("Error matching track: {}", e));
            }
        }


        out
    }

    // Tag all files with threads specified in config
    pub fn tag_dir(files: &Vec<String>, tagger: &mut Box<dyn AutotaggerSourceBuilder>, config: &TaggerConfig, threads: u16) -> Option<Receiver<TaggingStatus>> {
        info!("Starting tagging: {} files, {} threads!", files.len(), threads);
        let (tx, rx) = unbounded();
        let (file_tx, file_rx): (Sender<String>, Receiver<String>) = unbounded();

        let mut ok_sources = 0;
        for _ in 0..threads {
            let tx = tx.clone();
            let file_rx = file_rx.clone();
            let config = config.clone();
            let mut source = match tagger.get_source(&config) {
                Ok(s) => s,
                Err(e) => {
                    warn!("Failed creating AT source! {e}");
                    continue;
                }
            };
            ok_sources += 1;
            std::thread::spawn(move || {
                while let Ok(f) = file_rx.recv() {
                    // Stop tagging
                    if STOP_TAGGING.load(Ordering::SeqCst) {
                        break;
                    }

                    let res = Tagger::tag_track(&f, &mut source, &config);
                    tx.send(res).ok();
                }
            });
        }
        if ok_sources == 0 {
            error!("All AT sources failed to create!");
            return None;
        }
        // Send files
        for f in files {
            file_tx.send(f.to_string()).ok();
        }
        Some(rx)
    }

    /// Move file to target dir if enabled
    fn move_file(source: impl AsRef<Path>, target: impl AsRef<Path>) -> Result<PathBuf, Box<dyn Error>> {
        // Generate path
        let target_dir = Path::new(target.as_ref());
        let filename = Path::new(source.as_ref()).file_name().unwrap();
        std::fs::create_dir_all(&target_dir).ok();
        let target = Path::new(&target_dir).join(filename);
        if target.exists() {
            return Ok(target);
        }
        // Try to rename, if fails copy
        match std::fs::rename(source.as_ref(), &target) {
            Ok(_) => return Ok(target),
            Err(_) => {}
        }
        std::fs::copy(source.as_ref(), &target)?;
        std::fs::remove_file(source.as_ref())?;

        Ok(target)
    }
}

/// When AT finishes this will contain some extra data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggerFinishedData {
    pub failed_file: String,
    pub success_file: String
}
