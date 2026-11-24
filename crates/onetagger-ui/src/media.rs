//! Seekable media responses with bounded memory and a bounded conversion cache.
use std::collections::{hash_map::DefaultHasher, VecDeque};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use anyhow::Error;
use axum::{body::Body, extract::Query, http::{header, HeaderMap, Method, StatusCode}, response::{IntoResponse, Response}};
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use onetagger_player::AudioSources;

static STREAMS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(16);
static CONVERSIONS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
struct PreviewFile { file: Mutex<std::fs::File>, bytes: u64, key: String }
type Preview = (String, Arc<PreviewFile>, u64);
static PREVIEWS: LazyLock<Mutex<VecDeque<Preview>>> = LazyLock::new(|| Mutex::new(VecDeque::new()));

#[derive(Deserialize)]
pub(crate) struct AudioQuery { path: PathBuf, #[serde(default)] transcode: bool }

fn mime(path: &std::path::Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "mp3" => Some("audio/mpeg"), "m4a" | "mp4" => Some("audio/mp4"),
        "wav" => Some("audio/wav"), "flac" => Some("audio/flac"),
        "ogg" | "oga" | "opus" | "spx" => Some("audio/ogg"),
        "aif" | "aiff" => Some("audio/aiff"), _ => None,
    }
}

/// Inclusive byte bounds; reject unsupported multipart and invalid ranges.
fn range(value: Option<&str>, size: u64) -> Result<Option<(u64, u64)>, ()> {
    let Some(value) = value else { return Ok(None); };
    let value = value.trim().strip_prefix("bytes=").ok_or(())?;
    if value.contains(',') || size == 0 { return Err(()); }
    let (start, end) = value.split_once('-').ok_or(())?;
    if start.is_empty() {
        let suffix = end.parse::<u64>().map_err(|_| ())?;
        if suffix == 0 { return Err(()); }
        return Ok(Some((size.saturating_sub(suffix), size - 1)));
    }
    let start = start.parse::<u64>().map_err(|_| ())?;
    let end = if end.is_empty() { size - 1 } else { end.parse::<u64>().map_err(|_| ())?.min(size - 1) };
    if start >= size || start > end { return Err(()); }
    Ok(Some((start, end)))
}

async fn preview(path: PathBuf) -> Result<Arc<PreviewFile>, Error> {
    // Serialize conversion and recheck after admission: simultaneous browser
    // range requests for one unsupported codec must share a single preview.
    let permit = CONVERSIONS.acquire().await?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let fingerprint = crate::library::fingerprint(&path)?;
        let key = format!("{}:{fingerprint}", crate::library::key(&path)?);
        {
            let mut cache = PREVIEWS.lock().unwrap();
            if let Some(index) = cache.iter().position(|entry| entry.0 == key) {
                let entry = cache.remove(index).unwrap();
                let file = entry.1.clone(); cache.push_back(entry); return Ok(file);
            }
        }
        // Anonymous temporary files are removed by the OS even when the app
        // exits without running Rust destructors (including its Exit command).
        let mut file = tempfile::tempfile()?;
        AudioSources::from_path(&path)?.write_wav(&mut file)?;
        if crate::library::fingerprint(&path)? != fingerprint { return Err(anyhow!("Audio file changed while preparing its preview")); }
        let bytes = file.metadata()?.len();
        let file = Arc::new(PreviewFile { file: Mutex::new(file), bytes, key: key.clone() });
        let mut cache = PREVIEWS.lock().unwrap();
        cache.push_back((key, file.clone(), bytes));
        while cache.len() > 2 || cache.iter().map(|entry| entry.2).sum::<u64>() > 1024 * 1024 * 1024 {
            cache.pop_front();
        }
        // An active response keeps its file alive even when evicted from the LRU.
        Ok(file)
    }).await?
}

enum MediaReader {
    Original(tokio::fs::File),
    Preview { file: Arc<PreviewFile>, offset: u64 },
}
impl MediaReader {
    async fn position(&mut self, position: u64) -> Result<(), std::io::Error> {
        match self {
            Self::Original(file) => { file.seek(std::io::SeekFrom::Start(position)).await?; },
            Self::Preview { offset, .. } => { *offset = position; },
        }
        Ok(())
    }
    async fn chunk(&mut self, bytes: usize) -> Result<Vec<u8>, std::io::Error> {
        match self {
            Self::Original(file) => {
                let mut buffer = vec![0; bytes];
                let count = file.read(&mut buffer).await?; buffer.truncate(count); Ok(buffer)
            },
            Self::Preview { file, offset } => {
                let file = file.clone(); let position = *offset;
                let buffer = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, std::io::Error> {
                    // Independent logical cursors allow concurrent Range requests
                    // to share an anonymous file on Windows, macOS and Linux.
                    let mut file = file.file.lock().map_err(|_| std::io::Error::other("Preview lock poisoned"))?;
                    std::io::Seek::seek(&mut *file, std::io::SeekFrom::Start(position))?;
                    let mut buffer = vec![0; bytes];
                    let count = std::io::Read::read(&mut *file, &mut buffer)?;
                    buffer.truncate(count); Ok(buffer)
                }).await.map_err(std::io::Error::other)??;
                *offset += buffer.len() as u64; Ok(buffer)
            },
        }
    }
}

pub(crate) async fn audio(Query(query): Query<AudioQuery>, headers: HeaderMap, method: Method) -> Response {
    match response(query, headers, method).await {
        Ok(response) => response,
        Err(error) => { warn!("Audio response failed: {error}"); (StatusCode::NOT_FOUND, "Audio unavailable").into_response() },
    }
}

async fn response(query: AudioQuery, headers: HeaderMap, method: Method) -> Result<Response, Error> {
    let Some(content_type) = mime(&query.path) else { return Ok(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response()); };
    let (mut reader, size, identity) = if query.transcode {
        let preview = preview(query.path.clone()).await?;
        let size = preview.bytes; let identity = preview.key.clone();
        (MediaReader::Preview { file: preview, offset: 0 }, size, identity)
    } else {
        let file = tokio::fs::File::open(&query.path).await?;
        let metadata = file.metadata().await?;
        let identity = format!("{:?}:{:?}", query.path, metadata.modified()?);
        (MediaReader::Original(file), metadata.len(), identity)
    };
    let mut hasher = DefaultHasher::new();
    (identity, size).hash(&mut hasher);
    let etag = format!("\"{:016x}\"", hasher.finish());
    let requested = headers.get(header::RANGE).and_then(|value| value.to_str().ok());
    let requested = if headers.get(header::IF_RANGE).is_some_and(|value| value.as_bytes() != etag.as_bytes()) { None } else { requested };
    let bounds = match range(requested, size) {
        Ok(bounds) => bounds,
        Err(_) => return Ok((StatusCode::RANGE_NOT_SATISFIABLE,
            [(header::CONTENT_RANGE, format!("bytes */{size}")), (header::ACCEPT_RANGES, "bytes".into())]).into_response()),
    };
    let (start, length) = bounds.map_or((0, size), |(start, end)| (start, end - start + 1));
    let mut builder = Response::builder().status(if bounds.is_some() { StatusCode::PARTIAL_CONTENT } else { StatusCode::OK })
        .header(header::CONTENT_TYPE, if query.transcode { "audio/wav" } else { content_type })
        .header(header::CONTENT_LENGTH, length).header(header::ACCEPT_RANGES, "bytes").header(header::ETAG, etag);
    if let Some((start, end)) = bounds { builder = builder.header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}")); }
    if method == Method::HEAD { return Ok(builder.body(Body::empty())?); }
    reader.position(start).await?;
    let permit = STREAMS.acquire().await?;
    let stream = futures_util::stream::try_unfold((reader, length, permit),
        |(mut reader, remaining, permit)| async move {
            if remaining == 0 { return Ok::<_, std::io::Error>(None); }
            let buffer = reader.chunk(remaining.min(64 * 1024) as usize).await?;
            if buffer.is_empty() { return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof)); }
            let remaining = remaining - buffer.len() as u64;
            Ok(Some((buffer, (reader, remaining, permit))))
        });
    Ok(builder.body(Body::from_stream(stream))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_cover_open_ended_suffix_clamped_and_invalid_requests() {
        assert_eq!(range(None, 100), Ok(None));
        assert_eq!(range(Some("bytes=20-39"), 100), Ok(Some((20, 39))));
        assert_eq!(range(Some("bytes=20-"), 100), Ok(Some((20, 99))));
        assert_eq!(range(Some("bytes=-20"), 100), Ok(Some((80, 99))));
        assert_eq!(range(Some("bytes=20-999"), 100), Ok(Some((20, 99))));
        for value in ["bytes=100-", "bytes=9-2", "bytes=-0", "bytes=0-1,3-4", "broken"] {
            assert_eq!(range(Some(value), 100), Err(()), "{value}");
        }
    }

    #[tokio::test]
    async fn actual_response_streams_ranges_and_head_without_transcoding() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("track.wav");
        std::fs::write(&path, (0u8..100).collect::<Vec<_>>()).unwrap();
        let mut headers = HeaderMap::new(); headers.insert(header::RANGE, "bytes=20-29".parse().unwrap());
        let reply = response(AudioQuery {path: path.clone(), transcode: false}, headers, Method::GET).await.unwrap();
        assert_eq!(reply.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(reply.headers()[header::CONTENT_RANGE], "bytes 20-29/100");
        assert_eq!(axum::body::to_bytes(reply.into_body(), 100).await.unwrap().as_ref(), &(20u8..30).collect::<Vec<_>>());
        let reply = response(AudioQuery {path, transcode: false}, HeaderMap::new(), Method::HEAD).await.unwrap();
        assert_eq!(reply.headers()[header::CONTENT_LENGTH], "100");
        assert!(axum::body::to_bytes(reply.into_body(), 100).await.unwrap().is_empty());
    }
    #[tokio::test]
    async fn converted_previews_share_a_file_but_keep_range_cursors_independent() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../onetagger-player/test-fixtures/tone.aiff");
        let first = preview(path.clone()).await.unwrap();
        let second = preview(path.clone()).await.unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        let request = |range: &'static str| {
            let mut headers = HeaderMap::new(); headers.insert(header::RANGE, range.parse().unwrap());
            response(AudioQuery { path: path.clone(), transcode: true }, headers, Method::GET)
        };
        let (first, second) = tokio::join!(request("bytes=0-3"), request("bytes=8-11"));
        let first = axum::body::to_bytes(first.unwrap().into_body(), 4).await.unwrap();
        let second = axum::body::to_bytes(second.unwrap().into_body(), 4).await.unwrap();
        assert_eq!(first.as_ref(), b"RIFF"); assert_eq!(second.as_ref(), b"WAVE");
    }

}
