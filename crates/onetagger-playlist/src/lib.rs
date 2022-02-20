use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};

use onetagger_tag::EXTENSIONS;
use onetagger_shared::OTError;

pub const PLAYLIST_EXTENSIONS: [&str; 2] = ["m3u", "m3u8"];

// Playlist info from UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIPlaylist {
    // base64
    pub data: String,
    pub filename: String,
    pub format: PlaylistFormat
}

impl UIPlaylist {
    pub fn get_files(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let files = match self.format {
            PlaylistFormat::M3U => {
                // Decode base64 from JS
                let bytes = base64::decode(self.data[self.data.find(';').ok_or("Invalid data!")? + 8..].trim())?;
                let m3u = String::from_utf8(bytes)?;
                get_files_from_m3u(&m3u, None)
            }
        };
        // Filter extensions
        let out = files.iter().filter(|f| EXTENSIONS.iter().any(|e| f.to_lowercase().ends_with(e)))
            .map(String::from).collect();
        Ok(out)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaylistFormat {
    M3U
}


pub fn get_files_from_playlist_file(path: impl AsRef<Path>) -> Result<Vec<String>, Box<dyn Error>> {
    // Validate extension
    if !PLAYLIST_EXTENSIONS.iter().any(|e| &&path.as_ref().extension().unwrap_or_default().to_string_lossy().to_lowercase() == e) {
        return Err(OTError::new("Unsupported playlist!").into());
    };
    
    // Load file
    let mut file = File::open(&path)?;
    let mut buf = vec![];
    file.read_to_end(&mut buf)?;

    // TODO: Check format if multiple

    // M3U
    let data = String::from_utf8(buf)?;
    Ok(get_files_from_m3u(&data, Some(path.as_ref().parent().unwrap().to_owned())))
}


// Get file list from M3U playlist
pub fn get_files_from_m3u(m3u: &str, base_path: Option<PathBuf>) -> Vec<String> {
    let clean = m3u.replace("\r", "\n").replace("\n\n", "\n");
    let entries = clean.split("\n");
    let mut out = vec![];
    for entry in entries {
        if !entry.starts_with("#") && !entry.starts_with("http://") && !entry.is_empty() {
            if base_path.is_none() {
                out.push(entry.trim().to_string());
            } else {
                // Add base path
                out.push(base_path.clone().unwrap().join(entry).to_str().unwrap().to_string());
            }
        }
    }
    out
}