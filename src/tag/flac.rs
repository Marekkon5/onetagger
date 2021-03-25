use std::error::Error;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;
use metaflac::Tag;
use metaflac::block::PictureType;
use crate::tag::{Field, TagDate, CoverType, TagImpl};

pub struct FLACTag {
    tag: Tag
}

impl FLACTag {
    //Load from file
    pub fn load_file(path: &str) -> Result<FLACTag, Box<dyn Error>> {
        //Load header
        let mut file = File::open(path)?;
        let mut header: [u8; 4] = [0; 4];
        file.read_exact(&mut header)?;
        //Check if not ID3
        if &header[0..3] == b"ID3" {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "FLAC ID3 not supported!").into());
        }
        //Check if FLAC
        if &header != b"fLaC" {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Not a valid FLAC!").into());
        }
        file.seek(SeekFrom::Start(0))?;

        Ok(FLACTag {
            tag: Tag::read_from(&mut file)?
        }.into())
    }

    //Set date in vorbis to tag
    fn set_date_inner(&mut self, tag: &str, date: &TagDate, overwrite: bool) {
        if overwrite || self.tag.get_vorbis(tag).is_none() {
            let v = match date.has_md() {
                true => format!("{}-{:02}-{:02}", date.year, date.month.unwrap(), date.day.unwrap()),
                false => date.year.to_string()
            };
            self.tag.set_vorbis(tag, vec![v]);
        }
    }

    //Convert CoverType to PictureType
    fn cover_type(&self, cover_type: &CoverType) -> PictureType {
        match cover_type {
            CoverType::Front => PictureType::CoverFront
        }
    }

    //Get field tag name
    fn field(&self, field: Field) -> String {
        match field {
            Field::Title => "TITLE".to_string(),
            Field::Artist => "ARTIST".to_string(),
            Field::Album => "ALBUM".to_string(),
            Field::Key => "INITIALKEY".to_string(),
            Field::BPM => "BPM".to_string(),
            Field::Genre => "GENRE".to_string(),
            Field::Label => "LABEL".to_string(),
            Field::Style => "STYLE".to_string()
        }
    }
}

impl TagImpl for FLACTag {
    //Save to path
    fn save_file(&mut self, path: &str) -> Result<(), Box<dyn Error>> {
        self.tag.write_to_path(path)?;
        Ok(())
    }

    //Get all tags
    fn all_tags(&self) -> HashMap<String, Vec<String>> {
        if let Some(vorbis) = self.tag.vorbis_comments() {
            return vorbis.comments.clone();
        }
        HashMap::new()
    }

    //Set date in tag
    fn set_date(&mut self, date: &TagDate, overwrite: bool) {
        self.set_date_inner("DATE", date, overwrite);
    }
    fn set_publish_date(&mut self, date: &TagDate, overwrite: bool) {
        self.set_date_inner("ORIGINALDATE", date, overwrite);
    }
    fn get_date(&self) -> Option<String> {
        self.get_raw("DATE")?.first().map(String::from)
    }

    //Rating, in vorbis saved as 20,40,60,80,100
    fn get_rating(&self) -> Option<u8> {
        let rating = self.get_raw("RATING")?.first()?.parse::<i32>().ok()? / 20;
        if rating <= 5 {
            if rating == 0 {
               return Some(1)
            }
            return Some(rating as u8)
        }
        None
    }
    fn set_rating(&mut self, rating: u8, overwrite: bool) {
        let value = (rating * 20).to_string();
        self.set_raw("RATING", vec![value], overwrite);
    }

    //Set/Get album art
    fn set_art(&mut self, kind: CoverType, mime: &str, _description: Option<&str>, data: Vec<u8>) {
        self.tag.remove_picture_type(self.cover_type(&kind));
        self.tag.add_picture(mime, self.cover_type(&kind), data);
    }
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

    //Set raw tag
    fn set_raw(&mut self, tag: &str, value: Vec<String>, overwrite: bool) {
        if overwrite || self.tag.get_vorbis(&tag).is_none() || self.tag.get_vorbis(&tag).unwrap().next().is_none() {
            self.tag.set_vorbis(tag, value);
        }
    }
    //Get raw tag, None even if empty array
    fn get_raw(&self, tag: &str) -> Option<Vec<String>> {
        if let Some(values) = self.tag.get_vorbis(tag) {
            let v: Vec<&str> = values.collect();
            if v.is_empty() {
                return None;
            }
            return Some(v.into_iter().map(|v| v.to_string()).collect());
        }
        None
    }
}