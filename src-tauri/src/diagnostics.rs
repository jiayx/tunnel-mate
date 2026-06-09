use crate::config::{Endpoint, ForwardSpec, Tunnel};
use crate::ssh::engine::{ConnectOptions, KnownHostsPolicy, SshSession};
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

    if let Some(bind_target) = resolve_local_bind_check(tunnel) {
        match TcpListener::bind(format!("{}:{}", bind_target.host, bind_target.port)) {
            Ok(_) => steps.push(success(
                "Listener Availability",
                format!(
                    "Listener {}:{} is free and available to bind",
                    bind_target.host, bind_target.port
                ),
            )),
            Err(e) => {
                steps.push(error(
                    "Listener Availability",
                    format!("Listener {}:{} cannot be bound: {}. It is likely in use by another application.", bind_target.host, bind_target.port, e),
                ));
                return steps;
            }
        }
    } else {
        let remote_bind = resolve_remote_listener(tunnel);
        steps.push(success(
            "Remote Listener",
            format!(
                "Remote listener {}:{} will be requested on the SSH server when the tunnel starts",
                remote_bind.host, remote_bind.port
            ),
        ));
    }

    if let Some(forward_target) = resolve_forward_tcp_check(tunnel) {
        let addr =
            match format!("{}:{}", forward_target.host, forward_target.port).to_socket_addrs() {
                Ok(mut addrs) => {
                    if let Some(addr) = addrs.next() {
                        steps.push(success(
                            format!("{}DNS Resolution", forward_target.prefix),
                            format!("Resolved {} to {}", forward_target.host, addr.ip()),
                        ));
                        addr
                    } else {
                        steps.push(error(
                            format!("{}DNS Resolution", forward_target.prefix),
                            format!("Resolved {} to no IP addresses", forward_target.host),
                        ));
                        return steps;
                    }
                }
                Err(e) => {
                    steps.push(error(
                        format!("{}DNS Resolution", forward_target.prefix),
                        format!("Failed to resolve hostname {}: {}", forward_target.host, e),
                    ));
                    return steps;
                }
            };

        match TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
            Ok(_) => steps.push(success(
                format!("{}TCP Connection", forward_target.prefix),
                format!(
                    "Successfully established TCP socket to {}:{}",
                    forward_target.host, forward_target.port
                ),
            )),
            Err(e) => {
                steps.push(error(
                    format!("{}TCP Connection", forward_target.prefix),
                    format!(
                        "Failed to connect to {}:{}: {}",
                        forward_target.host, forward_target.port, e
                    ),
                ));
                return steps;
            }
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

    match SshSession::connect(ConnectOptions {
        host: &target.host,
        port: target.port,
        user: &target.user,
        identity_file: target.identity_file.as_deref(),
        password: target.password.as_deref(),
        passphrase,
        known_hosts_policy: KnownHostsPolicy::TrustOnce,
        jump_host_config: None,
    })
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
    password: Option<String>,
    prefix: &'static str,
}

#[derive(Debug, PartialEq, Eq)]
struct DiagnosticEndpoint {
    host: String,
    port: u16,
    prefix: &'static str,
}

fn resolve_local_bind_check(tunnel: &Tunnel) -> Option<DiagnosticEndpoint> {
    match &tunnel.forward {
        ForwardSpec::Local { listen, .. } | ForwardSpec::Socks5 { listen } => {
            Some(endpoint_to_diagnostic(listen, ""))
        }
        ForwardSpec::Remote { .. } => None,
    }
}

fn resolve_remote_listener(tunnel: &Tunnel) -> DiagnosticEndpoint {
    endpoint_to_diagnostic(tunnel.forward.listen(), "")
}

fn resolve_forward_tcp_check(tunnel: &Tunnel) -> Option<DiagnosticEndpoint> {
    match &tunnel.forward {
        ForwardSpec::Remote { target, .. } => Some(endpoint_to_diagnostic(target, "[Target] ")),
        ForwardSpec::Local { .. } | ForwardSpec::Socks5 { .. } => None,
    }
}

fn endpoint_to_diagnostic(endpoint: &Endpoint, prefix: &'static str) -> DiagnosticEndpoint {
    DiagnosticEndpoint {
        host: endpoint.host.clone(),
        port: endpoint.port,
        prefix,
    }
}

fn resolve_diagnostic_target(tunnel: &Tunnel) -> DiagnosticTarget {
    if tunnel.jump_host_enabled {
        DiagnosticTarget {
            host: tunnel.jump_host.clone().unwrap_or_default(),
            port: tunnel.jump_port.unwrap_or_default(),
            user: tunnel.jump_user.clone().unwrap_or_default(),
            identity_file: tunnel.jump_identity_file.clone(),
            password: tunnel.jump_password.clone(),
            prefix: "[Jump Host] ",
        }
    } else {
        DiagnosticTarget {
            host: tunnel.ssh_host.clone(),
            port: tunnel.ssh_port,
            user: tunnel.ssh_user.clone(),
            identity_file: tunnel.ssh_identity_file.clone(),
            password: tunnel.ssh_password.clone(),
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

    fn endpoint(host: &str, port: u16) -> Endpoint {
        Endpoint {
            host: host.to_string(),
            port,
        }
    }

    fn base_tunnel(forward: ForwardSpec) -> Tunnel {
        Tunnel {
            id: "t1".to_string(),
            name: "test".to_string(),
            description: None,
            group_id: None,
            ssh_host: "example.test".to_string(),
            ssh_port: 22,
            ssh_user: "root".to_string(),
            ssh_identity_file: Some("/tmp/key".to_string()),
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
    fn remote_diagnostics_skip_local_bind_and_check_target() {
        let tunnel = base_tunnel(ForwardSpec::Remote {
            listen: endpoint("0.0.0.0", 18080),
            target: endpoint("127.0.0.1", 3000),
        });

        assert_eq!(resolve_local_bind_check(&tunnel), None);
        assert_eq!(
            resolve_remote_listener(&tunnel),
            DiagnosticEndpoint {
                host: "0.0.0.0".to_string(),
                port: 18080,
                prefix: "",
            }
        );
        assert_eq!(
            resolve_forward_tcp_check(&tunnel),
            Some(DiagnosticEndpoint {
                host: "127.0.0.1".to_string(),
                port: 3000,
                prefix: "[Target] ",
            })
        );
    }

    #[test]
    fn local_diagnostics_keep_local_bind_and_skip_target_precheck() {
        let tunnel = base_tunnel(ForwardSpec::Local {
            listen: endpoint("127.0.0.1", 18080),
            target: endpoint("127.0.0.1", 80),
        });

        assert_eq!(
            resolve_local_bind_check(&tunnel),
            Some(DiagnosticEndpoint {
                host: "127.0.0.1".to_string(),
                port: 18080,
                prefix: "",
            })
        );
        assert_eq!(resolve_forward_tcp_check(&tunnel), None);
    }

    #[test]
    fn socks5_diagnostics_have_local_bind_but_no_target() {
        let tunnel = base_tunnel(ForwardSpec::Socks5 {
            listen: endpoint("127.0.0.1", 1080),
        });

        assert!(resolve_local_bind_check(&tunnel).is_some());
        assert_eq!(resolve_forward_tcp_check(&tunnel), None);
    }

    #[test]
    fn resolve_direct_diagnostic_target() {
        let tunnel = base_tunnel(ForwardSpec::Local {
            listen: endpoint("127.0.0.1", 18080),
            target: endpoint("127.0.0.1", 80),
        });
        let target = resolve_diagnostic_target(&tunnel);

        assert_eq!(target.host, "example.test");
        assert_eq!(target.port, 22);
        assert_eq!(target.user, "root");
        assert_eq!(target.identity_file.as_deref(), Some("/tmp/key"));
        assert_eq!(target.prefix, "");
    }
}
