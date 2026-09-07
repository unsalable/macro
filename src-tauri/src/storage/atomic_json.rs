//! Crash-safe JSON persistence (§2.6).
//!
//! Every write goes to a sibling temp file and is then renamed over the
//! target, so a power cut mid-write leaves the previous file intact.
//! A file that fails to parse is moved aside rather than deleted.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Could not reach the FlowMacro data folder: {0}")]
    Location(String),
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{path} is not valid JSON: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
}

pub type StorageResult<T> = Result<T, StorageError>;

fn io_err(path: &Path, source: std::io::Error) -> StorageError {
    StorageError::Io {
        path: path.display().to_string(),
        source,
    }
}

/// `%APPDATA%\FlowMacro`, created on first use.
pub fn data_dir() -> StorageResult<PathBuf> {
    let base = dirs_roaming().ok_or_else(|| {
        StorageError::Location("the roaming application data folder is unavailable".into())
    })?;
    let dir = base.join("FlowMacro");
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|source| io_err(&dir, source))?;
    }
    Ok(dir)
}

#[cfg(windows)]
fn dirs_roaming() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from)
}

#[cfg(not(windows))]
fn dirs_roaming() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))
}

pub fn data_file(name: &str) -> StorageResult<PathBuf> {
    Ok(data_dir()?.join(name))
}

/// Reads a file, falling back to `T::default()` when it does not exist yet.
/// A corrupt file is renamed to `<name>.corrupt` so the user can recover it
/// while the app still starts (§40).
pub fn read_or_default<T>(path: &Path) -> StorageResult<T>
where
    T: DeserializeOwned + Default,
{
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(T::default()),
        Err(source) => return Err(io_err(path, source)),
    };

    match serde_json::from_str(&raw) {
        Ok(value) => Ok(value),
        Err(_) => {
            let backup = path.with_extension("corrupt");
            let _ = fs::rename(path, backup);
            Ok(T::default())
        }
    }
}

/// Strict read used by import, where a bad file must surface as an error.
pub fn read_strict<T: DeserializeOwned>(path: &Path) -> StorageResult<T> {
    let raw = fs::read_to_string(path).map_err(|source| io_err(path, source))?;
    serde_json::from_str(&raw).map_err(|source| StorageError::Parse {
        path: path.display().to_string(),
        source,
    })
}

pub fn write_atomic<T: Serialize>(path: &Path, value: &T) -> StorageResult<()> {
    let json = serde_json::to_string_pretty(value).map_err(|source| StorageError::Parse {
        path: path.display().to_string(),
        source,
    })?;
    write_bytes_atomic(path, json.as_bytes())
}

pub fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> StorageResult<()> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|source| io_err(parent, source))?;
        }
    }

    let temp = path.with_extension(format!(
        "tmp{}",
        std::process::id() as u64 ^ nanos_since_epoch()
    ));

    {
        let mut file = fs::File::create(&temp).map_err(|source| io_err(&temp, source))?;
        file.write_all(bytes).map_err(|source| io_err(&temp, source))?;
        // Flush to disk before the rename, otherwise the rename can land while
        // the payload is still in the page cache.
        file.sync_all().map_err(|source| io_err(&temp, source))?;
    }

    // std::fs::rename replaces the destination on Windows.
    fs::rename(&temp, path).map_err(|source| {
        let _ = fs::remove_file(&temp);
        io_err(path, source)
    })
}

fn nanos_since_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
    struct Sample {
        value: u32,
        label: String,
    }

    fn temp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("flowmacro-tests");
        fs::create_dir_all(&dir).expect("temp dir");
        dir.join(name)
    }

    #[test]
    fn round_trips_a_value() {
        let path = temp_path("round-trip.json");
        let _ = fs::remove_file(&path);
        let sample = Sample { value: 7, label: "hi".into() };

        write_atomic(&path, &sample).expect("write");
        let loaded: Sample = read_or_default(&path).expect("read");

        assert_eq!(loaded, sample);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn missing_file_yields_default() {
        let path = temp_path("definitely-missing.json");
        let _ = fs::remove_file(&path);
        let loaded: Sample = read_or_default(&path).expect("read");
        assert_eq!(loaded, Sample::default());
    }

    #[test]
    fn corrupt_file_is_moved_aside_and_defaults_are_used() {
        let path = temp_path("corrupt.json");
        let backup = path.with_extension("corrupt");
        let _ = fs::remove_file(&backup);
        fs::write(&path, b"{ not json at all").expect("seed");

        let loaded: Sample = read_or_default(&path).expect("read");

        assert_eq!(loaded, Sample::default());
        assert!(backup.exists(), "corrupt file should be preserved");
        let _ = fs::remove_file(&backup);
    }

    #[test]
    fn strict_read_reports_a_parse_error() {
        let path = temp_path("strict.json");
        fs::write(&path, b"nope").expect("seed");
        let result: StorageResult<Sample> = read_strict(&path);
        assert!(matches!(result, Err(StorageError::Parse { .. })));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn overwrites_an_existing_file_in_place() {
        let path = temp_path("overwrite.json");
        write_atomic(&path, &Sample { value: 1, label: "a".into() }).expect("first");
        write_atomic(&path, &Sample { value: 2, label: "b".into() }).expect("second");

        let loaded: Sample = read_or_default(&path).expect("read");
        assert_eq!(loaded.value, 2);

        let leftovers = fs::read_dir(path.parent().expect("parent"))
            .expect("dir")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("overwrite.tmp")
            })
            .count();
        assert_eq!(leftovers, 0, "temp files must not linger");
        let _ = fs::remove_file(&path);
    }
}
