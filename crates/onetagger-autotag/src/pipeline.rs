use super::*;
use crate::scheduler::{self, Attempt, Schedule};

type SourceSlot = Mutex<Option<Result<Box<dyn AutotaggerSource>, TaggingMessage>>>;
type SourceBuilder = Box<dyn AutotaggerSourceBuilder + Send + Sync>;

struct Provider<'a> {
    id: String,
    name: String,
    sources: Vec<SourceSlot>,
    builder: Mutex<&'a mut SourceBuilder>,
}

pub(super) fn deduplicate_files(files: &mut Vec<PathBuf>) {
    let mut seen = BTreeSet::new();
    files.retain(|path| seen.insert(dunce::canonicalize(path).unwrap_or_else(|_| path.clone())));
}

fn failed(path: &Path, message: Option<TaggingMessage>) -> TaggingStatus {
    let mut status = TaggingStatus {
        status: TaggingState::Error,
        path: path.into(),
        message: None,
        message_key: None,
        message_args: None,
        accuracy: None,
        lookup_ms: None,
        used_shazam: false,
        release_id: None,
        track_id: None,
        reason: None,
    };
    if let Some(message) = message {
        status.set_message(message);
    }
    status
}

impl Provider<'_> {
    fn tag(&self, slot: usize, path: &Path, config: &TaggerConfig, run: &RunTagState) -> TaggingStatus {
        let mut error = None;
        // A failed source constructor must not send a track to a lower-priority
        // provider while this provider still has healthy source instances.
        for offset in 0..self.sources.len() {
            match self.tag_with_slot((slot + offset) % self.sources.len(), path, config, run) {
                Ok(status) => return status,
                Err(message) => error = Some(message),
            }
        }
        failed(path, error)
    }

    fn tag_with_slot(&self, slot: usize, path: &Path, config: &TaggerConfig, run: &RunTagState)
        -> Result<TaggingStatus, TaggingMessage> {
        let mut stopped = failed(path, None);
        if Tagger::mark_cancelled(&mut stopped, &|| STOP_TAGGING.load(Ordering::SeqCst)) { return Ok(stopped); }
        let mut source = match self.sources[slot].lock() {
            Ok(source) => source,
            Err(_) => return Err(TaggingMessage::ProviderWorkerPanicked),
        };
        if source.is_none() {
            // Keep construction serial within a provider. Authentication can
            // block on I/O, so each provider owns its own builder lock.
            let mut builder = self.builder.lock().unwrap();
            if Tagger::mark_cancelled(&mut stopped, &|| STOP_TAGGING.load(Ordering::SeqCst)) { return Ok(stopped); }
            let created = std::panic::catch_unwind(std::panic::AssertUnwindSafe(||
                builder.get_source(config)
            )).map_err(|_| TaggingMessage::ProviderInitializationPanicked)
                .and_then(|r| r.map_err(|e| TaggingMessage::ProviderInitializationFailed(e.to_string())));
            *source = Some(created);
        }
        if Tagger::mark_cancelled(&mut stopped, &|| STOP_TAGGING.load(Ordering::SeqCst)) { return Ok(stopped); }
        match source.as_mut().unwrap() {
            Ok(source) => Ok(Tagger::tag_track_with_cancel(path, source, config, run,
                &|| STOP_TAGGING.load(Ordering::SeqCst))),
            Err(message) => Err(message.clone()),
        }
    }
}

impl Tagger {
    pub(super) fn tag_pipeline(files: &[PathBuf], config: &TaggerConfig,
        tx: &Sender<TaggingStatusWrap>, results: &mut TaggingResults,
        report: &mut Option<report::ReportBuilder>, run: &RunTagState, rotate: bool) {
        if files.is_empty() { return; }
        let threads = usize::from(config.threads).max(1).min(files.len());
        // Keep custom plugin libraries alive for the lifetime of their sources.
        let mut platforms = AUTOTAGGER_PLATFORMS.lock().unwrap();
        let mut builders = HashMap::new();
        for platform in &mut platforms.platforms {
            builders.entry(platform.info.platform.id.as_str()).or_insert(&mut platform.platform);
        }
        let providers: Vec<Provider<'_>> = {
            let mut seen = BTreeSet::new();
            config.platforms.iter().filter(|id| seen.insert((*id).clone())).filter_map(|id| {
                let builder = match builders.remove(id.as_str()) {
                    Some(builder) => builder,
                    None => {
                        let (message, _, _) = TaggingMessage::ProviderUnavailable(id.clone()).describe();
                        error!("{message}");
                        return None;
                    }
                };
                let info = builder.info();
                let limit = if info.max_threads == 0 { threads }
                    else { usize::from(info.max_threads).min(threads) };
                Some(Provider { id: id.clone(), name: info.name,
                    sources: (0..limit).map(|_| Mutex::new(None)).collect(),
                    builder: Mutex::new(builder) })
            }).collect()
        };
        let limits: Vec<usize> = providers.iter().map(|p| p.sources.len()).collect();
        info!("Starting provider pipeline: {} files, {} providers, {threads} shared workers",
            files.len(), providers.len());
        let schedule = if rotate { Schedule::Rotating } else { Schedule::Pipelined };
        scheduler::run(files, &limits, threads, config.multiplatform, schedule,
            || STOP_TAGGING.load(Ordering::SeqCst),
            |stage, slot, path| {
                let status = providers[stage].tag(slot, path, config, run);
                Attempt { matched: status.status == TaggingState::Ok, value: status }
            },
            |stage, item, value, completed| {
                let provider = &providers[stage];
                let status = value.unwrap_or_else(|_| failed(&files[item],
                    Some(TaggingMessage::ProviderPanickedTryingNext)));
                info!("[{}] State: {:?}, Accuracy: {:?}, Path: {:?}",
                    provider.id, status.status, status.accuracy, status.path);
                results.record(&status);
                if let Some(report) = report { report.record(&provider.id, &status); }
                tx.send(TaggingStatusWrap { status, platform: provider.name.clone(),
                    provider_id: Some(provider.id.clone()), progress: completed as f64 / files.len() as f64 }).ok();
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use onetagger_tagger::PlatformInfo;
    use std::sync::mpsc;
    use std::thread;

    struct TestBuilder {
        gate: Option<(mpsc::Sender<()>, Mutex<mpsc::Receiver<()>>)>,
    }

    impl AutotaggerSourceBuilder for TestBuilder {
        fn new() -> Self { Self { gate: None } }

        fn get_source(&mut self, _: &TaggerConfig) -> Result<Box<dyn AutotaggerSource>, Error> {
            if let Some((started, release)) = &self.gate {
                started.send(()).unwrap();
                release.lock().unwrap().recv_timeout(Duration::from_secs(10)).unwrap();
            }
            Err(anyhow!("Synthetic provider initialization result"))
        }

        fn info(&self) -> PlatformInfo { unreachable!("This test constructs providers directly") }
    }

    #[test]
    fn one_providers_authentication_does_not_block_another_provider() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (other_tx, other_rx) = mpsc::channel();
        let mut slow_builder: SourceBuilder = Box::new(TestBuilder {
            gate: Some((started_tx, Mutex::new(release_rx))),
        });
        let mut fast_builder: SourceBuilder = Box::new(TestBuilder::new());
        let slow = Provider { id: "slow".into(), name: "Slow".into(),
            sources: vec![Mutex::new(None)], builder: Mutex::new(&mut slow_builder) };
        let fast = Provider { id: "fast".into(), name: "Fast".into(),
            sources: vec![Mutex::new(None)], builder: Mutex::new(&mut fast_builder) };
        let config = TaggerConfig::default();
        let run = RunTagState::new(&config);

        thread::scope(|scope| {
            scope.spawn(|| {
                assert!(slow.tag_with_slot(0, Path::new("fixture.mp3"), &config, &run).is_err());
            });
            started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            scope.spawn(|| {
                let finished = fast.tag_with_slot(0, Path::new("fixture.mp3"), &config, &run).is_err();
                other_tx.send(finished).unwrap();
            });
            let independent = other_rx.recv_timeout(Duration::from_secs(5));
            release_tx.send(()).unwrap();
            assert_eq!(independent, Ok(true));
        });
    }
}
