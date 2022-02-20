use std::error::Error;
use std::path::{Path, PathBuf};
use directories::UserDirs;
use serde::{Serialize, Deserialize};
use walkdir::WalkDir;
use dunce::canonicalize;

use onetagger_tag::EXTENSIONS;
use onetagger_playlist::{PLAYLIST_EXTENSIONS, get_files_from_playlist_file};

pub struct FileBrowser {
    playlists: bool,
    files: bool
}

impl FileBrowser {
    /// List files in dir with a join path or use default directory
    /// returns new path because it can change
    pub fn list_dir_or_default(path: Option<PathBuf>, subdir: Option<String>, playlists: bool, files: bool, recursive: bool) -> Result<(PathBuf, Vec<FolderEntry>), Box<dyn Error>> {
        let user_dirs = UserDirs::new().ok_or("Invalid home dir!")?;
        let path = path.unwrap_or(user_dirs.audio_dir().ok_or("Missing path!")?.to_owned());
        let subdir = subdir.unwrap_or(String::new());
        // Override for playlists
        let path = if !path.is_dir() {
            if subdir == ".." {
                path.parent().ok_or("Invalid playlist parent!")?.to_owned()
            } else {
                path.to_owned()
            }
        } else {
            canonicalize(path.join(subdir))?
        };
        Ok((path.clone(), Self::list_dir(path, playlists, files, recursive)?))
    }

    /// List files in dir
    pub fn list_dir(path: impl AsRef<Path>, playlists: bool, files: bool, recursive: bool) -> Result<Vec<FolderEntry>, Box<dyn Error>> {
        let browser = FileBrowser {
            playlists, files
        };
        match recursive {
            true => browser.list_dir_recursive(path),
            false => browser.list_dir_internal(path)
        }
    }
 
    fn list_dir_internal(&self, path: impl AsRef<Path>) -> Result<Vec<FolderEntry>, Box<dyn Error>> {
        // Load playlist tracks
        if !path.as_ref().is_dir() {
            let files = get_files_from_playlist_file(path)?;
            return Ok(files.iter().filter_map(|e| self.validate_path(Path::new(e).to_owned())).collect());
        }
        
        // Load files from directory
        let mut out = vec![];
        for e in std::fs::read_dir(path)? {
            if let Ok(e) = e {
                if let Some(fe) = self.validate_path(e.path()) {
                    out.push(fe);
                }
            }
        }
        Ok(out)
    }

    // Get only supported files from all subdirectories
    fn list_dir_recursive(&self, path: impl AsRef<Path>) -> Result<Vec<FolderEntry>, Box<dyn Error>> {
        // Check if playlist
        if !path.as_ref().is_dir() {
            return self.list_dir_internal(path);
        }

        let mut out = vec![];
        for e in WalkDir::new(path) {
            if let Ok(e) = e {
                if let Some(fe) = self.validate_path(e.path().to_owned()) {
                    if !fe.dir {
                        out.push(fe);
                    }
                }
            }
        }
        Ok(out)
    }

    // Check if path is supported
    fn validate_path(&self, path: PathBuf) -> Option<FolderEntry> {
        let dir = path.is_dir();
        let mut playlist = false;
        let filename = path.file_name()?.to_str()?.to_owned();
        // Filter extensions
        if !dir {
            // Playlist
            if self.playlists && PLAYLIST_EXTENSIONS.iter().any(|e| filename.to_lowercase().ends_with(e)) {
                playlist = true;
            } else {
                // Music files
                if !self.files || path.extension().is_none() {
                    return None;
                }
                let extension = path.extension().unwrap().to_str()?.to_lowercase();
                if !EXTENSIONS.iter().any(|e| e[1..] == extension) {
                    return None;
                }
            }
        }
        
        if filename.starts_with('.') {
            return None;
        }
        Some(FolderEntry {
            dir,
            playlist,
            path: path.to_str()?.to_string(),
            filename
        })
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderEntry {
    pub path: String,
    pub filename: String,
    pub dir: bool,
    pub playlist: bool
}