use std::error::Error;
use std::fs::read_dir;
use std::io::Cursor;
use std::collections::HashMap;
use std::path::Path;
use serde::{Serialize, Deserialize};
use image::{GenericImageView, io::Reader as ImageReader};

use crate::tag::{AudioFileFormat, CoverType, Picture, Tag};

pub struct TagEditor {}

impl TagEditor {
    pub fn list_dir(path: &str) -> Result<Vec<FolderEntry>, Box<dyn Error>> {
        let extensions = ["flac", "mp3", "aif", "aiff"];
        let mut out = vec![];
        for e in read_dir(path)? {
            let e = e?;
            let path = e.path();
            let dir = path.is_dir();

            //Filter extensions
            if !dir {
                if path.extension().is_none() {
                    continue;
                }
                let extension = path.extension().unwrap().to_str().ok_or("Invalid extension")?.to_lowercase();
                if !extensions.iter().any(|e| e == &&extension) {
                    continue;
                }
            }
            let filename = path.file_name().ok_or("Invalid filename!")?.to_str().ok_or("Invalid filename!")?.to_owned();
            if filename.starts_with('.') {
                continue;
            }
            out.push(FolderEntry {
                dir,
                filename 
            });
        }
        Ok(out)
    }

    //Load tags from file
    pub fn load_file(path: &str) -> Result<TagEditorFile, Box<dyn Error>> {
        let filename = Path::new(path).file_name().ok_or("Invalid filename")?.to_str().ok_or("Invalid filename!")?;
        let tag_wrap = Tag::load_file(path)?;
        //Load tags
        let tag = tag_wrap.tag().ok_or("No tag")?;
        let tags = tag.all_tags().iter().map(|(k, v)| {
            (k.to_owned(), v.join(",").split("\0").collect::<Vec<&str>>().join(","))
        }).collect();

        //Load images
        let mut images = vec![];
        for picture in tag.get_art() {
            if let Ok(art) = TagEditor::load_art(picture) {
                images.push(art);
            }
        }

        Ok(TagEditorFile {
            tags,
            filename: filename.to_owned(),
            format: tag_wrap.format,
            path: path.to_owned(),
            images
        })
    }

    //Load art and encode
    fn load_art(picture: Picture) -> Result<TagEditorImage, Box<dyn Error>> {
        let img = ImageReader::new(Cursor::new(&picture.data)).with_guessed_format()?.decode()?;
        Ok(TagEditorImage {
            mime: picture.mime.to_string(),
            data: format!("data:{};base64,{}", &picture.mime, base64::encode(picture.data)),
            kind: picture.kind.to_owned(),
            description: picture.description.to_owned(),
            width: img.dimensions().0,
            height: img.dimensions().1,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderEntry {
    pub filename: String,
    pub dir: bool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagEditorFile {
    pub tags: HashMap<String, String>,
    pub filename: String,
    pub format: AudioFileFormat,
    pub path: String,
    pub images: Vec<TagEditorImage>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagEditorImage {
    pub mime: String,
    pub data: String,
    pub kind: CoverType,
    pub description: String,
    pub width: u32,
    pub height: u32
}