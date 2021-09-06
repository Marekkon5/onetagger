use std::sync::mpsc::channel;
use threadpool::ThreadPool;

use crate::tagger::{AudioFileInfo, MatchingUtils, TaggerConfig, Track, AudioFileIDs};
use crate::tagger::beatport::Beatport;
use crate::tag::AudioFileFormat;

pub fn run_benchmark() {
    benchmark_track_matching();
    benchmark_beatport(16);
    benchmark_beatport(24);
    benchmark_beatport(1);
}

pub fn benchmark_track_matching() {
    info!("Starting track matching benchmark of 10000 tracks...");

    let info = AudioFileInfo {
        title: Some("Some Random Title".to_string()),
        artists: vec!["Artist".to_string(), "Lyricist".to_string()],
        // Sample deafults
        format: AudioFileFormat::MP3, isrc: None, path: String::new(), duration: None, track_number: None,
        ids: AudioFileIDs::default()
    };
    let tracks = vec![
        Track {
            // Missspelled for fuzzy
            title: "Some randm title".to_string(),
            artists: vec!["lyrici".to_owned(), "artist".to_owned()],
            // Sample values
            ..Default::default()
        }
    ];
    let mut config = TaggerConfig::default();
    config.strictness = 0.5;
    // Start benchmark
    let start = timestamp!();
    for _ in 0..10_000 {
        MatchingUtils::match_track(&info, &tracks, &config, true).unwrap();
    }
    info!("Matched 10000 tracks, took: {}ms", timestamp!() - start);
}

// Beatport's servers are very inconsistent, so test for it here
pub fn benchmark_beatport(threads: usize) {
    info!("Starting Beatport benchmark with {} threads for 50 iterations...", threads);

    let pool = ThreadPool::new(threads);
    let (tx, rx) = channel();
    // Run threadpool
    for _ in 0..50 {
        let tx = tx.clone();
        pool.execute(move || {
            Beatport::new().search("test", 1, 150).ok();
            tx.send(()).ok();
        })
    }
    // Bench
    let start = timestamp!();
    let _ = rx.iter().take(50).collect::<Vec<()>>();
    info!("Beatport benchmark with {} threads and 50 iterations finished in {}ms", threads, timestamp!() - start);
}