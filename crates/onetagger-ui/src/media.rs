//! Seekable media responses with bounded memory and a bounded conversion cache.
use std::collections::{hash_map::DefaultHasher, VecDeque};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use anyhow::Error;
use axum::{body::Body, extract::Query, http::{header, HeaderMap, Method, StatusCode}, response::{IntoResponse, Response}};
use serde::Deserialize;
use tempfile::NamedTempFile;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use onetagger_player::AudioSources;

static STREAMS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(16);
static CONVERSIONS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
type Preview = (String, Arc<NamedTempFile>, u64);
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

async fn preview(path: PathBuf) -> Result<Arc<NamedTempFile>, Error> {
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
        let mut file = tempfile::Builder::new().prefix("onetagger-preview-").suffix(".wav").tempfile()?;
        AudioSources::from_path(&path)?.write_wav(file.as_file_mut())?;
        if crate::library::fingerprint(&path)? != fingerprint { return Err(anyhow!("Audio file changed while preparing its preview")); }
        let bytes = file.as_file().metadata()?.len();
        let file = Arc::new(file);
        let mut cache = PREVIEWS.lock().unwrap();
        cache.push_back((key, file.clone(), bytes));
        while cache.len() > 2 || cache.iter().map(|entry| entry.2).sum::<u64>() > 1024 * 1024 * 1024 {
            cache.pop_front();
        }
        // An active response keeps its file alive even when evicted from the LRU.
        Ok(file)
    }).await?
}

pub(crate) async fn audio(Query(query): Query<AudioQuery>, headers: HeaderMap, method: Method) -> Response {
    match response(query, headers, method).await {
        Ok(response) => response,
        Err(error) => { warn!("Audio response failed: {error}"); (StatusCode::NOT_FOUND, "Audio unavailable").into_response() },
    }
}

async fn response(query: AudioQuery, headers: HeaderMap, method: Method) -> Result<Response, Error> {
    let Some(content_type) = mime(&query.path) else { return Ok(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response()); };
    let preview = if query.transcode { Some(preview(query.path.clone()).await?) } else { None };
    let path = preview.as_ref().map_or(query.path.as_path(), |file| file.path());
    let mut file = tokio::fs::File::open(path).await?;
    let metadata = file.metadata().await?;
    let size = metadata.len();
    let mut hasher = DefaultHasher::new();
    (path, size, metadata.modified()?).hash(&mut hasher);
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
    file.seek(std::io::SeekFrom::Start(start)).await?;
    let permit = STREAMS.acquire().await?;
    let stream = futures_util::stream::try_unfold((file, length, preview, permit),
        |(mut file, remaining, preview, permit)| async move {
            if remaining == 0 { return Ok::<_, std::io::Error>(None); }
            let mut buffer = vec![0; remaining.min(64 * 1024) as usize];
            let read = file.read(&mut buffer).await?;
            if read == 0 { return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof)); }
            buffer.truncate(read);
            Ok(Some((buffer, (file, remaining - read as u64, preview, permit))))
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
}
