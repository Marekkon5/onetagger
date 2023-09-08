use std::path::Path;
use std::io::{BufWriter, BufReader};
use std::fs::File;
use anyhow::Error;
use regex::Regex;
use walkdir::WalkDir;
use zip::{ZipWriter, CompressionMethod};
use zip::write::FileOptions;

const PYTHON_VERSION: &'static str = "3.10";

fn main() {
    setup_all("crates/onetagger-python/pyembedded").expect("Failed");
    println!("Done");
}

/// Do all setups
fn setup_all(out_dir: impl AsRef<Path>) -> Result<(), Error> {
    // Create outdir
    if !out_dir.as_ref().exists() {
        std::fs::create_dir_all(&out_dir)?;
    }

    // Pyoxidizer
    println!("Setting up pyoxidizer...");
    setup_pyoxidizer(&out_dir, PYTHON_VERSION)?;
    println!("Zipping stdlib...");
    zip_stdlib(out_dir.as_ref().join("stdlib"), out_dir.as_ref().join("stdlib.zip"))?;

    // Windows also has libs folder
    if cfg!(target_os = "windows") {
        zip_stdlib(out_dir.as_ref().join("lib"), out_dir.as_ref().join("lib.zip"))?;
    }

    // Patch & copy config
    println!("Copy config...");
    let re = Regex::new("packed_resources: .*").unwrap();
    let config = std::fs::read_to_string(out_dir.as_ref().join("default_python_config.rs"))?;
    let config = re.replace(&config, "packed_resources: vec![],\n");
    std::fs::write(out_dir.as_ref().join("config.rs"), config.as_bytes())?;

    // Write python version
    println!("Write version...");
    std::fs::write(out_dir.as_ref().join("VERSION"), PYTHON_VERSION)?;

    // Download pip
    println!("Downloading pip...");
    std::io::copy(
        &mut ureq::get("https://bootstrap.pypa.io/pip/pip.pyz").call()?.into_reader(), 
        &mut BufWriter::new(File::create(out_dir.as_ref().join("pip.pyz"))?)
    )?;

    // Clean
    println!("Cleanup...");
    std::fs::remove_dir_all(out_dir.as_ref().join("stdlib"))?;
    std::fs::remove_dir_all(out_dir.as_ref().join("tcl"))?;
    std::fs::remove_file(out_dir.as_ref().join("default_python_config.rs"))?;
    if cfg!(windows) {
        std::fs::remove_dir_all(out_dir.as_ref().join("lib"))?;
    }

    Ok(())
}

/// Setup python embedding
fn setup_pyoxidizer(path: impl AsRef<Path>, version: &str) -> Result<(), Error> {
    pyoxidizerlib::projectmgmt::generate_python_embedding_artifacts(
        &pyoxidizerlib::environment::Environment::new()?,
        env!("TARGET"),
        "standalone",
        Some(version),
        path.as_ref()
    )?;
    Ok(())
}

/// Zip python stdlib
fn zip_stdlib(path: impl AsRef<Path>, out: impl AsRef<Path>) -> Result<(), Error> {
    let prefix = path.as_ref().canonicalize()?;
    let mut zip = ZipWriter::new(BufWriter::new(File::create(&out)?));
    for entry in WalkDir::new(&path).into_iter().filter_map(|e| e.ok()).filter(|e| e.path().is_file()) {
        let name = entry.path().canonicalize()?.strip_prefix(&prefix)?.to_owned();
        zip.start_file(name.to_string_lossy(), FileOptions::default().compression_method(CompressionMethod::Deflated).compression_level(Some(9)))?;
        std::io::copy(&mut BufReader::new(File::open(entry.path())?), &mut zip)?;
    }
    zip.finish()?;
    Ok(())
}