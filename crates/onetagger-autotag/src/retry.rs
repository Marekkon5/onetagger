use super::*;

#[derive(Clone, Debug, Default)]
pub(super) struct RunOptions {
    pub report: bool,
    pub selection: Option<report::Selection>,
    pub rotate: bool,
    pub keep_retry: bool,
    pub previous_report: Option<Arc<report::Report>>,
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
    pub(super) fn capture(config: &TaggerConfig, options: RunOptions, results: &TaggingResults, run: &RunTagState) -> Option<Arc<Self>> {
        if !options.keep_retry || results.retryable.is_empty() { return None; }
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
        if let Some(selection) = &self.options.selection {
            let mut subset = selection.clone();
            subset.tracks.retain(|row| self.files.binary_search(&row.local_path).is_ok());
            subset.validate()?;
        }
        Ok(Tagger::tag_files_inner(&self.config, self.files.clone(), finished, self.options.clone(), self.resume_state()))
    }

    pub(super) fn resume_state(&self) -> RunTagState {
        let mut state = RunTagState::new(&self.config);
        state.resumed = true;
        *state.pending_fields.lock().unwrap() = self.pending.clone();
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regression_tests::status;

    #[test]
    fn cooldown_and_membership_follow_only_unresolved_files() {
        let mut results = TaggingResults::default();
        let done = Path::new("already complete.mp3");
        let waiting = Path::new("waiting.mp3");
        let mut limited = status(waiting, TaggingState::Deferred);
        limited.set_message(TaggingMessage::ProviderRateLimited(30));
        results.record(&limited);
        results.record(&status(done, TaggingState::Partial));
        results.record(&status(done, TaggingState::Ok));
        results.record(&status(done, TaggingState::Deferred));
        results.record(&status(Path::new("failed.mp3"), TaggingState::Error));
        let config = TaggerConfig::default();
        let options = RunOptions { keep_retry: true, ..Default::default() };
        let retry = RetryPlan::capture(&config, options.clone(), &results, &RunTagState::default()).unwrap();
        assert_eq!(retry.files, [waiting]);
        assert!(retry.remaining() > Duration::from_secs(25));
        assert!(retry.start(Arc::default()).is_err());
        results.record(&status(waiting, TaggingState::Ok));
        assert!(RetryPlan::capture(&config, options, &results, &RunTagState::default()).is_none());
    }
}
