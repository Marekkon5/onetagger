use std::collections::HashMap;
use std::error::Error;
use id3::{Version, Tag, Timestamp, Frame, Content};
use id3::frame::{Picture, PictureType, Comment};
use crate::tag::{TagDate, CoverType, Field, TagImpl};

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
    //Read from file
    pub fn load_file(path: &str) -> Result<ID3Tag, Box<dyn Error>> {        
        //MP3
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
        //AIFF
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

        //Unsupported
        Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unsupported format!").into())
    }

    //ID3 Settings
    pub fn set_id3_separator(&mut self, separator: &str) {
        self.id3_separator = separator.to_owned();
    }
    pub fn set_id3v24(&mut self, id3v24: bool) {
        self.id3v24 = id3v24;
    }

    //Convert CoverType to PictureType
    fn cover_type(&self, cover_type: CoverType) -> PictureType {
        match cover_type {
            CoverType::Front => PictureType::CoverFront
        }
    }
    //Convert Field to tag name
    fn field(&self, field: Field) -> String {
        match field {
            Field::Title => "TIT2".to_string(),
            Field::Artist => "TPE1".to_string(),
            Field::Album => "TALB".to_string(),
            Field::Key => "TKEY".to_string(),
            Field::BPM => "TBPM".to_string(),
            Field::Genre => "TCON".to_string(),
            Field::Label => "TPUB".to_string(),
            Field::Style => "STYLE".to_string(),
            Field::ISRC => "TSRC".to_string(),
        }
    }
}

impl TagImpl for ID3Tag {
    //Write tag to file
    fn save_file(&mut self, path: &str) -> Result<(), Box<dyn Error>> {
        let version = match self.id3v24 {
            true => Version::Id3v24,
            false => Version::Id3v23
        };
        //MP3
        if self.format == ID3AudioFormat::MP3 {
            self.tag.write_to_path(path, version)?;
        }
        //AIFF
        if self.format == ID3AudioFormat::AIFF {
            self.tag.write_to_aiff(path, version)?;
        }
        Ok(())
    }

    //Get all tags
    fn all_tags(&self) -> HashMap<String, Vec<String>> {
        let mut tags = HashMap::new();
        for frame in self.tag.frames() {
            //Comment override for compatibility
            if frame.id() == "COMM" {
                tags.insert("COMM".to_string(), self.get_raw("COMM").unwrap_or(vec![]));
            }

            if let Content::Text(v) = frame.content() {
                tags.insert(frame.id().to_owned(), v.split(&self.id3_separator).map(String::from).collect());
            }
        }
        //Add TXXX
        for extended in self.tag.extended_texts() {
            tags.insert(extended.description.to_string(), extended.value.split(&self.id3_separator).map(String::from).collect());
        }
        tags
    }

    //Set date to tag
    fn set_date(&mut self, date: &TagDate, overwrite: bool) {
        //ID3 v2.3
        if !self.id3v24 {
            //Year
            if overwrite || self.tag.get("TYER").is_none() {
                //Remove v2.4
                self.tag.remove_date_recorded();
                self.tag.set_text("TYER", date.year.to_string());
            }
            //Date
            if date.has_md() && (overwrite || self.tag.get("TDAT").is_none()) {
                self.tag.remove_date_recorded();
                self.tag.set_text("TDAT", &format!("{:02}{:02}", date.day.unwrap(), date.month.unwrap()));
            }
            return;
        }
        //ID3 v2.4
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

    //Set publish date
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

    //Rating
    fn get_rating(&self) -> Option<u8> {
        let tag = self.tag.get("POPM")?;
        let value = tag.content().unknown()?;
        //Find rating index
        let i = value.iter().position(|b| b == &0u8)? + 1;
        //255/5
        let rating = (value[i] as f32 / 51.0).ceil() as u8;
        if rating == 0 {
            return Some(1)
        }
        Some(rating)
    }
    fn set_rating(&mut self, rating: u8, overwrite: bool) {
        //no@email \0 rating (single byte), counter = 4 bytes
        let value = vec![0x6e, 0x6f, 0x40, 0x65, 0x6d, 0x61, 0x69, 0x6c, 0x00, rating * 51, 0, 0, 0, 0];
        if overwrite || self.tag.get("POPM").is_none() {
            self.tag.remove("POPM");
            self.tag.add_frame(Frame::with_content("POPM", Content::Unknown(value)));
        }
    }

    //Get release date
    fn get_date(&self) -> Option<String> {
        let date = self.tag.date_recorded()?;
        //Generate string
        let mut out = date.year.to_string();
        if let Some(m) = date.month {
            out += &format!("-{:02}", m);
        }
        if let Some(d) = date.day {
            out += &format!("-{:02}", d);
        }
        Some(out)
    }

    //Set album art
    fn set_art(&mut self, kind: CoverType, mime: &str, description: Option<&str>, data: Vec<u8>) {
        let picture_type = self.cover_type(kind);
        self.tag.remove_picture_by_type(picture_type);
        self.tag.add_picture(Picture {
            mime_type: mime.to_string(),
            picture_type,
            description: description.unwrap_or("Cover").to_owned(),
            data
        });
    }
    //Get album art by type
    fn get_art(&self) -> Vec<crate::tag::Picture> {
        self.tag.pictures().map(
            |p| crate::tag::Picture {
                kind: match p.picture_type {
                    _ => CoverType::Front
                },
                description: p.description.to_string(),
                data: p.data.clone(),
                mime: p.mime_type.to_string()
            }
        ).collect()
    }
    //Check if has album art
    fn has_art(&self) -> bool {
        self.tag.pictures().next().is_some()
    }

    //Set/Get named field
    fn set_field(&mut self, field: Field, value: Vec<String>, overwrite: bool) {
        self.set_raw(&self.field(field), value, overwrite);
    }
    fn get_field(&self, field: Field) -> Option<Vec<String>> {
        self.get_raw(&self.field(field))
    }

    //Set/Get by tag
    fn set_raw(&mut self, tag: &str, value: Vec<String>, overwrite: bool) {
        //TXXX
        if tag.len() != 4 {
            if overwrite || self.get_raw(tag).is_none() {
                //Remove if empty
                if value.is_empty() {
                    self.tag.remove_extended_text(Some(tag), None);
                    return;
                }
                self.tag.add_extended_text(tag, value.join(&self.id3_separator));
            }
            return;
        }

        //COMM tag override for compatibility with DJ apps
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
                //Add value
                self.tag.remove("COMM");
                if !value.is_empty() {
                    comment.text = value.join(&self.id3_separator);
                    self.tag.add_comment(comment);
                }
            }
            return;
        }

        
        //Normal
        if overwrite || self.tag.get(tag).is_none() {
            //Remove if empty
            if value.is_empty() {
                self.tag.remove(tag);
                return;
            }
            self.tag.set_text(tag, value.join(&self.id3_separator));
        }
    }
    //Get raw TEXT field
    fn get_raw(&self, tag: &str) -> Option<Vec<String>> {
        //Custom tag (TXXX)
        if tag.len() != 4 {
            if let Some(t) = self.tag.extended_texts().find(|t| t.description == tag) {
                return Some(vec![t.value.to_string()]);
            }
            return None;
        }

        //COMM tag override for compatibility with DJ apps
        if tag == "COMM" {
            return match self.tag.comments().next() {
                Some(comment) => Some(comment.text.split(&self.id3_separator).map(String::from).collect()),
                None => None
            };
        }


        //Get tag
        if let Some(t) = self.tag.get(tag) {
            if let Some(content) = t.content().text() {
                Some(vec![content.to_owned()])
            } else {
                None
            }
        } else {
            None
        }
    }
}