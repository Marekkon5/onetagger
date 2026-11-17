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
    assert_eq!(files, &[selected.clone()]);
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
    let statuses = Tagger::tag_album(&[path.clone()], "album", &mut source, &TaggerConfig::default()).unwrap();
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
        Tagger::move_file(failed, &dir.path().join("failed")).unwrap();
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
