use crate::config::ConfigStore;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use uuid::Uuid;

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

impl EventLogger {
    pub fn new() -> Self {
        Self {
            store: ConfigStore::new(),
        }
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

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut events = self.get_events().unwrap_or_default();
        events.push(event.clone());

        // Limit event log size to last 1000 events to prevent bloating
        if events.len() > 1000 {
            events.remove(0);
        }

        let content = serde_json::to_string_pretty(&events)
            .map_err(|e| format!("Failed to serialize events: {}", e))?;

        // Atomic write
        let tmp_path = path.with_extension("tmp");
        {
            let mut file = fs::File::create(&tmp_path)
                .map_err(|e| format!("Failed to create temp events file: {}", e))?;
            file.write_all(content.as_bytes())
                .map_err(|e| format!("Failed to write events data: {}", e))?;
            file.sync_all()
                .map_err(|e| format!("Failed to sync events file: {}", e))?;
        }

        fs::rename(&tmp_path, &path).map_err(|e| format!("Failed to commit events file: {}", e))?;

        Ok(event)
    }

    pub fn get_events(&self) -> Result<Vec<LogEvent>, String> {
        let path = self.store.get_events_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content =
            fs::read_to_string(&path).map_err(|e| format!("Failed to read events file: {}", e))?;

        let events: Vec<LogEvent> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse events file: {}", e))?;

        Ok(events)
    }

    pub fn clear_events(&self) -> Result<(), String> {
        let path = self.store.get_events_path();
        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("Failed to delete events file: {}", e))?;
        }
        Ok(())
    }
}
