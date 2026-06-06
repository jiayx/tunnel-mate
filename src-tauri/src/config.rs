use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TunnelType {
    Local,
    Remote,
    Socks5,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TunnelStatus {
    Stopped,
    Running,
    Connecting,
    Reconnecting,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tunnel {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub group_id: Option<String>,
    pub tunnel_type: TunnelType,

    // SSH connection settings
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_user: String,
    pub ssh_identity_file: Option<String>,

    // Jump Host
    pub jump_host_enabled: bool,
    pub jump_host: Option<String>,
    pub jump_port: Option<u16>,
    pub jump_user: Option<String>,
    pub jump_identity_file: Option<String>,

    // Forwarding
    pub local_host: Option<String>,  // e.g. "127.0.0.1"
    pub local_port: u16, // local port to listen (or local port to forward to for Remote)
    pub remote_host: Option<String>, // remote destination host (for Local)
    pub remote_port: Option<u16>, // remote destination port (for Local/Remote)

    // Behaviors
    pub start_with_app: bool,
    pub auto_reconnect: bool,
    pub retry_count: u32,
    pub retry_interval: u32, // in seconds
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSettings {
    pub launch_on_startup: bool,
    pub start_minimized: bool,
    pub close_to_tray: bool,
    pub keep_alive_interval: u32,
    pub connect_timeout: u32,
    pub ssh_config_path: Option<String>,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            launch_on_startup: false,
            start_minimized: false,
            close_to_tray: false,
            keep_alive_interval: 30,
            connect_timeout: 15,
            ssh_config_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    pub groups: Vec<Group>,
    pub tunnels: Vec<Tunnel>,
    pub settings: Option<GlobalSettings>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            groups: Vec::new(),
            tunnels: Vec::new(),
            settings: None,
        }
    }
}

pub struct ConfigStore {
    base_path: PathBuf,
}

impl ConfigStore {
    pub fn new() -> Self {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("TunnelMate");
        Self { base_path: path }
    }

    pub fn get_config_path(&self) -> PathBuf {
        self.base_path.join("config.json")
    }

    pub fn get_events_path(&self) -> PathBuf {
        self.base_path.join("events.json")
    }

    pub fn load_config(&self) -> Result<AppConfig, String> {
        let config_path = self.get_config_path();
        if !config_path.exists() {
            return Ok(AppConfig::default());
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        let config: AppConfig = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;

        Ok(config)
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<(), String> {
        let config_path = self.get_config_path();

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let content = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        // Atomic write
        let tmp_path = config_path.with_extension("tmp");
        {
            let mut file = fs::File::create(&tmp_path)
                .map_err(|e| format!("Failed to create temp config file: {}", e))?;
            file.write_all(content.as_bytes())
                .map_err(|e| format!("Failed to write config data: {}", e))?;
            file.sync_all()
                .map_err(|e| format!("Failed to sync config data: {}", e))?;
        }

        fs::rename(&tmp_path, &config_path)
            .map_err(|e| format!("Failed to commit config file: {}", e))?;

        Ok(())
    }
}
