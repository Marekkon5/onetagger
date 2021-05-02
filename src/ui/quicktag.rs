use std::error::Error;
use std::collections::HashMap;
use std::fs::read_dir;
use std::io::Cursor;
use image::ImageOutputFormat;
use image::io::Reader as ImageReader;
use serde::{Deserialize, Serialize};
use crate::tag::{AudioFileFormat, Field, Tag};

pub struct QuickTag {}

impl QuickTag {

    //Load all supported files from folder
    pub fn load_files(path: &str) -> Result<Vec<QuickTagFile>, Box<dyn Error>> {
        let mut out = vec![];
        for entry in read_dir(path)? {
            //Check if valid
            if entry.is_err() {
                continue;
            }
            let entry = entry.unwrap();
            //Skip dirs
            if entry.path().is_dir() {
                continue;
            }
            //Load tags
            let path = entry.path();
            let path = path.to_str().unwrap();
            match QuickTagFile::from_path(&path) {
                Ok(t) => out.push(t),
                Err(e) => error!("Error loading file: {} {}", path, e)
            }
        }

        Ok(out)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QuickTagFile {
    path: String,
    format: AudioFileFormat,
    title: String,
    artists: Vec<String>,
    genres: Vec<String>,
    bpm: Option<i64>,
    rating: u8,
    tags: HashMap<String, Vec<String>>,
}

impl QuickTagFile {
    //Load tags from path
    pub fn from_path(path: &str) -> Result<QuickTagFile, Box<dyn Error>> {
        let tag_wrap = Tag::load_file(path)?;
        Ok(QuickTagFile::from_tag(path, &tag_wrap).ok_or("Unable to load tags!")?)
    }

    pub fn from_tag(path: &str, tag_wrap: &Tag) -> Option<QuickTagFile> {
        let tag = tag_wrap.tag()?;
        let mut all_tags = tag.all_tags();
        //Insert overriden tags
        if let Some(v) = tag.get_raw("COMM") {
            all_tags.insert("COMM".to_string(), v);
        }
        if let Some(v) = tag.get_raw("USLT") {
            all_tags.insert("USLT".to_string(), v);
        }

        Some(QuickTagFile {
            path: path.to_string(),
            format: tag_wrap.format.clone(),
            title: tag.get_field(Field::Title)?.first()?.to_string(),
            artists: tag.get_field(Field::Artist)?,
            genres: tag.get_field(Field::Genre).unwrap_or(vec![]),
            rating: tag.get_rating().unwrap_or(0),
            bpm: match tag.get_field(Field::BPM) {
                Some(t) => t.first().unwrap_or(&"can't parse".to_string()).parse().ok(),
                None => None
            },
            tags: all_tags,
        })
    }

    //Load album art from tag and downscale
    pub fn get_art(path: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        //Load
        let tag_wrap = Tag::load_file(path)?;
        let tag = tag_wrap.tag().ok_or("Missing tag!")?;
        let pictures = tag.get_art();
        let picture = pictures.first().ok_or("Missing album art!")?;
        let img = ImageReader::new(Cursor::new(&picture.data)).with_guessed_format()?.decode()?;
        //Downscale and save
        let scaled = img.thumbnail_exact(50, 50);
        let mut out = vec![];
        scaled.write_to(&mut out, ImageOutputFormat::Jpeg(95))?;
        Ok(out)
    }
}
