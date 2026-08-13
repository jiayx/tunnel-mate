use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::path::Path;

const APP_SUPPORT_DIR: &str = "com.jiayx.tunnel-mate";
const LOCK_FILE_NAME: &str = "instance.lock";

/// Holds the process-wide advisory lock for as long as the application is running.
pub struct SingleInstanceGuard {
    _file: File,
}

impl SingleInstanceGuard {
    /// Returns `Ok(None)` when another Tunnel Mate process already owns the lock.
    pub fn acquire() -> io::Result<Option<Self>> {
        let data_dir = dirs::data_local_dir().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "macOS application support directory is unavailable",
            )
        })?;
        Self::acquire_at(&data_dir.join(APP_SUPPORT_DIR).join(LOCK_FILE_NAME))
    }

    fn acquire_at(path: &Path) -> io::Result<Option<Self>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if result == 0 {
            return Ok(Some(Self { _file: file }));
        }

        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::WouldBlock {
            Ok(None)
        } else {
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SingleInstanceGuard;
    use std::path::PathBuf;

    fn test_lock_path() -> PathBuf {
        std::env::temp_dir()
            .join(format!(
                "tunnel-mate-single-instance-{}",
                uuid::Uuid::new_v4()
            ))
            .join("instance.lock")
    }

    #[test]
    fn rejects_a_second_guard_for_the_same_path() {
        let path = test_lock_path();
        let first = SingleInstanceGuard::acquire_at(&path).unwrap();
        assert!(first.is_some());
        assert!(SingleInstanceGuard::acquire_at(&path).unwrap().is_none());
    }

    #[test]
    fn lock_can_be_reacquired_after_the_guard_is_dropped() {
        let path = test_lock_path();
        let first = SingleInstanceGuard::acquire_at(&path).unwrap().unwrap();
        drop(first);
        assert!(SingleInstanceGuard::acquire_at(&path).unwrap().is_some());
    }
}
