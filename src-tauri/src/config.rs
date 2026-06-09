use keyring::{Entry, Error as KeyringError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub const CONFIG_VERSION: u32 = 1;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ForwardSpec {
    Local { listen: Endpoint, target: Endpoint },
    Remote { listen: Endpoint, target: Endpoint },
    Socks5 { listen: Endpoint },
}

impl ForwardSpec {
    pub fn listen(&self) -> &Endpoint {
        match self {
            ForwardSpec::Local { listen, .. }
            | ForwardSpec::Remote { listen, .. }
            | ForwardSpec::Socks5 { listen } => listen,
        }
    }

    pub fn target(&self) -> Option<&Endpoint> {
        match self {
            ForwardSpec::Local { target, .. } | ForwardSpec::Remote { target, .. } => Some(target),
            ForwardSpec::Socks5 { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Tunnel {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub group_id: Option<String>,

    // SSH connection settings
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_user: String,
    pub ssh_identity_file: Option<String>,
    #[serde(skip_serializing)]
    pub ssh_password: Option<String>,

    // Jump Host
    pub jump_host_enabled: bool,
    pub jump_host: Option<String>,
    pub jump_port: Option<u16>,
    pub jump_user: Option<String>,
    pub jump_identity_file: Option<String>,
    #[serde(skip_serializing)]
    pub jump_password: Option<String>,

    // Forwarding
    pub forward: ForwardSpec,

    // Behaviors
    pub start_with_app: bool,
    pub auto_reconnect: bool,
    pub retry_count: u32,
    pub retry_interval: u32, // in seconds
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppConfig {
    pub version: u32,
    pub groups: Vec<Group>,
    pub tunnels: Vec<Tunnel>,
    pub settings: GlobalSettings,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            groups: Vec::new(),
            tunnels: Vec::new(),
            settings: GlobalSettings::default(),
        }
    }
}

pub fn validate_config(config: &AppConfig) -> Result<(), String> {
    if config.version != CONFIG_VERSION {
        return Err(format!(
            "Unsupported configuration version: expected {}, got {}",
            CONFIG_VERSION, config.version
        ));
    }

    for tunnel in &config.tunnels {
        validate_tunnel(tunnel)?;
    }

    Ok(())
}

pub fn validate_tunnel(tunnel: &Tunnel) -> Result<(), String> {
    if tunnel.name.trim().is_empty() {
        return Err("Tunnel name is required".to_string());
    }
    if tunnel.ssh_host.trim().is_empty() {
        return Err(format!("Tunnel '{}' SSH host is required", tunnel.name));
    }
    if tunnel.ssh_port == 0 {
        return Err(format!("Tunnel '{}' SSH port is invalid", tunnel.name));
    }
    if tunnel.ssh_user.trim().is_empty() {
        return Err(format!("Tunnel '{}' SSH user is required", tunnel.name));
    }

    validate_endpoint(tunnel.forward.listen(), "listen endpoint")?;
    if let Some(target) = tunnel.forward.target() {
        validate_endpoint(target, "target endpoint")?;
    }

    if tunnel.jump_host_enabled {
        if tunnel
            .jump_host
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            return Err(format!("Tunnel '{}' jump host is required", tunnel.name));
        }
        if tunnel.jump_port.unwrap_or_default() == 0 {
            return Err(format!("Tunnel '{}' jump port is invalid", tunnel.name));
        }
        if tunnel
            .jump_user
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            return Err(format!("Tunnel '{}' jump user is required", tunnel.name));
        }
    }

    Ok(())
}

pub fn validate_endpoint(endpoint: &Endpoint, label: &str) -> Result<(), String> {
    if endpoint.host.trim().is_empty() {
        return Err(format!("{} host is required", label));
    }
    if endpoint.port == 0 {
        return Err(format!("{} port is invalid", label));
    }
    Ok(())
}

pub struct ConfigStore {
    base_path: PathBuf,
}

const KEYRING_SERVICE: &str = "com.jiayx.tunnel-mate";

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
        reject_persisted_secrets(&content)?;

        let mut config: AppConfig = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;
        validate_config(&config)?;
        self.hydrate_secrets(&mut config);

        Ok(config)
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<(), String> {
        validate_config(config)?;
        self.persist_secrets(config)?;

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

    fn hydrate_secrets(&self, config: &mut AppConfig) {
        for tunnel in &mut config.tunnels {
            tunnel.ssh_password = read_secret(&secret_account(&tunnel.id, "ssh_password"));
            tunnel.jump_password = read_secret(&secret_account(&tunnel.id, "jump_password"));
        }
    }

    fn persist_secrets(&self, config: &AppConfig) -> Result<(), String> {
        if let Ok(existing) = self.load_config_from_disk() {
            for tunnel in existing.tunnels {
                if !config.tunnels.iter().any(|current| current.id == tunnel.id) {
                    write_secret(&secret_account(&tunnel.id, "ssh_password"), None)?;
                    write_secret(&secret_account(&tunnel.id, "jump_password"), None)?;
                }
            }
        }

        for tunnel in &config.tunnels {
            write_secret(
                &secret_account(&tunnel.id, "ssh_password"),
                tunnel.ssh_password.as_deref(),
            )?;
            write_secret(
                &secret_account(&tunnel.id, "jump_password"),
                tunnel.jump_password.as_deref(),
            )?;
        }
        Ok(())
    }

    fn load_config_from_disk(&self) -> Result<AppConfig, String> {
        let config_path = self.get_config_path();
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config file: {}", e))
    }
}

fn secret_account(tunnel_id: &str, name: &str) -> String {
    format!("{}:{}", tunnel_id, name)
}

fn reject_persisted_secrets(content: &str) -> Result<(), String> {
    let value: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("Failed to parse config file: {}", e))?;
    let Some(tunnels) = value.get("tunnels").and_then(|value| value.as_array()) else {
        return Ok(());
    };

    if tunnels
        .iter()
        .any(|tunnel| tunnel.get("sshPassword").is_some() || tunnel.get("jumpPassword").is_some())
    {
        return Err("Config file must not contain persisted SSH passwords".to_string());
    }

    Ok(())
}

fn read_secret(account: &str) -> Option<String> {
    Entry::new(KEYRING_SERVICE, account)
        .ok()
        .and_then(|entry| entry.get_password().ok())
        .filter(|value| !value.is_empty())
}

fn write_secret(account: &str, value: Option<&str>) -> Result<(), String> {
    let entry = Entry::new(KEYRING_SERVICE, account)
        .map_err(|e| format!("Failed to open credential store: {}", e))?;
    match value.filter(|password| !password.is_empty()) {
        Some(password) => entry
            .set_password(password)
            .map_err(|e| format!("Failed to save credential: {}", e)),
        None => match entry.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(e) => Err(format!("Failed to delete credential: {}", e)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_endpoint(host: &str, port: u16) -> Endpoint {
        Endpoint {
            host: host.to_string(),
            port,
        }
    }

    fn test_tunnel(forward: ForwardSpec) -> Tunnel {
        Tunnel {
            id: "t1".to_string(),
            name: "test".to_string(),
            description: None,
            group_id: None,
            ssh_host: "example.test".to_string(),
            ssh_port: 22,
            ssh_user: "root".to_string(),
            ssh_identity_file: None,
            ssh_password: None,
            jump_host_enabled: false,
            jump_host: None,
            jump_port: None,
            jump_user: None,
            jump_identity_file: None,
            jump_password: None,
            forward,
            start_with_app: false,
            auto_reconnect: false,
            retry_count: 3,
            retry_interval: 5,
        }
    }

    #[test]
    fn default_config_uses_current_version() {
        assert_eq!(AppConfig::default().version, CONFIG_VERSION);
    }

    #[test]
    fn serializes_forward_spec_schema() {
        let mut tunnel = test_tunnel(ForwardSpec::Local {
            listen: test_endpoint("127.0.0.1", 13306),
            target: test_endpoint("db.internal", 3306),
        });
        tunnel.ssh_password = Some("secret".to_string());
        tunnel.jump_password = Some("jump-secret".to_string());
        let config = AppConfig {
            version: CONFIG_VERSION,
            groups: Vec::new(),
            tunnels: vec![tunnel],
            settings: GlobalSettings::default(),
        };

        let json = serde_json::to_string(&config).unwrap();

        assert!(json.contains("\"forward\""));
        assert!(json.contains("\"kind\":\"local\""));
        assert!(json.contains("\"listen\""));
        assert!(json.contains("\"target\""));
        assert!(json.contains("settings"));
        assert!(!json.contains("sshPassword"));
        assert!(!json.contains("jumpPassword"));
        assert!(!json.contains("secret"));
    }

    #[test]
    fn validates_socks5_without_target() {
        let config = AppConfig {
            version: CONFIG_VERSION,
            groups: Vec::new(),
            tunnels: vec![test_tunnel(ForwardSpec::Socks5 {
                listen: test_endpoint("127.0.0.1", 1080),
            })],
            settings: GlobalSettings::default(),
        };

        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn rejects_unsupported_config_version() {
        let config = AppConfig {
            version: CONFIG_VERSION + 1,
            groups: Vec::new(),
            tunnels: Vec::new(),
            settings: GlobalSettings::default(),
        };

        assert!(validate_config(&config)
            .unwrap_err()
            .contains("Unsupported configuration version"));
    }

    #[test]
    fn rejects_empty_endpoint_host() {
        let endpoint = test_endpoint(" ", 8080);

        assert!(validate_endpoint(&endpoint, "listen endpoint")
            .unwrap_err()
            .contains("host is required"));
    }

    #[test]
    fn rejects_zero_endpoint_port() {
        let endpoint = test_endpoint("127.0.0.1", 0);

        assert!(validate_endpoint(&endpoint, "listen endpoint")
            .unwrap_err()
            .contains("port is invalid"));
    }
}
