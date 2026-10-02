use crate::config::{Endpoint, ForwardSpec, Tunnel};
use crate::ssh::engine::{ConnectOptions, KnownHostsPolicy, SshSession};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::net::{lookup_host, TcpListener, TcpStream};
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticStep {
    pub name: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLanguage {
    Chinese,
    English,
}

fn label(
    language: DiagnosticLanguage,
    chinese: &'static str,
    english: &'static str,
) -> &'static str {
    if language == DiagnosticLanguage::Chinese {
        chinese
    } else {
        english
    }
}

pub async fn run_diagnostics(
    tunnel: &Tunnel,
    all_tunnels: &[Tunnel],
    passphrase: Option<&str>,
    language: DiagnosticLanguage,
    listener_is_current_tunnel: bool,
) -> Vec<DiagnosticStep> {
    let mut steps = Vec::new();

    if let Some(bind_target) = resolve_local_bind_check(tunnel) {
        if listener_is_current_tunnel {
            steps.push(success(
                label(language, "监听端口", "Listener availability"),
                if language == DiagnosticLanguage::Chinese {
                    format!(
                        "当前隧道正在监听 {}:{}，端口占用属于正常状态",
                        bind_target.host, bind_target.port
                    )
                } else {
                    format!(
                        "The current tunnel is listening on {}:{}; this port usage is expected",
                        bind_target.host, bind_target.port
                    )
                },
            ));
        } else {
            match TcpListener::bind((bind_target.host.as_str(), bind_target.port)).await {
                Ok(_) => steps.push(success(
                    label(language, "监听端口", "Listener availability"),
                    if language == DiagnosticLanguage::Chinese {
                        format!("监听地址 {}:{} 可用", bind_target.host, bind_target.port)
                    } else {
                        format!(
                            "Listener {}:{} is free and available to bind",
                            bind_target.host, bind_target.port
                        )
                    },
                )),
                Err(e) => {
                    steps.push(error(
                    label(language, "监听端口", "Listener availability"),
                    if language == DiagnosticLanguage::Chinese {
                        format!("无法绑定 {}:{}：{}。该端口可能已被其他应用占用。", bind_target.host, bind_target.port, e)
                    } else {
                        format!("Listener {}:{} cannot be bound: {}. It is likely in use by another application.", bind_target.host, bind_target.port, e)
                    },
                ));
                    return steps;
                }
            }
        }
    } else {
        let remote_bind = tunnel.forward.listen();
        steps.push(success(
            label(language, "远程监听端口", "Remote listener"),
            if language == DiagnosticLanguage::Chinese {
                format!("启动隧道时将在 SSH 服务器上请求监听 {}:{}", remote_bind.host, remote_bind.port)
            } else {
                format!(
                    "Remote listener {}:{} will be requested on the SSH server when the tunnel starts",
                    remote_bind.host, remote_bind.port
                )
            },
        ));
    }

    if let Some(forward_target) = resolve_forward_tcp_check(tunnel) {
        if !check_tcp_endpoint(&forward_target, language, &mut steps).await {
            return steps;
        }
    }

    let jump_config = match tunnel.resolve_jump_host(all_tunnels) {
        Ok(jump_config) => jump_config,
        Err(message) => {
            steps.push(error(label(language, "跳板机", "Jump host"), message));
            return steps;
        }
    };
    let ssh_target = jump_config.as_ref().unwrap_or(tunnel);
    let target = DiagnosticEndpoint {
        host: ssh_target.ssh_host.clone(),
        port: ssh_target.ssh_port,
        prefix: if jump_config.is_some() {
            "[Jump Host] "
        } else {
            ""
        },
    };
    if !check_tcp_endpoint(&target, language, &mut steps).await {
        return steps;
    }

    match SshSession::connect(ConnectOptions {
        host: &tunnel.ssh_host,
        port: tunnel.ssh_port,
        user: &tunnel.ssh_user,
        identity_file: tunnel.ssh_identity_file.as_deref(),
        password: tunnel.ssh_password.as_deref(),
        passphrase,
        known_hosts_policy: KnownHostsPolicy::TrustOnce,
        jump_host_config: jump_config.as_ref(),
    })
    .await
    {
        Ok(mut session) => {
            steps.push(success(
                format!(
                    "{}{}",
                    target.prefix,
                    label(language, "SSH 认证", "SSH authentication")
                ),
                label(
                    language,
                    "SSH 握手和身份认证成功",
                    "SSH handshake and authentication completed successfully",
                )
                .to_string(),
            ));
            session.disconnect().await;
        }
        Err(err) if err == "PASSPHRASE_REQUIRED" => steps.push(warning(
            format!(
                "{}{}",
                target.prefix,
                label(language, "SSH 认证", "SSH authentication")
            ),
            label(
                language,
                "私钥已加密，需要输入口令。",
                "Private key is encrypted and requires a passphrase.",
            )
            .to_string(),
        )),
        Err(err) => steps.push(error(
            format!(
                "{}{}",
                target.prefix,
                label(language, "SSH 认证", "SSH authentication")
            ),
            if language == DiagnosticLanguage::Chinese {
                format!("SSH 认证失败：{}", err)
            } else {
                format!("SSH authentication failed: {}", err)
            },
        )),
    }

    steps
}

async fn check_tcp_endpoint(
    target: &DiagnosticEndpoint,
    language: DiagnosticLanguage,
    steps: &mut Vec<DiagnosticStep>,
) -> bool {
    let addr = match lookup_host((target.host.as_str(), target.port)).await {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                steps.push(success(
                    format!(
                        "{}{}",
                        target.prefix,
                        label(language, "DNS 解析", "DNS resolution")
                    ),
                    if language == DiagnosticLanguage::Chinese {
                        format!("已将 {} 解析为 {}", target.host, addr.ip())
                    } else {
                        format!("Resolved {} to {}", target.host, addr.ip())
                    },
                ));
                addr
            } else {
                steps.push(error(
                    format!(
                        "{}{}",
                        target.prefix,
                        label(language, "DNS 解析", "DNS resolution")
                    ),
                    if language == DiagnosticLanguage::Chinese {
                        format!("{} 没有解析到任何 IP 地址", target.host)
                    } else {
                        format!("Resolved {} to no IP addresses", target.host)
                    },
                ));
                return false;
            }
        }
        Err(e) => {
            steps.push(error(
                format!(
                    "{}{}",
                    target.prefix,
                    label(language, "DNS 解析", "DNS resolution")
                ),
                if language == DiagnosticLanguage::Chinese {
                    format!("无法解析主机名 {}：{}", target.host, e)
                } else {
                    format!("Failed to resolve hostname {}: {}", target.host, e)
                },
            ));
            return false;
        }
    };

    match timeout(Duration::from_secs(5), TcpStream::connect(addr)).await {
        Ok(Ok(_)) => steps.push(success(
            format!(
                "{}{}",
                target.prefix,
                label(language, "TCP 连接", "TCP connection")
            ),
            if language == DiagnosticLanguage::Chinese {
                format!("已成功连接 {}:{}", target.host, target.port)
            } else {
                format!(
                    "Successfully established TCP socket to {}:{}",
                    target.host, target.port
                )
            },
        )),
        Ok(Err(e)) => {
            steps.push(error(
                format!(
                    "{}{}",
                    target.prefix,
                    label(language, "TCP 连接", "TCP connection")
                ),
                if language == DiagnosticLanguage::Chinese {
                    format!("无法连接 {}:{}：{}", target.host, target.port, e)
                } else {
                    format!(
                        "Failed to connect to {}:{}: {}",
                        target.host, target.port, e
                    )
                },
            ));
            return false;
        }
        Err(_) => {
            steps.push(error(
                format!(
                    "{}{}",
                    target.prefix,
                    label(language, "TCP 连接", "TCP connection")
                ),
                if language == DiagnosticLanguage::Chinese {
                    format!("连接 {}:{} 超时", target.host, target.port)
                } else {
                    format!("Connection to {}:{} timed out", target.host, target.port)
                },
            ));
            return false;
        }
    }

    true
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
            jump_host_id: None,
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
}
