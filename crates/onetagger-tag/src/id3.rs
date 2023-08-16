use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use id3::{Version, Tag, Timestamp, Content, TagLike, Encoder, Frame, Encoding};
use id3::frame::{Picture, PictureType, Comment, Lyrics, Popularimeter, ExtendedText, SynchronisedLyrics, TimestampFormat, SynchronisedLyricsType};
use serde::{Serialize, Deserialize};
use crate::{TagDate, CoverType, Field, TagImpl};

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
    MP3, AIFF, WAV
}

pub struct ID3Tag {
    tag: Tag,
    pub format: ID3AudioFormat,
    pub id3_separator: String,
    pub id3v24: bool,
    pub comm_lang: String,
}

impl ID3Tag {
    // Read from file
    pub fn load_file(path: impl AsRef<Path>) -> Result<ID3Tag, Box<dyn Error>> {
        let ext = path.as_ref().extension().unwrap_or_default().to_ascii_lowercase();
        // MP3
        if ext == "mp3" {
            let tag = Tag::read_from_path(path)?;
            let version = tag.version();
            return Ok(ID3Tag {
                tag,
                format: ID3AudioFormat::MP3,
                id3_separator: String::from(", "),
                id3v24: match version {
                    Version::Id3v24 => true,
                    _ => false
                },
                comm_lang: "xxx".to_string()
            }.into());
        }
        // AIFF
        if ext == "aif" || ext == "aiff" {
            let tag = Tag::read_from_aiff_path(path)?;
            let version = tag.version();
            return Ok(ID3Tag {
                tag,
                format: ID3AudioFormat::AIFF,
                id3_separator: String::from(", "),
                id3v24: match version {
                    Version::Id3v24 => true,
                    _ => false
                },
                comm_lang: "xxx".to_string()
            }.into());
        }
        // WAV
        if ext == "wav" {
            let tag = crate::wav::read_wav(path)?;
            let version = tag.version();
            return Ok(ID3Tag { 
                tag,
                format: ID3AudioFormat::WAV,
                id3_separator: String::from(", "),
                id3v24: match version {
                    Version::Id3v24 => true,
                    _ => false
                },
                comm_lang: "xxx".to_string()
            }.into());
        }


        // Unsupported
        Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unsupported format!").into())
    }

    // Load tag from file or create new
    pub fn load_or_new(path: impl AsRef<Path>) -> ID3Tag {
        let ext = path.as_ref().extension().unwrap_or_default().to_ascii_lowercase();
        let format = if ext == "mp3" {
            ID3AudioFormat::MP3
        } else if ext == "wav" {
            ID3AudioFormat::WAV
        } else {
            ID3AudioFormat::AIFF
        };

        match ID3Tag::load_file(&path) {
            Ok(tag) => tag,
            Err(e) => {
                warn!("Failed loading: {:?}, creating new tag. {:?}", path.as_ref(), e);
                ID3Tag {
                    tag: Tag::new(),
                    format,
                    id3_separator: String::from(", "),
                    id3v24: true,
                    comm_lang: "xxx".to_string()
                }
            }
        }
    }

    // ID3 Settings
    pub fn set_id3v24(&mut self, id3v24: bool) {
        self.id3v24 = id3v24;
    }

    /// Set COMM frame language
    pub fn set_comm_lang(&mut self, lang: String) {
        self.comm_lang = lang;
    }

    // Read and write all comments
    pub fn get_comments(&self) -> Vec<ID3Comment> {
        self.tag.comments().map(|c| c.clone().into()).collect()
    }

    pub fn set_comments(&mut self, comments: &Vec<ID3Comment>) {
        self.tag.remove("COMM");
        for c in comments {
            let comment: Comment = c.clone().into();
            self.tag.add_frame(comment);
        }
    }

    // Read write all unsynchronized lyrics
    pub fn get_unsync_lyrics(&self) -> Vec<ID3Comment> {
        self.tag.lyrics().map(|l| l.clone().into()).collect()
    }

    pub fn set_unsync_lyrics(&mut self, lyrics: &Vec<ID3Comment>) {
        self.tag.remove("USLT");
        for l in lyrics {
            let lyric: Lyrics = l.clone().into();
            self.tag.add_frame(lyric);
        }
    }

    // POPM
    pub fn get_popularimeter(&self) -> Option<ID3Popularimeter> {
        let tag = self.tag.get("POPM")?;
        if let Content::Popularimeter(popm) = tag.content() {
            return Some(popm.clone().into());
        }
        None
    }

    pub fn set_popularimeter(&mut self, popm: &ID3Popularimeter) {
        self.tag.remove("POPM");
        let popularimeter: Popularimeter = popm.clone().into();
        self.tag.add_frame(popularimeter);
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

}

impl TagImpl for ID3Tag {
    // Write tag to file
    fn save_file(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let version = match self.id3v24 {
            true => Version::Id3v24,
            false => Version::Id3v23
        };

        // Fix art for serato
        if !self.id3v24 {
            let pictures = self.tag.pictures().map(|p| p.clone()).collect::<Vec<_>>();
            self.tag.remove_all_pictures();
            for picture in pictures {
                self.tag.add_frame(Frame::with_content("APIC", Content::Picture(picture)).set_encoding(Some(Encoding::Latin1)));
            }
        }

        // Write
        match self.format {
            ID3AudioFormat::MP3 => {
                Encoder::new()
                    .version(version)
                    .padding(2048)
                    .encode_to_path(&self.tag, path)?;
            },
            ID3AudioFormat::AIFF => self.tag.write_to_aiff_path(path, version)?,
            ID3AudioFormat::WAV => crate::wav::write_wav(path, self.tag.clone(), version)?,
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
        if let Content::Popularimeter(popm) = tag.content() {
            let rating = (popm.rating as f32 / 51.0).ceil() as u8;
            if rating == 0 {
                return Some(1)
            }
            return Some(rating)
        }
        None
    }
    
    fn set_rating(&mut self, rating: u8, overwrite: bool) {
        let frame: Popularimeter = ID3Popularimeter::new("no@email", rating * 51, 0).into();
        if overwrite || self.tag.get("POPM").is_none() {
            self.tag.remove("POPM");
            if rating > 0 {
                self.tag.add_frame(frame);
            }
        }
        // Rating WMP
        if overwrite || self.tag.get("RATING WMP").is_none() {
            self.tag.remove("RATING WMP");
            if rating > 0 {
                let value = match rating {
                    1 => 1,
                    5 => 255,
                    i => (i - 1) * 64
                };
                self.set_raw("RATING WMP", vec![value.to_string()], overwrite);
            }
        }
    }

    // Set album art
    fn set_art(&mut self, kind: CoverType, mime: &str, description: Option<&str>, data: Vec<u8>) {
        let picture_type = self.picture_type(&kind);
        self.tag.remove_picture_by_type(picture_type);
        self.tag.add_frame(Picture {
            mime_type: mime.to_string(),
            picture_type,
            description: description.unwrap_or("Cover").to_owned(),
            data
        });
    }
    // Get album art by type
    fn get_art(&self) -> Vec<crate::Picture> {
        self.tag.pictures().map(
            |p| crate::Picture {
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
        // Remove all undefined kinds
        if kind == CoverType::Undefined {
            self.tag.pictures()
                .filter(|p| matches!(p.picture_type, PictureType::Undefined(_)))
                .map(|p| p.picture_type)
                .collect::<Vec<_>>()
                .into_iter()
                .for_each(|t| self.tag.remove_picture_by_type(t));
            return;
        }
        self.tag.remove_picture_by_type(self.picture_type(&kind));
    }

    // Set/Get named field
    fn set_field(&mut self, field: Field, mut value: Vec<String>, overwrite: bool) {
        // Modify duration from seconds to milliseconds
        if field == Field::Duration {
            value[0] = format!("{}000", value[0]);
        }
        self.set_raw(field.id3(), value, overwrite);
    }
    fn get_field(&self, field: Field) -> Option<Vec<String>> {
        // Overrides
        if field == Field::TrackNumber {
            return self.tag.track().map(|v| vec![v.to_string()]);
        }
        if field == Field::TrackTotal {
            return self.tag.total_tracks().map(|v| vec![v.to_string()]);
        }
        if field == Field::DiscNumber {
            return self.tag.disc().map(|v| vec![v.to_string()]);
        }

        self.get_raw(field.id3())
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
                self.tag.add_frame(ExtendedText {
                    description: tag.to_string(),
                    value: value.join(&self.id3_separator),
                });
            }
            return;
        }

        // COMM tag override for compatibility with DJ apps
        if tag.to_uppercase() == "COMM" {
            if overwrite || self.tag.comments().next().is_none() {
                let mut comment = match self.tag.comments().cloned().next() {
                    Some(comment) => comment.to_owned(),
                    None => Comment {
                        lang: self.comm_lang.to_string(),
                        description: String::new(),
                        text: String::new()
                    }
                };
                comment.lang = self.comm_lang.to_owned();
                // Add value
                self.tag.remove("COMM");
                if !value.is_empty() {
                    comment.text = value.join(&self.id3_separator);
                    self.tag.add_frame(comment);
                }
            }
            return;
        }

        // USLT override so can be used as normal tag
        if tag.to_uppercase() == "USLT" {
            if overwrite || self.tag.lyrics().next().is_none() {
                self.tag.remove_all_lyrics();
                self.tag.add_frame(Lyrics {
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

    fn get_date(&self) -> Option<TagDate> {
        if let Some(date) = self.tag.date_recorded() {
            return Some(TagDate {
                year: date.year,
                month: date.month,
                day: date.day
            });
        }
        // ID3v2.3
        if let Some(tag) = self.get_raw("TYER") {
            return Some(TagDate {
                year: tag[0].parse().ok()?,
                month: None,
                day: None
            });
        }
        None
    }

    fn set_track_number(&mut self, track_number: &str, track_total: Option<u16>, overwrite: bool) {
        let mut value = format!("{track_number}");
        if let Some(total) = track_total {
            value = format!("{value}/{total}");
        }
        self.set_raw("TRCK", vec![value], overwrite);
    }

    fn set_lyrics(&mut self, lyrics: &crate::Lyrics, synced: bool, overwrite: bool) {
        // Add synced
        if synced {
            if !lyrics.synced() || (!overwrite && self.tag.synchronised_lyrics().next().is_some()) {
                return;
            }
            self.tag.remove_all_synchronised_lyrics();
            self.tag.add_frame(SynchronisedLyrics {
                lang: lyrics.language.to_string(),
                timestamp_format: TimestampFormat::Ms,
                content_type: SynchronisedLyricsType::Lyrics,
                description: "Lyrics".to_string(),
                content: lyrics.iter_lines().filter_map(|l| match l.start {
                    Some(start) => Some((start.as_millis() as u32, l.text.to_string())),
                    None => None
                }).collect(),
            });
        }
        // Add unsynced
        if !overwrite && self.tag.lyrics().next().is_some() {
            return;
        }
        self.tag.remove_all_lyrics();
        self.tag.add_frame(Lyrics {
            lang: lyrics.language.to_string(),
            description: "Lyrics".to_string(),
            text: lyrics.text()
        });
    }

    fn set_explicit(&mut self, explicit: bool) {
        self.set_raw("ITUNESADVISORY", vec![
            match explicit { true => "1", false => "2" }.to_string()
        ], true)
    }

    fn get_separator(&self) -> Option<String> {
        Some(self.id3_separator.clone())
    }
    
    
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ID3Popularimeter {
    pub email: String,
    pub rating: u8,
    pub counter: u64
}

impl ID3Popularimeter {
    pub fn new(email: &str, rating: u8, counter: u64) -> ID3Popularimeter {
        ID3Popularimeter {
            email: email.to_string(),
            rating, counter
        }
    }
}

impl Into<Popularimeter> for ID3Popularimeter {
    fn into(self) -> Popularimeter {
        Popularimeter {
            user: self.email,
            rating: self.rating,
            counter: self.counter,
        }
    }
}

impl From<Popularimeter> for ID3Popularimeter {
    fn from(p: Popularimeter) -> Self {
        ID3Popularimeter {
            email: p.user,
            rating: p.rating,
            counter: p.counter
        }
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