//! CI-only offline provider; exercises the production batch, writers and report.
//! Run from Helper's packaging/check_onetagger_roundtrip.py, never in a release.
use std::fs;
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use anyhow::{anyhow, Error};
use onetagger_autotag::{Tagger, STOP_TAGGING, AUTOTAGGER_PLATFORMS};
use onetagger_autotag::platforms::{AutotaggerPlatform, AutotaggerPlatformInfo};
use onetagger_autotag::report::{Report, Selection};
use onetagger_tagger::{AudioFileInfo, AutotaggerSource, AutotaggerSourceBuilder,
    PlatformInfo, SupportedTag, TaggerConfig, Track, TrackMatch};

struct OfflineBuilder { id: &'static str }
struct OfflineSource { fail: bool }

impl AutotaggerSourceBuilder for OfflineBuilder {
    fn new() -> Self { Self { id: "offline-success" } }
    fn get_source(&mut self, _: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Error> {
        assert_ne!(self.id, "must-not-run", "Successful files reached a later provider");
        Ok(Box::new(OfflineSource { fail: self.id == "offline-miss" }))
    }
    fn info(&self) -> PlatformInfo {
        PlatformInfo { id: self.id.into(), name: self.id.into(), description: "Offline CI fixture".into(),
            version: "1.0.0".into(), icon: &[], max_threads: 1, custom_options: Default::default(),
            supported_tags: vec![SupportedTag::Title, SupportedTag::Genre, SupportedTag::Label], requires_auth: false }
    }
}
impl AutotaggerSource for OfflineSource {
    fn match_track(&mut self, _: &AudioFileInfo, _: &TaggerConfig) -> Result<Vec<TrackMatch>, Error> {
        if std::env::var("ONETAGGER_ROUNDTRIP_CANCEL").as_deref() == Ok("1") {
            STOP_TAGGING.store(true, Ordering::SeqCst);
        }
        if self.fail { return Err(anyhow!("Synthetic offline miss")); }
        Ok(vec![TrackMatch::new(0.91, Track { title: "Tagged Café 日本".into(),
            artists: vec!["Artist".into()], genres: vec!["Techno".into()], label: Some("Independent label".into()),
            track_id: Some("offline-42".into()), release_id: Some("offline-album".into()), ..Default::default() })])
    }
    fn extend_track(&mut self, _: &mut Track, _: &TaggerConfig) -> Result<(), Error> { Ok(()) }
}

#[test]
#[ignore = "Requires a real Helper selection and output path supplied by cross-repository CI"]
fn produce_helper_report() {
    let input = std::env::var("ONETAGGER_ROUNDTRIP_SELECTION").unwrap();
    let output = std::env::var("ONETAGGER_ROUNDTRIP_REPORT").unwrap();
    let selection = Selection::parse(&fs::read_to_string(input).unwrap()).unwrap();
    let count = selection.tracks.len();
    {
        let mut registry = AUTOTAGGER_PLATFORMS.lock().unwrap();
        registry.platforms.clear();
        for id in ["offline-miss", "offline-success", "must-not-run"] {
            let builder = OfflineBuilder { id }; let info = builder.info();
            registry.platforms.push(AutotaggerPlatform { info: AutotaggerPlatformInfo {
                built_in: true, icon: String::new(), requires_auth: false,
                supported_tags: info.supported_tags.clone(), platform: info }, platform: Box::new(builder) });
        }
    }
    let config = TaggerConfig { platforms: vec!["offline-miss".into(), "offline-success".into(), "must-not-run".into()],
        tags: vec![SupportedTag::Title, SupportedTag::Genre, SupportedTag::Label], overwrite: true,
        threads: 1, match_duration: false, enable_shazam: false, skip_tagged: false, album_tagging: false,
        ..Default::default() };
    let finished = Arc::new(Mutex::new(None));
    let receiver = Tagger::tag_files_report(&config, vec![], finished.clone(), Some(selection)).unwrap();
    for _ in receiver {}
    let result = finished.lock().unwrap(); let result = result.as_ref().unwrap();
    assert!(result.report_error.is_none(), "{:?}", result.report_error);
    let bytes = fs::read(result.report_file.as_ref().unwrap()).unwrap();
    let report: Report = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(report.tracks.len(), count);
    if std::env::var("ONETAGGER_ROUNDTRIP_CANCEL").as_deref() == Ok("1") {
        assert!(report.cancelled);
        assert!(report.tracks.iter().all(|row| row.status != "written"));
    } else {
        assert!(!report.cancelled);
        for row in &report.tracks {
            assert_eq!(row.status, "written");
            assert_eq!(row.attempts.len(), 2);
            assert_eq!(row.attempts[0].status, "failed");
            assert_eq!(row.attempts[1].status, "written");
        }
    }
    fs::write(output, bytes).unwrap();
}
