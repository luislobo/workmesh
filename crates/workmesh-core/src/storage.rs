use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use fs2::FileExt;

struct PathLock {
    file: File,
}

impl PathLock {
    fn acquire(path: &Path) -> io::Result<Self> {
        let lock_path = lock_path(path);
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&lock_path)?;
        file.lock_exclusive()?;
        Ok(Self { file })
    }
}

impl Drop for PathLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

pub fn with_path_lock<T, E, F>(path: &Path, action: F) -> Result<T, E>
where
    E: From<io::Error>,
    F: FnOnce() -> Result<T, E>,
{
    let _lock = PathLock::acquire(path).map_err(E::from)?;
    action()
}

pub fn with_path_lock_io<T, F>(path: &Path, action: F) -> io::Result<T>
where
    F: FnOnce() -> io::Result<T>,
{
    with_path_lock(path, action)
}

pub fn write_string_atomic(path: &Path, body: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp = temp_path(path);
    let mut file = File::create(&tmp)?;
    file.write_all(body.as_bytes())?;
    file.sync_all()?;
    if let Err(err) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(err);
    }
    Ok(())
}

pub fn write_string_atomic_locked(path: &Path, body: &str) -> io::Result<()> {
    with_path_lock_io(path, || write_string_atomic(path, body))
}

pub fn append_line_locked(path: &Path, line: &str) -> io::Result<()> {
    with_path_lock_io(path, || {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(file, "{}", line)?;
        file.sync_data()?;
        Ok(())
    })
}

fn lock_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let lock_name = match path.file_name().and_then(|name| name.to_str()) {
        Some(name) if !name.is_empty() => format!("{}.lock", name),
        _ => "workmesh.lock".to_string(),
    };
    parent.join(lock_name)
}

fn temp_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("workmesh");
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    parent.join(format!(".{}.{}.{}.tmp", stem, pid, nanos))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;

    use super::*;
    use tempfile::TempDir;

    #[test]
    fn write_string_atomic_locked_writes_file() {
        let temp = TempDir::new().expect("tempdir");
        let path = temp.path().join("state.json");
        write_string_atomic_locked(&path, "{\n  \"ok\": true\n}\n").expect("write");
        let raw = fs::read_to_string(path).expect("read");
        assert!(raw.contains("\"ok\""));
    }

    #[test]
    fn append_line_locked_creates_and_appends() {
        let temp = TempDir::new().expect("tempdir");
        let path = temp.path().join("events.jsonl");
        append_line_locked(&path, r#"{"a":1}"#).expect("append 1");
        append_line_locked(&path, r#"{"a":2}"#).expect("append 2");
        let raw = fs::read_to_string(path).expect("read");
        assert_eq!(raw.lines().count(), 2);
    }

    #[test]
    fn with_path_lock_serializes_read_modify_write_updates() {
        let temp = TempDir::new().expect("tempdir");
        let path = Arc::new(temp.path().join("counter.txt"));
        write_string_atomic(path.as_ref(), "0\n").expect("seed");

        let workers = 6usize;
        let increments_per_worker = 40usize;
        let mut handles = Vec::new();

        for _ in 0..workers {
            let shared_path = Arc::clone(&path);
            handles.push(thread::spawn(move || {
                for _ in 0..increments_per_worker {
                    with_path_lock_io(shared_path.as_ref(), || {
                        let current = fs::read_to_string(shared_path.as_ref())?;
                        let parsed = current.trim().parse::<usize>().map_err(|err| {
                            io::Error::new(io::ErrorKind::InvalidData, err.to_string())
                        })?;
                        write_string_atomic(shared_path.as_ref(), &format!("{}\n", parsed + 1))?;
                        Ok(())
                    })
                    .expect("locked update");
                }
            }));
        }

        for handle in handles {
            handle.join().expect("join");
        }

        let final_value = fs::read_to_string(path.as_ref())
            .expect("read final")
            .trim()
            .parse::<usize>()
            .expect("parse");
        assert_eq!(final_value, workers * increments_per_worker);
    }
}
