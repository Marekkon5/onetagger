use std::io::{BufWriter, BufReader};
use std::fs::File;
use anyhow::Error;
use onetagger_shared::Settings;
use serde_json::Value;
use tempfile::NamedTempFile;
use zip::ZipArchive;

const MANIFEST_URL: &str = "https://raw.githubusercontent.com/Marekkon5/onetagger-platforms/master/platforms.json";
const DOWNLOAD_URL: &str = "https://github.com/Marekkon5/onetagger-platforms/releases/download/platforms";

/// Fetch custom platforms manifest
/// TODO: Serialization not implemented right now since it just gets passed to UI
pub fn fetch_manifest() -> Result<Value, Error> {
    Ok(reqwest::blocking::get(MANIFEST_URL)?.json()?)
}

/// Download and install custom platform
pub fn install_platform(id: &str, version: &str, is_native: bool) -> Result<(), Error> {
    info!("Installing platform {id}@{version}");

    // Generate filename
    let name = format!("{id}_{version}");
    let name = match is_native {
        true => {
            let name = format!("{name}_{}_{}", std::env::consts::OS, std::env::consts::ARCH);
            match std::env::consts::OS {
                "windows" => format!("{name}.dll"),
                "linux" => format!("{name}.so"),
                "macos" => format!("{name}.dylib"),
                // Fallback assume linux
                _ => format!("{name}.so")
            }
        },
        false => format!("{name}"),
    };

    // Get paths
    let platforms_dir = Settings::get_folder()?.join("platforms");
    let path = platforms_dir.join(&name);

    // Download native
    if is_native {
        std::io::copy(&mut reqwest::blocking::get(format!("{DOWNLOAD_URL}/{name}"))?, &mut BufWriter::new(File::create(path)?))?;
        return Ok(())
    }
   
    // Download Python
    let mut tmp_file = NamedTempFile::new()?;
    std::io::copy(&mut reqwest::blocking::get(format!("{DOWNLOAD_URL}/{name}.zip"))?, &mut BufWriter::new(File::create(tmp_file.path())?))?;
    if path.exists() {
        std::fs::create_dir_all(&path)?;
    }
    let mut zip = ZipArchive::new(BufReader::new(tmp_file.as_file_mut()))?;
    zip.extract(path)?;

    Ok(())
}