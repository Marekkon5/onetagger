use super::*;

#[derive(Clone, Debug, Default)]
pub(super) struct RunOptions {
    pub report: bool,
    pub selection: Option<report::Selection>,
    pub rotate: bool,
    pub keep_retry: bool,
}

/// In-memory continuation. Paths and outstanding fields come from the completed
/// worker, so a retry never expands a playlist or rediscovers the input folder.
#[derive(Debug)]
pub struct RetryPlan {
    config: TaggerConfig,
    options: RunOptions,
    files: Vec<PathBuf>,
    pending: HashMap<PathBuf, Vec<SupportedTag>>,
    available: (Instant, Duration),
}

impl RetryPlan {
    pub(super) fn capture(config: &TaggerConfig, mut options: RunOptions, results: &TaggingResults, run: &RunTagState) -> Option<Arc<Self>> {
        if !options.keep_retry || results.retryable.is_empty() { return None; }
        if let Some(selection) = &mut options.selection {
            selection.tracks.retain(|row| results.retryable.contains(&row.local_path));
        }
        let pending = run.pending_fields.lock().unwrap();
        let missing = results.retryable.iter().filter_map(|path|
            pending.get(path).map(|fields| (path.clone(), fields.clone()))).collect();
        let delay = results.retryable.iter().map(|path| results.retry_delays.get(path)
            .map_or(Duration::ZERO, |(start, delay)| delay.saturating_sub(start.elapsed()))).min().unwrap_or_default();
        Some(Arc::new(Self { config: config.clone(), options, files: results.retryable.iter().cloned().collect(),
            pending: missing, available: (Instant::now(), delay) }))
    }

    pub fn file_count(&self) -> usize { self.files.len() }

    pub fn remaining(&self) -> Duration { self.available.1.saturating_sub(self.available.0.elapsed()) }

    pub fn start(&self, finished: Arc<Mutex<Option<TaggerFinishedData>>>) -> Result<Receiver<TaggingStatusWrap>, Error> {
        if !self.remaining().is_zero() { bail!("The provider cooldown is still active"); }
        if let Some(selection) = &self.options.selection { selection.validate()?; }
        let state = RunTagState::new(&self.config);
        *state.pending_fields.lock().unwrap() = self.pending.clone();
        Ok(Tagger::tag_files_inner(&self.config, self.files.clone(), finished, self.options.clone(), state))
    }
}
