//! Opt-in, disk-backed evidence for reviewed library imports. Never serialize config.
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use onetagger_tag::{AudioFileFormat, Field, Tag};
use onetagger_tagger::MatchReason;
use crate::{TaggingState, TaggingStatus};

pub const MAX_SELECTION_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectionIdentity {
    pub id: String,
    pub snapshot: String,
    pub snapshot_digest: String,
    pub scope_key: String,
    pub mapping_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionTrack {
    pub track_id: String,
    pub local_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub schema: u32,
    #[serde(flatten)]
    pub identity: SelectionIdentity,
    pub tracks: Vec<SelectionTrack>,
}

impl Selection {
    pub fn parse(data: &str) -> Result<Self> {
        if data.len() > MAX_SELECTION_BYTES { bail!("Selection manifest exceeds 2 MiB"); }
        let selection: Self = serde_json::from_str(data).context("Invalid Helper selection manifest")?;
        selection.validate()?;
        Ok(selection)
    }

    pub fn validate(&self) -> Result<()> {
        if self.schema != 1 || self.tracks.is_empty() || self.tracks.len() > 500 {
            bail!("Choose a schema-1 Helper selection with 1–500 tracks");
        }
        let id = &self.identity;
        if id.id.len() != 32 || !id.id.bytes().all(|b| b.is_ascii_hexdigit())
            || [&id.snapshot, &id.scope_key, &id.mapping_revision, &id.snapshot_digest]
                .iter().any(|s| s.is_empty() || s.len() > 256 || s.chars().any(char::is_control)) {
            bail!("Invalid Helper selection identity");
        }
        let mut seen = BTreeSet::new();
        for row in &self.tracks {
            if row.track_id.is_empty() || row.track_id.len() > 255 || !seen.insert(&row.track_id)
                || row.track_id.chars().any(char::is_control) {
                bail!("Selection track IDs must be distinct and nonempty");
            }
            let path = &row.local_path;
            if !path.is_absolute() || path.to_str().is_none_or(|s| s.chars().any(char::is_control))
                || !path.is_file() || dunce::canonicalize(path)? != *path {
                bail!("A selected file moved or is unavailable; create a new Helper selection");
            }
            let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
            if !["mp3", "flac", "aif", "aiff", "m4a", "wav", "ogg", "opus"].contains(&extension.as_str()) {
                bail!("Unsupported Helper selection audio format");
            }
        }
        Ok(())
    }

    /// Several library IDs may legitimately reference one audio file. Tag it once.
    pub fn files(&self) -> Vec<PathBuf> {
        self.tracks.iter().map(|r| r.local_path.clone()).collect::<BTreeSet<_>>().into_iter().collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileState {
    pub size: u64,
    /// Decimal string: nanosecond timestamps exceed JavaScript's safe integer range.
    pub modified_ns: String,
}

impl FileState {
    fn read(path: &Path) -> Result<Self> {
        let metadata = fs::metadata(path)?;
        if !metadata.is_file() { bail!("Not a regular audio file"); }
        Ok(Self { size: metadata.len(), modified_ns: metadata.modified()?.duration_since(UNIX_EPOCH)?.as_nanos().to_string() })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<i32>,
}

fn read_tags(path: &Path) -> Result<(Tags, FileState)> {
    let state = FileState::read(path)?;
    let wrapper = Tag::load_file(path, true)?;
    let tag = wrapper.tag();
    // Preserve the stored text separator for ID3/MP4; Vorbis primary values are
    // used by the Helper's scalar metadata editor. Do not apply tagger settings.
    let text = |field| tag.get_field(field).and_then(|values| {
        let value = match wrapper.format() {
            AudioFileFormat::FLAC | AudioFileFormat::OGG => values.first().cloned().unwrap_or_default(),
            _ => values.join(", "),
        };
        if value.is_empty() { None } else { Some(value) }
    });
    let date = match wrapper.format() {
        AudioFileFormat::FLAC | AudioFileFormat::OGG => tag.get_raw("DATE"),
        AudioFileFormat::MP4 => tag.get_raw("©day"),
        _ => tag.get_raw("TDRC").or_else(|| tag.get_raw("TYER")),
    };
    let year = date.and_then(|v| v.first().and_then(|s| s.get(..4)).and_then(|s| s.parse().ok()));
    let tags = Tags { title: text(Field::Title), artist: text(Field::Artist), album: text(Field::Album), genre: text(Field::Genre), year };
    if FileState::read(path)? != state { bail!("File changed during tag read-back"); }
    Ok((tags, state))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attempt {
    pub provider: String,
    pub track_id: Option<String>,
    pub release_id: Option<String>,
    pub status: String,
    pub score: Option<f64>,
    pub reason: Option<MatchReason>,
    pub used_shazam: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub track_id: Option<String>,
    pub local_path: PathBuf,
    pub final_path: PathBuf,
    pub before: Option<Tags>,
    pub after: Option<Tags>,
    pub file_state: Option<FileState>,
    pub before_error: Option<String>,
    pub after_error: Option<String>,
    pub status: String,
    pub attempts: Vec<Attempt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub schema: String,
    pub version: u32,
    pub run_id: String,
    pub application_version: String,
    pub application_commit: String,
    pub created_at: String,
    pub completed_at: String,
    pub cancelled: bool,
    pub selection: Option<SelectionIdentity>,
    pub tracks: Vec<Row>,
}

pub struct ReportBuilder {
    report: Report,
    rows: BTreeMap<PathBuf, Row>,
    selection: Option<Selection>,
}

impl ReportBuilder {
    pub fn new(files: &[PathBuf], selection: Option<Selection>) -> Self {
        let rows = files.iter().map(|path| {
            let (before, before_error) = match read_tags(path) {
                Ok((tags, _)) => (Some(tags), None),
                Err(_) => (None, Some("Could not read original audio tags".into())),
            };
            (path.clone(), Row { track_id: None, local_path: path.clone(), final_path: path.clone(), before,
                after: None, file_state: None, before_error, after_error: None, status: "unprocessed".into(), attempts: vec![] })
        }).collect();
        Self { report: Report { schema: "onetagger.tagging-results".into(), version: 1,
            run_id: format!("{:032x}", rand::random::<u128>()),
            application_version: onetagger_shared::VERSION.into(), application_commit: onetagger_shared::COMMIT.into(),
            created_at: Utc::now().to_rfc3339(), completed_at: String::new(), cancelled: false,
            selection: selection.as_ref().map(|s| s.identity.clone()), tracks: vec![] }, rows, selection }
    }

    pub fn record(&mut self, provider: &str, status: &TaggingStatus) {
        if let Some(row) = self.rows.get_mut(&status.path) {
            let outcome = match status.status { TaggingState::Ok => "written", TaggingState::Error => "failed", TaggingState::Skipped => "skipped" };
            if outcome == "written" || (row.status != "written" && (outcome == "failed" || row.status != "failed")) {
                row.status = outcome.into();
            }
            row.attempts.push(Attempt { provider: provider.into(), track_id: status.track_id.clone(),
                release_id: status.release_id.clone(), status: outcome.into(), score: status.accuracy.filter(|s| s.is_finite()),
                reason: status.reason, used_shazam: status.used_shazam });
        }
    }

    pub fn moved(&mut self, original: &Path, destination: &Path) {
        if let Some(row) = self.rows.get_mut(original) { row.final_path = destination.into(); }
    }

    pub fn finish(mut self, cancelled: bool) -> Report {
        for row in self.rows.values_mut() {
            match read_tags(&row.final_path) {
                Ok((tags, state)) => { row.after = Some(tags); row.file_state = Some(state); },
                Err(_) => row.after_error = Some("Could not verify final audio tags".into()),
            }
        }
        self.report.tracks = if let Some(selection) = self.selection {
            selection.tracks.into_iter().filter_map(|selected| self.rows.get(&selected.local_path).map(|r| {
                let mut row = r.clone(); row.track_id = Some(selected.track_id); row
            })).collect()
        } else { self.rows.into_values().collect() };
        self.report.completed_at = Utc::now().to_rfc3339();
        self.report.cancelled = cancelled;
        self.report
    }
}

impl Report {
    pub fn save(&self, folder: &Path) -> Result<PathBuf> {
        fs::create_dir_all(folder)?;
        let path = folder.join(format!("tagging-results-{}.json", self.run_id));
        let data = serde_json::to_vec_pretty(self)?;
        let mut file = OpenOptions::new().write(true).create_new(true).open(&path)?;
        if let Err(error) = file.write_all(&data).and_then(|_| file.sync_all()) {
            drop(file); fs::remove_file(&path).ok(); return Err(error.into());
        }
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regression_tests::status;

    fn audio(path: &Path, title: &str) {
        fs::write(path, [0u8; 64]).unwrap();
        let mut tag = Tag::load_file(path, true).unwrap();
        tag.tag_mut().set_field(Field::Title, vec![title.into()], true);
        tag.tag_mut().set_field(Field::Genre, vec!["House".into()], true);
        tag.tag_mut().save_file(path).unwrap();
    }

    fn selection(path: &Path) -> Selection {
        Selection { schema: 1, identity: SelectionIdentity { id: "a".repeat(32), snapshot: "snapshot".into(),
            snapshot_digest: "b".repeat(64), scope_key: "capture:fixture".into(), mapping_revision: "c".repeat(64) },
            tracks: vec![SelectionTrack { track_id: "library-1".into(), local_path: dunce::canonicalize(path).unwrap() }] }
    }

    #[test]
    fn reports_actual_tags_fallback_evidence_and_preserves_duplicate_library_ids() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("100%20 Café.mp3"); audio(&path, "Before, mix");
        let mut selected = selection(&path);
        selected.tracks.push(SelectionTrack { track_id: "library-2".into(), local_path: selected.tracks[0].local_path.clone() });
        selected.validate().unwrap(); let files = selected.files(); assert_eq!(files.len(), 1);
        let mut report = ReportBuilder::new(&files, Some(selected.clone()));
        let mut failed = status(&files[0], TaggingState::Error);
        failed.message = Some("secret=DO-NOT-EXPORT".into()); report.record("first", &failed);
        audio(&path, "After, mix");
        let mut ok = status(&files[0], TaggingState::Ok); ok.track_id = Some("provider-42".into());
        ok.accuracy = Some(0.91); ok.reason = Some(MatchReason::Fuzzy); report.record("second", &ok);
        let result = report.finish(false);
        assert_eq!(result.selection, Some(selected.identity)); assert_eq!(result.tracks.len(), 2);
        let row = &result.tracks[0]; assert_eq!(row.status, "written"); assert_eq!(row.attempts.len(), 2);
        assert_eq!(row.before.as_ref().unwrap().title.as_deref(), Some("Before, mix"));
        assert_eq!(row.after.as_ref().unwrap().title.as_deref(), Some("After, mix"));
        assert_eq!(row.attempts[1].track_id.as_deref(), Some("provider-42"));
        assert_eq!(row.file_state, Some(FileState::read(&path).unwrap()));
        let saved = result.save(dir.path()).unwrap();
        let json = fs::read_to_string(saved).unwrap(); assert!(!json.contains("DO-NOT-EXPORT"));
        let read: Report = serde_json::from_str(&json).unwrap(); assert_eq!(read.version, 1);
        assert!(result.save(dir.path()).is_err(), "reports must not overwrite previous evidence");
    }

    #[test]
    fn cancellation_failed_readback_and_moves_remain_explicit() {
        let dir = tempfile::tempdir().unwrap(); let path = dir.path().join("track.mp3"); audio(&path, "Original");
        let mut builder = ReportBuilder::new(std::slice::from_ref(&path), None);
        let moved = dir.path().join("moved.mp3"); fs::rename(&path, &moved).unwrap(); builder.moved(&path, &moved);
        let result = builder.finish(true); assert!(result.cancelled);
        assert_eq!(result.tracks[0].status, "unprocessed"); assert_eq!(result.tracks[0].final_path, moved);
        assert!(result.tracks[0].after.is_some());
        let mut builder = ReportBuilder::new(std::slice::from_ref(&moved), None);
        builder.record("fixture", &status(&moved, TaggingState::Ok)); fs::remove_file(&moved).unwrap();
        let result = builder.finish(false); assert!(result.tracks[0].after.is_none());
        assert!(result.tracks[0].after_error.is_some());
    }

    #[test]
    fn rejects_unsupported_manifests_duplicate_ids_and_helper_moves() {
        let dir = tempfile::tempdir().unwrap(); let path = dir.path().join("track.mp3"); audio(&path, "Original");
        let selected = selection(&path); Selection::parse(&serde_json::to_string(&selected).unwrap()).unwrap();
        let mut bad = selected.clone(); bad.schema = 2; assert!(bad.validate().is_err());
        bad = selected.clone(); bad.tracks.push(bad.tracks[0].clone()); assert!(bad.validate().is_err());
        bad = selected.clone(); bad.tracks[0].local_path = PathBuf::from("relative.mp3"); assert!(bad.validate().is_err());
        let config = onetagger_tagger::TaggerConfig { move_success: true, ..Default::default() };
        let finished = std::sync::Arc::new(std::sync::Mutex::new(None));
        assert!(crate::Tagger::tag_files_report(&config, vec![], finished, Some(selected)).is_err());
    }
}
