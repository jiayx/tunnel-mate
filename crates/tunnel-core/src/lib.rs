//! UI-independent Tunnel Mate configuration, diagnostics, and SSH runtime.

pub mod config;
pub mod diagnostics;
pub mod event_logger;
pub mod manager;
pub mod ssh;
pub mod ssh_config;

pub use config::{
    export_config_string, import_config_string, AppConfig, ConfigStore, Endpoint, ForwardSpec,
    GlobalSettings, Group, Tunnel, TunnelStatus,
};
pub use manager::{EventSink, RuntimeEvent, StatusPayload, TunnelManager};
pub use ssh::tunnel::{LogSink, TunnelWorker};
pub use ssh_config::{parse_ssh_config, SshHostConfig};
