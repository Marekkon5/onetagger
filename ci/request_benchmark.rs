//! Real loopback HTTP requests through the production release-response cache.
//! Synthetic metadata/latency only: this never contacts a live provider.
#[path = "../crates/onetagger-platforms/src/request_cache.rs"]
mod request_cache;

use request_cache::RequestCache;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicUsize, Ordering}};
use std::thread;
use std::time::{Duration, Instant};

struct Server {
    address: SocketAddr,
    requests: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Server {
    fn start() -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let count = requests.clone(); let stopped = stop.clone();
        let worker = thread::spawn(move || thread::scope(|scope| {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                if stopped.load(Ordering::SeqCst) { break; }
                let count = &count;
                scope.spawn(move || {
                    stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
                    let mut reader = BufReader::new(&stream);
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    let release = line.split_whitespace().nth(1).unwrap()
                        .strip_prefix("/release/").unwrap().parse::<usize>().unwrap();
                    loop {
                        let mut header = String::new();
                        assert!(reader.read_line(&mut header).unwrap() > 0);
                        if header == "\r\n" { break; }
                    }
                    drop(reader);
                    count.fetch_add(1, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(2));
                    let body = format!("release={release};label=fixture;edition=original");
                    write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
                });
            }
        }));
        Self { address, requests, stop, worker: Some(worker) }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        TcpStream::connect(self.address).unwrap();
        self.worker.take().unwrap().join().unwrap();
    }
}

fn fetch(address: SocketAddr, release: usize) -> std::io::Result<String> {
    let mut stream = TcpStream::connect(address)?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    write!(stream, "GET /release/{release} HTTP/1.1\r\nHost: fixture\r\nConnection: close\r\n\r\n")?;
    let mut response = String::new(); stream.read_to_string(&mut response)?;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    let body = response.split_once("\r\n\r\n").unwrap().1;
    assert_eq!(body, format!("release={release};label=fixture;edition=original"));
    Ok(body.to_owned())
}

fn run(case: &str, releases: usize, cached: bool, trial: usize) -> Vec<String> {
    let server = Server::start();
    let cache = RequestCache::new(if cached { 256 } else { 0 }, Duration::from_secs(600));
    let next = AtomicUsize::new(0);
    let results = Mutex::new(vec![String::new(); 1000]);
    let started = Instant::now();
    thread::scope(|scope| {
        for _ in 0..16 {
            scope.spawn(|| loop {
                let song = next.fetch_add(1, Ordering::SeqCst);
                if song >= 1000 { break; }
                let release = song * releases / 1000;
                let body = cache.get_or_try_init(release, || fetch(server.address, release)).unwrap();
                results.lock().unwrap()[song] = format!("song={song};{body}");
            });
        }
    });
    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
    let requests = server.requests.load(Ordering::SeqCst);
    assert_eq!(requests, if cached { releases } else { 1000 });
    println!("{case},{cached},{trial},{elapsed:.3},{requests}");
    results.into_inner().unwrap()
}

fn main() {
    println!("case,cached,trial,elapsed_ms,http_requests");
    for trial in 0..5 {
        for (case, releases) in [("unique_releases", 1000), ("ten_songs_per_release", 100), ("one_shared_release", 1)] {
            let (first, second) = if trial % 2 == 0 { (false, true) } else { (true, false) };
            assert_eq!(run(case, releases, first, trial), run(case, releases, second, trial),
                "per-song release identity or metadata changed");
        }
    }
}
