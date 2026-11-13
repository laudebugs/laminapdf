//! Small JSON files in the app's config folder (the recent list, the preferences): read leniently,
//! written whole and atomically, so a crash mid-write leaves the old file, never half a new one.

use std::io::{self, Write as _};
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

/// `path`'s value; `None` when the file is missing, and also, with a warning, when it can't be read
/// or isn't the expected JSON (the caller starts afresh rather than failing to start).
pub(crate) fn read_json<T: DeserializeOwned>(path: &Path, what: &str) -> Option<T> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return None,
        Err(error) => {
            tracing::warn!("could not read the {what}; starting afresh: {error}");
            return None;
        }
    };
    match serde_json::from_slice(&bytes) {
        Ok(value) => Some(value),
        Err(error) => {
            tracing::warn!("the {what} was damaged; starting afresh: {error}");
            None
        }
    }
}

/// Writes `value` to `path`: to a temporary file beside it, flushed to disk, then renamed over it.
pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    let mut temp = path.as_os_str().to_owned();
    temp.push(".tmp");
    let temp = std::path::PathBuf::from(temp);
    let mut file = std::fs::File::create(&temp)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&temp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("laminapdf-store-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_written_value_should_read_back() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("value.json");
        write_json(&path, &vec![1_u32, 2, 3]).unwrap();
        assert_eq!(read_json::<Vec<u32>>(&path, "test"), Some(vec![1, 2, 3]));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_damaged_file_should_read_as_missing() {
        let dir = temp_dir("damaged");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("value.json");
        std::fs::write(&path, b"{ not json").unwrap();
        assert_eq!(read_json::<Vec<u32>>(&path, "test"), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn writing_should_leave_no_temporary_file() {
        let dir = temp_dir("temp");
        let path = dir.join("value.json");
        write_json(&path, &1_u32).unwrap();
        let names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("value.json")]);
        let _ = std::fs::remove_dir_all(dir);
    }
}
