use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::internal::claude::hook::store::is_safe_id;
use crate::internal::claude::session::transcript_stamps;

pub const STATE_DIRECTORY: &str = ".iwe/claude";
pub const RECORDS_DIRECTORY: &str = ".iwe/claude/sessions";
pub const REMINDED_FILE: &str = ".reminded";

const RECORD_EXTENSION: &str = "yaml";
const STATE_GITIGNORE: &str = ".reminded\n";

#[derive(Serialize, Deserialize, Default)]
pub struct SessionRecord {
    pub session: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcript: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcript_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcript_lines: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended: Option<String>,
    #[serde(default)]
    pub distilled_lines: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distilled_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offered: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kept: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rejected: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub captures: Vec<Capture>,
}

#[derive(Serialize, Deserialize)]
pub struct Capture {
    pub at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub through: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wrote: Vec<String>,
}

impl SessionRecord {
    pub fn new(session: &str) -> Self {
        SessionRecord {
            session: session.to_string(),
            ..Default::default()
        }
    }

    pub fn remembered_lines(&self, size: u64) -> Option<usize> {
        if self.transcript_bytes? != size {
            return None;
        }
        self.transcript_lines
    }

    pub fn set_transcript(&mut self, path: &Path) {
        self.transcript = Some(path.to_string_lossy().to_string());
        let stamps = transcript_stamps(path);
        if let Some(started) = stamps.started {
            self.started = Some(started);
        }
        if let Some(ended) = stamps.ended {
            self.ended = Some(ended);
        }
    }

    pub fn set_size(&mut self, size: u64, lines: usize) {
        self.transcript_bytes = Some(size);
        self.transcript_lines = Some(lines);
    }

    pub fn add_ledger(&mut self, offered: usize, rejected: &[String]) {
        if offered > 0 {
            self.offered = Some(self.offered.unwrap_or(0) + offered);
        }
        for title in rejected {
            let title = title.trim();
            if !title.is_empty() {
                self.rejected.push(title.to_string());
            }
        }
    }

    pub fn add_capture(&mut self, at: &str, through: Option<usize>, wrote: Vec<String>) {
        self.kept = Some(self.kept.unwrap_or(0) + wrote.len());
        self.captures.push(Capture {
            at: at.to_string(),
            through,
            wrote,
        });
    }

    pub fn set_title_once(&mut self, title: Option<&str>) {
        if self.title.is_none() {
            self.title = single_line(title);
        }
    }

    pub fn set_summary_once(&mut self, summary: Option<&str>) {
        if self.summary.is_none() {
            self.summary = single_line(summary);
        }
    }
}

pub fn state_directory() -> PathBuf {
    PathBuf::from(STATE_DIRECTORY)
}

pub fn records_directory() -> PathBuf {
    PathBuf::from(RECORDS_DIRECTORY)
}

pub fn record_path(session: &str) -> PathBuf {
    records_directory().join(format!("{}.{}", session, RECORD_EXTENSION))
}

pub fn load_session_record(session: &str) -> Option<SessionRecord> {
    read_session_record(&record_path(session))
}

pub fn save_session_record(record: &SessionRecord) -> bool {
    let path = record_path(&record.session);
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    match serde_yaml::to_string(record) {
        Ok(serialized) => std::fs::write(path, serialized).is_ok(),
        Err(_) => false,
    }
}

pub fn all_session_records() -> Vec<SessionRecord> {
    let entries = match std::fs::read_dir(records_directory()) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut records: Vec<SessionRecord> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some(RECORD_EXTENSION) {
                return None;
            }
            let stem = path.file_stem().and_then(|value| value.to_str())?;
            if !is_safe_id(stem) {
                return None;
            }
            read_session_record(&path)
        })
        .collect();

    records.sort_by(|left, right| left.session.cmp(&right.session));
    records
}

pub fn ensure_state_ignore() {
    let directory = state_directory();
    let ignore = directory.join(".gitignore");
    if ignore.is_file() || std::fs::create_dir_all(&directory).is_err() {
        return;
    }
    std::fs::write(ignore, STATE_GITIGNORE).ok();
}

pub fn reminded_at() -> Option<String> {
    let stamp = std::fs::read_to_string(state_directory().join(REMINDED_FILE)).ok()?;
    let stamp = stamp.trim().to_string();
    (!stamp.is_empty()).then_some(stamp)
}

pub fn mark_reminded(now: &str) -> bool {
    ensure_state_ignore();
    std::fs::write(state_directory().join(REMINDED_FILE), format!("{}\n", now)).is_ok()
}

fn read_session_record(path: &Path) -> Option<SessionRecord> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_yaml::from_str(&raw).ok()
}

fn single_line(value: Option<&str>) -> Option<String> {
    let line = value?.lines().next().unwrap_or("").trim();
    (!line.is_empty()).then(|| line.to_string())
}
