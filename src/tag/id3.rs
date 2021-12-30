use std::collections::HashMap;
use std::error::Error;
use std::convert::TryInto;
use id3::{Version, Tag, Timestamp, Frame, Content};
use id3::frame::{Picture, PictureType, Comment, Lyrics};
use serde::{Serialize, Deserialize};
use crate::tag::{TagDate, CoverType, Field, TagImpl};

const COVER_TYPES: [(PictureType, CoverType); 21] = [
    (PictureType::Other, CoverType::Other),
    (PictureType::Icon, CoverType::Icon),
    (PictureType::OtherIcon, CoverType::OtherIcon),
    (PictureType::CoverFront, CoverType::CoverFront),
    (PictureType::CoverBack, CoverType::CoverBack),
    (PictureType::Leaflet, CoverType::Leaflet),
    (PictureType::Media, CoverType::Media),
    (PictureType::LeadArtist, CoverType::LeadArtist),
    (PictureType::Artist, CoverType::Artist),
    (PictureType::Conductor, CoverType::Conductor),
    (PictureType::Band, CoverType::Band),
    (PictureType::Composer, CoverType::Composer),
    (PictureType::Lyricist, CoverType::Lyricist),
    (PictureType::RecordingLocation, CoverType::RecordingLocation),
    (PictureType::DuringRecording, CoverType::DuringRecording),
    (PictureType::DuringPerformance, CoverType::DuringPerformance),
    (PictureType::ScreenCapture, CoverType::ScreenCapture),
    (PictureType::BrightFish, CoverType::BrightFish),
    (PictureType::Illustration, CoverType::Illustration),
    (PictureType::BandLogo, CoverType::BandLogo),
    (PictureType::PublisherLogo, CoverType::PublisherLogo),
];

#[derive(Debug, Clone, PartialEq)]
pub enum ID3AudioFormat {
    MP3, AIFF
}

pub struct ID3Tag {
    tag: Tag,
    pub format: ID3AudioFormat,
    pub id3_separator: String,
    pub id3v24: bool
}

impl ID3Tag {
    // Read from file
    pub fn load_file(path: &str) -> Result<ID3Tag, Box<dyn Error>> {        
        // MP3
        if path.to_lowercase().ends_with(".mp3") {
            let tag = Tag::read_from_path(path)?;
            let version = tag.version();
            return Ok(ID3Tag {
                tag,
                format: ID3AudioFormat::MP3,
                id3_separator: String::from(", "),
                id3v24: match version {
                    Version::Id3v24 => true,
                    _ => false
                }
            }.into());
        }
        // AIFF
        if path.to_lowercase().ends_with(".aif") || path.to_lowercase().ends_with(".aiff") {
            let tag = Tag::read_from_aiff(path)?;
            let version = tag.version();
            return Ok(ID3Tag {
                tag,
                format: ID3AudioFormat::AIFF,
                id3_separator: String::from(", "),
                id3v24: match version {
                    Version::Id3v24 => true,
                    _ => false
                }
            }.into());
        }

        // Unsupported
        Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unsupported format!").into())
    }

    // Load tag from file or create new
    pub fn load_or_new(path: &str) -> ID3Tag {
        let format = if path.to_lowercase().ends_with(".mp3") {
            ID3AudioFormat::MP3
        } else {
            ID3AudioFormat::AIFF
        };

        match ID3Tag::load_file(path) {
            Ok(tag) => tag,
            Err(e) => {
                warn!("Failed loading: {}, creating new tag. {}", path, e);
                ID3Tag {
                    tag: Tag::new(),
                    format,
                    id3_separator: String::from(", "),
                    id3v24: true
                }
            }
        }
    }

    // ID3 Settings
    pub fn set_id3v24(&mut self, id3v24: bool) {
        self.id3v24 = id3v24;
    }

    // Read and write all comments
    pub fn get_comments(&self) -> Vec<ID3Comment> {
        self.tag.comments().map(|c| c.clone().into()).collect()
    }

    pub fn set_comments(&mut self, comments: &Vec<ID3Comment>) {
        self.tag.remove("COMM");
        for c in comments {
            self.tag.add_comment(c.clone().into())
        }
    }

    // Read write all unsynchronized lyrics
    pub fn get_unsync_lyrics(&self) -> Vec<ID3Comment> {
        self.tag.lyrics().map(|l| l.clone().into()).collect()
    }

    pub fn set_unsync_lyrics(&mut self, lyrics: &Vec<ID3Comment>) {
        self.tag.remove("USLT");
        for l in lyrics {
            self.tag.add_lyrics(l.clone().into());
        }
    }

    // POPM
    pub fn get_popularimeter(&self) -> Option<ID3Popularimeter> {
        let tag = self.tag.get("POPM")?;
        let data = tag.content().unknown()?;
        let popm = ID3Popularimeter::from_bytes(data)?;
        Some(popm)
    }

    pub fn set_popularimeter(&mut self, popm: &ID3Popularimeter) {
        self.tag.remove("POPM");
        self.tag.add_frame(popm.to_frame());
    }

    // Convert between different cover/picture types
    fn picture_type(&self, cover_type: &CoverType) -> PictureType {
        COVER_TYPES.iter().find(|(_, c)| c == cover_type).unwrap().0
    }
    fn cover_type(&self, picture_type: &PictureType) -> CoverType {
        COVER_TYPES.iter().find(|(p, _)| p == picture_type).unwrap_or(
            &(PictureType::Undefined(20u8), CoverType::Undefined)
        ).1.clone()
    }

    // Convert Field to tag name
    fn field(&self, field: Field) -> String {
        match field {
            Field::Title => "TIT2".to_string(),
            Field::Artist => "TPE1".to_string(),
            Field::AlbumArtist => "TPE2".to_string(),
            Field::Album => "TALB".to_string(),
            Field::Key => "TKEY".to_string(),
            Field::BPM => "TBPM".to_string(),
            Field::Genre => "TCON".to_string(),
            Field::Label => "TPUB".to_string(),
            Field::Style => "STYLE".to_string(),
            Field::ISRC => "TSRC".to_string(),
            Field::CatalogNumber => "CATALOGNUMBER".to_string(),
            Field::Version => "TIT3".to_string(),
            Field::TrackNumber => "TRCK".to_string(),
            Field::Duration => "TLEN".to_string(),
            Field::Remixer => "TPE4".to_string()
        }
    }
}

impl TagImpl for ID3Tag {
    // Write tag to file
    fn save_file(&mut self, path: &str) -> Result<(), Box<dyn Error>> {
        let version = match self.id3v24 {
            true => Version::Id3v24,
            false => Version::Id3v23
        };
        // MP3
        if self.format == ID3AudioFormat::MP3 {
            self.tag.write_to_path(path, version)?;
        }
        // AIFF
        if self.format == ID3AudioFormat::AIFF {
            self.tag.write_to_aiff(path, version)?;
        }
        Ok(())
    }

    fn set_separator(&mut self, separator: &str) {
        self.id3_separator = separator.replace("\\0", "\0");
    }

    // Get all tags
    fn all_tags(&self) -> HashMap<String, Vec<String>> {
        let mut tags = HashMap::new();
        for frame in self.tag.frames() {
            if let Content::Text(v) = frame.content() {
                tags.insert(frame.id().to_owned(), v.split(&self.id3_separator).map(String::from).collect());
            }
        }
        // Add TXXX
        for extended in self.tag.extended_texts() {
            tags.insert(extended.description.to_string(), extended.value.split(&self.id3_separator).map(String::from).collect());
        }
        tags
    }

    // Set date to tag
    fn set_date(&mut self, date: &TagDate, overwrite: bool) {
        // ID3 v2.3
        if !self.id3v24 {
            // Year
            if overwrite || self.tag.get("TYER").is_none() {
                // Remove v2.4
                self.tag.remove_date_recorded();
                self.tag.set_text("TYER", date.year.to_string());
            }
            // Date
            if date.has_md() && (overwrite || self.tag.get("TDAT").is_none()) {
                self.tag.remove_date_recorded();
                self.tag.set_text("TDAT", &format!("{:02}{:02}", date.day.unwrap(), date.month.unwrap()));
            }
            return;
        }
        // ID3 v2.4
        if overwrite || self.tag.date_recorded().is_none() {
            let ts = Timestamp {
                year: date.year,
                month: date.month,
                day: date.day,
                hour: None,
                minute: None,
                second: None
            };
            self.tag.set_date_recorded(ts);
        }
    }

    // Set publish date
    fn set_publish_date(&mut self, date: &TagDate, overwrite: bool) {
        if overwrite || self.tag.date_released().is_none() {
            let ts = Timestamp {
                year: date.year,
                month: date.month,
                day: date.day,
                hour: None,
                minute: None,
                second: None
            };
            self.tag.set_date_released(ts);
        }
    }

    // Rating
    fn get_rating(&self) -> Option<u8> {
        let tag = self.tag.get("POPM")?;
        let value = tag.content().unknown()?;
        let popm = ID3Popularimeter::from_bytes(value)?;
        // Byte to 1 - 5
        let rating = (popm.rating as f32 / 51.0).ceil() as u8;
        if rating == 0 {
            return Some(1)
        }
        Some(rating)
    }
    fn set_rating(&mut self, rating: u8, overwrite: bool) {
        let frame = ID3Popularimeter::new("no@email", rating * 51, 0).to_frame();
        if overwrite || self.tag.get("POPM").is_none() {
            self.tag.remove("POPM");
            self.tag.add_frame(frame);
        }
    }

    // Set album art
    fn set_art(&mut self, kind: CoverType, mime: &str, description: Option<&str>, data: Vec<u8>) {
        let picture_type = self.picture_type(&kind);
        self.tag.remove_picture_by_type(picture_type);
        self.tag.add_picture(Picture {
            mime_type: mime.to_string(),
            picture_type,
            description: description.unwrap_or("Cover").to_owned(),
            data
        });
    }
    // Get album art by type
    fn get_art(&self) -> Vec<crate::tag::Picture> {
        self.tag.pictures().map(
            |p| crate::tag::Picture {
                kind: self.cover_type(&p.picture_type),
                description: p.description.to_string(),
                data: p.data.clone(),
                mime: p.mime_type.to_string()
            }
        ).collect()
    }
    // Check if has album art
    fn has_art(&self) -> bool {
        self.tag.pictures().next().is_some()
    }
    fn remove_art(&mut self, kind: CoverType) { 
        self.tag.remove_picture_by_type(self.picture_type(&kind));
    }

    // Set/Get named field
    fn set_field(&mut self, field: Field, mut value: Vec<String>, overwrite: bool) {
        // Modify duration from seconds to milliseconds
        if field == Field::Duration {
            value[0] = format!("{}000", value[0]);
        }

        self.set_raw(&self.field(field), value, overwrite);
    }
    fn get_field(&self, field: Field) -> Option<Vec<String>> {
        // Track number override (tag value: Track number/Total track)
        if field == Field::TrackNumber {
            return Some(vec![self.get_raw(&self.field(field))?.first()?
                .split("/").map(|v| v.to_string()).collect::<Vec<String>>().first()?.to_string()]);
        }

        self.get_raw(&self.field(field))
    }

    // Set/Get by tag
    fn set_raw(&mut self, tag: &str, value: Vec<String>, overwrite: bool) {
        // TXXX
        if tag.len() != 4 {
            if overwrite || self.get_raw(tag).is_none() {
                // Remove if empty
                if value.is_empty() {
                    self.tag.remove_extended_text(Some(tag), None);
                    return;
                }
                self.tag.add_extended_text(tag, value.join(&self.id3_separator));
            }
            return;
        }

        // COMM tag override for compatibility with DJ apps
        if tag.to_uppercase() == "COMM" {
            if overwrite || self.tag.comments().next().is_none() {
                let mut comment = match self.tag.comments().cloned().next() {
                    Some(comment) => comment.to_owned(),
                    None => Comment {
                        lang: "eng".to_string(),
                        description: String::new(),
                        text: String::new()
                    }
                };
                // Add value
                self.tag.remove("COMM");
                if !value.is_empty() {
                    comment.text = value.join(&self.id3_separator);
                    self.tag.add_comment(comment);
                }
            }
            return;
        }

        // USLT override so can be used as normal tag
        if tag.to_uppercase() == "USLT" {
            if overwrite || self.tag.lyrics().next().is_none() {
                self.tag.remove_all_lyrics();
                self.tag.add_lyrics(Lyrics {
                    lang: "eng".to_string(),
                    description: String::new(),
                    text: value.join(&self.id3_separator),
                });
            }
            return;
        }

        
        // Normal
        if overwrite || self.tag.get(tag).is_none() {
            // Remove if empty
            if value.is_empty() {
                self.tag.remove(tag);
                return;
            }
            self.tag.set_text(tag, value.join(&self.id3_separator));
        }
    }
    // Get raw TEXT field
    fn get_raw(&self, tag: &str) -> Option<Vec<String>> {
        // Custom tag (TXXX)
        if tag.len() != 4 {
            if let Some(t) = self.tag.extended_texts().find(|t| t.description == tag) {
                return Some(vec![t.value.to_string()]);
            }
            return None;
        }

        // COMM tag override for compatibility with DJ apps
        if tag.to_uppercase() == "COMM" {
            return match self.tag.comments().next() {
                Some(comment) => Some(comment.text.split(&self.id3_separator).map(String::from).collect()),
                None => None
            };
        }

        // USLT override
        if tag.to_uppercase() == "USLT" {
            return match self.tag.lyrics().next() {
                Some(lyrics) => Some(lyrics.text.split(&self.id3_separator).map(String::from).collect()),
                None => None
            };
        }

        // Get tag
        if let Some(t) = self.tag.get(tag) {
            if let Some(content) = t.content().text() {
                Some(content.split(&self.id3_separator).map(String::from).collect())
            } else {
                None
            }
        } else {
            None
        }
    }

    fn remove_raw(&mut self, tag: &str) {
        // TXXX
        if tag.len() != 4 {
            self.tag.remove_extended_text(Some(tag), None);
            return;
        }

        self.tag.remove(tag);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ID3Popularimeter {
    pub email: String,
    pub rating: u8,
    pub counter: u32
}

impl ID3Popularimeter {
    pub fn new(email: &str, rating: u8, counter: u32) -> ID3Popularimeter {
        ID3Popularimeter {
            email: email.to_string(),
            rating, counter
        }
    }

    //  EMAIL \0 RATING (u8) COUNTER (u32)
    pub fn from_bytes(data: &[u8]) -> Option<ID3Popularimeter> {
        let pos = data.iter().position(|b| b == &0u8)?;
        if pos + 6 > data.len() {
            warn!("POMP Tag has invalid length! Len: {}, null: {}", data.len(), pos);
            return None;
        }

        Some(ID3Popularimeter {
            email: String::from_utf8(data[0..pos].to_vec()).ok()?,
            rating: data[pos+1],
            counter: u32::from_be_bytes(data[pos+2..pos+6].try_into().unwrap_or([0,0,0,0]))
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out: Vec<u8> = vec![];
        out.extend(self.email.as_bytes());
        out.push(0);
        out.push(self.rating);
        out.extend(self.counter.to_be_bytes().iter());
        out
    }

    pub fn to_frame(&self) -> Frame {
        Frame::with_content("POPM", Content::Unknown(self.to_bytes()))
    }
}

// Reimplementations of rust-id3 for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ID3Comment {
    pub lang: String,
    pub description: String,
    pub text: String
}

impl From<Comment> for ID3Comment {
    fn from(c: Comment) -> Self {
        Self {
            lang: c.lang,
            description: c.description,
            text: c.text.replace("\0", "")
        }
    }
}

impl From<ID3Comment> for Comment {
    fn from(c: ID3Comment) -> Self {
        Self {
            lang: c.lang,
            description: c.description,
            text: c.text
        }
    }
}

// Unsynchronized lyrics have exactly same schema as comment
impl From<Lyrics> for ID3Comment {
    fn from(l: Lyrics) -> Self {
        Self {
            lang: l.lang,
            description: l.description,
            text: l.text.replace("\0", "")
        }
    }
}

impl From<ID3Comment> for Lyrics {
    fn from(c: ID3Comment) -> Self {
        Self {
            lang: c.lang,
            description: c.description,
            text: c.text
        }
    }
}