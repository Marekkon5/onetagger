//! Publish completed file updates without exposing partially written audio files.
use anyhow::{bail, Context, Error};
use std::fs;
use std::path::Path;
use tempfile::Builder;

/// Update a private sibling copy, then replace the original only after a successful sync.
/// Following the source's canonical path preserves symlink targets rather than replacing links.
pub fn atomic_update(path: &Path, update: impl FnOnce(&Path) -> Result<(), Error>) -> Result<(), Error> {
    let path = fs::canonicalize(path)?;
    let metadata = fs::metadata(&path)?;
    if !metadata.is_file() || metadata.permissions().readonly() {
        bail!("Audio file is not a writable regular file: {}", path.display());
    }
    let suffix = path.extension().map(|s| format!(".{}", s.to_string_lossy())).unwrap_or_default();
    let staged = Builder::new().prefix(".onetagger-").suffix(&suffix).tempfile_in(path.parent().unwrap())?;
    fs::copy(&path, staged.path())?;
    update(staged.path()).context("Tag update failed; the original file was preserved")?;
    fs::OpenOptions::new().write(true).open(staged.path())?.sync_all()?;
    staged.persist(&path).map_err(|e| e.error).context("Could not replace the original audio file")?;
    Ok(())
}

/// Copy or move a file. Returns false for an unchanged source/destination pair.
/// A conflicting destination is never silently reported as a successful move.
pub fn transfer_file(from: &Path, to: &Path, copy: bool, overwrite: bool) -> Result<bool, Error> {
    if !copy && fs::symlink_metadata(from)?.file_type().is_symlink() {
        bail!("Moving a symbolic link is not supported; select its target file explicitly: {}", from.display());
    }
    let source = fs::canonicalize(from)?;
    if !source.is_file() {
        bail!("Source is not a regular file: {}", from.display());
    }
    if same_file::is_same_file(&source, to).unwrap_or(false) {
        return Ok(false);
    }
    if !overwrite && to.symlink_metadata().is_ok() {
        bail!("Destination already exists: {}", to.display());
    }
    let parent = to.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;

    if !copy {
        if overwrite {
            if fs::rename(&source, to).is_ok() {
                return Ok(true);
            }
        } else {
            // Atomic no-clobber publication on the same filesystem, including Windows.
            match fs::hard_link(&source, to) {
                Ok(()) => {
                    fs::remove_file(&source).context("Destination created, but source could not be removed")?;
                    return Ok(true);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => return Err(e.into()),
                Err(_) => {} // Cross-device or unsupported hard links: stage a copy instead.
            }
        }
    }

    let staged = Builder::new().prefix(".onetagger-transfer-").tempfile_in(parent)?;
    fs::copy(&source, staged.path())?;
    staged.as_file().sync_all()?;
    if overwrite {
        staged.persist(to).map_err(|e| e.error)?;
    } else {
        staged.persist_noclobber(to).map_err(|e| e.error)?;
    }
    if !copy {
        fs::remove_file(&source).context("Destination copied, but source could not be removed")?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_update_preserves_original_and_cleans_staging_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("track.wav");
        fs::write(&path, b"original audio").unwrap();
        let result = atomic_update(&path, |staged| {
            fs::write(staged, b"partial")?;
            bail!("simulated disk/write failure")
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original audio");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn successful_update_publishes_complete_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("track.mp3");
        fs::write(&path, b"old").unwrap();
        atomic_update(&path, |staged| Ok(fs::write(staged, b"complete")?)).unwrap();
        assert_eq!(fs::read(path).unwrap(), b"complete");
    }

    #[test]
    fn collision_preserves_both_files() {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("source");
        let to = dir.path().join("target");
        fs::write(&from, b"source").unwrap();
        fs::write(&to, b"target").unwrap();
        for copy in [true, false] {
            assert!(transfer_file(&from, &to, copy, false).is_err());
            assert_eq!(fs::read(&from).unwrap(), b"source");
            assert_eq!(fs::read(&to).unwrap(), b"target");
        }
    }

    #[test]
    fn same_file_is_unchanged_even_with_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("track");
        fs::write(&path, b"audio").unwrap();
        for copy in [true, false] {
            assert!(!transfer_file(&path, &path, copy, true).unwrap());
            assert_eq!(fs::read(&path).unwrap(), b"audio");
        }
    }

    #[test]
    fn hard_link_alias_is_unchanged_in_copy_and_move_modes() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source"); let alias = dir.path().join("alias");
        fs::write(&source, b"audio").unwrap();
        fs::hard_link(&source, &alias).unwrap();
        for copy in [true, false] {
            for overwrite in [true, false] {
                assert!(!transfer_file(&source, &alias, copy, overwrite).unwrap());
                assert_eq!(fs::read(&source).unwrap(), b"audio");
                assert_eq!(fs::read(&alias).unwrap(), b"audio");
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn moving_symlink_does_not_delete_its_target() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let link = dir.path().join("link");
        fs::write(&source, b"audio").unwrap();
        std::os::unix::fs::symlink(&source, &link).unwrap();
        assert!(transfer_file(&link, &dir.path().join("target"), false, false).is_err());
        assert_eq!(fs::read(source).unwrap(), b"audio");
        assert!(link.is_symlink());
    }

    #[test]
    fn copy_and_move_publish_expected_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("source");
        let to = dir.path().join("folder/target");
        fs::write(&from, b"audio").unwrap();
        assert!(transfer_file(&from, &to, true, false).unwrap());
        assert!(from.exists());
        assert!(transfer_file(&from, &to, false, true).unwrap());
        assert!(!from.exists());
        assert_eq!(fs::read(to).unwrap(), b"audio");
    }
}
