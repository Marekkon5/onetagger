use std::error::Error;
use std::fs::read_dir;
use std::io::Cursor;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use serde::{Serialize, Deserialize};
use image::{GenericImageView, io::Reader as ImageReader};

use crate::tag::{AudioFileFormat, CoverType, Picture, Tag, EXTENSIONS};
use crate::tag::id3::{ID3Comment, ID3Popularimeter};

pub struct TagEditor {}

impl TagEditor {
    pub fn list_dir(path: &str) -> Result<Vec<FolderEntry>, Box<dyn Error>> {
        let mut out = vec![];
        for e in read_dir(path)? {
            if let Ok(e) = e {
                if let Some(fe) = TagEditor::validate_path(e.path()) {
                    out.push(fe);
                }
            }
        }
        Ok(out)
    }

    //Get only supported files from all subdirectories
    pub fn list_dir_recursive(path: &str) -> Result<Vec<FolderEntry>, Box<dyn Error>> {
        let mut out = vec![];
        for e in WalkDir::new(path) {
            if let Ok(e) = e {
                if let Some(fe) = TagEditor::validate_path(e.path().to_owned()) {
                    if !fe.dir {
                        out.push(fe);
                    }
                }
            }
        }
        Ok(out)
    }

    //Check if path is supported
    fn validate_path(path: PathBuf) -> Option<FolderEntry> {
        let dir = path.is_dir();
        //Filter extensions
        if !dir {
            if path.extension().is_none() {
                return None;
            }
            let extension = path.extension().unwrap().to_str()?.to_lowercase();
            if !EXTENSIONS.iter().any(|e| e[1..] == extension) {
                return None;
            }
        }
        let filename = path.file_name()?.to_str()?.to_owned();
        if filename.starts_with('.') {
            return None;
        }
        Some(FolderEntry {
            dir,
            path: path.to_str()?.to_string(),
            filename
        })
    }

    //Load tags from file
    pub fn load_file(path: &str) -> Result<TagEditorFile, Box<dyn Error>> {
        let filename = Path::new(path).file_name().ok_or("Invalid filename")?.to_str().ok_or("Invalid filename!")?;
        let tag_wrap = Tag::load_file(path)?;
        let id3_binary = ID3Binary::from_tag(&tag_wrap);
        //Load tags
        let tag = tag_wrap.tag().ok_or("No tag")?;
        let tags = tag.all_tags().iter().map(|(k, v)| {
            (k.to_owned(), v.join(",").replace('\0', ""))
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
            images,
            id3: id3_binary
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
    pub path: String,
    pub filename: String,
    pub dir: bool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagEditorFile {
    pub tags: HashMap<String, String>,
    pub filename: String,
    pub format: AudioFileFormat,
    pub path: String,
    pub images: Vec<TagEditorImage>,
    pub id3: Option<ID3Binary>
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

//Binary ID3 tags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ID3Binary {
    pub comments: Vec<ID3Comment>,
    pub unsync_lyrics: Vec<ID3Comment>,
    pub popularimeter: Option<ID3Popularimeter>
}

impl ID3Binary {
    pub fn from_tag(tag: &Tag) -> Option<ID3Binary> {
        match tag.id3.as_ref() {
            Some(t) => {
                Some(ID3Binary {
                    comments: t.get_comments(),
                    unsync_lyrics: t.get_unsync_lyrics(),
                    popularimeter: t.get_popularimeter()
                })
            },
            None => None
        }
    }
}