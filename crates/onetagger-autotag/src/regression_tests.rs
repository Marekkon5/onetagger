use super::*;
use onetagger_tagger::Album;
use std::fs;

fn mp3(path: &Path, title: Option<&str>) {
    fs::write(path, [0u8; 64]).unwrap();
    let mut tag = Tag::load_file(path, true).unwrap();
    if let Some(title) = title {
        tag.tag_mut().set_field(Field::Title, vec![title.into()], true);
    }
    tag.tag_mut().set_field(Field::Artist, vec!["Test artist".into()], true);
    tag.tag_mut().save_file(path).unwrap();
}

struct AlbumSource;
impl AutotaggerSource for AlbumSource {
    fn match_track(&mut self, _: &AudioFileInfo, _: &TaggerConfig) -> Result<Vec<TrackMatch>, Error> {
        unreachable!("Album processing must use the supplied album")
    }
    fn extend_track(&mut self, _: &mut Track, _: &TaggerConfig) -> Result<(), Error> { Ok(()) }
    fn get_album(&mut self, _: &str, _: &TaggerConfig) -> Result<Option<Album>, Error> {
        Ok(Some(Album { id: "album".into(), name: "Fixture".into(), tracks: vec![Track {
            title: "Selected".into(), genres: vec!["Updated genre".into()], ..Default::default()
        }] }))
    }
}

#[test]
fn album_tagging_preserves_unselected_neighbours_and_deduplicates_selection() {
    let dir = tempfile::tempdir().unwrap();
    let selected = dir.path().join("selected.mp3");
    let neighbour = dir.path().join("not-in-playlist.mp3");
    mp3(&selected, Some("Selected"));
    mp3(&neighbour, Some("Neighbour"));
    let untouched = fs::read(&neighbour).unwrap();
    let mut context = AlbumTagContext::new();
    context.init(&[selected.clone(), selected.clone()]);
    let files = &context.folders.get(dir.path()).unwrap().files;
    assert_eq!(files, std::slice::from_ref(&selected));
    let mut source: Box<dyn AutotaggerSource> = Box::new(AlbumSource);
    let config = TaggerConfig { tags: vec![SupportedTag::Genre], ..Default::default() };
    let statuses = Tagger::tag_album(files, "album", &mut source, &config).unwrap();
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].path, selected);
    assert_eq!(statuses[0].status, TaggingState::Ok);
    assert_eq!(Tag::load_file(&selected, false).unwrap().tag().get_field(Field::Genre).unwrap(), vec!["Updated genre"]);
    assert_eq!(fs::read(neighbour).unwrap(), untouched);
}

#[test]
fn selected_file_without_album_match_reports_error_without_panicking_or_writing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("no-title.mp3");
    mp3(&path, None);
    let original = fs::read(&path).unwrap();
    let mut source: Box<dyn AutotaggerSource> = Box::new(AlbumSource);
    let statuses = Tagger::tag_album(std::slice::from_ref(&path), "album", &mut source, &TaggerConfig::default()).unwrap();
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].status, TaggingState::Error);
    assert!(statuses[0].message.as_ref().unwrap().contains("No album match"));
    assert_eq!(fs::read(path).unwrap(), original);
}

fn status(path: &Path, status: TaggingState) -> TaggingStatus {
    TaggingStatus { path: path.into(), status, message: None, accuracy: None,
        used_shazam: false, release_id: None, reason: None }
}

#[test]
fn provider_fallback_success_cannot_enter_failed_moves_or_failed_playlist() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("eventually-successful.mp3");
    fs::write(&path, b"audio").unwrap();
    let mut results = TaggingResults::default();
    for state in [TaggingState::Error, TaggingState::Error, TaggingState::Ok, TaggingState::Error] {
        results.record(&status(&path, state));
    }
    // Exercise the same failed-file move input as the batch finalizer.
    for failed in &results.failed {
        Tagger::move_file(failed, dir.path().join("failed")).unwrap();
    }
    assert_eq!(results.successful.into_iter().collect::<Vec<_>>(), vec![path.clone()]);
    assert!(results.failed.is_empty());
    assert_eq!(fs::read(path).unwrap(), b"audio");
    assert!(!dir.path().join("failed").exists());
}

#[test]
fn repeated_provider_failures_have_one_final_failure_and_skips_stay_separate() {
    let failed = Path::new("failed.mp3");
    let mut results = TaggingResults::default();
    results.record(&status(failed, TaggingState::Error));
    results.record(&status(failed, TaggingState::Error));
    results.record(&status(Path::new("skipped.mp3"), TaggingState::Skipped));
    assert!(results.successful.is_empty());
    assert_eq!(results.failed.into_iter().collect::<Vec<_>>(), vec![failed.to_path_buf()]);
}

fn atom(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut bytes = ((data.len() + 8) as u32).to_be_bytes().to_vec();
    bytes.extend(kind);
    bytes.extend(data);
    bytes
}

fn mp4_with_cover(path: &Path) -> Vec<u8> {
    // Minimal ISO-BMFF metadata container plus a sentinel media payload. The test
    // exercises MP4 tag parsing/writing, not decoding or playback.
    let mut metadata = vec![0; 4];
    metadata.extend(atom(b"ilst", &[]));
    let mut bytes = atom(b"ftyp", b"M4A \0\0\0\0M4A isom");
    // Version-0 movie header (100-byte payload), required by mp4ameta.
    let mut movie_header = vec![0; 100];
    movie_header[12..16].copy_from_slice(&1000u32.to_be_bytes()); // timescale
    movie_header[16..20].copy_from_slice(&1000u32.to_be_bytes()); // duration
    movie_header[20..24].copy_from_slice(&0x00010000u32.to_be_bytes()); // rate
    movie_header[24..26].copy_from_slice(&0x0100u16.to_be_bytes()); // volume
    for (offset, value) in [(36, 0x00010000u32), (52, 0x00010000), (68, 0x40000000)] {
        movie_header[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }
    movie_header[96..100].copy_from_slice(&1u32.to_be_bytes());
    let mut movie = atom(b"mvhd", &movie_header);
    movie.extend(atom(b"udta", &atom(b"meta", &metadata)));
    bytes.extend(atom(b"moov", &movie));
    bytes.extend(atom(b"mdat", b"preserve-media-payload"));
    fs::write(path, bytes).unwrap();
    let old_cover = b"existing artwork bytes".to_vec();
    let mut tag = Tag::load_file(path, false).unwrap();
    tag.tag_mut().set_art(CoverType::CoverFront, "image/jpeg", Some("Original"), old_cover.clone());
    tag.tag_mut().save_file(path).unwrap();
    old_cover
}

fn artwork_response(status: &str, body: Vec<u8>) -> (String, std::thread::JoinHandle<()>) {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/cover", listener.local_addr().unwrap());
    let response = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        let mut request = Vec::new();
        let mut buffer = [0; 1024];
        while !request.ends_with(b"\r\n\r\n") {
            let count = stream.read(&mut buffer).unwrap();
            assert!(count > 0, "Connection closed before the request headers");
            request.extend_from_slice(&buffer[..count]);
            assert!(request.len() <= 16384, "Unexpectedly large fixture request");
        }
        stream.write_all(response.as_bytes()).unwrap();
        stream.write_all(&body).unwrap();
    });
    (url, server)
}

#[test]
fn failed_or_invalid_mp4_artwork_download_preserves_existing_cover() {
    for (http_status, body) in [("404 Not Found", vec![]), ("200 OK", vec![b'x'; 5000])] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("track.m4a");
        let old_cover = mp4_with_cover(&path);
        let (url, server) = artwork_response(http_status, body);
        let track = Track { title: "Updated title".into(), art: Some(url), ..Default::default() };
        let config = TaggerConfig { tags: vec![SupportedTag::Title, SupportedTag::AlbumArt],
            overwrite: true, remove_all_covers: true, ..Default::default() };
        track.write_to_file(&path, &config).unwrap();
        server.join().unwrap();
        let tag = Tag::load_file(&path, false).unwrap();
        assert_eq!(tag.tag().get_art().len(), 1);
        assert_eq!(tag.tag().get_art()[0].data, old_cover);
        assert_eq!(tag.tag().get_field(Field::Title).unwrap(), vec!["Updated title"]);
        assert!(fs::read(path).unwrap().windows(22).any(|w| w == b"preserve-media-payload"));
    }
}

#[test]
fn valid_mp4_artwork_replaces_old_cover_after_validation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("track.m4a");
    let old_cover = mp4_with_cover(&path);
    let mut seed = 0x12345678u32;
    let image = image::RgbImage::from_fn(128, 128, |_, _| {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        image::Rgb([seed as u8, (seed >> 8) as u8, (seed >> 16) as u8])
    });
    let mut body = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image).write_to(&mut body, image::ImageFormat::Png).unwrap();
    assert!(body.get_ref().len() >= 4096);
    let (url, server) = artwork_response("200 OK", body.into_inner());
    let track = Track { art: Some(url), ..Default::default() };
    let config = TaggerConfig { tags: vec![SupportedTag::AlbumArt], overwrite: true, ..Default::default() };
    track.write_to_file(&path, &config).unwrap();
    server.join().unwrap();
    let tag = Tag::load_file(&path, false).unwrap();
    let pictures = tag.tag().get_art();
    assert_eq!(pictures.len(), 1);
    assert_ne!(pictures[0].data, old_cover);
    assert_eq!(image::guess_format(&pictures[0].data).unwrap(), image::ImageFormat::Jpeg);
    assert_eq!(image::load_from_memory(&pictures[0].data).unwrap().width(), 128);
}
