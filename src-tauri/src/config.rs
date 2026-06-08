use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub const CONFIG_VERSION: u32 = 2;

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
    pub ssh_password: Option<String>,

    // Jump Host
    pub jump_host_enabled: bool,
    pub jump_host: Option<String>,
    pub jump_port: Option<u16>,
    pub jump_user: Option<String>,
    pub jump_identity_file: Option<String>,
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
    pub settings: Option<GlobalSettings>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            groups: Vec::new(),
            tunnels: Vec::new(),
            settings: None,
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
        validate_config(&config)?;

        Ok(config)
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<(), String> {
        validate_config(config)?;

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
    fn serializes_forward_spec_without_legacy_fields() {
        let config = AppConfig {
            version: CONFIG_VERSION,
            groups: Vec::new(),
            tunnels: vec![test_tunnel(ForwardSpec::Local {
                listen: test_endpoint("127.0.0.1", 13306),
                target: test_endpoint("db.internal", 3306),
            })],
            settings: None,
        };

        let json = serde_json::to_string(&config).unwrap();

        assert!(json.contains("\"forward\""));
        assert!(json.contains("\"kind\":\"local\""));
        assert!(json.contains("\"listen\""));
        assert!(json.contains("\"target\""));
        assert!(!json.contains("tunnelType"));
        assert!(!json.contains("localHost"));
        assert!(!json.contains("localPort"));
        assert!(!json.contains("remoteHost"));
        assert!(!json.contains("remotePort"));
    }

    #[test]
    fn validates_socks5_without_target() {
        let config = AppConfig {
            version: CONFIG_VERSION,
            groups: Vec::new(),
            tunnels: vec![test_tunnel(ForwardSpec::Socks5 {
                listen: test_endpoint("127.0.0.1", 1080),
            })],
            settings: None,
        };

        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn rejects_unsupported_config_version() {
        let config = AppConfig {
            version: 1,
            groups: Vec::new(),
            tunnels: Vec::new(),
            settings: None,
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
