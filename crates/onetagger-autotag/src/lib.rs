#[macro_use] extern crate log;
#[macro_use] extern crate anyhow;
#[macro_use] extern crate onetagger_shared;

use std::collections::{BTreeSet, HashMap};
use anyhow::Error;
use onetagger_renamer::{Renamer, RenamerConfig, TemplateParser};
use rand::seq::SliceRandom;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::time::{Duration, Instant};
use onetagger_platforms::ProviderRateLimited;
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
pub mod report;
mod scheduler;
mod pipeline;

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

trait TrackFieldWriter {
    fn write_fields(&self, path: impl AsRef<Path>, config: &TaggerConfig, recovering: bool) -> Result<Vec<SupportedTag>, Error>;
}

impl TrackFieldWriter for Track {
    fn write_fields(&self, path: impl AsRef<Path>, config: &TaggerConfig, recovering: bool) -> Result<Vec<SupportedTag>, Error> {
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
        let tag = tag_wrap.tag_mut();
        // Record supplied fields at the same point the writer accepts their values.
        let mut supplied = Vec::new();
        let mut select = |field| {
            if !config.tag_enabled(field) { return false; }
            if recovering { supplied.push(field); }
            true
        };
        if !self.title.trim().is_empty() && select(SupportedTag::Title) {
            match config.short_title {
                true => tag.set_field(Field::Title, vec![self.title.to_string()], config.overwrite_tag(SupportedTag::Title)),
                false => tag.set_field(Field::Title, vec![self.full_title()], config.overwrite_tag(SupportedTag::Title))
            }
        }
        // Version
        if self.version.as_deref().is_some_and(|s| !s.trim().is_empty()) && select(SupportedTag::Version) {
            tag.set_field(Field::Version, vec![self.version.as_ref().unwrap().to_string()], config.overwrite_tag(SupportedTag::Version));
        }
        if self.artists.iter().any(|a| !a.trim().is_empty()) && select(SupportedTag::Artist) {
            tag.set_field(Field::Artist, self.artists.clone(), config.overwrite_tag(SupportedTag::Artist));
        }
        if !self.album_artists.is_empty() && select(SupportedTag::AlbumArtist) {
            tag.set_field(Field::AlbumArtist, self.album_artists.clone(), config.overwrite_tag(SupportedTag::AlbumArtist));
        }
        if self.album.as_deref().is_some_and(|s| !s.trim().is_empty()) && select(SupportedTag::Album) {
            tag.set_field(Field::Album, vec![self.album.as_ref().unwrap().to_string()], config.overwrite_tag(SupportedTag::Album));
        }
        if self.key.as_deref().is_some_and(|s| !s.trim().is_empty()) && select(SupportedTag::Key) {
            let mut value = self.key.as_ref().unwrap().to_string();
            // Convert to camelot
            if config.camelot {
                value = onetagger_tagger::to_camelot(&value).to_owned();
            }
            tag.set_field(Field::Key, vec![value], config.overwrite_tag(SupportedTag::Key));
        }
        if self.bpm.is_some() && select(SupportedTag::BPM) {
            tag.set_field(Field::BPM, vec![self.bpm.unwrap().to_string()], config.overwrite_tag(SupportedTag::BPM));
        }
        if self.label.as_deref().is_some_and(|s| !s.trim().is_empty()) && select(SupportedTag::Label) {
            tag.set_field(Field::Label, vec![self.label.as_ref().unwrap().to_string()], config.overwrite_tag(SupportedTag::Label));
        }
        if !self.genres.is_empty() && select(SupportedTag::Genre) {
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
        if !self.styles.is_empty() && select(SupportedTag::Style) {
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
        if (self.release_date.is_some() || self.release_year.is_some()) && select(SupportedTag::ReleaseDate) {
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
        if (self.publish_date.is_some() || self.publish_year.is_some()) && select(SupportedTag::PublishDate) {
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
        if !self.url.trim().is_empty() && select(SupportedTag::URL) {
            tag.set_raw("WWWAUDIOFILE", vec![self.url.to_string()], config.overwrite_tag(SupportedTag::URL));
        }
        // Other tags
        if !self.other.is_empty() && select(SupportedTag::OtherTags) {
            for (t, value) in &self.other {
                tag.set_raw(&t.by_format(&format), value.to_owned(), config.overwrite_tag(SupportedTag::OtherTags));
            }
        }
        // IDs
        if self.track_id.as_deref().is_some_and(|s| !s.trim().is_empty()) && select(SupportedTag::TrackId) {
            let t = format!("{}_TRACK_ID", serde_json::to_value(self.platform.clone()).unwrap().as_str().unwrap().to_uppercase());
            tag.set_raw(&t, vec![self.track_id.as_ref().unwrap().to_string()], config.overwrite_tag(SupportedTag::TrackId));
        }
        if self.release_id.as_deref().is_some_and(|s| !s.trim().is_empty()) && select(SupportedTag::ReleaseId) {
            let t = format!("{}_RELEASE_ID", serde_json::to_value(self.platform.clone()).unwrap().as_str().unwrap().to_uppercase());
            tag.set_raw(&t, vec![self.release_id.as_ref().unwrap().to_string()], config.overwrite_tag(SupportedTag::ReleaseId));
        }
        // Catalog number
        if self.catalog_number.as_deref().is_some_and(|s| !s.trim().is_empty()) && select(SupportedTag::CatalogNumber) {
            tag.set_field(Field::CatalogNumber, vec![self.catalog_number.as_ref().unwrap().to_string()], config.overwrite_tag(SupportedTag::CatalogNumber));
        }
        // Duration
        if self.duration.as_secs() > 0 && select(SupportedTag::Duration) {
            tag.set_field(Field::Duration, vec![self.duration.as_secs().to_string()], config.overwrite_tag(SupportedTag::Duration));
        }
        // Remixers
        if !self.remixers.is_empty() && select(SupportedTag::Remixer) {
            tag.set_field(Field::Remixer, self.remixers.clone(), config.overwrite_tag(SupportedTag::Remixer));
        }
        // ISRC
        if self.isrc.as_deref().is_some_and(|s| !s.trim().is_empty()) && select(SupportedTag::ISRC) {
            tag.set_field(Field::ISRC, vec![self.isrc.clone().unwrap()], config.overwrite_tag(SupportedTag::ISRC));
        }
        // Mood
        if self.mood.as_deref().is_some_and(|s| !s.trim().is_empty()) && select(SupportedTag::Mood) {
            tag.set_field(Field::Mood, vec![self.mood.clone().unwrap()], config.overwrite_tag(SupportedTag::Mood));
        }
        // Disc number
        if self.disc_number.is_some() && select(SupportedTag::DiscNumber) {
            tag.set_field(Field::DiscNumber, vec![self.disc_number.clone().unwrap().to_string()], config.overwrite_tag(SupportedTag::DiscNumber));
        }
        // Track number
        if self.track_number.is_some() && select(SupportedTag::TrackNumber) {
            match self.track_total.is_some() && select(SupportedTag::TrackTotal) {
                true => tag.set_track_number(&self.track_number.as_ref().unwrap().to_string_with_zeroes(config.track_number_leading_zeroes), self.track_total.clone(), config.overwrite_tag(SupportedTag::TrackNumber)),
                false => tag.set_track_number(&self.track_number.as_ref().unwrap().to_string_with_zeroes(config.track_number_leading_zeroes), None, config.overwrite_tag(SupportedTag::TrackNumber)),
            }
        }
        // On a field-only retry the earlier provider may already have supplied the track number.
        if recovering && !config.tag_enabled(SupportedTag::TrackNumber) && self.track_total.is_some() {
            if let Some(number) = tag.get_field(Field::TrackNumber).and_then(|v| v.into_iter().next()) {
                if select(SupportedTag::TrackTotal) {
                    let number = number.split('/').next().unwrap_or(&number);
                    tag.set_track_number(number, self.track_total, config.overwrite_tag(SupportedTag::TrackTotal));
                }
            }
        }
        // Lyrics
        if self.lyrics.as_ref().is_some_and(|lyrics| lyrics.synced()) && select(SupportedTag::SyncedLyrics) {
            tag.set_lyrics(self.lyrics.as_ref().unwrap(), true, config.overwrite_tag(SupportedTag::SyncedLyrics));
        }
        if self.lyrics.is_some() && select(SupportedTag::UnsyncedLyrics) {
            tag.set_lyrics(self.lyrics.as_ref().unwrap(), false, config.overwrite_tag(SupportedTag::UnsyncedLyrics));
        }
        // Explicit
        if self.explicit.is_some() && select(SupportedTag::Explicit) {
            tag.set_explicit(self.explicit.unwrap());
        }

        // Download and validate before removing anything from the existing tag.
        let mut cover_data = None;
        if (config.overwrite_tag(SupportedTag::AlbumArt) || tag_wrap.tag().get_art().is_empty()) && self.art.is_some() && config.tag_enabled(SupportedTag::AlbumArt) {
            info!("Downloading art: {:?}", self.art);
            match self.download_art(self.art.as_ref().unwrap()) {
                Ok(Some(data)) => {
                    // MP4 artwork kinds are synthetic indexes; replace the collection
                    // only after a usable replacement is available.
                    if let Tag::MP4(mp4) = &mut tag_wrap {
                        mp4.remove_all_artworks();
                    }
                    let tag = tag_wrap.tag_mut();
                    if config.remove_all_covers {
                        for kind in CoverType::types() {
                            tag.remove_art(kind);
                        }
                    }
                    tag.set_art(CoverType::CoverFront, "image/jpeg", Some("Cover"), data.clone());
                    if recovering { supplied.push(SupportedTag::AlbumArt); }
                    cover_data = Some(data);
                }
                Ok(None) => warn!("Invalid album art; existing artwork preserved"),
                Err(e) => warn!("Error downloading album art; existing artwork preserved: {e}"),
            }
        }
        if recovering && config.tag_enabled(SupportedTag::AlbumArt) && !config.overwrite_tag(SupportedTag::AlbumArt)
            && !supplied.contains(&SupportedTag::AlbumArt) && tag_wrap.tag().has_art() {
            supplied.push(SupportedTag::AlbumArt);
        }
        let tag = tag_wrap.tag_mut();

        // A retryable result must not mark the audio as fully tagged for the next run.
        let complete = !recovering || config.tags.iter().all(|t| *t == SupportedTag::MetaTags || supplied.contains(t));
        if recovering && supplied.is_empty() && !(complete && config.tag_enabled(SupportedTag::MetaTags)) { return Ok(supplied); }
        // Meta tags (date / success)
        if config.tag_enabled(SupportedTag::MetaTags) && complete {
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

        Ok(supplied)
    }
}

impl TrackImpl for Track {
    fn write_to_file(&self, path: impl AsRef<Path>, config: &TaggerConfig) -> Result<(), Error> {
        self.write_fields(path, config, false).map(drop)
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
       
        let data = response.bytes()?.to_vec();
        // Reject truncated/non-image responses and keep the embedded MIME and .jpg
        // sidecar consistent when a provider returns PNG or another image format.
        let decoded = image::load_from_memory(&data)?;
        if image::guess_format(&data)? == image::ImageFormat::Jpeg {
            return Ok(Some(data));
        }
        let mut jpeg = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(decoded.into_rgb8()).write_to(&mut jpeg, image::ImageFormat::Jpeg)?;
        Ok(Some(jpeg.into_inner()))
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

fn tagged_status(tag: &dyn TagImpl) -> FileTaggedStatus {
    match tag.get_raw("1T_TAGGEDDATE").and_then(|values| values.into_iter().next()) {
        Some(value) if value.ends_with("_AT") => FileTaggedStatus::AutoTagger,
        Some(value) if value.ends_with("_AF") => FileTaggedStatus::AudioFeatures,
        Some(_) => FileTaggedStatus::Tagged,
        None => FileTaggedStatus::Untagged,
    }
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

        let tagged = tagged_status(*tag);

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
    Ok, Error, Skipped, Partial, Deferred
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggingStatus {
    pub status: TaggingState,
    pub path: PathBuf,
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_args: Option<serde_json::Map<String, serde_json::Value>>,
    pub accuracy: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lookup_ms: Option<u64>,
    pub used_shazam: bool,
    pub release_id: Option<String>,
    #[serde(default)]
    pub track_id: Option<String>,
    pub reason: Option<MatchReason>
}

/// Application-owned status identity. Raw provider/OS details are arguments;
/// they are never parsed for English text or treated as translation source.
#[derive(Clone)]
pub(crate) enum TaggingMessage {
    AlreadyTagged,
    AudioFeaturesAlreadyTagged,
    NoMatch,
    StoppedBeforeWriting,
    NoAlbumMatch,
    AlbumWriteFailed,
    ShazamFailed(String),
    FileLoadFailed(String),
    TrackMatchFailed(String),
    TrackWriteFailed(String),
    AudioFeaturesWriteFailed(String),
    AudioFeaturesSearchFailed(String),
    ProviderWorkerPanicked,
    ProviderInitializationPanicked,
    ProviderInitializationFailed(String),
    ProviderUnavailable(String),
    ProviderRateLimited(u64),
    ProviderDetailsUnavailable,
    ProviderPanickedTryingNext,
}

impl TaggingMessage {
    fn describe(self) -> (String, &'static str, serde_json::Map<String, serde_json::Value>) {
        let (message, key, argument) = match self {
            Self::AlreadyTagged => ("Already tagged".into(), "backend.alreadyTagged", None),
            Self::AudioFeaturesAlreadyTagged => ("Already tagged!".into(), "backend.alreadyTagged", None),
            Self::NoMatch => ("No match!".into(), "backend.noMatch", None),
            Self::StoppedBeforeWriting => ("Stopped before writing tags".into(), "backend.stoppedBeforeWriting", None),
            Self::NoAlbumMatch => ("No album match for selected file".into(), "backend.noAlbumMatch", None),
            Self::AlbumWriteFailed => ("Failed writing album tags".into(), "backend.albumWriteFailed", None),
            Self::ShazamFailed(detail) => (format!("Error Shazaming file: {detail}"), "backend.shazamFailed", Some(("detail", detail))),
            Self::FileLoadFailed(detail) => (format!("Error loading file: {detail}"), "backend.fileLoadFailed", Some(("detail", detail))),
            Self::TrackMatchFailed(detail) => (format!("Error matching track: {detail}"), "backend.trackMatchFailed", Some(("detail", detail))),
            Self::TrackWriteFailed(detail) => (format!("Failed writing tags to file: {detail}"), "backend.trackWriteFailed", Some(("detail", detail))),
            Self::AudioFeaturesWriteFailed(detail) => (format!("Audio features failed writing to tag: {detail}"), "backend.audioFeaturesWriteFailed", Some(("detail", detail))),
            Self::AudioFeaturesSearchFailed(detail) => (format!("Audio features search track by ISRC error: {detail}"), "backend.audioFeaturesSearchFailed", Some(("detail", detail))),
            Self::ProviderWorkerPanicked => ("Provider worker panicked; source disabled for this run".into(), "backend.providerWorkerPanicked", None),
            Self::ProviderInitializationPanicked => ("Provider initialization panicked".into(), "backend.providerInitializationPanicked", None),
            Self::ProviderInitializationFailed(detail) => (format!("Could not initialize provider: {detail}"), "backend.providerInitializationFailed", Some(("detail", detail))),
            Self::ProviderUnavailable(provider) => (format!("Provider unavailable: {provider}"), "backend.providerUnavailable", Some(("provider", provider))),
            Self::ProviderDetailsUnavailable => ("Provider details are unavailable. Remaining fields can be retried.".into(), "backend.providerDetailsUnavailable", None),
            Self::ProviderRateLimited(seconds) => (format!("Provider rate limited. Retry in {seconds} s."),
                "backend.providerRateLimited", Some(("seconds", seconds.to_string()))),
            Self::ProviderPanickedTryingNext => ("Provider panicked; trying the next provider".into(), "backend.providerPanickedTryingNext", None),
        };
        let arguments = argument.into_iter().map(|(name, value)| (name.into(), serde_json::Value::String(value))).collect();
        (message, key, arguments)
    }
}

impl TaggingStatus {
    pub(crate) fn set_message(&mut self, message: TaggingMessage) {
        let (message, key, arguments) = message.describe();
        self.message = Some(message);
        self.message_key = Some(key.into());
        self.message_args = Some(arguments);
    }
}

/// Final outcomes are disjoint even when several providers fail before one succeeds.
#[derive(Default)]
struct TaggingResults {
    successful: BTreeSet<PathBuf>,
    failed: BTreeSet<PathBuf>,
    retryable: BTreeSet<PathBuf>,
}

impl TaggingResults {
    fn record(&mut self, status: &TaggingStatus) {
        match status.status {
            TaggingState::Ok => {
                self.failed.remove(&status.path);
                self.successful.insert(status.path.clone());
            }
            TaggingState::Partial | TaggingState::Deferred => {
                self.failed.remove(&status.path);
                self.retryable.insert(status.path.clone());
            }
            TaggingState::Error if !self.successful.contains(&status.path) && !self.retryable.contains(&status.path) => {
                self.failed.insert(status.path.clone());
            }
            _ => {}
        }
    }
}

/// Remember each file's marker state before this run can write any tags.
#[derive(Clone, Default)]
struct RunTagState {
    initially_tagged: Option<Arc<Mutex<HashMap<PathBuf, bool>>>>,
    pending_fields: Arc<Mutex<HashMap<PathBuf, Vec<SupportedTag>>>>,
}

impl RunTagState {
    fn new(config: &TaggerConfig) -> Self {
        Self { initially_tagged: config.skip_tagged.then(Default::default), ..Default::default() }
    }

    /// A partial detail response can still contribute tags. Later ordered fallbacks
    /// get exactly the outstanding fields and cannot replace an accepted value.
    fn save_match(&self, track: Track, config: &TaggerConfig, status: &mut TaggingStatus, details_failed: bool) -> Result<(), Error> {
        let pending = self.pending_fields.lock().unwrap().get(&status.path).cloned();
        let recovering = details_failed || pending.is_some();
        let mut narrowed;
        let write_config = if let Some(fields) = pending {
            narrowed = config.clone();
            narrowed.tags = fields;
            if config.tag_enabled(SupportedTag::MetaTags) { narrowed.tags.push(SupportedTag::MetaTags); }
            &narrowed
        } else { config };
        if details_failed && write_config.tags.iter().all(|tag| *tag == SupportedTag::MetaTags) {
            status.status = TaggingState::Deferred;
            return Ok(());
        }
        let supplied = track.merge_styles(&config.styles_options).write_fields(&status.path, write_config, recovering)?;
        if recovering {
            let missing: Vec<_> = write_config.tags.iter().copied()
                .filter(|t| *t != SupportedTag::MetaTags && !supplied.contains(t)).collect();
            if !missing.is_empty() || (details_failed && supplied.is_empty()) {
                if !config.multiplatform {
                    self.pending_fields.lock().unwrap().insert(status.path.clone(), missing);
                }
                status.status = if supplied.is_empty() { TaggingState::Deferred } else { TaggingState::Partial };
                if status.message_key.is_none() { status.set_message(TaggingMessage::ProviderDetailsUnavailable); }
                return Ok(());
            }
            self.pending_fields.lock().unwrap().remove(&status.path);
        }
        status.status = TaggingState::Ok;
        Ok(())
    }

    fn was_tagged(&self, path: &Path, tagged: bool) -> bool {
        let Some(initially_tagged) = &self.initially_tagged else { return tagged; };
        let mut initial = initially_tagged.lock().unwrap();
        if let Some(previous) = initial.get(path) { return *previous; }
        initial.insert(path.to_owned(), tagged);
        tagged
    }
}

// Wrap for sending into UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggingStatusWrap {
    pub status: TaggingStatus,
    pub platform: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    pub progress: f64,
}
impl TaggingStatusWrap {
    // pi = platform index, pl = platforms length, p = processed, total = total tracks in this platform
    pub fn wrap(platform: &str, status: &TaggingStatus, pi: usize, pl: usize, p: i64, total: usize) -> TaggingStatusWrap {
        TaggingStatusWrap {
            platform: platform.to_string(),
            provider_id: None,
            status: status.to_owned(),
            progress: (pi as f64 / pl as f64) + ((p as f64 / total as f64) / pl as f64)
        }
    }
}

pub struct Tagger {}
impl Tagger {

    // Returtns progress receiver, and file count
    pub fn tag_files(cfg: &TaggerConfig, files: Vec<PathBuf>, finished: Arc<Mutex<Option<TaggerFinishedData>>>) -> Receiver<TaggingStatusWrap> {
        Self::tag_files_inner(cfg, files, finished, false, None)
    }

    /// Opt-in report; a Helper manifest is the entire file-selection boundary.
    pub fn tag_files_report(cfg: &TaggerConfig, mut files: Vec<PathBuf>, finished: Arc<Mutex<Option<TaggerFinishedData>>>, selection: Option<report::Selection>) -> Result<Receiver<TaggingStatusWrap>, Error> {
        if let Some(selection) = &selection {
            selection.validate()?;
            if cfg.move_success || cfg.move_failed {
                return Err(anyhow!("Disable file moves for a Helper selection; keep its original locations"));
            }
            files = selection.files();
        }
        Ok(Self::tag_files_inner(cfg, files, finished, true, selection))
    }

    fn tag_files_inner(cfg: &TaggerConfig, mut files: Vec<PathBuf>, finished: Arc<Mutex<Option<TaggerFinishedData>>>, report_enabled: bool, selection: Option<report::Selection>) -> Receiver<TaggingStatusWrap> {
        STOP_TAGGING.store(false, Ordering::SeqCst);

        // A playlist can contain the same file more than once, including aliases.
        // Keep its first spelling for reports, but never schedule it twice.
        pipeline::deduplicate_files(&mut files);

        // Shuffle so album tag is more "efficient"
        if cfg.album_tagging {
            let mut rng = rand::rng();
            files.shuffle(&mut rng);
        }
        
        // let original_files = files.clone();
        let mut results = TaggingResults::default();
        let total_files = files.len();
        info!("Starting tagger with: {} files!", total_files);

        // Create thread
        let (tx, rx) = unbounded();
        let config = cfg.clone();
        std::thread::spawn(move || {
            let mut report = report_enabled.then(|| report::ReportBuilder::new(&files, selection));
            let run = RunTagState::new(&config);
            // Tag
            if !config.album_tagging {
                Self::tag_pipeline(&files, &config, &tx, &mut results, &mut report, &run);
            } else {
            // Album matching depends on provider-wide folder grouping.
            for (platform_index, platform) in config.platforms.iter().enumerate() {
                // For progress
                let mut processed = BTreeSet::new();
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
                let rx = match Tagger::tag_batch_in_run(&files, tagger, &config, threads, &run) {
                    Some(t) => t,
                    None => {
                        error!("Failed creating platform: {platform:?}, skipping...");
                        continue;
                    }
                };
                // Start tagging
                info!("Starting {platform}");
                let mut matched = BTreeSet::new();
                for status in rx {
                    info!("[{platform}] State: {:?}, Accuracy: {:?}, Path: {:?}", status.status, status.accuracy, status.path);
                    // A track-level and an album-level attempt can describe the same file.
                    processed.insert(status.path.clone());
                    // Send to UI
                    let mut update = TaggingStatusWrap::wrap(&platform_info.name, &status, platform_index,
                        config.platforms.len(), processed.len() as i64, total);
                    update.provider_id = Some(platform.clone());
                    tx.send(update).ok();

                    results.record(&status);
                    if let Some(report) = &mut report { report.record(platform, &status); }
                    // A successful fallback no longer needs the remaining providers.
                    if status.status == TaggingState::Ok && !config.multiplatform {
                        matched.insert(status.path);
                    }

                }
                // Workers already own this batch. Filter once per provider instead
                // of scanning the remaining files after every successful result.
                if !matched.is_empty() { files.retain(|path| !matched.contains(path)); }
            }
            }

            // Move files
            let mut successful_paths = vec![];
            let mut move_errors = vec![];
            for file in &results.successful {
                if config.move_success && config.move_success_path.is_some() {
                    match Self::move_file(file, &config.move_success_path.as_ref().unwrap()) {
                        Ok(p) => {
                            if let Some(report) = &mut report { report.moved(file, &p); }
                            successful_paths.push(p);
                        },
                        Err(e) => {
                            warn!("Failed moving file: {file:?} {e}");
                            move_errors.push(format!("{}: {e:#}", file.display()));
                            successful_paths.push(file.to_owned());
                        },
                    }
                } else {
                    successful_paths.push(PathBuf::from(file));
                }
            }
            let mut failed_paths = vec![];
            for file in &results.failed {
                if config.move_failed && config.move_failed_path.is_some() {
                    match Self::move_file(file, &config.move_failed_path.as_ref().unwrap()) {
                        Ok(p) => {
                            if let Some(report) = &mut report { report.moved(file, &p); }
                            failed_paths.push(p);
                        },
                        Err(e) => {
                            warn!("Failed moving file: {file:?} {e}");
                            move_errors.push(format!("{}: {e:#}", file.display()));
                            failed_paths.push(file.to_owned());
                        },
                    }
                } else {
                    failed_paths.push(PathBuf::from(file))
                }
            }
            std::mem::drop(results);
            std::mem::drop(files);

            // Capture final disk values before post-command can change the files.
            let (report_file, report_error) = match report {
                Some(report) => match Settings::get_folder().and_then(|folder| report
                    .finish(STOP_TAGGING.load(Ordering::SeqCst)).save(&folder.join("runs"))) {
                    Ok(path) => (Some(path.to_string_lossy().into_owned()), None),
                    Err(error) => {
                        warn!("Could not save tagging report: {error}");
                        (None, Some("Could not save the results JSON; check the application log".into()))
                    },
                },
                None => (None, None),
            };

            // Tagging ended, save lists of files
            match Self::write_results(successful_paths, failed_paths, &config) {
                Ok((failed, success)) => {
                    info!("Written failed songs to: {}, successful to: {}", failed, success);
                    *finished.lock().unwrap() = Some(TaggerFinishedData {
                        failed_file: failed, success_file: success, move_errors, report_file, report_error
                    });
                },
                Err(e) => {
                    warn!("Failed writing result playlists: {e}");
                    move_errors.push("Failed writing result playlists; check the application log".into());
                    *finished.lock().unwrap() = Some(TaggerFinishedData {
                        failed_file: String::new(), success_file: String::new(), move_errors, report_file, report_error
                    });
                }
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
        Self::load_track_in_run(path, config, &RunTagState::default())
    }

    fn load_track_in_run(path: impl AsRef<Path>, config: &TaggerConfig, run: &RunTagState) -> (Option<AudioFileInfo>, TaggingStatus) {
        // Output
        let mut out = TaggingStatus {
            status: TaggingState::Error,
            path: path.as_ref().to_owned(),
            accuracy: None,
            lookup_ms: None,
            message: None,
            message_key: None,
            message_args: None,
            used_shazam: false,
            release_id: None,
            track_id: None,
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

        // Forced recognition still honours an existing AutoTagger marker.
        let force_shazam = config.enable_shazam && config.force_shazam;
        if config.skip_tagged && force_shazam {
            if let Ok(tags) = Tag::load_file(&path, true) {
                if run.was_tagged(path.as_ref(), tagged_status(*tags.tag()).at()) {
                    return Self::already_tagged(out);
                }
            }
        }

        // Load audio file info by shazam or tags
        let mut info = if force_shazam {
            match AudioFileInfo::shazam(&path) {
                Ok(i) => {
                    out.used_shazam = true;
                    i
                },
                Err(e) => {
                    out.status = TaggingState::Skipped;
                    out.set_message(TaggingMessage::ShazamFailed(e.to_string()));
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
                                out.set_message(TaggingMessage::FileLoadFailed(e.to_string()));
                                return (None, out);
                            }
                        }
                    } else {
                        out.status = TaggingState::Skipped;
                        warn!("Error loading file: {}", e);
                        out.set_message(TaggingMessage::FileLoadFailed(e.to_string()));
                        return (None, out);
                    }
                }
            }
        };

        // Skip tagged
        if config.skip_tagged && run.was_tagged(&info.path, info.tagged.at()) {
            return Self::already_tagged(out);
        }

        // Load duration for matching
        if config.match_duration {
            info.load_duration();
        }

        (Some(info), out)
    }

    fn already_tagged(mut status: TaggingStatus) -> (Option<AudioFileInfo>, TaggingStatus) {
        info!("Skipping (already tagged): {:?}", status.path);
        status.status = TaggingState::Skipped;
        status.set_message(TaggingMessage::AlreadyTagged);
        (None, status)
    }

    /// Tag single track
    pub fn tag_track<T>(path: impl AsRef<Path>, tagger: &mut Box<T>, config: &TaggerConfig) -> TaggingStatus 
    where T: AutotaggerSource + ?Sized
    {
        Self::tag_track_with_cancel(path, tagger, config, &RunTagState::default(), &|| STOP_TAGGING.load(Ordering::SeqCst))
    }

    fn tag_track_with_cancel<T>(
        path: impl AsRef<Path>, tagger: &mut Box<T>, config: &TaggerConfig,
        run: &RunTagState, cancelled: &impl Fn() -> bool,
    ) -> TaggingStatus
    where T: AutotaggerSource + ?Sized
    {
        info!("Tagging: {:?}", path.as_ref());
        // Load track
        let (info, mut out) = Self::load_track_in_run(&path, config, run);
        let info = match info {
            Some(info) => info,
            None => return out,
        };
        if Self::mark_cancelled(&mut out, cancelled) { return out; }
       
        // Time matching separately from local metadata reads and tag writes.
        let lookup_started = Instant::now();
        let result = tagger.match_track(&info, config);
        out.lookup_ms = Some(lookup_started.elapsed().as_millis() as u64);
        if Self::mark_cancelled(&mut out, cancelled) { return out; }
        let mut tracks = match result {
            Ok(o) => {
                if o.is_empty() {
                    out.set_message(TaggingMessage::NoMatch);
                    return out;
                }
                o
            },
            // Failed matching track
            Err(e) => {
                error!("Matching error: {} ({:?})", e, path.as_ref());
                if let Some(limited) = e.downcast_ref::<ProviderRateLimited>() {
                    out.status = TaggingState::Deferred;
                    if limited.deferred() { out.lookup_ms = None; }
                    out.set_message(TaggingMessage::ProviderRateLimited(limited.retry_after_seconds()));
                } else {
                    out.set_message(TaggingMessage::TrackMatchFailed(e.to_string()));
                }
                return out;
            }
        };

        // Get & extend track
        MatchingUtils::sort_tracks(&mut tracks, config);
        let mut track = tracks.remove(0);
        drop(tracks);
        let details_error = tagger.extend_track(&mut track.track, config).err();
        if let Some(error) = &details_error {
            warn!("Failed extending track: {error}");
            out.set_message(match error.downcast_ref::<ProviderRateLimited>() {
                Some(limited) => TaggingMessage::ProviderRateLimited(limited.retry_after_seconds()),
                None => TaggingMessage::ProviderDetailsUnavailable,
            });
        }
        if Self::mark_cancelled(&mut out, cancelled) { return out; }

        // Save
        out.release_id = track.track.release_id.clone();
        out.track_id = track.track.track_id.clone();
        out.reason = Some(track.reason);
        out.accuracy = Some(track.accuracy);
        if let Err(error) = run.save_match(track.track, config, &mut out, details_error.is_some()) {
            error!("Failed writing tags to file: {error}");
            out.set_message(TaggingMessage::TrackWriteFailed(error.to_string()));
        }

        out
    }

    fn mark_cancelled(status: &mut TaggingStatus, cancelled: &impl Fn() -> bool) -> bool {
        if !cancelled() { return false; }
        status.status = TaggingState::Skipped;
        status.set_message(TaggingMessage::StoppedBeforeWriting);
        true
    }

    // Tag all files with threads specified in config
    pub fn tag_batch(files: &Vec<PathBuf>, tagger: &mut Box<dyn AutotaggerSourceBuilder + Send + Sync>, config: &TaggerConfig, threads: u16) -> Option<Receiver<TaggingStatus>> {
        Self::tag_batch_in_run(files, tagger, config, threads, &RunTagState::new(config))
    }

    fn tag_batch_in_run(
        files: &[PathBuf], tagger: &mut Box<dyn AutotaggerSourceBuilder + Send + Sync>,
        config: &TaggerConfig, threads: u16, run: &RunTagState,
    ) -> Option<Receiver<TaggingStatus>> {
        info!("Starting tagging: {} files, {} threads!", files.len(), threads);
        let (tx, rx) = unbounded();
        let (file_tx, file_rx): (Sender<PathBuf>, Receiver<PathBuf>) = unbounded();
        let (finished_tx, finished_rx) = unbounded();

        let mut sources = Vec::new();
        for _ in 0..threads.max(1) {
            match tagger.get_source(config) {
                Ok(source) => sources.push(source),
                Err(error) => warn!("Failed creating tagging source: {error}"),
            }
        }
        if sources.is_empty() { return None; }

        let album_source = if config.album_tagging {
            tagger.get_source(config).map_err(|e| warn!("Album lookup unavailable: {e}")).ok()
        } else { None };
        let use_album = album_source.is_some();
        let album_tagging = Arc::new(Mutex::new(AlbumTagContext::new()));
        if use_album { album_tagging.lock().unwrap().init(files); }

        for mut source in sources {
            let tx = tx.clone();
            let file_rx = file_rx.clone();
            let config = config.clone();
            let finished_tx = finished_tx.clone();
            let album_tagging = album_tagging.clone();
            let run = run.clone();
            std::thread::spawn(move || {
                while let Ok(f) = file_rx.recv() {
                    // Stop tagging
                    if STOP_TAGGING.load(Ordering::SeqCst) {
                        break;
                    }

                    // Check if not marked for album tagging
                    if use_album && album_tagging.lock().unwrap().is_marked(&f) {
                        continue;
                    }

                    // Tag
                    let res = Tagger::tag_track_with_cancel(&f, &mut source, &config, &run,
                        &|| STOP_TAGGING.load(Ordering::SeqCst));
                    if use_album {
                        album_tagging.lock().unwrap().process(&res, &config);
                    }
                    tx.send(res).ok();
                }
                finished_tx.send(0u8).ok();
            });
        }

        if let Some(mut source) = album_source {
            let config = config.clone();
            let run = run.clone();
            std::thread::spawn(move || {
                for _ in finished_rx {}
                let folders = &album_tagging.lock().unwrap().folders;
                let cancelled = || STOP_TAGGING.load(Ordering::SeqCst);
                for (path, stats) in folders {
                    if cancelled() { break; }
                    let Some(release) = &stats.album_id else { continue };
                    let mut remaining: BTreeSet<_> = stats.files.iter()
                        .filter(|file| !stats.searched.contains(*file)).cloned().collect();
                    match Self::tag_album_with_cancel(&stats.files, release,
                        &mut source, &config, &run, &cancelled) {
                        Ok(statuses) => {
                            for status in statuses {
                                if status.message_key.as_deref() == Some("backend.noAlbumMatch")
                                    && remaining.contains(&status.path) { continue; }
                                remaining.remove(&status.path);
                                tx.send(status).ok();
                            }
                        }
                        Err(e) => warn!("Album lookup failed for {}: {e}", path.display()),
                    }
                    // An ambiguous album row or an unavailable release must not consume
                    // the only regular lookup for a file skipped by the folder shortcut.
                    for file in remaining {
                        if cancelled() { break; }
                        let status = Self::tag_track_with_cancel(&file, &mut source, &config, &run, &cancelled);
                        tx.send(status).ok();
                    }
                }
            });
        }

        // Send files
        for f in files {
            file_tx.send(f.to_owned()).ok();
        }
        Some(rx)
    }

    /// Tag an album by ID
    pub fn tag_album(files: &[PathBuf], release_id: &str, source: &mut Box<dyn AutotaggerSource>, config: &TaggerConfig) -> Result<Vec<TaggingStatus>, Error> {
        Self::tag_album_with_cancel(files, release_id, source, config, &RunTagState::default(),
            &|| STOP_TAGGING.load(Ordering::SeqCst))
    }

    fn tag_album_with_cancel(
        files: &[PathBuf], release_id: &str, source: &mut Box<dyn AutotaggerSource>,
        config: &TaggerConfig, run: &RunTagState, cancelled: &impl Fn() -> bool,
    ) -> Result<Vec<TaggingStatus>, Error> {
        if cancelled() { return Ok(vec![]); }
        info!("Album tagging release: {release_id} in {} selected files", files.len());

        let mut config = config.clone();
        config.enable_shazam = false;
        config.force_shazam = false;

        // Get album
        let album = source.get_album(&release_id, &config)?.ok_or(anyhow!("Album with id: {release_id} not found"))?;
        if album.tracks.is_empty() {
            return Err(anyhow!("Album {release_id} has no tracks!"))
        }

        let mut statuses = vec![];

        // Preserve the caller's playlist/selection boundary; never rescan the folder.
        for file in files {
            if cancelled() { break; }
            let (info, mut status) = Self::load_track_in_run(&file, &config, run);
            let info = match info {
                Some(i) => i,
                None => {
                    warn!("Failed to load track info for file: {}", file.display());
                    statuses.push(status);
                    continue;
                }
            };

            // Find closest match
            let Some(track) = MatchingUtils::match_album_track(&info, &album.tracks, &config) else {
                status.set_message(TaggingMessage::NoAlbumMatch);
                statuses.push(status);
                continue;
            };

            status.track_id = track.track.track_id.clone();
            status.release_id = track.track.release_id.clone().or_else(|| Some(release_id.into()));
            status.accuracy = Some(track.accuracy);
            if Self::mark_cancelled(&mut status, cancelled) {
                statuses.push(status);
                break;
            }
            if let Err(e) = run.save_match(track.track, &config, &mut status, false) {
                status.status = TaggingState::Error;
                status.set_message(TaggingMessage::AlbumWriteFailed);
                error!("Album tag writing tags failed: {e} ({})", file.display());
            }

            // Save status
            status.reason = Some(MatchReason::Album);
            statuses.push(status);
        }

        Ok(statuses)
    }

    /// Move file to target dir if enabled
    fn move_file(source: impl AsRef<Path>, target: impl AsRef<Path>) -> Result<PathBuf, Error> {
        let filename = source.as_ref().file_name().ok_or_else(|| anyhow!("Source has no filename"))?;
        let target = target.as_ref().join(filename);
        onetagger_tag::file_ops::transfer_file(source.as_ref(), &target, false, false)?;

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
        let mut seen = std::collections::HashSet::with_capacity(paths.len());
        for file in paths {
            if !seen.insert(file) { continue; }
            if let Some(folder) = file.parent() {
                self.folders.entry(folder.to_owned()).or_default().files.push(file.clone());
            }
        }
    }

    /// Save info from tagging status data
    /// Returns (Path, Release ID)
    pub fn process(&mut self, status: &TaggingStatus, config: &TaggerConfig) -> Option<(PathBuf, String)> {
        let path = status.path.parent()?;
        let stats = self.folders.get_mut(path)?;
        stats.searched.insert(status.path.clone());
        if status.status != TaggingState::Ok { return None; }
        let release_id = status.release_id.as_ref()?;
        if stats.album_id.is_some() {
            return None;
        }

        let count = stats.albums.entry(release_id.to_owned()).or_default();
        *count += 1;
        if (*count as f32 / stats.files.len() as f32) >= config.album_tagging_ratio {
            stats.album_id = Some(release_id.clone());
            return Some((path.to_owned(), release_id.to_owned()));
        }
        None
    }

    /// Check if path is marked
    pub fn is_marked(&self, path: impl AsRef<Path>) -> bool {
        if let Some(parent) = path.as_ref().parent() {
            if let Some(stats) = self.folders.get(parent) {
                return stats.album_id.is_some();
            }
        }
        false
    }

}

#[derive(Debug, Clone, Default)]
struct AlbumTagFolderStats {
    /// Only the files selected for this batch, including playlist subsets.
    pub files: Vec<PathBuf>,
    searched: BTreeSet<PathBuf>,
    /// album_id: count
    pub albums: HashMap<String, usize>,
    /// The first release to reach the configured folder threshold.
    album_id: Option<String>,
}

/// When AT finishes this will contain some extra data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggerFinishedData {
    pub failed_file: String,
    pub success_file: String,
    pub move_errors: Vec<String>,
    pub report_file: Option<String>,
    pub report_error: Option<String>,
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
            let r = s.match_track(&info, &config).map(|mut m| { MatchingUtils::deduplicate_matches(&mut m); m });
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

#[cfg(test)]
mod regression_tests;

