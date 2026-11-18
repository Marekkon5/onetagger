use super::*;
use crate::scheduler::{self, Attempt, Schedule};

type SourceSlot = Mutex<Option<Result<Box<dyn AutotaggerSource>, String>>>;

struct Provider {
    id: String,
    name: String,
    limit: usize,
    sources: Vec<SourceSlot>,
}

pub(super) fn deduplicate_files(files: &mut Vec<PathBuf>) {
    let mut seen = BTreeSet::new();
    files.retain(|path| seen.insert(dunce::canonicalize(path).unwrap_or_else(|_| path.clone())));
}

fn failed(path: &Path, message: String) -> TaggingStatus {
    TaggingStatus { status: TaggingState::Error, path: path.into(), message: Some(message),
        accuracy: None, used_shazam: false, release_id: None, track_id: None, reason: None }
}

impl Provider {
    fn tag(&self, slot: usize, path: &Path, config: &TaggerConfig,
        builders: &Mutex<&mut AutotaggerPlatforms>) -> TaggingStatus {
        let mut error = String::new();
        // A failed source constructor must not send a track to a lower-priority
        // provider while this provider still has healthy source instances.
        for offset in 0..self.sources.len() {
            match self.tag_with_slot((slot + offset) % self.sources.len(), path, config, builders) {
                Ok(status) => return status,
                Err(message) => error = message,
            }
        }
        failed(path, error)
    }

    fn tag_with_slot(&self, slot: usize, path: &Path, config: &TaggerConfig,
        builders: &Mutex<&mut AutotaggerPlatforms>) -> Result<TaggingStatus, String> {
        let mut stopped = failed(path, String::new());
        if Tagger::mark_cancelled(&mut stopped, &|| STOP_TAGGING.load(Ordering::SeqCst)) { return Ok(stopped); }
        let mut source = match self.sources[slot].lock() {
            Ok(source) => source,
            Err(_) => return Err("Provider worker panicked; source disabled for this run".into()),
        };
        if source.is_none() {
            // Sources are constructed only on demand, then reused for the run.
            // Only source construction uses this mutex; matching/writing happens
            // outside it, so different providers can run concurrently.
            let created = {
                let mut platforms = builders.lock().unwrap();
                if Tagger::mark_cancelled(&mut stopped, &|| STOP_TAGGING.load(Ordering::SeqCst)) { return Ok(stopped); }
                match platforms.get_builder(&self.id) {
                    Some(builder) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(||
                        builder.get_source(config)
                    )).map_err(|_| "Provider initialization panicked".to_string())
                        .and_then(|r| r.map_err(|e| format!("Could not initialize provider: {e}"))),
                    None => Err(format!("Provider unavailable: {}", self.id)),
                }
            };
            *source = Some(created);
        }
        match source.as_mut().unwrap() {
            Ok(source) => Ok(Tagger::tag_track(path, source, config)),
            Err(message) => Err(message.clone()),
        }
    }
}

impl Tagger {
    pub(super) fn tag_pipeline(files: &[PathBuf], config: &TaggerConfig,
        tx: &Sender<TaggingStatusWrap>, results: &mut TaggingResults,
        report: &mut Option<report::ReportBuilder>) {
        if files.is_empty() { return; }
        let threads = usize::from(config.threads).max(1).min(files.len());
        // Keep custom plugin libraries/builders alive until every source drops.
        // As with the batch path, reloads wait for tagging to finish. A scoped
        // inner mutex permits lazy construction without serializing lookups.
        let mut platforms = AUTOTAGGER_PLATFORMS.lock().unwrap();
        let providers: Vec<Provider> = {
            let mut seen = BTreeSet::new();
            config.platforms.iter().filter(|id| seen.insert((*id).clone())).filter_map(|id| {
                let builder = match platforms.get_builder(id) {
                    Some(builder) => builder,
                    None => { error!("Invalid platform: {id}"); return None; }
                };
                let info = builder.info();
                let limit = if info.max_threads == 0 { threads }
                    else { usize::from(info.max_threads).min(threads) };
                Some(Provider { id: id.clone(), name: info.name, limit,
                    sources: (0..limit).map(|_| Mutex::new(None)).collect() })
            }).collect()
        };
        let builders = Mutex::new(&mut *platforms);
        let limits: Vec<usize> = providers.iter().map(|p| p.limit).collect();
        info!("Starting ordered provider pipeline: {} files, {} providers, {threads} shared workers",
            files.len(), providers.len());
        scheduler::run(files, &limits, threads, config.multiplatform, Schedule::Pipelined,
            || STOP_TAGGING.load(Ordering::SeqCst),
            |stage, slot, path| {
                let status = providers[stage].tag(slot, path, config, &builders);
                Attempt { matched: status.status == TaggingState::Ok, value: status }
            },
            |stage, item, value, completed| {
                let provider = &providers[stage];
                let status = value.unwrap_or_else(|_| failed(&files[item],
                    "Provider panicked; trying the next provider".into()));
                info!("[{}] State: {:?}, Accuracy: {:?}, Path: {:?}",
                    provider.id, status.status, status.accuracy, status.path);
                results.record(&status);
                if let Some(report) = report { report.record(&provider.id, &status); }
                tx.send(TaggingStatusWrap { status, platform: provider.name.clone(),
                    progress: completed as f64 / files.len() as f64 }).ok();
            });
    }
}
