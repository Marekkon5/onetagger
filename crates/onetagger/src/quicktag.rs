use std::error::Error;
use std::path::PathBuf;
use std::collections::HashMap;
use std::fs::read_dir;
use std::path::Path;
use std::io::Cursor;
use walkdir::WalkDir;
use image::ImageOutputFormat;
use image::io::Reader as ImageReader;
use serde::{Deserialize, Serialize};
use onetagger_tag::{AudioFileFormat, Field, Tag, EXTENSIONS, TagSeparators};
use onetagger_playlist::{UIPlaylist, get_files_from_playlist_file};

pub struct QuickTag {}

impl QuickTag {
    /// Load all files from folder
    pub fn load_files_path(path: impl AsRef<Path>, recursive: bool, separators: &TagSeparators) -> Result<QuickTagData, Box<dyn Error>> {
        // Check if path to playlist
        if !path.as_ref().is_dir() {
            return QuickTag::load_files(get_files_from_playlist_file(path)?, separators);
        }
        
        let mut files = vec![];
        // Load recursivly
        if recursive {
            for e in WalkDir::new(path) {
                if let Ok(e) = e {
                    files.push(e.path().to_owned())
                }
            }
        } else {
            //  Load just dir
            for entry in read_dir(path)? {
                // Check if valid
                if entry.is_err() {
                    continue;
                }
                let entry = entry.unwrap();
                // Skip dirs
                if entry.path().is_dir() {
                    continue;
                }
                files.push(entry.path());
            }
        }
        
        QuickTag::load_files(files, separators)
    }

    /// Load all files from playlist
    pub fn load_files_playlist(playlist: &UIPlaylist, separators: &TagSeparators) -> Result<QuickTagData, Box<dyn Error>> {
        QuickTag::load_files(playlist.get_files()?, separators)
    }

    /// Check extension and load file
    pub fn load_files(files: Vec<PathBuf>, separators: &TagSeparators) -> Result<QuickTagData, Box<dyn Error>> {
        let mut out = vec![];
        let mut failed = vec![];
        for path in files {
            if EXTENSIONS.iter().any(|e| path.extension().unwrap_or_default().to_ascii_lowercase() == *e) {
                match QuickTagFile::from_path(&path, separators) {
                    Ok(t) => out.push(t),
                    Err(e) => {
                        failed.push(QuickTagFailed::new(&path, e.to_string()));
                        error!("Error loading file: {:?} {}", path, e);
                    }
                }
            }
        }
        Ok(QuickTagData {
            files: out,
            failed
        })
    }

}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuickTagData {
    pub files: Vec<QuickTagFile>,
    pub failed: Vec<QuickTagFailed>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickTagFailed {
    pub path: PathBuf,
    pub error: String
}

impl QuickTagFailed {
    /// Create new instance
    pub fn new(path: impl AsRef<Path>, error: impl Into<String>) -> QuickTagFailed {
        QuickTagFailed { path: path.as_ref().into(), error: error.into() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QuickTagFile {
    path: PathBuf,
    format: AudioFileFormat,
    title: String,
    artists: Vec<String>,
    genres: Vec<String>,
    bpm: Option<i64>,
    rating: u8,
    tags: HashMap<String, Vec<String>>,
    year: Option<i32>,
    key: Option<String>
}

impl QuickTagFile {
    /// Load tags from path
    pub fn from_path(path: impl AsRef<Path>, separators: &TagSeparators) -> Result<QuickTagFile, Box<dyn Error>> {
        let mut tag_wrap = Tag::load_file(&path, false)?;
        tag_wrap.set_separators(separators);
        Ok(QuickTagFile::from_tag(path, &tag_wrap)?)
    }

    /// Load tags from `Tag`
    pub fn from_tag(path: impl AsRef<Path>, tag_wrap: &Tag) -> Result<QuickTagFile, Box<dyn Error>> {
        let tag = tag_wrap.tag();
        let mut all_tags = tag.all_tags();
        // Insert overriden tags
        if let Some(v) = tag.get_raw("COMM") {
            all_tags.insert("COMM".to_string(), v);
        }
        if let Some(v) = tag.get_raw("USLT") {
            all_tags.insert("USLT".to_string(), v);
        }

        // Filter null bytes
        let all_tags = all_tags.into_iter().map(|(k, v)| (k, v.into_iter().map(|v| v.replace("\0", "")).collect::<Vec<_>>())).collect();

        Ok(QuickTagFile {
            path: path.as_ref().to_owned(),
            format: tag_wrap.format(),
            title: tag.get_field(Field::Title).ok_or("Missing title tag")?.first().ok_or("Missing title")?.to_string(),
            artists: tag.get_field(Field::Artist).ok_or("Missing artist tag")?,
            genres: tag.get_field(Field::Genre).unwrap_or(vec![]),
            rating: tag.get_rating().unwrap_or(0),
            bpm: match tag.get_field(Field::BPM) {
                Some(t) => t.first().unwrap_or(&"can't parse".to_string()).parse().ok(),
                None => None
            },
            tags: all_tags,
            year: tag.get_date().map(|d| d.year),
            key: tag.get_field(Field::Key).map(|f| f.first().map(String::from)).flatten()
        })
    }

    /// Load album art from tag and downscale
    pub fn get_art(path: impl AsRef<Path>) -> Result<Vec<u8>, Box<dyn Error>> {
        // Load
        let tag_wrap = Tag::load_file(path, false)?;
        let tag = tag_wrap.tag();
        let pictures = tag.get_art();
        let picture = pictures.first().ok_or("Missing album art!")?;
        let img = ImageReader::new(Cursor::new(&picture.data)).with_guessed_format()?.decode()?;
        // Downscale and save
        let scaled = img.thumbnail_exact(50, 50);
        let mut out = vec![];
        scaled.write_to(&mut Cursor::new(&mut out), ImageOutputFormat::Jpeg(95))?;
        Ok(out)
    }
}
