use std::error::Error;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

pub mod id3;
pub mod flac;

pub struct Tag {
    pub flac: Option<flac::FLACTag>,
    pub id3: Option<id3::ID3Tag>,
    pub format: AudioFileFormat
}

impl Tag {
    pub fn load_file(path: &str) -> Result<Tag, Box<dyn Error>> {
        //FLAC
        if path.to_lowercase().ends_with(".flac") {
            return Ok(Tag {
                flac: Some(flac::FLACTag::load_file(path)?),
                id3: None,
                format: AudioFileFormat::FLAC
            });
        }
        //ID3
        let tag = id3::ID3Tag::load_file(path)?;
        let format = match tag.format {
            id3::ID3AudioFormat::MP3 => AudioFileFormat::MP3,
            id3::ID3AudioFormat::AIFF => AudioFileFormat::AIFF
        };
        return Ok(Tag {
            id3: Some(tag),
            flac: None,
            format: format
        });
    }

    //Get generic
    pub fn tag(&self) -> Option<Box<&dyn TagImpl>> {
        if let Some(flac) = &self.flac {
            return Some(Box::new(flac));
        }
        if let Some(id3) = &self.id3 {
            return Some(Box::new(id3));
        }
        None
    }
    pub fn tag_mut(&mut self) -> Option<Box<&mut dyn TagImpl>> {
        if let Some(flac) = &mut self.flac {
            return Some(Box::new(flac));
        }
        if let Some(id3) = &mut self.id3 {
            return Some(Box::new(id3));
        }
        None 
    }
}

pub trait TagImpl {
    fn save_file(&mut self, path: &str) -> Result<(), Box<dyn Error>>;

    //Get all string tags
    fn all_tags(&self) -> HashMap<String, Vec<String>>;

    //Set/Get dates
    fn set_date(&mut self, date: &TagDate, overwrite: bool);
    fn get_date(&self) -> Option<String>;
    fn set_publish_date(&mut self, date: &TagDate, overwrite: bool);

    //Get/Set rating as 1 - 5 stars value
    fn get_rating(&self) -> Option<u8>;
    fn set_rating(&mut self, rating: u8, overwrite: bool);

    //Set/Get album art
    fn set_art(&mut self, kind: CoverType, mime: &str, description: Option<&str>, data: Vec<u8>);
    //To not load all album arts
    fn has_art(&self) -> bool;
    fn get_art(&self) -> Vec<Picture>;

    //Set/Get named field
    fn set_field(&mut self, field: Field, value: Vec<String>, overwrite: bool);
    fn get_field(&self, field: Field) -> Option<Vec<String>>;

    //Set/Get by tag field name
    fn set_raw(&mut self, tag: &str, value: Vec<String>, overwrite: bool);
    fn get_raw(&self, tag: &str) -> Option<Vec<String>>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFileFormat {
    FLAC, AIFF, MP3
}

#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    kind: CoverType,
    data: Vec<u8>,
    description: String,
    mime: String
}

#[derive(Debug, Clone, PartialEq)]
pub enum CoverType {
    Front
}

#[derive(Debug, Clone)]
pub struct TagDate {
    pub year: i32,
    pub month: Option<u8>,
    pub day: Option<u8>
}

impl TagDate {
    //If has day and month
    pub fn has_md(&self) -> bool {
        return self.month.is_some() && self.day.is_some();
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    Title,
    Artist,
    Album,
    Key,
    BPM,
    Genre,
    Style,
    Label
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum TagChange {
    Raw { tag: String, value: Vec<String> },
    Rating { value: u8 },
    Genre { value: Vec<String> }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagChanges {
    changes: Vec<TagChange>,
    pub path: String
}

impl TagChanges {
    //Save all changes to file
    pub fn commit(&self) -> Result<Tag, Box<dyn Error>> {
        let mut tag_wrap = Tag::load_file(&self.path)?;
        let tag = tag_wrap.tag_mut().ok_or("No tag!")?;
        //Match changes
        for change in self.changes.clone() {
            match change {
                TagChange::Raw {tag: t, value} => tag.set_raw(&t, value, true),
                TagChange::Rating {value} => tag.set_rating(value, true),
                TagChange::Genre {value} => tag.set_field(Field::Genre, value, true),
            }
        }
        //Save
        tag.save_file(&self.path)?;

        Ok(tag_wrap)
    }
}