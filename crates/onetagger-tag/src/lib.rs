// TODO: Use log somewhere
#[cfg(feature = "tag")]
#[macro_use] extern crate log;

use serde::{Serialize, Deserialize};
use std::time::Duration;
use std::error::Error;

#[cfg(feature = "tag")]
use std::collections::HashMap;

#[cfg(feature = "tag")]
pub mod id3;
#[cfg(feature = "tag")]
pub mod flac;
#[cfg(feature = "tag")]
pub mod mp4;
#[cfg(feature = "tag")]
pub mod vorbis;
#[cfg(feature = "tag")]
mod wav;

// Supported extensions
pub static EXTENSIONS : [&'static str; 11] = [".mp3", ".flac", ".aif", ".aiff", ".m4a", 
    ".mp4", ".wav", ".ogg", ".opus", ".spx", ".oga"];

#[cfg(feature = "tag")]
pub enum Tag {
    FLAC(flac::FLACTag),
    ID3(id3::ID3Tag),
    MP4(mp4::MP4Tag),
    Vorbis(vorbis::VorbisTag)
}

#[cfg(feature = "tag")]
impl Tag {
    pub fn load_file(path: &str, allow_new: bool) -> Result<Tag, Box<dyn Error>> {
        // FLAC
        if path.to_lowercase().ends_with(".flac") {
            return Ok(Tag::FLAC(flac::FLACTag::load_file(path)?));
        }
        // MP4
        if path.to_lowercase().ends_with(".m4a") || path.to_lowercase().ends_with(".mp4") {
            return Ok(Tag::MP4(mp4::MP4Tag::load_file(path)?));
        }

        // Vorbis
        if path.to_lowercase().ends_with(".ogg") || path.to_lowercase().ends_with(".opus") || path.to_lowercase().ends_with(".oga") || path.to_lowercase().ends_with(".spx") {
            return Ok(Tag::Vorbis(vorbis::VorbisTag::load_file(path)?));
        }

        // ID3
        let tag = if allow_new {
            id3::ID3Tag::load_or_new(path)
        } else {
            id3::ID3Tag::load_file(path)?
        };
        Ok(Tag::ID3(tag))
    }

    // Set proper separators for every format
    pub fn set_separators(&mut self, separators: &TagSeparators) {
        match self {
            Tag::FLAC(tag) => tag.set_separator(separators.vorbis.as_ref().unwrap_or(&String::new())),
            Tag::ID3(tag) => tag.set_separator(&separators.id3),
            Tag::MP4(tag) => tag.set_separator(&separators.mp4),
            Tag::Vorbis(tag) => tag.set_separator(separators.vorbis.as_ref().unwrap_or(&String::new())),
        }
    }

    // Get generic
    pub fn tag(&self) -> Box<&dyn TagImpl> {
        match self {
            Tag::FLAC(tag) => Box::new(tag),
            Tag::ID3(tag) => Box::new(tag),
            Tag::MP4(tag) => Box::new(tag),
            Tag::Vorbis(tag) => Box::new(tag),
        }
    }
    pub fn tag_mut(&mut self) -> Box<&mut dyn TagImpl> {
        match self {
            Tag::FLAC(tag) => Box::new(tag),
            Tag::ID3(tag) => Box::new(tag),
            Tag::MP4(tag) => Box::new(tag),
            Tag::Vorbis(tag) => Box::new(tag)
        }
    }

    // Get format
    pub fn format(&self) -> AudioFileFormat {
        match self {
            Tag::FLAC(_) => AudioFileFormat::FLAC,
            Tag::MP4(_) => AudioFileFormat::MP4,
            Tag::ID3(id3) => match id3.format {
                id3::ID3AudioFormat::MP3 => AudioFileFormat::MP3,
                id3::ID3AudioFormat::AIFF => AudioFileFormat::AIFF,
                id3::ID3AudioFormat::WAV => AudioFileFormat::WAV
            },
            Tag::Vorbis(_) => AudioFileFormat::OGG
        }
    }
}

#[cfg(feature = "tag")]
pub trait TagImpl {
    /// Write file to path
    fn save_file(&mut self, path: &str) -> Result<(), Box<dyn Error>>;

    /// Since all formats right now support separators
    fn set_separator(&mut self, separator: &str);
    /// Get the separator
    fn get_separator(&self) -> Option<String>;

    /// Get all string tags
    fn all_tags(&self) -> HashMap<String, Vec<String>>;

    /// Set/Get dates
    fn get_date(&self) -> Option<TagDate>;
    fn set_date(&mut self, date: &TagDate, overwrite: bool);
    fn set_publish_date(&mut self, date: &TagDate, overwrite: bool);

    /// Get/Set rating as 1 - 5 stars value
    fn get_rating(&self) -> Option<u8>;
    fn set_rating(&mut self, rating: u8, overwrite: bool);

    /// Set/Get album art
    fn set_art(&mut self, kind: CoverType, mime: &str, description: Option<&str>, data: Vec<u8>);
    /// To not load all album arts
    fn has_art(&self) -> bool;
    fn get_art(&self) -> Vec<Picture>;
    fn remove_art(&mut self, kind: CoverType);

    /// Set/Get named field
    fn set_field(&mut self, field: Field, value: Vec<String>, overwrite: bool);
    fn get_field(&self, field: Field) -> Option<Vec<String>>;

    /// Set/Get by tag field name
    fn set_raw(&mut self, tag: &str, value: Vec<String>, overwrite: bool);
    fn get_raw(&self, tag: &str) -> Option<Vec<String>>;
    fn remove_raw(&mut self, tag: &str);

    /// Set lyrics
    fn set_lyrics(&mut self, lyrics: &Lyrics, synced: bool, overwrite: bool);

    /// Set track number (because formats like MP3 and M4A use custom format)
    /// Track number is string because of platforms like discogs
    fn set_track_number(&mut self, track_number: &str, track_total: Option<u16>, overwrite: bool);

    /// Set whether the track is explicit
    fn set_explicit(&mut self, explicit: bool);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagSeparators {
    pub id3: String,
    pub vorbis: Option<String>,
    pub mp4: String
}

impl Default for TagSeparators {
    fn default() -> Self {
        TagSeparators {
            id3: ", ".to_string(),
            vorbis: None,
            mp4: ", ".to_string()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFileFormat {
    FLAC, AIFF, MP3, MP4, WAV, OGG
}

impl AudioFileFormat {
    // Recognize format from extension
    pub fn from_extension(ext: &str) -> Option<AudioFileFormat> {
        match &ext.to_lowercase()[..] {
            "flac" => Some(AudioFileFormat::FLAC),
            "aiff" | "aif" => Some(AudioFileFormat::AIFF),
            "mp3" => Some(AudioFileFormat::MP3),
            "m4a" | "mp4" => Some(AudioFileFormat::MP4),
            "wav" => Some(AudioFileFormat::WAV),
            "ogg" | "opus" | "spx" | "oga" => Some(AudioFileFormat::OGG),
            _ => None
        }
    }
}

/// Tag fields from UI
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub struct FrameName {
    pub id3: String,
    pub vorbis: String,
    pub mp4: String
}

impl FrameName {
    /// All formats the same Frame name
    pub fn same(name: &str) -> FrameName {
        FrameName {
            id3: name.to_string(),
            vorbis: name.to_string(),
            mp4: name.to_string()
        }
    }

    /// Shorthand for creating
    pub fn new(id3: &str, vorbis: &str, mp4: &str) -> FrameName {
        FrameName { id3: id3.to_string(), vorbis: vorbis.to_string(), mp4: mp4.to_string() }
    }

    /// Get raw value by format
    pub fn by_format(&self, format: &AudioFileFormat) -> String {
        match format.to_owned() {
            AudioFileFormat::AIFF => self.id3.to_string(),
            AudioFileFormat::MP3 => self.id3.to_string(),
            AudioFileFormat::WAV => self.id3.to_string(),
            AudioFileFormat::FLAC => self.vorbis.to_string(),
            AudioFileFormat::MP4 => self.mp4.to_string(),
            AudioFileFormat::OGG => self.vorbis.to_string()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    pub kind: CoverType,
    pub data: Vec<u8>,
    pub description: String,
    pub mime: String
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CoverType {
    CoverFront,
    CoverBack,
    Other,
    Artist,
    Icon,
    OtherIcon,
    Leaflet,
    Media,
    LeadArtist,
    Conductor,
    Band,
    Composer,
    Lyricist,
    RecordingLocation,
    DuringRecording,
    DuringPerformance,
    ScreenCapture,
    BrightFish,
    Illustration,
    BandLogo,
    PublisherLogo,
    Undefined
}

impl CoverType {
    // Get all the types
    pub fn types() -> [CoverType; 22] {
        [CoverType::CoverFront, CoverType::CoverBack, CoverType::Other, CoverType::Artist,
        CoverType::Icon, CoverType::OtherIcon, CoverType::Leaflet, CoverType::Media, CoverType::LeadArtist,
        CoverType::Conductor, CoverType::Band, CoverType::Composer, CoverType::Lyricist,
        CoverType::RecordingLocation, CoverType::DuringRecording, CoverType::DuringPerformance,
        CoverType::ScreenCapture, CoverType::BrightFish, CoverType::Illustration, CoverType::BandLogo,
        CoverType::PublisherLogo, CoverType::Undefined]
    }
}

#[derive(Debug, Clone)]
pub struct TagDate {
    pub year: i32,
    pub month: Option<u8>,
    pub day: Option<u8>
}

impl TagDate {
    // If has day and month
    pub fn has_md(&self) -> bool {
        return self.month.is_some() && self.day.is_some();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field {
    Title,
    Artist,
    Album,
    AlbumArtist,
    Key,
    BPM,
    Genre,
    Style,
    Label,
    ISRC,
    CatalogNumber,
    Version,
    TrackNumber,
    Duration,
    Remixer,
    Mood,
    TrackTotal,
    DiscNumber,
}

impl Field {
    /// Get tag name by format
    pub fn by_format(&self, format: &AudioFileFormat) -> &'static str {
        match format {
            AudioFileFormat::FLAC => self.vorbis(),
            AudioFileFormat::AIFF => self.id3(),
            AudioFileFormat::WAV => self.id3(),
            AudioFileFormat::MP3 => self.id3(),
            AudioFileFormat::MP4 => self.mp4(),
            AudioFileFormat::OGG => self.vorbis(),
        }
    }

    /// Convert to ID3 frame name
    pub fn id3(&self) -> &'static str {
        match self {
            Field::Title => "TIT2",
            Field::Artist => "TPE1",
            Field::AlbumArtist => "TPE2",
            Field::Album => "TALB",
            Field::Key => "TKEY",
            Field::BPM => "TBPM",
            Field::Genre => "TCON",
            Field::Label => "TPUB",
            Field::Style => "STYLE",
            Field::ISRC => "TSRC",
            Field::CatalogNumber => "CATALOGNUMBER",
            Field::Version => "TIT3",
            Field::TrackNumber => "TRCK",
            Field::Duration => "TLEN",
            Field::Remixer => "TPE4",
            Field::Mood => "TMOO",
            Field::TrackTotal => "TRCK",
            Field::DiscNumber => "TPOS",
        }
    }

    /// Convert to VORBIS frame name
    pub fn vorbis(&self) -> &'static str {
        match self {
            Field::Title => "TITLE",
            Field::Artist => "ARTIST",
            Field::AlbumArtist => "ALBUMARTIST",
            Field::Album => "ALBUM",
            Field::Key => "INITIALKEY",
            Field::BPM => "BPM",
            Field::Genre => "GENRE",
            Field::Label => "LABEL",
            Field::Style => "STYLE",
            Field::ISRC => "ISRC",
            Field::CatalogNumber => "CATALOGNUMBER",
            Field::Version => "SUBTITLE",
            Field::TrackNumber => "TRACKNUMBER",
            Field::Duration => "LENGTH",
            Field::Remixer => "REMIXER",
            Field::Mood => "MOOD",
            Field::TrackTotal => "TRACKTOTAL",
            Field::DiscNumber => "DISCNUMBER",
        }
    }

    /// Convert to MP4 frame name
    pub fn mp4(&self) -> &'static str {
        match self {
            Field::Title => "©nam",
            Field::Artist => "©ART",
            Field::AlbumArtist => "aART",
            Field::Album => "©alb",
            Field::BPM => "tmpo",
            Field::Genre => "©gen",
            Field::Label => "com.apple.iTunes:LABEL",
            Field::ISRC => "com.apple.iTunes:ISRC",
            Field::CatalogNumber => "com.apple.iTunes:CATALOGNUMBER",
            Field::Version => "desc",
            Field::TrackNumber => "trkn",
            Field::Remixer => "com.apple.iTunes:REMIXER",
            Field::Key => "com.apple.iTunes:initialkey",
            Field::Style => "com.apple.iTunes:STYLE",
            Field::Duration => "com.apple.iTunes:LENGTH",
            Field::Mood => "com.apple.iTunes:MOOD",
            Field::TrackTotal => "trkn",
            Field::DiscNumber => "disk",
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum TagChange {
    Raw { tag: String, value: Vec<String> },
    Rating { value: u8 },
    Genre { value: Vec<String> },
    Remove { tag: String },
    RemovePicture { kind: CoverType },

    // For adding from UI
    #[cfg(feature = "tag")]
    AddPictureBase64 { kind: CoverType, description: String, data: String, mime: String },

    #[cfg(feature = "tag")]
    #[serde(rename = "id3Comments")]
    ID3Comments { comments: Vec<id3::ID3Comment> },

    #[cfg(feature = "tag")]
    #[serde(rename = "id3UnsynchronizedLyrics")]
    ID3UnsynchronizedLyrics { lyrics: Vec<id3::ID3Comment> },

    #[cfg(feature = "tag")]
    #[serde(rename = "id3Popularimeter")]
    ID3Popularimeter { popm: id3::ID3Popularimeter }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagChanges {
    changes: Vec<TagChange>,
    pub path: String,
    separators: TagSeparators,
    id3v24: bool,
    id3_comm_lang: Option<String>
}

#[cfg(feature = "tag")]
impl TagChanges {
    // Save all changes to file
    pub fn commit(&self) -> Result<Tag, Box<dyn Error>> {
        use base64::Engine;
        
        let mut tag_wrap = Tag::load_file(&self.path, false)?;
        tag_wrap.set_separators(&self.separators);

        // Format specific changes
        if let Tag::ID3(id3) = &mut tag_wrap {
            id3.set_id3v24(self.id3v24);
            if let Some(lang) = self.id3_comm_lang.as_ref() {
                if !lang.is_empty() {
                    id3.set_comm_lang(lang.to_string());
                }
            }

            for change in self.changes.clone() {
                match change {
                    TagChange::ID3Comments {comments} => id3.set_comments(&comments),
                    TagChange::ID3UnsynchronizedLyrics {lyrics} => id3.set_unsync_lyrics(&lyrics),
                    TagChange::ID3Popularimeter {popm} => id3.set_popularimeter(&popm),
                    _ => {}
                }
            }
        }

        // MP4 doesn't have any way to distinguish between artwork types so abstraction to do that
        // Not very efficient, but rarely used and should work
        if let Tag::MP4(mp4) = &mut tag_wrap {
            // Get album art indexes
            let mut indicies: Vec<usize> = self.changes.iter().filter_map(|c| match c {
                TagChange::RemovePicture {kind} => CoverType::types().iter().position(|k| k == kind),
                _ => None
            }).collect();
            // Last to first
            indicies.sort();
            indicies.reverse();
            let types = CoverType::types();
            for i in indicies {
                mp4.remove_art(types[i].to_owned());
            };
        }

        let format = tag_wrap.format();
        let tag = tag_wrap.tag_mut();
        // Match changes
        for change in self.changes.clone() {
            match change {
                TagChange::Raw {tag: t, value} => tag.set_raw(&t, value, true),
                TagChange::Rating {value} => tag.set_rating(value, true),
                TagChange::Genre {value} => tag.set_field(Field::Genre, value, true),
                TagChange::Remove {tag: t} => tag.remove_raw(&t),
                TagChange::RemovePicture {kind} => if format != AudioFileFormat::MP4 { tag.remove_art(kind) },
                TagChange::AddPictureBase64 {kind, description, data, mime} => tag.set_art(kind, &mime, Some(&description), base64::engine::general_purpose::STANDARD.decode(&data)?),
                _ => {}
            }
        }
        // Save
        tag.save_file(&self.path)?;

        Ok(tag_wrap)
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub struct Lyrics {
    /// Double vec for paragraph separation
    pub paragraphs: Vec<Vec<LyricsLine>>,
    pub language: String,
}

impl Lyrics {
    /// Join into text
    pub fn text(&self) -> String {
        self.paragraphs.iter().map(|p| 
            p.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n")
        ).collect::<Vec<_>>().join("\n\n")
    }

    /// Parse MM:SS.ms (optionally just SS.ms)
    pub fn parse_lrc_timestamp(input: &str) -> Result<Duration, Box<dyn Error>> {
        let mut minutes = 0;
        let parts = input.split(":").collect::<Vec<_>>();
        let seconds = if parts.len() == 2 {
            minutes = parts[0].parse::<u32>()?;
            parts[1].parse::<f32>()?
        } else {
            parts[0].parse()?
        };
        return Ok(Duration::from_secs_f32(seconds + minutes as f32 * 60.0))
    }

    /// Are the lyrics synced?
    pub fn synced(&self) -> bool {
        self.paragraphs.first().map(|p| p.first().map(|l| l.start.is_some())).flatten().unwrap_or(false)
    }

    /// Iterate over lines
    pub fn iter_lines(&self) -> impl Iterator<Item = &LyricsLine> {
        self.paragraphs.iter().flatten()
    }

    /// Iterate over lines (owned)
    pub fn into_iter_lines(self) -> impl Iterator<Item = LyricsLine> {
        self.paragraphs.into_iter().flatten()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub struct LyricsLine {
    pub text: String,
    pub start: Option<Duration>,
    pub end: Option<Duration>,
    /// Optional
    pub parts: Vec<LyricsLinePart>
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub struct LyricsLinePart {
    pub text: String,
    pub start: Option<Duration>,
    pub end: Option<Duration>
}