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
        let path = path.unwrap_or(user_dirs.audio_dir().map(|p| p.to_owned()).unwrap_or(user_dirs.home_dir().to_owned()));
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
        out.sort_by(|a, b| a.path.to_string_lossy().to_lowercase().cmp(&b.path.to_string_lossy().to_lowercase()));
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
                if !EXTENSIONS.iter().any(|e| *e == extension) {
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
            path,
            filename
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

pub struct FolderBrowser;

impl FolderBrowser {
    /// List all subfolders in a directory
    pub fn list_dir(path: impl AsRef<Path>) -> Result<DirectoryEntry, Box<dyn Error>> {
        // Windows root dir override
        #[cfg(target_os = "windows")]
        if path.as_ref().to_string_lossy() == "/" {
            use sysinfo::{System, SystemExt, DiskExt};
            let sys = System::new_all();
            let disks = sys.disks().iter().map(|d| DirectoryEntry { 
                path: d.mount_point().to_string_lossy().replace(":\\", ":").into(), children: None 
            }).collect::<Vec<_>>();
            return Ok(DirectoryEntry {
                children: Some(disks),
                path: PathBuf::from("/".to_string())
            });
        }

        // List dir
        let mut folders = std::fs::read_dir(&path)?.into_iter().filter_map(|e| match e {
            Ok(e) => {
                let path = e.path();
                if path.is_dir() && path.file_name().is_some() {
                    Some(path)
                } else {
                    None
                }
            },
            Err(_) => None
        }).collect::<Vec<_>>();
        // Sort
        folders.sort_by(|a, b| a.to_string_lossy().to_lowercase().cmp(&b.to_string_lossy().to_lowercase()));
        let children = folders.into_iter().map(|f| DirectoryEntry { path: f, children: None }).collect();

        Ok(DirectoryEntry {
            children: Some(children), path: path.as_ref().into()
        })
    }

    /// Generate tree structure from base path
    pub fn generate_base(path: impl AsRef<Path>) -> Result<DirectoryEntry, Box<dyn Error>> {
        let mut path = canonicalize(path.as_ref())?;
        let mut last_entry: Option<DirectoryEntry> = None;

        loop {
            let children = Self::list_dir(&path)?;
            // Merge children
            let entry = DirectoryEntry {
                children: Some(
                    children.children.unwrap()
                        .into_iter()
                        .filter_map(|e| match last_entry.as_ref() {
                            Some(last) => {
                                if last.path == e.path {
                                    Some(last.clone())
                                } else {
                                    Some(e)
                                }
                            },
                            None => Some(e)
                        })
                        .collect()
                    ),
                path: path.to_owned(),
            };

            // Go to parent
            last_entry = Some(entry);
            path = match path.parent() {
                Some(parent) => parent.to_owned(),
                None => break
            }
        }
        
        Ok(last_entry.unwrap())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<DirectoryEntry>>,
    pub path: PathBuf,
}

