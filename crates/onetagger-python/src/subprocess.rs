use std::io::{Read, Write, BufReader, BufRead};
use std::path::PathBuf;
use std::process::{ChildStdin, ChildStdout, Child};
use anyhow::Error;
use log::{Record, Metadata, Log, LevelFilter, Level};
use onetagger_tagger::{AudioFileInfo, TaggerConfig, Track, TrackMatch};
use pyembed::MainPythonInterpreter;
use pyo3::types::PyTuple;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;
use pyo3::prelude::*;

use crate::stdlib_path;

static PYTHON_SUBPROCESS_LOG: PythonSubprocessLog = PythonSubprocessLog;

/// Request sent from parent process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PythonRequest {
    /// Initialize the python interpreter
    Init { path: PathBuf, code: String },

    /// Exit
    Exit,

    MatchTrack { info: AudioFileInfo, config: TaggerConfig },
    ExtendTrack { track: Track, config: TaggerConfig },
}

/// Response from child process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PythonResponse {
    Log { level: Level, message: String },
    Error { error: String },
    InitOk,

    MatchTrack { result: Result<Vec<TrackMatch>, String> },
    ExtendTrack { result: Result<Track, String> }
}

/// Start python process
pub fn python_process() {
    // Logging
    log::set_logger(&PYTHON_SUBPROCESS_LOG)
        .map(|()| log::set_max_level(LevelFilter::Debug))
        .ok();

    // Wait for init path
    let (path, code) = loop {
        match read_stdin() {
            Ok(PythonRequest::Init { path, code }) => break (path, code),
            Ok(PythonRequest::Exit) => return,
            Err(e) => { error!("{e}"); return },
            _ => {}
        }
    };

    // Respond
    write_stdout(&PythonResponse::InitOk).ok();

    // Run interpreter
    match python_interpreter(path, &code) {
        Ok(_) => {},
        Err(e) => {
            error!("Python failed: {e}");
            write_stdout(&PythonResponse::Error { error: e.to_string() }).ok();
        },
    }
}

/// Start and run python interpreter
fn python_interpreter(path: PathBuf, code: &str) -> Result<(), Error> {
    crate::module::setup();
    let config = crate::module::pyoxidizer_config(path, stdlib_path()?)?;
    let interpreter = MainPythonInterpreter::new(config)?;
    interpreter.with_gil(|py| -> Result<(), Error> {
        // Load utils
        let _util = PyModule::from_code(py, include_str!("util.py"), "", "")?;

        // Load module
        let module = PyModule::from_code(py, code, "", "")?;
        let match_track = module.getattr("match_track")?;
        let extend_track = module.getattr("extend_track")?;



        // Read loop
        while let Ok(request) = read_stdin() {
            match request {
                // Ignore
                PythonRequest::Init { .. } => {},
                PythonRequest::Exit => return Ok(()),

                PythonRequest::MatchTrack { info, config } => {
                    write_stdout(&PythonResponse::MatchTrack {
                        result: call_py::<(AudioFileInfo, TaggerConfig), Vec<TrackMatch>>(match_track, (info, config)).map_err(|e| e.to_string())
                    })?;
                },
                PythonRequest::ExtendTrack { track, config } => {
                    write_stdout(&PythonResponse::ExtendTrack {
                        result: call_py::<(Track, TaggerConfig), Track>(extend_track, (track, config)).map_err(|e| e.to_string())
                    })?;
                },
            }
        }
        Ok(())
    })?;
    Ok(())
}

/// Call python function and extract result
fn call_py<'a, A, R: FromPyObject<'a>>(f: &'a PyAny, a: impl IntoPy<Py<PyTuple>>) -> Result<R, Error> {
    Ok(f.call1(a)?.extract()?)
}

pub struct SubprocessWrap {
    stdin: ChildStdin,
    stdout: ChildStdout,
    pub child: Child
}

impl SubprocessWrap {
    /// Wrap a subprocess
    pub fn new(mut child: Child) -> SubprocessWrap {
        Self { stdin: child.stdin.take().unwrap(), stdout: child.stdout.take().unwrap(), child }
    }

    /// Send message
    pub fn send(&mut self, r: &PythonRequest) -> Result<(), Error> {
        write_message(&mut self.stdin, r)?;
        Ok(())
    }

    /// Receive message
    pub fn recv(&mut self) -> Result<PythonResponse, Error> {
        loop {
            let response: PythonResponse = read_message(&mut self.stdout)?;
            debug!("{response:?}");
            match response {
                PythonResponse::Log { level, message } => {
                    match level {
                        Level::Error => error!("{message}"),
                        Level::Warn => warn!("{message}"),
                        Level::Info => info!("{message}"),
                        Level::Debug => debug!("{message}"),
                        Level::Trace => trace!("{message}"),
                    }
                },
                PythonResponse::Error { error } => return Err(anyhow!("{error}")),
                r => return Ok(r)
            }
        }
    }
}

/// Read and deserialize message
fn read_message<R: Read, D: DeserializeOwned>(read: &mut R) -> Result<D, Error> {
    let mut size_buf = [0u8; 4];
    read.read_exact(&mut size_buf)?;
    debug!("len: {:?}", size_buf);
    let size = u32::from_be_bytes(size_buf) as usize;
    let mut buf = vec![0u8; size];
    read.read_exact(&mut buf)?;
    Ok(bitcode::deserialize(&buf)?)
}

/// Read from stdin
fn read_stdin() -> Result<PythonRequest, Error> {
    let mut stdin = std::io::stdin().lock();
    Ok(read_message(&mut stdin)?)
}

/// Serialize and write message
fn write_message<W: Write, S: Serialize>(write: &mut W, msg: &S) -> Result<(), Error> {
    let buf = bitcode::serialize(msg)?;
    let len = (buf.len() as u32).to_be_bytes();
    write.write_all(&len)?;
    write.write_all(&buf)?;
    write.flush()?;
    Ok(())
}

/// Write message to stdout
fn write_stdout(msg: &PythonResponse) -> Result<(), Error> {
    let mut stdout = std::io::stdout().lock();
    write_message(&mut stdout, msg)?;
    Ok(())
}

struct PythonSubprocessLog;

impl Log for PythonSubprocessLog {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        write_stdout(&PythonResponse::Log { level: record.level(), message: format!("{}", record.args()) }).ok();
    }

    fn flush(&self) {
        
    }
}