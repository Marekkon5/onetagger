use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[cfg(windows)]
extern crate winres;

#[cfg(windows)]
fn main() {
    get_commit();
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets\\icon.ico");
    res.compile().unwrap();
}

#[cfg(not(windows))]
fn main() {
    get_commit();
}

// Save commit to enviromnent variable
fn get_commit() {
    // Github Actions commit
    let mut commit = if let Ok(commit) = env::var("GITHUB_SHA") {
        commit
    } else {
        // Local commit
        if let Ok(mut f) = File::open(Path::new(".git").join("refs").join("heads").join("master")) {
            let mut buf = String::new();
            f.read_to_string(&mut buf).ok();
            buf
        } else {
            String::new()
        }
    };
    //  Trim
    if commit.len() > 8 {
        commit = commit[..8].to_string()
    }
    if commit.is_empty() {
        commit = "unknown".to_string();
    }
    println!("cargo:rustc-env=COMMIT={}", commit);
}