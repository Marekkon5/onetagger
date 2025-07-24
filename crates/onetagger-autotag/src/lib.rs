#[macro_use] extern crate log;
#[macro_use] extern crate anyhow;
#[macro_use] extern crate onetagger_shared;

use std::collections::HashMap;
use anyhow::Error;
use onetagger_renamer::{Renamer, RenamerConfig, TemplateParser};
use rand::seq::SliceRandom;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::time::Duration;
use std::default::Default;
use std::io::prelude::*;
use chrono::Local;
use execute::Execute;
use onetagger_tagger::{FileTaggedStatus, LyricsExt, MatchReason, MatchingUtils, SupportedTag, TrackMatch};
use regex::Regex;
use reqwest::StatusCode;
use walkdir::WalkDir;
use chrono::Datelike;
use serde::{Serialize, Deserialize};
use crossbeam_channel::{unbounded, Sender, Receiver};
use onetagger_tag::{AudioFileFormat, Tag, Field, TagDate, CoverType, TagImpl, EXTENSIONS};
use onetagger_shared::Settings;
use onetagger_player::AudioSources;
use onetagger_tagger::{Track, AudioFileInfo, TaggerConfig, StylesOptions, AutotaggerSource, AutotaggerSourceBuilder};

use crate::shazam::Shazam;
mod shazam;

pub mod repo;
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
        let autotagger_platforms = AUTOTAGGER_PLATFORMS.lock().unwrap();
        for platform in &autotagger_platforms.platforms {
            if !platform.info.platform.custom_options.options.is_empty() {
                let mut options = HashMap::new();
                for option in &platform.info.platform.custom_options.options {
                    options.insert(option.id.to_string(), option.value.json_value());
                }
                custom.insert(platform.info.platform.id.to_string(), serde_json::to_value(options).unwrap());
            }
        }
        let mut default = TaggerConfig::default();
        default.custom = custom.into();
        default
    }
} 


pub trait TrackImpl {
    fn write_to_file(&self, path: impl AsRef<Path>, config: &TaggerConfig) -> Result<(), Error>;
    fn download_art(&self, url: &str) -> Result<Option<Vec<u8>>, Error>;
    fn merge_styles(self, option: &StylesOptions) -> Self;
}

impl TrackImpl for Track {
    // Write tags to file
    fn write_to_file(&self, path: impl AsRef<Path>, config: &TaggerConfig) -> Result<(), Error> {        
        // Get tag
        let mut tag_wrap = Tag::load_file(&path, true)?;
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
                value = onetagger_tagger::to_camelot(&value).to_owned();
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
        if config.tag_enabled(SupportedTag::ReleaseId) && self.release_id.is_some() {
            let t = format!("{}_RELEASE_ID", serde_json::to_value(self.platform.clone()).unwrap().as_str().unwrap().to_uppercase());
            tag.set_raw(&t, vec![self.release_id.as_ref().unwrap().to_string()], config.overwrite_tag(SupportedTag::ReleaseId));
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
        let mut cover_data = None;
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
                            cover_data = Some(data);
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
            let path = path.as_ref().with_extension("lrc");
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
        tag.save_file(&path.as_ref())?;

        // Cover file
        if let Some(cover_data) = cover_data {
            match AudioFileInfo::load_file(&path, None, None) {
                Ok(info) => {
                    let cover_path = get_cover_path(&info, path.as_ref().parent().unwrap(), config);
                    match std::fs::write(&cover_path, cover_data) {
                        Ok(_) => debug!("Cover written to: {}", cover_path.display()),
                        Err(e) => error!("Failed to write cover file: {e}"),
                    }
                },
                Err(e) => {
                    error!("Failed generating cover path: {e}");
                }
            }
        }

        Ok(())
    }

    // Download album art, None if invalid album art
    fn download_art(&self, url: &str) -> Result<Option<Vec<u8>>, Error> {
        let response = reqwest::blocking::get(url)?;
        if response.status() != StatusCode::OK {
            return Ok(None);
        }
        // Too small, most likely a text response
        if let Some(cl) = response.content_length() {
            if cl < 4096 {
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

/// Get path to cover file
fn get_cover_path(info: &AudioFileInfo, folder: impl AsRef<Path>, config: &TaggerConfig) -> PathBuf {
    let mut path = folder.as_ref().join("cover.jpg");

    if let Some(template) = config.cover_filename.as_ref() {
        if !template.trim().is_empty() {
            // Generate new filename
            let renamer_config = RenamerConfig::default_with_paths(&folder, template);
            let mut renamer = Renamer::new(TemplateParser::parse(template));
            let new_path = renamer.generate_name(folder.as_ref(), info, &renamer_config);
            path = new_path.with_extension("jpg");
        }
    }

    path
}

pub trait AudioFileInfoImpl {
    /// Load audio file info from path
    fn load_file(path: impl AsRef<Path>, filename_template: Option<Regex>, title_regex: Option<Regex>) -> Result<AudioFileInfo, Error>;
    /// Load duration from file
    fn load_duration(&mut self);
    /// Parse the filename template
    fn parse_template(template: &str) -> Option<Regex>;
    /// Load using shazam
    fn shazam(path: impl AsRef<Path>) -> Result<AudioFileInfo, Error>;
    /// Get list of all files in with supported extensions
    fn get_file_list(path: impl AsRef<Path>, subfolders: bool) -> Vec<PathBuf>;
    /// Get iterator of all audio files in path 
    fn load_files_iter(path: impl AsRef<Path>, subfolders: bool, filename_template: Option<Regex>, title_regex: Option<Regex>) -> impl Iterator<Item = Result<AudioFileInfo, Error>>;
}

impl AudioFileInfoImpl for AudioFileInfo {
    fn load_file(path: impl AsRef<Path>, filename_template: Option<Regex>, title_regex: Option<Regex>) -> Result<AudioFileInfo, Error> {
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
            let filename = path.as_ref().file_name().ok_or(anyhow!("Missing filename!"))?.to_str().ok_or(anyhow!("Missing filename"))?;
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
            artists: artists.unwrap_or_default(),
            path: path.as_ref().to_owned(),
            isrc: tag.get_field(Field::ISRC).unwrap_or(vec![]).first().map(String::from),
            duration: None,
            track_number,
            tagged,
            tags: tag.all_tags()
        })
    }

    fn load_duration(&mut self) {
        // Mark as loaded
        self.duration = Some(Duration::ZERO.into());
        if let Ok(source) = AudioSources::from_path(&self.path) {
            self.duration = Some(Duration::from_millis(source.duration() as u64).into())
        } else {
            warn!("Failed loading duration from file! {:?}", self.path);
        }
    }

    // Convert template into a regex
    fn parse_template(template: &str) -> Option<Regex> {
        // Regex reserved (skip ? because can be used)
        let reserved = ".+*$^()[]/|";
        let mut template = template.to_string();
        for c in reserved.chars() {
            template = template.replace(c, &format!("\\{}", c));
        };
        // Replace variables
        template = template
            .replace("%title%", "(?P<title>.+?)")
            .replace("%artist%", "(?P<artists>.+?)")
            .replace("%artists%", "(?P<artists>.+?)");
        // Remove all remaining variables
        let re = Regex::new("%[a-zA-Z0-9 ]+%").unwrap();
        template = re.replace_all(&template, "(.+)").to_string();
        // Extension
        template = format!("{}\\.[a-zA-Z0-9]{{2,4}}$", template).trim().to_string();
        debug!("Filename template regex: `{template}`");
        // Final regex
        Regex::new(&template).ok()
    }

    // Recognize on Shazam
    fn shazam(path: impl AsRef<Path>) -> Result<AudioFileInfo, Error> {
        info!("Recognizing on Shazam: {:?}", path.as_ref());
        match Shazam::recognize_from_file(&path) {
            Ok((shazam_track, duration)) => {
                info!("Recognized on Shazam: {:?}: {} - {}", path.as_ref(), shazam_track.title, shazam_track.subtitle);
                return Ok(AudioFileInfo {
                    title: Some(shazam_track.title),
                    artists: AudioFileInfo::parse_artist_tag(vec![&shazam_track.subtitle]),
                    format: AudioFileFormat::from_extension(&path.as_ref().extension().unwrap_or_default().to_string_lossy()).unwrap(),
                    path: path.as_ref().to_owned(),
                    isrc: shazam_track.isrc,
                    duration: Some(Duration::from_millis(duration as u64).into()),
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
    fn get_file_list(path: impl AsRef<Path>, subfolders: bool) -> Vec<PathBuf> {
        if path.as_ref().to_string_lossy() == "" {
            return vec![];
        }

        if subfolders {
            WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| EXTENSIONS.iter().any(|ext| e.path().extension().unwrap_or_default().to_ascii_lowercase() == *ext))
                .map(|e| e.into_path())
                .collect()
        } else {
            // No subfolders
            match std::fs::read_dir(path) {
                Ok(readdir) => {
                    readdir
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|p| EXTENSIONS.iter().any(|i| p.path().extension().unwrap_or_default().to_ascii_lowercase() == *i))
                        .map(|e| e.path())
                        .collect()
                },
                Err(e) => {
                    warn!("Failed loading folder: {e}");
                    vec![]
                }
            }
        }
    }

    fn load_files_iter(path: impl AsRef<Path>, subfolders: bool, filename_template: Option<Regex>, title_regex: Option<Regex>) -> impl Iterator<Item = Result<AudioFileInfo, Error>> {
        let files = Self::get_file_list(path, subfolders);
        files.into_iter().map(move |f| Self::load_file(&f, filename_template.clone(), title_regex.clone()))
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
    pub path: PathBuf,
    pub message: Option<String>,
    pub accuracy: Option<f64>,
    pub used_shazam: bool,
    pub release_id: Option<String>,
    pub reason: Option<MatchReason>
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
    pub fn tag_files(cfg: &TaggerConfig, mut files: Vec<PathBuf>, finished: Arc<Mutex<Option<TaggerFinishedData>>>) -> Receiver<TaggingStatusWrap> {
        STOP_TAGGING.store(false, Ordering::SeqCst);

        // Shuffle so album tag is more "efficient"
        if cfg.album_tagging {
            let mut rng = rand::rng();
            files.shuffle(&mut rng);
        }
        
        // let original_files = files.clone();
        let mut succesful_files = vec![];
        let mut failed_files = vec![];
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
                let mut autotagger_platforms = AUTOTAGGER_PLATFORMS.lock().unwrap();
                let tagger = match autotagger_platforms.get_builder(platform) {
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
                let rx = match Tagger::tag_batch(&files, tagger, &config, threads) {
                    Some(t) => t,
                    None => {
                        error!("Failed creating platform: {platform:?}, skipping...");
                        continue;
                    }
                };
                // Start tagging
                info!("Starting {platform}");
                for status in rx {
                    info!("[{platform}] State: {:?}, Accuracy: {:?}, Path: {:?}", status.status, status.accuracy, status.path);
                    processed += 1;
                    // Send to UI
                    tx.send(TaggingStatusWrap::wrap(&platform_info.name, &status, platform_index, config.platforms.len(), processed, total)).ok();

                    if status.status == TaggingState::Ok {
                        // Save good files
                        if !succesful_files.contains(&status.path) {
                            succesful_files.push(status.path.to_owned());
                        }
                        // Fallback
                        if !config.multiplatform {
                            if let Some(index) = files.iter().position(|f| f == &status.path) {
                                files.remove(index);
                            }
                        }
                        // Remove from failed
                        if let Some(i) = failed_files.iter().position(|i| i == &status.path) {
                            failed_files.remove(i);
                        }
                    }
                    // Log failed
                    if status.status == TaggingState::Error && !succesful_files.contains(&status.path) {
                        failed_files.push(status.path.to_owned());
                    }

                }
            }

            // Move files
            let mut successful_paths = vec![];
            for file in &succesful_files {
                if config.move_success && config.move_success_path.is_some() {
                    match Self::move_file(file, &config.move_success_path.as_ref().unwrap()) {
                        Ok(p) => successful_paths.push(p),
                        Err(e) => warn!("Failed moving file: {file:?} {e}"),
                    }
                } else {
                    successful_paths.push(PathBuf::from(file));
                }
            }
            let mut failed_paths = vec![];
            for file in &failed_files {
                if config.move_failed && config.move_failed_path.is_some() {
                    match Self::move_file(file, &config.move_failed_path.as_ref().unwrap()) {
                        Ok(p) => failed_paths.push(p),
                        Err(e) => warn!("Failed moving file: {file:?} {e}"),
                    }
                } else {
                    failed_paths.push(PathBuf::from(file))
                }
            }
            std::mem::drop(succesful_files);
            std::mem::drop(failed_files);
            std::mem::drop(files);

            // Tagging ended, save lists of files
            match Self::write_results(successful_paths, failed_paths, &config) {
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

    /// Write playlists & execute command
    fn write_results(successful_paths: Vec<PathBuf>, failed_paths: Vec<PathBuf>, config: &TaggerConfig) -> Result<(String, String), Error> {
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

    }

    /// Load track, shazam, prepare output
    pub fn load_track(path: impl AsRef<Path>, config: &TaggerConfig) -> (Option<AudioFileInfo>, TaggingStatus) {
        // Output
        let mut out = TaggingStatus {
            status: TaggingState::Error,
            path: path.as_ref().to_owned(),
            accuracy: None,
            message: None,
            used_shazam: false,
            release_id: None,
            reason: None
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
            match AudioFileInfo::shazam(&path) {
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
            match AudioFileInfo::load_file(&path, template, title_regex) {
                Ok(info) => info,
                Err(e) => {
                    // Try shazam if enabled
                    if config.enable_shazam {
                        match AudioFileInfo::shazam(&path) {
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
            info!("Skipping (already tagged): {:?}", path.as_ref());
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
    pub fn tag_track<T>(path: impl AsRef<Path>, tagger: &mut Box<T>, config: &TaggerConfig) -> TaggingStatus 
    where T: AutotaggerSource + ?Sized
    {
        info!("Tagging: {:?}", path.as_ref());
        // Load track
        let (info, mut out) = Self::load_track(&path, config);
        let info = match info {
            Some(info) => info,
            None => return out,
        };
       
        // Match track
        let result = tagger.match_track(&info, &config);
        let mut tracks = match result {
            Ok(o) => {
                if o.is_empty() {
                    out.message = Some("No match!".to_owned());
                    return out;
                }
                o
            },
            // Failed matching track
            Err(e) => {
                error!("Matching error: {} ({:?})", e, path.as_ref());
                out.message = Some(format!("Error matching track: {}", e));
                return out;
            }
        };

        // Get & extend track
        MatchingUtils::sort_tracks(&mut tracks, config);
        let mut track = tracks.remove(0);
        drop(tracks);
        match tagger.extend_track(&mut track.track, config) {
            Ok(_) => {},
            Err(e) => warn!("Failed extending track: {e}"),
        }

        // Save
        out.release_id = track.track.release_id.clone();
        out.reason = Some(track.reason);
        match track.track.merge_styles(&config.styles_options).write_to_file(&info.path, &config) {
            Ok(_) => {
                out.accuracy = Some(track.accuracy);
                out.status = TaggingState::Ok;
            },
            Err(e) => {
                error!("Failed writing tags to file: {e}");
                out.message = Some(format!("Failed writing tags to file: {}", e));
            }
        }

        out
    }

    // Tag all files with threads specified in config
    pub fn tag_batch(files: &Vec<PathBuf>, tagger: &mut Box<dyn AutotaggerSourceBuilder + Send + Sync>, config: &TaggerConfig, threads: u16) -> Option<Receiver<TaggingStatus>> {
        info!("Starting tagging: {} files, {} threads!", files.len(), threads);
        let (tx, rx) = unbounded();
        let (file_tx, file_rx): (Sender<PathBuf>, Receiver<PathBuf>) = unbounded();
        let (finished_tx, finished_rx) = unbounded();

        // Album tagging
        let album_tagging = Arc::new(Mutex::new(AlbumTagContext::new()));
        if config.album_tagging {
            album_tagging.lock().unwrap().init(files);
        }

        let mut ok_sources = 0;
        for _ in 0..threads {
            let tx = tx.clone();
            let file_rx = file_rx.clone();
            let config = config.clone();
            let finished_tx = finished_tx.clone();
            let album_tagging = album_tagging.clone();
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

                    // Check if not marked for album tagging
                    if album_tagging.lock().unwrap().is_marked(&f) {
                        continue;
                    }

                    // Tag
                    let res = Tagger::tag_track(&f, &mut source, &config);
                    if config.album_tagging {
                        album_tagging.lock().unwrap().process(&res, &config);
                    }
                    tx.send(res).ok();
                }
                finished_tx.send(0u8).ok();
            });
        }

        // Spawn album tag thread
        if config.album_tagging {
            let config = config.clone();
            match tagger.get_source(&config) {
                Ok(mut source) => {
                    std::thread::spawn(move || {
                        // Wait for all threads to finish
                        for _ in finished_rx.into_iter() {}

                        // Check all album statuses
                        let album_tagging = album_tagging.lock().unwrap();
                        for (path, stats) in &album_tagging.folders {
                            if !stats.marked {
                                continue;
                            }

                            // Tag
                            match Self::tag_album(path, &stats.get_album_id().unwrap(), &mut source, &config) {
                                Ok(statuses) => {
                                    for status in statuses {
                                        tx.send(status).ok();
                                    }
                                },
                                Err(e) => error!("Album tagging failed: {e}, path: {}", path.display()),
                            }
                        }

                    });
                },
                Err(e) => error!("Failed to get source for album tagging, album tagging will be disabled! {e}")
            }
        }
 

        if ok_sources == 0 {
            error!("All AT sources failed to create!");
            return None;
        }
        // Send files
        for f in files {
            file_tx.send(f.to_owned()).ok();
        }
        Some(rx)
    }

    /// Tag an album by ID
    pub fn tag_album(path: impl AsRef<Path>, release_id: &str, source: &mut Box<dyn AutotaggerSource>, config: &TaggerConfig) -> Result<Vec<TaggingStatus>, Error> {
        info!("Album tagging release: {release_id} in {}", path.as_ref().display());

        // Change strictness since we're working in context of album, and just care about most likely match
        let mut config = config.clone();
        config.strictness = 0.0;
        config.match_duration = false;
        config.match_by_id = true;
        config.enable_shazam = false;
        config.force_shazam = false;

        // Get album
        let album = source.get_album(&release_id, &config)?.ok_or(anyhow!("Album with id: {release_id} not found"))?;
        if album.tracks.is_empty() {
            return Err(anyhow!("Album {release_id} has no tracks!"))
        }

        let mut statuses = vec![];

        // Load files
        let files = std::fs::read_dir(&path)?.filter_map(|e| e.ok()).map(|f| f.path()).collect::<Vec<_>>();
        for file in files {
            let (info, mut status) = Self::load_track(&file, &config);
            let info = match info {
                Some(i) => i,
                None => {
                    warn!("Failed to load track info for file: {}", file.display());
                    continue;
                }
            };

            // Find closest match
            let mut tracks = MatchingUtils::match_track(&info, &album.tracks, &config, false);
            MatchingUtils::sort_tracks(&mut tracks, &config);
            let track = tracks.remove(0);
            
            // TODO: Extend track if needed (?)
            if let Err(e) = track.track.merge_styles(&config.styles_options).write_to_file(&info.path, &config) {
                status.status = TaggingState::Error;
                error!("Album tag writing tags failed: {e} ({})", file.display());
            } else {
                status.status = TaggingState::Ok;
            }

            // Save status
            status.accuracy = Some(1.0);
            status.reason = Some(MatchReason::Album);
            statuses.push(status);
        }

        Ok(statuses)
    }

    /// Move file to target dir if enabled
    fn move_file(source: impl AsRef<Path>, target: impl AsRef<Path>) -> Result<PathBuf, Error> {
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

/// For keeping track of per-album tagging
struct AlbumTagContext {
    /// path: stats
    folders: HashMap<PathBuf, AlbumTagFolderStats>
}

impl AlbumTagContext {
    /// Create new instance
    pub fn new() -> AlbumTagContext {
        AlbumTagContext {
            folders: Default::default()
        }
    }

    /// Initialize internal counters
    pub fn init(&mut self, paths: &[PathBuf]) {
        for path in paths {
            if let Some(path) = path.parent() {
                let stats = match self.folders.get_mut(path) {
                    Some(v) => v,
                    None => {
                        self.folders.insert(path.to_owned(), AlbumTagFolderStats::default());
                        self.folders.get_mut(path).unwrap()
                    }
                };
                stats.files += 1;
            }
        }
    }

    /// Save info from tagging status data
    /// Returns (Path, Release ID)
    pub fn process(&mut self, status: &TaggingStatus, config: &TaggerConfig) -> Option<(PathBuf, String)> {
        let release_id = status.release_id.as_ref()?;
        // Get folder path
        let path = status.path.parent()?;
        let stats = self.folders.get_mut(path)?;
        if stats.marked {
            return None;
        }

        let count = match stats.albums.get_mut(release_id) {
            Some(v) => {
                *v = *v + 1;
                *v
            },
            None => {
                stats.albums.insert(release_id.to_string(), 1);
                1
            }
        };

        // Should be considered as 
        if (count as f32 / stats.files as f32) >= config.album_tagging_ratio {
            stats.marked = true;
            return Some((path.to_owned(), release_id.to_owned()));
        }
        None
    }

    /// Check if path is marked
    pub fn is_marked(&self, path: impl AsRef<Path>) -> bool {
        if let Some(parent) = path.as_ref().parent() {
            if let Some(stats) = self.folders.get(parent) {
                return stats.marked;
            }
        }
        false
    }

}

#[derive(Debug, Clone, Default)]
struct AlbumTagFolderStats {
    /// How many files
    pub files: usize,
    /// album_id: count
    pub albums: HashMap<String, usize>,
    /// Is already marked as album
    pub marked: bool,
}

impl AlbumTagFolderStats {
    /// Get album ID with highest count
    pub fn get_album_id(&self) -> Option<String> {
        self.albums.iter().max_by_key(|(_, c)| **c).map(|(i, _)| i.to_string())
    }
}


/// When AT finishes this will contain some extra data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggerFinishedData {
    pub failed_file: String,
    pub success_file: String
}


/// Start manual tagging mode
/// Return: receiver with results for every platform
pub fn manual_tagger(path: impl AsRef<Path>, config: &TaggerConfig) -> Result<Receiver<(String, Result<Vec<TrackMatch>, Error>)>, Error> {
    // Get filename template
    let filename_template = config.filename_template.as_ref().map(|template| {
        match AudioFileInfo::parse_template(template) {
            Some(template) => Some(template),
            None => {
                warn!("Failed parsing filename template");
                None
            },
        }
    }).flatten();

    // Title regex
    let title_regex = config.title_regex.as_ref().map(|re| {
        match Regex::new(re) {
            Ok(r) => Some(r),
            Err(e) => {
                warn!("Failed parsing title regex: \"{re}\": {e}");
                None
            },
        }
    }).flatten();

    // Load file
    let file = AudioFileInfo::load_file(path, filename_template, title_regex)?;
    let (tx, rx) = unbounded();

    // Setup platforms
    let mut platforms = vec![];
    for platform in &config.platforms {
        let mut autotagger_platforms = AUTOTAGGER_PLATFORMS.lock().unwrap();
        let p = match autotagger_platforms.get_builder(platform) {
            Some(p) => p,
            None => {
                warn!("Invalid platform: {platform}");
                continue;
            },
        };

        // Get platform
        let mut s = match p.get_source(&config) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed creating platform source for platform: {platform}. {e}");
                continue;
            }
        };

        // Spawn thread to match track
        let mut config = config.clone();
        config.fetch_all_results = true;
        let info = file.clone();
        let tx = tx.clone();
        let platform = platform.to_string();
        platforms.push(std::thread::spawn(move || {
            // Match
            let r = s.match_track(&info, &config).map(|mut m| { m.dedup(); m });
            tx.send((platform, r)).ok();
        }));
    }

    Ok(rx)
}

/// Apply manual tag results
pub fn manual_tagger_apply(mut matches: Vec<TrackMatch>, path: impl AsRef<Path>, config: &TaggerConfig) -> Result<(), Error> {
    if matches.is_empty() {
        return Ok(())
    }
    
    // Extend each match
    for m in matches.iter_mut() {
        // Get platform
        let mut autotagger_platforms = AUTOTAGGER_PLATFORMS.lock().unwrap();
        let p = match autotagger_platforms.get_builder(&m.track.platform) {
            Some(p) => p,
            None => {
                warn!("Invalid platform: {}", m.track.platform);
                continue;
            },
        };
        let mut s = match p.get_source(config) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed creating platform source for platform: {}. {e}", m.track.platform);
                continue;
            }
        };
        // Extend
        debug!("Extending track on: {}", m.track.platform);
        match s.extend_track(&mut m.track, config) {
            Ok(_) => {},
            Err(e) => warn!("Failed extending track using: {}. {e}", m.track.platform),
        }
    }

    // Merge
    let mut track = matches.remove(0).track;
    for t in matches {
        track = track.merge(t.track);
    }

    // Save
    track.merge_styles(&config.styles_options).write_to_file(&path, &config)?;
    Ok(())
}