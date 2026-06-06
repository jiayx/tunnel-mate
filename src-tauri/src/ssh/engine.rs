use crate::config::{ConfigStore, Tunnel};
use ssh2::{KnownHostFileKind, Session};
use std::fs;
use std::net::TcpStream;
use std::path::Path;

pub struct SshSession {
    pub session: Session,
    _tcp: TcpStream,
    _bastion: Option<Box<SshSession>>,
}

impl SshSession {
    pub fn connect(
        host: &str,
        port: u16,
        user: &str,
        identity_file: Option<&str>,
        passphrase: Option<&str>,
        known_hosts_policy: &str, // "strict", "trustOnce", "trustPermanently"
        jump_host_config: Option<&Tunnel>,
    ) -> Result<Self, String> {
        let store = ConfigStore::new();
        let known_hosts_path = store.get_known_hosts_path();

        let timeout_secs = store
            .load_config()
            .ok()
            .and_then(|cfg| cfg.settings)
            .map(|s| s.connect_timeout)
            .unwrap_or(15) as u64;
        let timeout = std::time::Duration::from_secs(timeout_secs);

        // 1. Establish the connection (either direct or via Jump Host)
        let (tcp, bastion) = if let Some(jump) = jump_host_config {
            // Connect to Jump Host first
            let jump_session = SshSession::connect(
                &jump.ssh_host,
                jump.ssh_port,
                &jump.ssh_user,
                jump.ssh_identity_file.as_deref(),
                None, // We can assume no passphrase or we could extend this
                known_hosts_policy,
                None,
            )?;

            // Bind a temporary listener on localhost to bridge the connection
            let listener = std::net::TcpListener::bind("127.0.0.1:0")
                .map_err(|e| format!("Jump Host bridge: failed to bind local listener: {}", e))?;
            let local_addr = listener
                .local_addr()
                .map_err(|e| format!("Jump Host bridge: failed to get local addr: {}", e))?;

            let target_host = host.to_string();
            let target_port = port;
            let bastion_sess = jump_session.session.clone();

            // Spawn a bridge task on a separate thread to handle the single connection
            std::thread::spawn(move || {
                if let Ok((mut local_stream, _)) = listener.accept() {
                    if let Ok(mut channel) =
                        bastion_sess.channel_direct_tcpip(&target_host, target_port, None)
                    {
                        let local_clone = local_stream.try_clone().ok();
                        let mut channel_clone = channel.clone();

                        // Pipe local to channel
                        std::thread::spawn(move || {
                            if let Some(mut lc) = local_clone {
                                std::io::copy(&mut lc, &mut channel_clone).ok();
                            }
                        });

                        // Pipe channel to local
                        std::io::copy(&mut channel, &mut local_stream).ok();
                    }
                }
            });

            // Connect target TCP stream to our local bridge
            let target_tcp = TcpStream::connect(local_addr)
                .map_err(|e| format!("Jump Host bridge: failed to connect to local port: {}", e))?;

            (target_tcp, Some(Box::new(jump_session)))
        } else {
            // Direct connection with timeout and DNS resolution loop
            use std::net::ToSocketAddrs;
            let addrs = (host, port)
                .to_socket_addrs()
                .map_err(|e| format!("Failed to resolve {}:{}: {}", host, port, e))?;

            let mut target_tcp = None;
            let mut last_err = None;
            for addr in addrs {
                match TcpStream::connect_timeout(&addr, timeout) {
                    Ok(stream) => {
                        target_tcp = Some(stream);
                        break;
                    }
                    Err(e) => {
                        last_err = Some(e);
                    }
                }
            }
            let target_tcp = target_tcp.ok_or_else(|| {
                format!(
                    "Failed to connect to {}:{}: {}",
                    host,
                    port,
                    last_err
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "No addresses found".to_string())
                )
            })?;
            (target_tcp, None)
        };

        // 2. Initialize SSH session
        let mut session =
            Session::new().map_err(|e| format!("Failed to create SSH session: {}", e))?;
        session.set_tcp_stream(
            tcp.try_clone()
                .map_err(|e| format!("Failed to clone TCP socket: {}", e))?,
        );

        let keep_alive = store
            .load_config()
            .ok()
            .and_then(|cfg| cfg.settings)
            .map(|s| s.keep_alive_interval)
            .unwrap_or(30);

        session
            .handshake()
            .map_err(|e| format!("SSH Handshake failed: {}", e))?;

        // 3. Verify Host Key
        verify_host_key(&session, host, port, &known_hosts_path, known_hosts_policy)?;

        // 4. Authenticate
        authenticate_session(&session, user, identity_file, passphrase)?;

        if keep_alive > 0 {
            session.set_keepalive(true, keep_alive);
        }

        Ok(Self {
            session,
            _tcp: tcp,
            _bastion: bastion,
        })
    }
}

fn verify_host_key(
    session: &Session,
    host: &str,
    port: u16,
    known_hosts_path: &Path,
    policy: &str,
) -> Result<(), String> {
    let mut known_hosts = session
        .known_hosts()
        .map_err(|e| format!("Failed to get known hosts: {}", e))?;

    // Create known_hosts file if it doesn't exist
    if !known_hosts_path.exists() {
        if let Some(parent) = known_hosts_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::File::create(known_hosts_path);
    }

    known_hosts
        .read_file(known_hosts_path, KnownHostFileKind::OpenSSH)
        .ok();

    let (key, key_type) = session
        .host_key()
        .ok_or("Failed to retrieve remote host key")?;

    // Check key against known hosts
    let check_result = known_hosts.check_port(host, port, key);

    match check_result {
        ssh2::CheckResult::Match => Ok(()),
        ssh2::CheckResult::NotFound => {
            if policy == "strict" {
                return Err(format!(
                    "Host key verification failed. Host {} is not in known_hosts file and policy is strict.",
                    host
                ));
            }

            // Trust once or permanently
            if policy == "trustPermanently" {
                known_hosts.add(host, key, &format!("added by TunnelMate for {}:{}", host, port), key_type.into())
                    .map_err(|e| format!("Failed to add host key to memory: {}", e))?;
                known_hosts.write_file(known_hosts_path, KnownHostFileKind::OpenSSH)
                    .map_err(|e| format!("Failed to write known_hosts file: {}", e))?;
            }
            Ok(())
        }
        ssh2::CheckResult::Mismatch => {
            Err(format!(
                "WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED! Host key mismatch for {}:{}. Connection refused.",
                host, port
            ))
        }
        ssh2::CheckResult::Failure => Err("Host key check encountered a system failure.".to_string()),
    }
}

fn authenticate_session(
    session: &Session,
    user: &str,
    identity_file: Option<&str>,
    passphrase: Option<&str>,
) -> Result<(), String> {
    // 1. If a private key file is provided, use it
    if let Some(key_path_str) = identity_file {
        let key_path = Path::new(key_path_str);
        if !key_path.exists() {
            return Err(format!("Private key file does not exist: {}", key_path_str));
        }

        let res = session.userauth_pubkey_file(user, None, key_path, passphrase);
        if res.is_ok() {
            return Ok(());
        }

        let err = res.err().unwrap();
        if err.code() == ssh2::ErrorCode::Session(-18) || err.message().contains("passphrase") {
            return Err("PASSPHRASE_REQUIRED".to_string());
        }

        return Err(format!("Private key authentication failed: {}", err));
    }

    // 2. Try SSH Agent
    if let Ok(mut agent) = session.agent() {
        if agent.connect().is_ok() {
            if agent.list_identities().is_ok() {
                if let Ok(identities) = agent.identities() {
                    for identity in identities {
                        if agent.userauth(user, &identity).is_ok() {
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    // 3. Try default SSH keys
    if let Some(home) = dirs::home_dir() {
        let default_keys = vec![
            home.join(".ssh").join("id_ed25519"),
            home.join(".ssh").join("id_rsa"),
            home.join(".ssh").join("id_ecdsa"),
            home.join(".ssh").join("id_dsa"),
        ];

        for key_path in default_keys {
            if key_path.exists() {
                let res = session.userauth_pubkey_file(user, None, &key_path, passphrase);
                if res.is_ok() {
                    return Ok(());
                }
                let err = res.err().unwrap();
                if err.code() == ssh2::ErrorCode::Session(-18)
                    || err.message().contains("passphrase")
                {
                    return Err("PASSPHRASE_REQUIRED".to_string());
                }
            }
        }
    }

    Err("All authentication methods failed. Please provide a valid SSH Agent or Private Key configuration.".to_string())
}
