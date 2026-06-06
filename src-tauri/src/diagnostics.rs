use crate::config::Tunnel;
use crate::ssh::engine::SshSession;
use serde::{Deserialize, Serialize};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticStep {
    pub name: String,
    pub status: String,
    pub message: String,
}

pub async fn run_diagnostics(tunnel: &Tunnel, passphrase: Option<&str>) -> Vec<DiagnosticStep> {
    let mut steps = Vec::new();

    let local_host = tunnel
        .local_host
        .clone()
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let local_port = tunnel.local_port;

    match TcpListener::bind(format!("{}:{}", local_host, local_port)) {
        Ok(_) => steps.push(success(
            "Local Environment",
            format!(
                "Port {} is free and available to bind on {}",
                local_port, local_host
            ),
        )),
        Err(e) => {
            steps.push(error(
                "Local Environment",
                format!("Port {} cannot be bound on {}: {}. It is likely in use by another application.", local_port, local_host, e),
            ));
            return steps;
        }
    }

    let target = resolve_diagnostic_target(tunnel);

    let addr = match format!("{}:{}", target.host, target.port).to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                steps.push(success(
                    format!("{}DNS Resolution", target.prefix),
                    format!("Resolved {} to {}", target.host, addr.ip()),
                ));
                addr
            } else {
                steps.push(error(
                    format!("{}DNS Resolution", target.prefix),
                    format!("Resolved {} to no IP addresses", target.host),
                ));
                return steps;
            }
        }
        Err(e) => {
            steps.push(error(
                format!("{}DNS Resolution", target.prefix),
                format!("Failed to resolve hostname {}: {}", target.host, e),
            ));
            return steps;
        }
    };

    match TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
        Ok(_) => steps.push(success(
            format!("{}TCP Connection", target.prefix),
            format!(
                "Successfully established TCP socket to {}:{}",
                target.host, target.port
            ),
        )),
        Err(e) => {
            steps.push(error(
                format!("{}TCP Connection", target.prefix),
                format!(
                    "Failed to connect to {}:{}: {}",
                    target.host, target.port, e
                ),
            ));
            return steps;
        }
    }

    match SshSession::connect(
        &target.host,
        target.port,
        &target.user,
        target.identity_file.as_deref(),
        passphrase,
        "trustOnce",
        None,
    )
    .await
    {
        Ok(mut session) => {
            steps.push(success(
                format!("{}SSH Authentication", target.prefix),
                "SSH handshake and authentication completed successfully".to_string(),
            ));
            session.disconnect().await;
        }
        Err(err) if err == "PASSPHRASE_REQUIRED" => steps.push(warning(
            format!("{}SSH Authentication", target.prefix),
            "Private key is encrypted and requires a passphrase.".to_string(),
        )),
        Err(err) => steps.push(error(
            format!("{}SSH Authentication", target.prefix),
            format!("SSH authentication failed: {}", err),
        )),
    }

    steps
}

#[derive(Debug, PartialEq, Eq)]
struct DiagnosticTarget {
    host: String,
    port: u16,
    user: String,
    identity_file: Option<String>,
    prefix: &'static str,
}

fn resolve_diagnostic_target(tunnel: &Tunnel) -> DiagnosticTarget {
    if tunnel.jump_host_enabled {
        DiagnosticTarget {
            host: tunnel
                .jump_host
                .clone()
                .unwrap_or_else(|| tunnel.ssh_host.clone()),
            port: tunnel.jump_port.unwrap_or(22),
            user: tunnel
                .jump_user
                .clone()
                .unwrap_or_else(|| tunnel.ssh_user.clone()),
            identity_file: tunnel
                .jump_identity_file
                .clone()
                .or_else(|| tunnel.ssh_identity_file.clone()),
            prefix: "[Jump Host] ",
        }
    } else {
        DiagnosticTarget {
            host: tunnel.ssh_host.clone(),
            port: tunnel.ssh_port,
            user: tunnel.ssh_user.clone(),
            identity_file: tunnel.ssh_identity_file.clone(),
            prefix: "",
        }
    }
}

fn success(name: impl Into<String>, message: impl Into<String>) -> DiagnosticStep {
    DiagnosticStep {
        name: name.into(),
        status: "success".to_string(),
        message: message.into(),
    }
}

fn warning(name: impl Into<String>, message: impl Into<String>) -> DiagnosticStep {
    DiagnosticStep {
        name: name.into(),
        status: "warning".to_string(),
        message: message.into(),
    }
}

fn error(name: impl Into<String>, message: impl Into<String>) -> DiagnosticStep {
    DiagnosticStep {
        name: name.into(),
        status: "error".to_string(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TunnelType;

    fn base_tunnel() -> Tunnel {
        Tunnel {
            id: "t1".to_string(),
            name: "test".to_string(),
            description: None,
            group_id: None,
            tunnel_type: TunnelType::Local,
            ssh_host: "example.test".to_string(),
            ssh_port: 22,
            ssh_user: "root".to_string(),
            ssh_identity_file: Some("/tmp/key".to_string()),
            jump_host_enabled: false,
            jump_host: None,
            jump_port: None,
            jump_user: None,
            jump_identity_file: None,
            local_host: Some("127.0.0.1".to_string()),
            local_port: 18080,
            remote_host: Some("127.0.0.1".to_string()),
            remote_port: Some(80),
            start_with_app: false,
            auto_reconnect: false,
            retry_count: 3,
            retry_interval: 5,
        }
    }

    #[test]
    fn resolve_direct_diagnostic_target() {
        let tunnel = base_tunnel();
        let target = resolve_diagnostic_target(&tunnel);

        assert_eq!(target.host, "example.test");
        assert_eq!(target.port, 22);
        assert_eq!(target.user, "root");
        assert_eq!(target.identity_file.as_deref(), Some("/tmp/key"));
        assert_eq!(target.prefix, "");
    }
}
