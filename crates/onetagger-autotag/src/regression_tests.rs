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

struct CancellingSource {
    stopped: Arc<AtomicBool>,
    during_extension: bool,
}
impl AutotaggerSource for CancellingSource {
    fn match_track(&mut self, _: &AudioFileInfo, _: &TaggerConfig) -> Result<Vec<TrackMatch>, Error> {
        if !self.during_extension { self.stopped.store(true, Ordering::SeqCst); }
        Ok(vec![TrackMatch::new(1.0, Track { title: "Must not be written".into(), ..Default::default() })])
    }
    fn extend_track(&mut self, _: &mut Track, _: &TaggerConfig) -> Result<(), Error> {
        assert!(self.during_extension, "Stop after matching must prevent further provider work");
        self.stopped.store(true, Ordering::SeqCst);
        Ok(())
    }
    fn get_album(&mut self, _: &str, _: &TaggerConfig) -> Result<Option<Album>, Error> {
        self.stopped.store(true, Ordering::SeqCst);
        AlbumSource.get_album("album", &TaggerConfig::default())
    }
}

#[test]
fn stop_during_provider_work_prevents_late_track_and_album_writes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("selected.mp3");
    mp3(&path, Some("Selected"));
    let original = fs::read(&path).unwrap();
    let config = TaggerConfig { tags: vec![SupportedTag::Title], overwrite: true, ..Default::default() };
    for during_extension in [false, true] {
        let stopped = Arc::new(AtomicBool::new(false));
        let mut source = Box::new(CancellingSource { stopped: stopped.clone(), during_extension });
        let result = Tagger::tag_track_with_cancel(&path, &mut source, &config, &|| stopped.load(Ordering::SeqCst));
        assert_eq!(result.status, TaggingState::Skipped);
        assert_eq!(result.message.as_deref(), Some("Stopped before writing tags"));
        assert_eq!(result.message_key.as_deref(), Some("backend.stoppedBeforeWriting"));
        assert_eq!(result.message_args, Some(serde_json::Map::new()));
        assert_eq!(fs::read(&path).unwrap(), original);
    }
    let stopped = Arc::new(AtomicBool::new(false));
    let mut source: Box<dyn AutotaggerSource> = Box::new(CancellingSource { stopped: stopped.clone(), during_extension: false });
    let statuses = Tagger::tag_album_with_cancel(std::slice::from_ref(&path), "album", &mut source, &config,
        &|| stopped.load(Ordering::SeqCst)).unwrap();
    assert!(statuses.is_empty());
    assert_eq!(fs::read(path).unwrap(), original);
}
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

pub(super) fn status(path: &Path, status: TaggingState) -> TaggingStatus {
    TaggingStatus { path: path.into(), status, message: None, message_key: None, message_args: None, accuracy: None,
        used_shazam: false, release_id: None, track_id: None, reason: None }
}

#[test]
fn legacy_tagging_status_round_trips_without_new_presentation_fields() {
    let old = serde_json::json!({
        "status": "error", "path": "library/Музыка/track.mp3", "message": "Provider detail",
        "accuracy": null, "usedShazam": false, "releaseId": null, "trackId": null, "reason": null
    });
    let parsed: TaggingStatus = serde_json::from_value(old.clone()).unwrap();
    assert!(parsed.message_key.is_none());
    assert!(parsed.message_args.is_none());
    assert_eq!(serde_json::to_value(parsed).unwrap(), old);
}

#[test]
fn typed_status_messages_preserve_english_and_named_argument_contracts() {
    let detail = "provider: /Музыка/Ελληνικά/<img onerror=alert(1)> {count}";
    let cases = [
        (TaggingMessage::AlreadyTagged, "backend.alreadyTagged", "Already tagged".to_string()),
        (TaggingMessage::AudioFeaturesAlreadyTagged, "backend.alreadyTagged", "Already tagged!".to_string()),
        (TaggingMessage::NoMatch, "backend.noMatch", "No match!".to_string()),
        (TaggingMessage::StoppedBeforeWriting, "backend.stoppedBeforeWriting", "Stopped before writing tags".to_string()),
        (TaggingMessage::NoAlbumMatch, "backend.noAlbumMatch", "No album match for selected file".to_string()),
        (TaggingMessage::AlbumWriteFailed, "backend.albumWriteFailed", "Failed writing album tags".to_string()),
        (TaggingMessage::ShazamFailed(detail.into()), "backend.shazamFailed", format!("Error Shazaming file: {detail}")),
        (TaggingMessage::FileLoadFailed(detail.into()), "backend.fileLoadFailed", format!("Error loading file: {detail}")),
        (TaggingMessage::TrackMatchFailed(detail.into()), "backend.trackMatchFailed", format!("Error matching track: {detail}")),
        (TaggingMessage::TrackWriteFailed(detail.into()), "backend.trackWriteFailed", format!("Failed writing tags to file: {detail}")),
        (TaggingMessage::AudioFeaturesWriteFailed(detail.into()), "backend.audioFeaturesWriteFailed", format!("Audio features failed writing to tag: {detail}")),
        (TaggingMessage::AudioFeaturesSearchFailed(detail.into()), "backend.audioFeaturesSearchFailed", format!("Audio features search track by ISRC error: {detail}")),
    ];
    let catalogs = [
        include_str!("../../../client/src/locales/en/backend.json"),
        include_str!("../../../client/src/locales/it/backend.json"),
        include_str!("../../../client/src/locales/fr/backend.json"),
        include_str!("../../../client/src/locales/nl/backend.json"),
        include_str!("../../../client/src/locales/de/backend.json"),
        include_str!("../../../client/src/locales/es/backend.json"),
        include_str!("../../../client/src/locales/el/backend.json"),
        include_str!("../../../client/src/locales/ru/backend.json"),
    ];
    for (message, key, english) in cases {
        let mut output = status(Path::new("selected.mp3"), TaggingState::Error);
        let mut legacy = serde_json::to_value(&output).unwrap();
        output.set_message(message);
        assert_eq!(output.message.as_deref(), Some(english.as_str()));
        assert_eq!(output.message_key.as_deref(), Some(key));
        let arguments = output.message_args.as_ref().unwrap();
        if english.ends_with(detail) { assert_eq!(arguments["detail"], detail); }
        else { assert!(arguments.is_empty()); }
        let mut event = serde_json::to_value(&output).unwrap();
        event.as_object_mut().unwrap().remove("messageKey");
        event.as_object_mut().unwrap().remove("messageArgs");
        legacy["message"] = serde_json::json!(english);
        assert_eq!(event, legacy, "message metadata must not alter status, paths or matching data");
        for catalog in catalogs {
            let catalog: serde_json::Value = serde_json::from_str(catalog).unwrap();
            let text = catalog[key.strip_prefix("backend.").unwrap()].as_str().unwrap();
            assert!(!text.trim().is_empty(), "missing translation for {key}");
            for argument in arguments.keys() {
                assert!(text.contains(format!("{{{argument}}}").as_str()), "missing {argument} in {key}");
            }
        }
    }
}

#[test]
fn translation_arguments_do_not_enter_exported_tagging_reports() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("selected.mp3");
    mp3(&path, Some("Selected"));
    let mut report = report::ReportBuilder::new(std::slice::from_ref(&path), None);
    let mut failed = status(&path, TaggingState::Error);
    failed.set_message(TaggingMessage::TrackMatchFailed("secret=DO-NOT-EXPORT".into()));
    report.record("fixture-provider", &failed);
    let json = serde_json::to_string(&report.finish(false)).unwrap();
    assert!(!json.contains("DO-NOT-EXPORT"));
    assert!(!json.contains("messageKey"));
    assert!(!json.contains("messageArgs"));
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
