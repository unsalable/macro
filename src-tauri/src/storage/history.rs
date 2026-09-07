//! Append-only run history (§24).
//!
//! One JSON object per line so a single append never rewrites the file. The
//! log is trimmed back to `limit` once it drifts past a slack threshold, which
//! keeps writes O(1) in the common case.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::atomic_json::{data_file, write_bytes_atomic, StorageResult};
use crate::util::{new_id, now_iso};

pub const FILE_NAME: &str = "history.jsonl";
pub const DEFAULT_LIMIT: usize = 500;
/// Trim only after the file grows this much past the limit.
const SLACK: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HistoryEventKind {
    Started,
    Stopped,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub at: String,
    pub macro_id: String,
    pub macro_name: String,
    pub event: HistoryEventKind,
    #[serde(default)]
    pub iterations: u64,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub detail: Option<String>,
}

impl HistoryEntry {
    pub fn new(
        macro_id: impl Into<String>,
        macro_name: impl Into<String>,
        event: HistoryEventKind,
    ) -> Self {
        Self {
            id: new_id("h"),
            at: now_iso(),
            macro_id: macro_id.into(),
            macro_name: macro_name.into(),
            event,
            iterations: 0,
            duration_ms: 0,
            detail: None,
        }
    }

    pub fn with_run(mut self, iterations: u64, duration_ms: u64) -> Self {
        self.iterations = iterations;
        self.duration_ms = duration_ms;
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

pub fn path() -> StorageResult<PathBuf> {
    data_file(FILE_NAME)
}

pub fn append(entry: &HistoryEntry, limit: usize) -> StorageResult<()> {
    let file_path = path()?;
    append_to(&file_path, entry, limit)
}

fn append_to(file_path: &Path, entry: &HistoryEntry, limit: usize) -> StorageResult<()> {
    let line = serde_json::to_string(entry).map_err(|source| {
        super::atomic_json::StorageError::Parse {
            path: file_path.display().to_string(),
            source,
        }
    })?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .map_err(|source| super::atomic_json::StorageError::Io {
            path: file_path.display().to_string(),
            source,
        })?;

    writeln!(file, "{line}").map_err(|source| super::atomic_json::StorageError::Io {
        path: file_path.display().to_string(),
        source,
    })?;
    drop(file);

    trim_if_needed(file_path, limit)
}

fn trim_if_needed(file_path: &Path, limit: usize) -> StorageResult<()> {
    let raw = match std::fs::read_to_string(file_path) {
        Ok(raw) => raw,
        Err(_) => return Ok(()),
    };

    let lines: Vec<&str> = raw.lines().filter(|line| !line.trim().is_empty()).collect();
    if lines.len() <= limit + SLACK {
        return Ok(());
    }

    let keep = lines[lines.len() - limit..].join("\n");
    write_bytes_atomic(file_path, format!("{keep}\n").as_bytes())
}

/// Newest first, capped at `limit`.
pub fn list(limit: usize) -> StorageResult<Vec<HistoryEntry>> {
    let file_path = path()?;
    Ok(read_from(&file_path, limit))
}

fn read_from(file_path: &Path, limit: usize) -> Vec<HistoryEntry> {
    let Ok(raw) = std::fs::read_to_string(file_path) else {
        return Vec::new();
    };

    let mut entries: Vec<HistoryEntry> = raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        // A half-written trailing line is skipped, not fatal.
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    entries.reverse();
    entries.truncate(limit);
    entries
}

pub fn clear() -> StorageResult<()> {
    let file_path = path()?;
    write_bytes_atomic(&file_path, b"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("flowmacro-tests");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join(name);
        let _ = std::fs::remove_file(&path);
        path
    }

    fn entry(name: &str) -> HistoryEntry {
        HistoryEntry::new("m1", name, HistoryEventKind::Completed)
    }

    #[test]
    fn appends_and_reads_newest_first() {
        let path = temp_file("history-order.jsonl");
        append_to(&path, &entry("first"), 100).expect("append");
        append_to(&path, &entry("second"), 100).expect("append");

        let entries = read_from(&path, 10);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].macro_name, "second");
    }

    #[test]
    fn trims_back_to_the_limit_once_slack_is_exceeded() {
        let path = temp_file("history-trim.jsonl");
        let limit = 10;
        for index in 0..(limit + SLACK + 5) {
            append_to(&path, &entry(&format!("run{index}")), limit).expect("append");
        }

        // Trimming fires once the file passes limit + SLACK and cuts back to
        // `limit`; the appends after that point are still on disk.
        let raw = std::fs::read_to_string(&path).expect("read");
        let count = raw.lines().filter(|l| !l.trim().is_empty()).count();
        assert!(count < limit + SLACK, "file was never trimmed: {count} lines");
        assert!(count >= limit, "trimmed too far: {count} lines");

        let entries = read_from(&path, 100);
        assert_eq!(entries[0].macro_name, format!("run{}", limit + SLACK + 4));
    }

    #[test]
    fn skips_a_corrupt_line_instead_of_failing() {
        let path = temp_file("history-corrupt.jsonl");
        append_to(&path, &entry("good"), 100).expect("append");
        let mut file = OpenOptions::new().append(true).open(&path).expect("open");
        writeln!(file, "{{ half written").expect("write");
        drop(file);

        let entries = read_from(&path, 10);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].macro_name, "good");
    }

    #[test]
    fn missing_file_reads_as_empty() {
        let path = temp_file("history-missing.jsonl");
        assert!(read_from(&path, 10).is_empty());
    }
}
