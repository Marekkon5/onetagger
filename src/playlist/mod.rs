use std::error::Error;
use serde::{Serialize, Deserialize};

use crate::tag::EXTENSIONS;

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
                let bytes = base64::decode(self.data.replace("data:audio/x-mpegurl;base64,", "").trim())?;
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

//Get file list from M3U playlist
pub fn get_files_from_m3u(m3u: &str) -> Vec<String> {
    let clean = m3u.replace("\r\n", "\n");
    let entries = clean.split("\n");
    let mut out = vec![];
    for entry in entries {
        if !entry.starts_with("#") && !entry.starts_with("http://") {
            out.push(entry.trim().to_string());
        }
    }
    out
}