//! Disposable, local metadata cache. Files remain authoritative on every visit.
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use anyhow::Error;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use onetagger_playlist::{get_files_from_playlist_file, UIPlaylist};
use onetagger_shared::Settings;
use onetagger_tag::{TagSeparators, EXTENSIONS};
use crate::quicktag::{QuickTagData, QuickTagFailed, QuickTagFile};

const MAX_CACHE_BYTES: i64 = 128 * 1024 * 1024;
const MAX_ENTRY_BYTES: usize = 512 * 1024;
const MAX_ENTRIES: i64 = 100_000;
const BATCH_FILES: usize = 64;
const BATCH_BYTES: usize = 256 * 1024;

pub(crate) struct LibraryIndex { connection: Connection, enabled: bool }

fn now() -> i64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64 }

pub(crate) fn key(path: &Path) -> Result<String, Error> {
    // Cache aliases once, but restore the caller's exact path in each response.
    let canonical = dunce::canonicalize(path)?;
    Ok(serde_json::to_string(&canonical)?)
}

pub(crate) fn fingerprint(path: &Path) -> Result<String, Error> {
    let metadata = fs::metadata(path)?;
    let modified = match metadata.modified()?.duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("+{}", duration.as_nanos()),
        Err(error) => format!("-{}", error.duration().as_nanos()),
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        return Ok(format!("{}:{modified}:{}:{}:{}:{}", metadata.len(), metadata.dev(), metadata.ino(), metadata.ctime(), metadata.ctime_nsec()));
    }
    #[cfg(not(unix))]
    Ok(format!("{}:{modified}:{:?}", metadata.len(), metadata.created().ok()))
}

impl LibraryIndex {
    pub(crate) fn open_at(path: &Path) -> Result<Self, Error> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_millis(200))?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > 1 { return Err(anyhow!("Library index schema is newer than this application")); }
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;
            CREATE TABLE IF NOT EXISTS metadata (
                path TEXT NOT NULL, separators TEXT NOT NULL, fingerprint TEXT NOT NULL,
                payload TEXT NOT NULL, accessed INTEGER NOT NULL,
                PRIMARY KEY(path, separators));
            CREATE INDEX IF NOT EXISTS metadata_accessed ON metadata(accessed);
            CREATE TABLE IF NOT EXISTS waveforms (path TEXT PRIMARY KEY, fingerprint TEXT NOT NULL, peaks TEXT NOT NULL);
            PRAGMA user_version=1;")?;
        Ok(Self { connection, enabled: true })
    }

    fn open() -> Result<Self, Error> { Self::open_at(&Settings::get_folder()?.join("library-index-v1.sqlite")) }

    fn disable(&mut self, error: impl std::fmt::Display) {
        if self.enabled { warn!("Library index unavailable; reading music files directly: {error}"); }
        self.enabled = false;
    }

    pub(crate) fn load(&mut self, path: &Path, separators: &TagSeparators) -> Result<(QuickTagFile, bool), Error> {
        let cache_key = key(path).ok();
        let before = fingerprint(path)?;
        let separator_key = serde_json::to_string(separators)?;
        if self.enabled {
            if let Some(key) = &cache_key {
                let payload: rusqlite::Result<Option<String>> = self.connection.query_row(
                    "SELECT payload FROM metadata WHERE path=?1 AND separators=?2 AND fingerprint=?3",
                    params![key, separator_key, before], |row| row.get(0)).optional();
                match payload {
                    Ok(Some(payload)) => if let Ok(mut file) = serde_json::from_str::<QuickTagFile>(&payload) {
                        // A concurrent write must not turn a cache hit into stale metadata.
                        if fingerprint(path).as_ref().ok() == Some(&before) {
                            file.path = path.into();
                            // Avoid a SQLite write on every warm visit.
                            if let Err(error) = self.connection.execute(
                                "UPDATE metadata SET accessed=?1 WHERE path=?2 AND separators=?3 AND accessed<?4",
                                params![now(), key, separator_key, now() - 86400]) { self.disable(error); }
                            return Ok((file, true));
                        }
                    },
                    Ok(None) => {},
                    Err(error) => self.disable(error),
                }
            }
        }
        let file = QuickTagFile::from_path(path, separators)?;
        if self.enabled && fingerprint(path).as_ref().ok() == Some(&before) {
            if let Some(key) = cache_key {
                let payload = serde_json::to_string(&file)?;
                if payload.len() <= MAX_ENTRY_BYTES {
                    if let Err(error) = self.connection.execute(
                        "INSERT INTO metadata(path,separators,fingerprint,payload,accessed) VALUES(?1,?2,?3,?4,?5)
                         ON CONFLICT(path,separators) DO UPDATE SET fingerprint=excluded.fingerprint,
                         payload=excluded.payload, accessed=excluded.accessed",
                        params![key, separator_key, before, payload, now()]) { self.disable(error); }
                }
            }
        }
        Ok((file, false))
    }

    pub(crate) fn trim(&mut self) {
        if !self.enabled { return; }
        let result = (|| -> rusqlite::Result<()> {
            self.connection.execute("DELETE FROM metadata WHERE accessed<?1", [now() - 90 * 86400])?;
            loop {
                let (count, bytes): (i64, i64) = self.connection.query_row(
                    "SELECT count(*),coalesce(sum(length(CAST(payload AS BLOB))),0) FROM metadata", [],
                    |row| Ok((row.get(0)?, row.get(1)?)))?;
                if count <= MAX_ENTRIES && bytes <= MAX_CACHE_BYTES { break; }
                self.connection.execute("DELETE FROM metadata WHERE rowid IN
                    (SELECT rowid FROM metadata ORDER BY accessed,rowid LIMIT 256)", [])?;
            }
            Ok(())
        })();
        if let Err(error) = result { self.disable(error); }
    }
}

pub(crate) fn invalidate(path: &Path) {
    let result = (|| -> Result<(), Error> {
        let database = Settings::get_folder()?.join("library-index-v1.sqlite");
        if !database.exists() { return Ok(()); }
        let Some(key) = key(path).ok() else { return Ok(()); };
        let index = LibraryIndex::open_at(&database)?;
        index.connection.execute("DELETE FROM metadata WHERE path=?1", [&key])?;
        index.connection.execute("DELETE FROM waveforms WHERE path=?1", [&key])?;
        Ok(())
    })();
    if let Err(error) = result { warn!("Cannot invalidate library index entry: {error}"); }
}

pub(crate) fn waveform(path: &Path, cancelled: &AtomicBool) -> Result<Vec<f32>, Error> {
    if cancelled.load(Ordering::Relaxed) { return Err(anyhow!("Waveform cancelled")); }
    let fingerprint = fingerprint(path)?;
    let key = key(path)?;
    let index = LibraryIndex::open().ok();
    if let Some(index) = &index {
        let cached: Option<String> = index.connection.query_row(
            "SELECT peaks FROM waveforms WHERE path=?1 AND fingerprint=?2", params![key, fingerprint],
            |row| row.get(0)).optional().ok().flatten();
        if let Some(peaks) = cached.and_then(|value| serde_json::from_str::<Vec<f32>>(&value).ok())
            .filter(|peaks| peaks.len() == 180 && peaks.iter().all(|value| value.is_finite() && (0.0..=1.0).contains(value))) {
            return Ok(peaks);
        }
    }
    let peaks = onetagger_player::AudioSources::from_path(path)?.waveform(180, &|| cancelled.load(Ordering::Relaxed))?;
    if cancelled.load(Ordering::Relaxed) { return Err(anyhow!("Waveform cancelled")); }
    if crate::library::fingerprint(path)? == fingerprint {
        if let Some(index) = index {
            let store = index.connection.execute(
                "INSERT INTO waveforms(path,fingerprint,peaks) VALUES(?1,?2,?3)
                 ON CONFLICT(path) DO UPDATE SET fingerprint=excluded.fingerprint,peaks=excluded.peaks",
                params![key, fingerprint, serde_json::to_string(&peaks)?]);
            if let Err(error) = store { warn!("Cannot cache waveform: {error}"); }
            index.connection.execute("DELETE FROM waveforms WHERE rowid IN
                (SELECT rowid FROM waveforms ORDER BY rowid DESC LIMIT -1 OFFSET 10000)", []).ok();
        }
    }
    Ok(peaks)
}

fn audio(path: &Path) -> bool {
    path.extension().is_some_and(|extension| EXTENSIONS.iter().any(|known| extension.to_ascii_lowercase() == *known))
}

pub(crate) fn inputs(path: Option<String>, playlist: Option<UIPlaylist>, recursive: bool)
    -> Result<Box<dyn Iterator<Item = PathBuf> + Send>, Error> {
    if let Some(path) = path {
        let path = PathBuf::from(path);
        if !path.is_dir() {
            let files = if audio(&path) { vec![path] } else { get_files_from_playlist_file(path)? };
            return Ok(Box::new(files.into_iter().filter(|path| audio(path))));
        }
        // Validate the root now; an unreadable root must not silently look empty.
        fs::read_dir(&path)?;
        let depth = if recursive { usize::MAX } else { 1 };
        return Ok(Box::new(walkdir::WalkDir::new(path).max_depth(depth).into_iter()
            .filter_map(|entry| entry.ok()).filter(|entry| audio(entry.path()) && entry.path().is_file())
            .map(|entry| entry.into_path())));
    }
    let files = playlist.map(|playlist| playlist.get_files()).transpose()?.unwrap_or_default();
    Ok(Box::new(files.into_iter().filter(|path| audio(path))))
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanStats {
    pub scanned: usize, pub cached: usize, pub parsed: usize, pub failed: usize,
    pub limited: bool, pub cancelled: bool, pub elapsed_ms: u128,
}

struct ScanIndex(Option<LibraryIndex>);
impl Drop for ScanIndex {
    fn drop(&mut self) { if let Some(index) = &mut self.0 { index.trim(); } }
}

pub(crate) fn scan(files: impl Iterator<Item = PathBuf>, separators: &TagSeparators,
    limit: usize, cancelled: &AtomicBool, mut emit: impl FnMut(QuickTagData) -> Result<(), Error>) -> Result<ScanStats, Error> {
    let started = Instant::now();
    let mut index = ScanIndex(match LibraryIndex::open() {
        Ok(index) => Some(index),
        Err(error) => { warn!("Library index unavailable; scanning without cache: {error}"); None },
    });
    let mut stats = ScanStats::default();
    let mut batch = QuickTagData::default();
    let mut bytes = 0;
    let mut last_emit = Instant::now();
    for path in files {
        if cancelled.load(Ordering::Relaxed) { stats.cancelled = true; break; }
        if stats.scanned >= limit { stats.limited = true; break; }
        stats.scanned += 1;
        let result = match &mut index.0 {
            Some(index) => index.load(&path, separators),
            None => QuickTagFile::from_path(&path, separators).map(|file| (file, false)),
        };
        match result {
            Ok((file, cached)) => {
                if cached { stats.cached += 1; } else { stats.parsed += 1; }
                bytes += serde_json::to_vec(&file)?.len();
                batch.files.push(file);
            },
            Err(error) => { stats.failed += 1; batch.failed.push(QuickTagFailed::new(path, error.to_string())); },
        }
        if batch.files.len() + batch.failed.len() >= BATCH_FILES || bytes >= BATCH_BYTES || last_emit.elapsed() >= Duration::from_millis(50) {
            emit(std::mem::take(&mut batch))?;
            last_emit = Instant::now(); bytes = 0;
        }
    }
    if !batch.files.is_empty() || !batch.failed.is_empty() { emit(batch)?; }
    stats.elapsed_ms = started.elapsed().as_millis();
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use onetagger_tag::{Field, Tag};

    fn track(path: &Path, title: &str) {
        if !path.exists() { fs::write(path, [0u8; 64]).unwrap(); }
        let mut tag = Tag::load_file(path, true).unwrap();
        tag.tag_mut().set_field(Field::Title, vec![title.into()], true);
        tag.tag_mut().set_field(Field::Artist, vec!["Fixture artist".into()], true);
        tag.tag_mut().save_file(path).unwrap();
    }

    #[test]
    fn unchanged_files_are_cached_and_changed_files_are_reparsed() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("track.mp3");
        track(&path, "First title");
        let mut index = LibraryIndex::open_at(&directory.path().join("index.sqlite")).unwrap();
        let separators = TagSeparators::default();
        assert!(!index.load(&path, &separators).unwrap().1);
        assert!(index.load(&path, &separators).unwrap().1);
        track(&path, "A changed and longer title");
        let (file, cached) = index.load(&path, &separators).unwrap();
        assert!(!cached);
        assert_eq!(serde_json::to_value(file).unwrap()["title"], "A changed and longer title");
        assert!(index.load(&path, &separators).unwrap().1);
        fs::remove_file(path.clone()).unwrap();
        assert!(index.load(&path, &separators).is_err());
    }

    #[test]
    fn malformed_cached_metadata_is_rebuilt_and_future_schemas_are_ignored() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("track.mp3");
        track(&path, "Real title");
        let database = directory.path().join("index.sqlite");
        let mut index = LibraryIndex::open_at(&database).unwrap();
        let separators = TagSeparators::default();
        index.load(&path, &separators).unwrap();
        index.connection.execute("UPDATE metadata SET payload='broken JSON'", []).unwrap();
        assert!(!index.load(&path, &separators).unwrap().1);
        assert!(index.load(&path, &separators).unwrap().1);
        index.connection.pragma_update(None, "user_version", 99).unwrap();
        assert!(LibraryIndex::open_at(&database).is_err());
    }

    #[test]
    fn limited_scans_do_not_enumerate_the_whole_library_and_cancel_before_reading() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("track.mp3");
        track(&path, "Fixture");
        let mut yielded = 0;
        let files = std::iter::repeat_with(|| { yielded += 1; path.clone() }).take(10_000);
        let stats = scan(files, &TagSeparators::default(), 2, &AtomicBool::new(false), |_| Ok(())).unwrap();
        assert_eq!(stats.scanned, 2);
        assert!(stats.limited);
        assert_eq!(yielded, 3);
        let stats = scan(std::iter::once(path), &TagSeparators::default(), 500,
            &AtomicBool::new(true), |_| panic!("Cancelled scan must not emit files")).unwrap();
        assert!(stats.cancelled);
        assert_eq!(stats.scanned, 0);
    }
    #[test]
    #[ignore = "Performance evidence; invoked explicitly on the GitHub Linux runner"]
    fn library_benchmark() {
        let directory = tempfile::tempdir().unwrap();
        let files: Vec<_> = (0..1000).map(|index| {
            let path = directory.path().join(format!("track-{index}.mp3"));
            track(&path, &format!("Fixture {index}"));
            let mut tag = Tag::load_file(&path, false).unwrap();
            for field in 0..32 { tag.tag_mut().set_raw(&format!("FIXTURE_{field}"), vec![format!("Metadata value {index} {field}")], true); }
            tag.tag_mut().save_file(&path).unwrap();
            path
        }).collect();
        let mut index = LibraryIndex::open_at(&directory.path().join("benchmark.sqlite")).unwrap();
        let separators = TagSeparators::default();
        let mut cold = Vec::new(); let mut warm = Vec::new(); let mut direct = Vec::new();
        for _ in 0..5 {
            let start = Instant::now();
            for path in &files { QuickTagFile::from_path(path, &separators).unwrap(); }
            direct.push(start.elapsed().as_micros());
            index.connection.execute("DELETE FROM metadata", []).unwrap();
            let start = Instant::now();
            for path in &files { assert!(!index.load(path, &separators).unwrap().1); }
            cold.push(start.elapsed().as_micros());
            let start = Instant::now();
            for path in &files { assert!(index.load(path, &separators).unwrap().1); }
            warm.push(start.elapsed().as_micros());
        }
        track(&files[17], "Changed benchmark title");
        let parsed = files.iter().filter(|path| !index.load(path, &separators).unwrap().1).count();
        assert_eq!(parsed, 1);
        let report = serde_json::json!({"files": files.len(), "samples": 5,
            "directUs": direct, "coldUs": cold, "warmUs": warm, "parsedAfterOneChange": parsed,
            "scope": "SQLite metadata reads with real synthetic ID3 tags, warm OS filesystem cache; not provider or rendering throughput"});
        println!("LIBRARY_BENCHMARK={report}");
        if let Ok(path) = std::env::var("LIBRARY_BENCHMARK_OUTPUT") { fs::write(path, format!("{report}\n")).unwrap(); }
    }

}
