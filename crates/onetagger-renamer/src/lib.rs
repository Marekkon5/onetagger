#[macro_use] extern crate log;
#[macro_use] extern crate anyhow;
#[macro_use] extern crate lazy_static;

use std::path::{Path, PathBuf, Component};
use std::collections::{HashMap, HashSet};
use anyhow::Error;
use onetagger_tagger::AudioFileInfo;
use serde::{Serialize, Deserialize};

pub mod ac;
pub mod docs;
pub mod parser;

// Re-export
pub use parser::{TemplateParser, SyntaxData, SyntaxType};


/// Renamer itself
pub struct Renamer {
    template: TemplateParser
}

impl Renamer {
    /// Create new instance
    pub fn new(template: TemplateParser) -> Renamer {
        Renamer { template }
    }

    /// Generate new filename
    pub fn generate_name(&mut self, output_dir: impl AsRef<Path>, info: &AudioFileInfo, config: &RenamerConfig) -> PathBuf {
        let mut name = self.template.evaluate(info, config);
        while name.starts_with("/") {
            name = name[1..].to_string()
        }
        if cfg!(windows) {
            name = name.replace("/", "\\");
        }
        let ext = info.path.extension().unwrap_or_default().to_string_lossy();
        output_dir.as_ref().join(format!("{name}.{ext}"))
    }


    /// Generate names - output: [(from, to),...]
    pub fn generate<I>(&mut self, files: I, config: &RenamerConfig) -> Result<Vec<(PathBuf, PathBuf)>, Error>
    where
        I: IntoIterator<Item = Result<AudioFileInfo, Error>>
    {
        let input_path = dunce::canonicalize(&config.path)?;
        if !input_path.exists() {
            return Err(anyhow!("Invalid path!"));
        }
        
        // Get output path
        let mut out_dir = config.out_dir.clone().unwrap_or(config.path.to_owned());
        if out_dir.to_string_lossy().trim().is_empty() {
            out_dir = config.path.to_owned();
        }

        let mut output = vec![];
        for file in files.into_iter() {
            // Load files
            let info = match file {
                Ok(i) => i,
                Err(e) => {
                    warn!("Failed loading file: {e}");
                    continue;
                },
            };
            let file = &info.path;

            // Get output dir
            let mut output_dir = Path::new(&out_dir).to_owned();
            if config.keep_subfolders {
                // Try to strip prefix and join with original or else fallback to parent
                output_dir = dunce::canonicalize(file)
                    .ok()
                    .map(|p| p.strip_prefix(&input_path).map(|p| p.parent().map(|p| p.to_owned())).ok().flatten())
                    .flatten()
                    .map(|p| output_dir.join(p))
                    .or_else(|| dunce::canonicalize(file)
                        .ok()
                        .map(|p| p.parent().map(|p| p.to_owned()))
                        .flatten()
                    )
                    .unwrap_or(output_dir);
            }

            let new_name = self.generate_name(output_dir, &info, config);
            output.push((file.to_owned(), new_name));
        }
        Ok(output)
    }

    /// Preflight the complete batch before moving anything; return every file's outcome.
    pub fn rename(&mut self, files: &[(PathBuf, PathBuf)], config: &RenamerConfig) -> Result<RenameReport, Error> {
        let mut destinations = HashMap::new();
        let sources: HashSet<_> = files.iter().map(|(from, _)| path_key(from)).collect();
        for (_, to) in files {
            *destinations.entry(path_key(to)).or_insert(0usize) += 1;
        }
        let mut report = RenameReport::default();
        for (from, to) in files {
            let source = path_key(from);
            let target = path_key(to);
            let (outcome, message) = if destinations[&target] > 1 {
                (RenameOutcome::Conflict, Some("Several source files have the same destination".to_string()))
            } else if source != target && sources.contains(&target) {
                (RenameOutcome::Conflict, Some("Destination is another source in this batch".to_string()))
            } else if source != target && !config.overwrite && to.symlink_metadata().is_ok() {
                (RenameOutcome::Conflict, Some("Destination already exists".to_string()))
            } else {
                match onetagger_tag::file_ops::transfer_file(from, to, config.copy, config.overwrite) {
                    Ok(false) => (RenameOutcome::Unchanged, None),
                    Ok(true) => (if config.copy { RenameOutcome::Copied } else { RenameOutcome::Moved }, None),
                    Err(e) => (RenameOutcome::Failed, Some(format!("{e:#}"))),
                }
            };
            match outcome {
                RenameOutcome::Moved | RenameOutcome::Copied => report.succeeded += 1,
                RenameOutcome::Unchanged => report.skipped += 1,
                RenameOutcome::Conflict | RenameOutcome::Failed => report.failed += 1,
            }
            report.results.push(RenameResult { from: from.clone(), to: to.clone(), outcome, message });
        }
        Ok(report)
    }

    /// Generate html from the syntax highlighting
    pub fn generate_html(&self, input: &str) -> String {
        // class prefix
        let prefix = "__renamer_";
        
        let mut output = String::new();
        for syntax in &self.template.syntax {
            let text = input.chars().skip(syntax.start).take(syntax.length).collect::<String>().replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace(" ", "&nbsp;");
            let class = match syntax.syntax {
                SyntaxType::Text => "syntax_text",
                SyntaxType::String => "syntax_string",
                SyntaxType::Number => "syntax_number",
                SyntaxType::Function => "syntax_function",
                SyntaxType::Operator => "syntax_operator",
                SyntaxType::Property => "syntax_property",
                SyntaxType::Variable => "syntax_variable",
            };
            output.push_str(&format!("<span class=\"{prefix}{class}\">{text}</span>"));
        }
        output
    }
}

// Resolve existing aliases and normalize lexical components for destinations not created yet.
fn path_key(path: &Path) -> PathBuf {
    let absolute = dunce::canonicalize(path).unwrap_or_else(|_| {
        let parent = path.parent().unwrap_or(Path::new("."));
        dunce::canonicalize(parent).map(|p| p.join(path.file_name().unwrap_or_default()))
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default().join(path))
    });
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {},
            Component::ParentDir => { normalized.pop(); },
            c => normalized.push(c.as_os_str()),
        }
    }
    if cfg!(any(windows, target_os = "macos")) {
        PathBuf::from(normalized.to_string_lossy().to_lowercase())
    } else {
        normalized
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RenameOutcome { Moved, Copied, Unchanged, Conflict, Failed }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameResult {
    pub from: PathBuf,
    pub to: PathBuf,
    pub outcome: RenameOutcome,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RenameReport {
    pub succeeded: usize,
    pub skipped: usize,
    pub failed: usize,
    pub results: Vec<RenameResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamerConfig {
    pub path: PathBuf,
    pub out_dir: Option<PathBuf>,
    pub template: String,
    pub copy: bool,
    pub subfolders: bool,
    pub overwrite: bool,
    pub separator: String,
    pub keep_subfolders: bool,
}

impl RenamerConfig {
    /// Create new instance with default properties and paths
    pub fn default_with_paths(path: impl AsRef<Path>, template: &str) -> RenamerConfig {
        RenamerConfig {
            path: path.as_ref().to_path_buf(),
            out_dir: None,
            template: template.to_string(),
            copy: false,
            overwrite: false,
            keep_subfolders: false,
            separator: ", ".to_owned(),
            subfolders: true,
        }
    }
}

/// HTML generation test
#[test]
fn generate_html() {
    let items = [
        "%artist% - %title%",
        "%track%. %artist% - %title%",
        "%artist% - %title% - %bpm% - %key%",
        "%artist% - %album%/%track% - %title%",
        "%year% - %album%/%track% - %artist% - %title%"
    ];
    for i in items {
        let renamer = Renamer::new(TemplateParser::parse(i));
        let output = renamer.generate_html(i);
        println!("{output}")
    }
}


#[cfg(test)]
mod safety_tests {
    use super::*;
    use std::fs;

    #[test]
    fn duplicate_destinations_are_reported_without_touching_sources() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.mp3"); let b = dir.path().join("b.mp3");
        let target = dir.path().join("target.mp3");
        fs::write(&a, b"a").unwrap(); fs::write(&b, b"b").unwrap();
        let mut config = RenamerConfig::default_with_paths(dir.path(), "%title%");
        config.overwrite = true;
        let mut renamer = Renamer::new(TemplateParser::parse("%title%"));
        let report = renamer.rename(&[(a.clone(), target.clone()), (b.clone(), target.clone())], &config).unwrap();
        assert_eq!(report.failed, 2); assert_eq!(report.succeeded, 0);
        assert_eq!(fs::read(a).unwrap(), b"a"); assert_eq!(fs::read(b).unwrap(), b"b");
        assert!(!target.exists());
    }

    #[test]
    fn rename_cycles_do_not_overwrite_other_sources() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a"); let b = dir.path().join("b");
        fs::write(&a, b"a").unwrap(); fs::write(&b, b"b").unwrap();
        let mut config = RenamerConfig::default_with_paths(dir.path(), "%title%"); config.overwrite = true;
        let report = Renamer::new(TemplateParser::parse("%title%")).rename(&[(a.clone(), b.clone()), (b.clone(), a.clone())], &config).unwrap();
        assert_eq!(report.failed, 2);
        assert_eq!(fs::read(a).unwrap(), b"a"); assert_eq!(fs::read(b).unwrap(), b"b");
    }

    #[test]
    fn partial_failure_reports_real_outcomes() {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("source"); let to = dir.path().join("target");
        fs::write(&from, b"audio").unwrap();
        let config = RenamerConfig::default_with_paths(dir.path(), "%title%");
        let report = Renamer::new(TemplateParser::parse("%title%")).rename(&[
            (from.clone(), to.clone()), (dir.path().join("missing"), dir.path().join("other")),
        ], &config).unwrap();
        assert_eq!(report.succeeded, 1); assert_eq!(report.failed, 1);
        assert!(!from.exists()); assert_eq!(fs::read(to).unwrap(), b"audio");
    }

    #[test]
    fn template_preview_escapes_html() {
        let input = "<img src=x onerror=alert(1)>";
        let html = Renamer::new(TemplateParser::parse(input)).generate_html(input);
        assert!(!html.contains("<img")); assert!(html.contains("&lt;img"));
    }
}
