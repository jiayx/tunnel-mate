use crate::config::Tunnel;
use serde::{Deserialize, Serialize};
use ssh2::Session;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticStep {
    pub name: String,
    pub status: String, // "success", "warning", "error"
    pub message: String,
}

pub fn run_diagnostics(tunnel: &Tunnel, passphrase: Option<&str>) -> Vec<DiagnosticStep> {
    let mut steps = Vec::new();

    // Step 1: Local Port Availability Check
    let local_host = tunnel
        .local_host
        .clone()
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let local_port = tunnel.local_port;

    match TcpListener::bind(format!("{}:{}", local_host, local_port)) {
        Ok(_) => {
            steps.push(DiagnosticStep {
                name: "Local Environment".to_string(),
                status: "success".to_string(),
                message: format!(
                    "Port {} is free and available to bind on {}",
                    local_port, local_host
                ),
            });
        }
        Err(e) => {
            steps.push(DiagnosticStep {
                name: "Local Environment".to_string(),
                status: "error".to_string(),
                message: format!("Port {} cannot be bound on {}: {}. It is likely in use by another application.", local_port, local_host, e),
            });
            // Stop early since local port is blocked
            return steps;
        }
    }

    // If Jump Host is enabled, the path goes via the Bastion. We diagnose the Bastion instead.
    let target_host = if tunnel.jump_host_enabled {
        tunnel.jump_host.as_deref().unwrap_or(&tunnel.ssh_host)
    } else {
        &tunnel.ssh_host
    };

    let target_port = if tunnel.jump_host_enabled {
        tunnel.jump_port.unwrap_or(22)
    } else {
        tunnel.ssh_port
    };

    let prefix = if tunnel.jump_host_enabled {
        "[Jump Host] "
    } else {
        ""
    };

    // Step 2: DNS Resolve
    let socket_addrs = match format!("{}:{}", target_host, target_port).to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                steps.push(DiagnosticStep {
                    name: format!("{}DNS Resolution", prefix),
                    status: "success".to_string(),
                    message: format!("Resolved {} to {}", target_host, addr.ip()),
                });
                Some(addr)
            } else {
                steps.push(DiagnosticStep {
                    name: format!("{}DNS Resolution", prefix),
                    status: "error".to_string(),
                    message: format!("Resolved {} to no IP addresses", target_host),
                });
                None
            }
        }
        Err(e) => {
            steps.push(DiagnosticStep {
                name: format!("{}DNS Resolution", prefix),
                status: "error".to_string(),
                message: format!("Failed to resolve hostname {}: {}", target_host, e),
            });
            None
        }
    };

    let addr = match socket_addrs {
        Some(a) => a,
        None => return steps, // Stop early if DNS fails
    };

    // Step 3: TCP Connection
    let tcp_stream = match TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
        Ok(stream) => {
            steps.push(DiagnosticStep {
                name: format!("{}TCP Connection", prefix),
                status: "success".to_string(),
                message: format!(
                    "Successfully established TCP socket to {}:{}",
                    target_host, target_port
                ),
            });
            Some(stream)
        }
        Err(e) => {
            steps.push(DiagnosticStep {
                name: format!("{}TCP Connection", prefix),
                status: "error".to_string(),
                message: format!(
                    "Failed to connect to {}:{}: {}",
                    target_host, target_port, e
                ),
            });
            None
        }
    };

    let tcp = match tcp_stream {
        Some(t) => t,
        None => return steps, // Stop early if TCP fails
    };

    // Step 4: SSH Protocol Handshake
    let ssh_session = match Session::new() {
        Ok(mut sess) => {
            sess.set_tcp_stream(tcp);
            match sess.handshake() {
                Ok(_) => {
                    steps.push(DiagnosticStep {
                        name: format!("{}SSH Handshake", prefix),
                        status: "success".to_string(),
                        message: "SSH handshake and banner exchange completed successfully"
                            .to_string(),
                    });
                    Some(sess)
                }
                Err(e) => {
                    steps.push(DiagnosticStep {
                        name: format!("{}SSH Handshake", prefix),
                        status: "error".to_string(),
                        message: format!("SSH Handshake failed: {}", e),
                    });
                    None
                }
            }
        }
        Err(e) => {
            steps.push(DiagnosticStep {
                name: format!("{}SSH Handshake", prefix),
                status: "error".to_string(),
                message: format!("Failed to create SSH session object: {}", e),
            });
            None
        }
    };

    let session = match ssh_session {
        Some(s) => s,
        None => return steps,
    };

    // Step 5: SSH Authentication
    let ssh_user = if tunnel.jump_host_enabled {
        tunnel.jump_user.as_deref().unwrap_or(&tunnel.ssh_user)
    } else {
        &tunnel.ssh_user
    };

    let ssh_identity = if tunnel.jump_host_enabled {
        tunnel
            .jump_identity_file
            .as_deref()
            .or(tunnel.ssh_identity_file.as_deref())
    } else {
        tunnel.ssh_identity_file.as_deref()
    };

    let mut auth_success = false;

    // Try Private Key if provided
    if let Some(key_path_str) = ssh_identity {
        let key_path = Path::new(key_path_str);
        if key_path.exists() {
            let res = session.userauth_pubkey_file(ssh_user, None, key_path, passphrase);
            if res.is_ok() {
                auth_success = true;
                steps.push(DiagnosticStep {
                    name: format!("{}SSH Authentication", prefix),
                    status: "success".to_string(),
                    message: format!(
                        "Authenticated successfully using private key: {}",
                        key_path_str
                    ),
                });
            } else {
                let err = res.err().unwrap();
                if err.code() == ssh2::ErrorCode::Session(-18)
                    || err.message().contains("passphrase")
                {
                    steps.push(DiagnosticStep {
                        name: format!("{}SSH Authentication", prefix),
                        status: "warning".to_string(),
                        message: "Private key is encrypted and requires a passphrase.".to_string(),
                    });
                } else {
                    steps.push(DiagnosticStep {
                        name: format!("{}SSH Authentication", prefix),
                        status: "error".to_string(),
                        message: format!(
                            "Authentication failed with key {}: {}",
                            key_path_str, err
                        ),
                    });
                }
            }
        } else {
            steps.push(DiagnosticStep {
                name: format!("{}SSH Authentication", prefix),
                status: "error".to_string(),
                message: format!("Private key file does not exist: {}", key_path_str),
            });
        }
    }

    // Try SSH Agent if private key was not provided or failed
    if !auth_success {
        if let Ok(mut agent) = session.agent() {
            if agent.connect().is_ok() && agent.list_identities().is_ok() {
                if let Ok(identities) = agent.identities() {
                    for identity in identities {
                        if agent.userauth(ssh_user, &identity).is_ok() {
                            auth_success = true;
                            steps.push(DiagnosticStep {
                                name: format!("{}SSH Authentication", prefix),
                                status: "success".to_string(),
                                message: "Authenticated successfully using ssh-agent".to_string(),
                            });
                            break;
                        }
                    }
                }
            }
        }
    }

    // Try default SSH keys if still not authenticated
    if !auth_success {
        if let Some(home) = dirs::home_dir() {
            let default_keys = vec![
                home.join(".ssh").join("id_ed25519"),
                home.join(".ssh").join("id_rsa"),
                home.join(".ssh").join("id_ecdsa"),
                home.join(".ssh").join("id_dsa"),
            ];

            for key_path in default_keys {
                if key_path.exists() {
                    let res = session.userauth_pubkey_file(ssh_user, None, &key_path, passphrase);
                    if res.is_ok() {
                        auth_success = true;
                        steps.push(DiagnosticStep {
                            name: format!("{}SSH Authentication", prefix),
                            status: "success".to_string(),
                            message: format!(
                                "Authenticated successfully using default key: {}",
                                key_path.display()
                            ),
                        });
                        break;
                    }
                }
            }
        }
    }

    if !auth_success
        && steps
            .iter()
            .all(|s| s.name != format!("{}SSH Authentication", prefix))
    {
        steps.push(DiagnosticStep {
            name: format!("{}SSH Authentication", prefix),
            status: "error".to_string(),
            message: "All authentication methods failed. Check your SSH username, key path, and passphrase.".to_string(),
        });
    }

    steps
}
