#![windows_subsystem = "windows"]

#[macro_use]
extern crate slog;
#[macro_use]
extern crate slog_scope;
extern crate slog_term;

use std::env;
use std::panic;
use std::fs::OpenOptions;
use std::sync::Mutex;
use slog::{Drain, Duplicate};

//Get timestamp macro
macro_rules! timestamp {
    () => {
        std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_millis()
    };
}

mod tagger;
mod tag;
mod ui;

fn main() {
    //Logging setup
    let drain1 = slog_term::FullFormat::new(slog_term::TermDecorator::new().build()).build();
    let log = match OpenOptions::new()
        .append(true)
        .create(true)
        // .write(true)
        // .truncate(true)
        .open(ui::Settings::get_folder().unwrap().join("onetagger.log")) {
            //Log file
            Ok(file) => {
                let drain2 = slog_term::FullFormat::new(slog_term::PlainDecorator::new(file)).build().fuse();
                let both = Mutex::new(Duplicate::new(drain1, drain2)).fuse();
                slog::Logger::root(both, o!())
            },
            //Only terminal
            Err(_) => {
                slog::Logger::root(Mutex::new(drain1).fuse(), o!())
            }
        };
    let _guard = slog_scope::set_global_logger(log);
    //Panic hook
    panic::set_hook(Box::new(|p| {
        error!("PANIC: {}", p);
        if let Some(location) = p.location() {
            error!("LOCATION: File: {}, Line: {}", location.file(), location.line());
        }
    }));

    //Server mode
    if env::args().any(|a| a == "--server") {
        info!("Starting server mode! http://localhost:36913 ws://localhost:36912");
        ui::start_webserver_thread();
        ui::socket::start_socket_server();
        return;
    }
    //UI
    ui::start_all();
}
