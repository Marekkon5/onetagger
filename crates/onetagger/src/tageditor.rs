use std::error::Error;
use std::io::Cursor;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use base64::Engine;
use serde::{Serialize, Deserialize};
use image::{GenericImageView, io::Reader as ImageReader};

use onetagger_tag::{AudioFileFormat, CoverType, Picture, Tag};
use onetagger_tag::id3::{ID3Comment, ID3Popularimeter};

pub struct TagEditor {}

impl TagEditor {
    // Load tags from file
    pub fn load_file(path: impl AsRef<Path>) -> Result<TagEditorFile, Box<dyn Error>> {
        let filename = path.as_ref().file_name().ok_or("Invalid filename")?.to_str().ok_or("Invalid filename!")?;
        let tag_wrap = Tag::load_file(&path, true)?;
        let id3_binary = ID3Binary::from_tag(&tag_wrap);
        // Load tags
        let tag = tag_wrap.tag();
        let tags = tag.all_tags().iter().map(|(k, v)| {
            (k.to_owned(), v.join(",").replace('\0', ""))
        }).collect();

        // Load images
        let mut images = vec![];
        for picture in tag.get_art() {
            if let Ok(art) = TagEditor::load_art(picture) {
                images.push(art);
            }
        }

        Ok(TagEditorFile {
            tags,
            filename: filename.to_owned(),
            format: tag_wrap.format(),
            path: path.as_ref().to_owned(),
            images,
            id3: id3_binary
        })
    }

    // Load art and encode
    fn load_art(picture: Picture) -> Result<TagEditorImage, Box<dyn Error>> {
        let img = ImageReader::new(Cursor::new(&picture.data)).with_guessed_format()?.decode()?;
        Ok(TagEditorImage {
            mime: picture.mime.to_string(),
            data: format!("data:{};base64,{}", &picture.mime, base64::engine::general_purpose::STANDARD.encode(picture.data)),
            kind: picture.kind.to_owned(),
            description: picture.description.to_owned(),
            width: img.dimensions().0,
            height: img.dimensions().1,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderEntry {
    pub path: PathBuf,
    pub filename: String,
    pub dir: bool,
    pub playlist: bool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagEditorFile {
    pub tags: HashMap<String, String>,
    pub filename: String,
    pub format: AudioFileFormat,
    pub path: PathBuf,
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

// Binary ID3 tags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ID3Binary {
    pub comments: Vec<ID3Comment>,
    pub unsync_lyrics: Vec<ID3Comment>,
    pub popularimeter: Option<ID3Popularimeter>
}

impl ID3Binary {
    pub fn from_tag(tag: &Tag) -> Option<ID3Binary> {
        match tag {
            Tag::ID3(t) => Some(ID3Binary {
                comments: t.get_comments(),
                unsync_lyrics: t.get_unsync_lyrics(),
                popularimeter: t.get_popularimeter()
            }),
            _ => None
        }
    }
}