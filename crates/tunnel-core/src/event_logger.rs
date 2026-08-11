use crate::config::ConfigStore;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

const MAX_EVENT_COUNT: usize = 1_000;
const MAX_EVENT_FILE_BYTES: u64 = 2 * 1024 * 1024;

fn event_file_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventType {
    Created,
    Updated,
    Connecting,
    Started,
    Stopped,
    Restarted,
    Reconnected,
    Failed,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
    pub id: String,
    pub session_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub tunnel_id: Option<String>,
    pub tunnel_name: Option<String>,
    pub event_type: EventType,
    pub message: String,
}

pub struct EventLogger {
    store: ConfigStore,
}

impl Default for EventLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl EventLogger {
    pub fn new() -> Self {
        Self {
            store: ConfigStore::new(),
        }
    }

    pub fn with_store(store: ConfigStore) -> Self {
        Self { store }
    }

    pub fn log(
        &self,
        tunnel_id: Option<String>,
        tunnel_name: Option<String>,
        event_type: EventType,
        message: String,
    ) -> Result<LogEvent, String> {
        self.log_with_session(None, tunnel_id, tunnel_name, event_type, message)
    }

    pub fn log_with_session(
        &self,
        session_id: Option<String>,
        tunnel_id: Option<String>,
        tunnel_name: Option<String>,
        event_type: EventType,
        message: String,
    ) -> Result<LogEvent, String> {
        let event = LogEvent {
            id: Uuid::new_v4().to_string(),
            session_id,
            timestamp: Utc::now(),
            tunnel_id,
            tunnel_name,
            event_type,
            message,
        };

        let path = self.store.get_events_path();
        let _guard = event_file_lock()
            .lock()
            .map_err(|_| "Events file lock was poisoned".to_string())?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
            }
        }

        let content = serde_json::to_string(&event)
            .map_err(|e| format!("Failed to serialize events: {}", e))?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("Failed to open events file: {}", e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
                .map_err(|e| format!("Failed to secure events file: {}", e))?;
        }
        writeln!(file, "{}", content).map_err(|e| format!("Failed to write events data: {}", e))?;
        file.flush()
            .map_err(|e| format!("Failed to flush events data: {}", e))?;
        if file
            .metadata()
            .map(|metadata| metadata.len() > MAX_EVENT_FILE_BYTES)
            .unwrap_or(false)
        {
            drop(file);
            compact_events_file(&path)?;
        }

        Ok(event)
    }

    pub fn get_events(&self) -> Result<Vec<LogEvent>, String> {
        let path = self.store.get_events_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let _guard = event_file_lock()
            .lock()
            .map_err(|_| "Events file lock was poisoned".to_string())?;
        read_last_lines(&path)?
            .into_iter()
            .map(|line| {
                serde_json::from_str(&line)
                    .map_err(|e| format!("Failed to parse events file: {}", e))
            })
            .collect()
    }

    pub fn clear_events(&self) -> Result<(), String> {
        let path = self.store.get_events_path();
        let _guard = event_file_lock()
            .lock()
            .map_err(|_| "Events file lock was poisoned".to_string())?;
        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("Failed to delete events file: {}", e))?;
        }
        Ok(())
    }
}

fn read_last_lines(path: &Path) -> Result<VecDeque<String>, String> {
    let file = fs::File::open(path).map_err(|e| format!("Failed to read events file: {}", e))?;
    let mut lines: VecDeque<String> = VecDeque::with_capacity(MAX_EVENT_COUNT);
    let mut retained_bytes = 0usize;
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|e| format!("Failed to read events file: {}", e))?;
        if line.trim().is_empty() {
            continue;
        }
        let line_bytes = line.len() + 1;
        while !lines.is_empty()
            && (lines.len() >= MAX_EVENT_COUNT
                || retained_bytes + line_bytes > MAX_EVENT_FILE_BYTES as usize)
        {
            if let Some(removed) = lines.pop_front() {
                retained_bytes = retained_bytes.saturating_sub(removed.len() + 1);
            }
        }
        retained_bytes += line_bytes;
        lines.push_back(line);
    }
    Ok(lines)
}

fn compact_events_file(path: &Path) -> Result<(), String> {
    let lines = read_last_lines(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| "Events file has no parent directory".to_string())?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| format!("Failed to create compacted events file: {}", e))?;
    for line in lines {
        writeln!(temporary, "{}", line)
            .map_err(|e| format!("Failed to compact events file: {}", e))?;
    }
    temporary
        .as_file()
        .sync_all()
        .map_err(|e| format!("Failed to sync compacted events file: {}", e))?;
    temporary
        .persist(path)
        .map_err(|e| format!("Failed to replace events file: {}", e.error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn appends_reads_and_clears_events_in_order() {
        let directory = tempfile::tempdir().unwrap();
        let logger =
            EventLogger::with_store(ConfigStore::from_base_path(PathBuf::from(directory.path())));

        logger
            .log(
                Some("one".into()),
                Some("First".into()),
                EventType::Started,
                "connected".into(),
            )
            .unwrap();
        logger
            .log(
                Some("two".into()),
                Some("Second".into()),
                EventType::Stopped,
                "stopped".into(),
            )
            .unwrap();

        let events = logger.get_events().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].tunnel_name.as_deref(), Some("First"));
        assert_eq!(events[1].tunnel_name.as_deref(), Some("Second"));

        logger.clear_events().unwrap();
        assert!(logger.get_events().unwrap().is_empty());
    }

    #[test]
    fn rotates_large_event_files_to_the_latest_entries() {
        let directory = tempfile::tempdir().unwrap();
        let store = ConfigStore::from_base_path(PathBuf::from(directory.path()));
        let path = store.get_events_path();
        let logger = EventLogger::with_store(store);
        let payload = "x".repeat(3_000);

        for index in 0..1_050 {
            logger
                .log(None, None, EventType::Started, format!("{index}:{payload}"))
                .unwrap();
        }

        let events = logger.get_events().unwrap();
        assert!(events.len() <= MAX_EVENT_COUNT);
        assert!(events.last().unwrap().message.starts_with("1049:"));
        assert!(fs::metadata(path).unwrap().len() < MAX_EVENT_FILE_BYTES);
    }
}
