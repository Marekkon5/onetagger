#[macro_use] extern crate anyhow;

use anyhow::Error;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use base64::Engine;
use onetagger_tag::EXTENSIONS;
use url::Url;

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
    pub fn get_files(&self) -> Result<Vec<PathBuf>, Error> {
        let files = match self.format {
            PlaylistFormat::M3U => {
                // Decode base64 from JS
                let bytes = base64::engine::general_purpose::STANDARD.decode(self.data.split_once(";base64,").ok_or(anyhow!("Invalid playlist data URL"))?.1.trim())?;
                let m3u = String::from_utf8(bytes)?;
                get_files_from_m3u(&m3u, None)
            }
        };
        // Filter extensions
        let out = files
            .iter()
            .filter(|f| is_audio_file(f))
            .map(PathBuf::from).collect();
        Ok(out)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaylistFormat {
    M3U
}


/// Get files from any playlist format
pub fn get_files_from_playlist_file(path: impl AsRef<Path>) -> Result<Vec<PathBuf>, Error> {
    let path = path.as_ref();
    if !has_extension(path, &PLAYLIST_EXTENSIONS) {
        return Err(anyhow!("Unsupported playlist: {}", path.display()));
    }
    let data = std::fs::read_to_string(path)?;
    Ok(get_files_from_m3u(&data, path.parent().map(Path::to_owned)))
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension().and_then(|ext| ext.to_str())
        .is_some_and(|ext| extensions.iter().any(|known| ext.eq_ignore_ascii_case(known)))
}

pub fn is_audio_file(path: &Path) -> bool {
    has_extension(path, &EXTENSIONS)
}

/// Plain M3U entries are filesystem paths. Only file URIs have percent escapes.
pub fn get_files_from_m3u(m3u: &str, base_path: Option<PathBuf>) -> Vec<PathBuf> {
    m3u.trim_start_matches('\u{feff}').split(['\r', '\n']).filter_map(|entry| {
        if entry.trim().is_empty() || entry.starts_with('#') {
            return None;
        }
        if entry.get(..5).is_some_and(|prefix| prefix.eq_ignore_ascii_case("file:")) {
            return Url::parse(entry).ok()?.to_file_path().ok();
        }
        if entry.split_once("://").is_some_and(|(scheme, _)| {
            scheme.len() > 1 && scheme.bytes().next().is_some_and(|c| c.is_ascii_alphabetic())
                && scheme.bytes().all(|c| c.is_ascii_alphanumeric() || b"+.-".contains(&c))
        }) {
            return None;
        }
        Some(base_path.as_deref().unwrap_or(Path::new("")).join(entry))
    }).collect()
}

/// Generate an M3U playlist without re-copying earlier paths for every entry.
pub fn create_m3u_playlist(paths: &[PathBuf]) -> String {
    let mut playlist = String::from("#EXTM3U\r\n");
    for path in paths {
        let entry = path.to_string_lossy();
        if entry.starts_with('#') { playlist.push_str("./"); }
        playlist.push_str(&entry);
        playlist.push_str("\r\n");
    }
    playlist
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_paths_and_file_uris_round_trip_independently() {
        let root = std::env::current_dir().unwrap();
        let literal = root.join("Mix%20One – 日本.mp3");
        let space = root.join("Mix One – 日本.mp3");
        let paths = vec![literal.clone(), space.clone()];
        let exported = create_m3u_playlist(&paths);
        assert_eq!(exported, format!("#EXTM3U\r\n{}\r\n{}\r\n", literal.display(), space.display()));
        assert_eq!(get_files_from_m3u(&exported, None), paths);
        let uri = Url::from_file_path(&literal).unwrap();
        let inputs = format!("\u{feff}#EXTM3U\rMix%20One – 日本.mp3\r\n{uri}\nhttps://example.com/live.mp3\n");
        assert_eq!(get_files_from_m3u(&inputs, Some(root)), vec![literal.clone(), literal]);
        let ui = UIPlaylist { filename: "mix.m3u8".into(), format: PlaylistFormat::M3U,
            data: format!("data:audio/x-mpegurl;base64,{}", base64::engine::general_purpose::STANDARD.encode(exported)) };
        assert_eq!(ui.get_files().unwrap(), paths);
    }
}
