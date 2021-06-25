use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use serde::{Serialize, Deserialize};

use crate::tag::EXTENSIONS;
use crate::ui::OTError;

pub const PLAYLIST_EXTENSIONS: [&str; 2] = [".m3u", ".m3u8"];

//Playlist info from UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIPlaylist {
    //base64
    pub data: String,
    pub filename: String,
    pub format: PlaylistFormat
}

impl UIPlaylist {
    pub fn get_files(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let files = match self.format {
            PlaylistFormat::M3U => {
                //Decode base64 from JS
                let bytes = base64::decode(self.data[self.data.find(';').ok_or("Invalid data!")? + 8..].trim())?;
                let m3u = String::from_utf8(bytes)?;
                get_files_from_m3u(&m3u)
            }
        };
        //Filter extensions
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


pub fn get_files_from_playlist_file(path: &str) -> Result<Vec<String>, Box<dyn Error>> {
    //Validate extension
    if !PLAYLIST_EXTENSIONS.iter().any(|e| path.to_lowercase().ends_with(e)) {
        return Err(OTError::new("Unsupported playlist!").into());
    };
    
    //Load file
    let mut file = File::open(path)?;
    let mut buf = vec![];
    file.read_to_end(&mut buf)?;

    //TODO: Check format if multiple

    //M3U
    let data = String::from_utf8(buf)?;
    Ok(get_files_from_m3u(&data))
}


//Get file list from M3U playlist
pub fn get_files_from_m3u(m3u: &str) -> Vec<String> {
    let clean = m3u.replace("\r", "\n").replace("\n\n", "\n");
    let entries = clean.split("\n");
    let mut out = vec![];
    for entry in entries {
        if !entry.starts_with("#") && !entry.starts_with("http://") && !entry.is_empty() {
            out.push(entry.trim().to_string());
        }
    }
    out
}