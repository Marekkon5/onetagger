use std::io::BufWriter;
use std::fs::File;
use anyhow::Error;
use onetagger_shared::Settings;
use serde_json::Value;

const MANIFEST_URL: &str = "https://raw.githubusercontent.com/Marekkon5/onetagger-platforms/master/platforms.json";
const DOWNLOAD_URL: &str = "https://github.com/Marekkon5/onetagger-platforms/releases/download/platforms";

/// Fetch custom platforms manifest
/// TODO: Serialization not implemented right now since it just gets passed to UI
pub fn fetch_manifest() -> Result<Value, Error> {
    Ok(reqwest::blocking::get(MANIFEST_URL)?.json()?)
}

/// Fetch custom platforms repo async version because WS is handled by async runtime
pub async fn fetch_manifest_async() -> Result<Value, Error> {
    Ok(reqwest::get(MANIFEST_URL).await?.error_for_status()?.json().await?)
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
   
    Ok(())
}