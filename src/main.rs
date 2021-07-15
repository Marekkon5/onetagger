#![windows_subsystem = "windows"]

#[macro_use]
extern crate slog;
#[macro_use]
extern crate slog_scope;
extern crate slog_term;

use std::env;
use std::panic;
use std::path::Path;
use std::fs::OpenOptions;
use std::sync::Mutex;
use backtrace::Backtrace;
use slog::{Drain, Duplicate};

// Get timestamp macro
macro_rules! timestamp {
    () => {
        std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_millis()
    };
}

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

mod tagger;
mod tag;
mod ui;
mod playlist;

// Testing and benchmarking
mod test;

fn main() {
    // Logging setup
    let drain1 = slog_term::FullFormat::new(slog_term::TermDecorator::new().build()).build();
    let log = match OpenOptions::new()
        .append(true)
        .create(true)
        .open(ui::Settings::get_folder().unwrap().join("onetagger.log")) {
            // Log file
            Ok(file) => {
                let drain2 = slog_term::FullFormat::new(slog_term::PlainDecorator::new(file)).build().fuse();
                let both = Mutex::new(Duplicate::new(drain1, drain2)).fuse();
                slog::Logger::root(both, o!())
            },
            // Only terminal
            Err(_) => {
                slog::Logger::root(Mutex::new(drain1).fuse(), o!())
            }
        };
    let _guard = slog_scope::set_global_logger(log);
    // Panic hook
    panic::set_hook(Box::new(|p| {
        let bt = Backtrace::new();
        error!("PANIC: {}", p);
        if let Some(location) = p.location() {
            error!("LOCATION: File: {}, Line: {}", location.file(), location.line());
        }
        // Show backtrace
        if env::var_os("RUST_BACKTRACE").is_some() {
            debug!("BACKTRACE:\n{:?}", bt);
        }
    }));

    info!("\n\nStarting OneTagger v{} Commit: {} OS: {}\n", VERSION, env!("COMMIT"), env::consts::OS);

    // Parse arguments
    let args: Vec<String> = env::args().skip(1).collect();
    let mut server_mode = false;
    let mut start_path = None;
    let mut expose = false;
    for arg in args {
        match arg.as_str() {
            "--server" => server_mode = true,
            "-S" => server_mode = true,
            // Benchmark mode
            "--benchmark" => {
                #[cfg(target_os = "windows")]
                msgbox::create(
                    "One Tagger", 
                    "After you press OK benchmark mode will start. Messagebox will appear when it's done.", 
                    msgbox::IconType::Info
                ).unwrap();
                test::run_benchmark();
                #[cfg(target_os = "windows")]
                msgbox::create("One Tagger", "Benchmarking finished! Results are in logs.", msgbox::IconType::Info).unwrap();
                return;
            },
            "--expose" => expose = true,
            // Webview2 bootstrap
            #[cfg(target_os = "windows")]
            "--bootstrap-webview2" => {
                ui::bootstrap_webview2_wrap();
                return;
            },
            _ => {
                // Use argument as start path
                if !arg.starts_with("-") {
                    if Path::new(&arg).exists() {
                        start_path = Some(arg.to_string());
                    }
                }
            }
        }
    }
    // Start
    let context = ui::StartContext {
        start_path, server_mode, expose
    };
    ui::start_all(context);
}