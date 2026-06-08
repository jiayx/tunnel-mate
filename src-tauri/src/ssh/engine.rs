use crate::config::{ConfigStore, Tunnel};
use russh::client::{self, Config, Handle, Msg};
use russh::keys::agent::client::AgentClient;
use russh::keys::key::PrivateKeyWithHashAlg;
use russh::keys::{load_secret_key, ssh_key};
use russh::{Channel, Disconnect};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};

pub type SshHandle = Handle<TunnelClient>;
pub type SharedSshHandle = Arc<Mutex<SshHandle>>;
pub type ForwardedChannel = Channel<Msg>;

#[derive(Debug)]
pub struct ForwardedTcp {
    pub channel: ForwardedChannel,
    pub connected_address: String,
    pub connected_port: u32,
    pub originator_address: String,
    pub originator_port: u32,
}

#[derive(Clone)]
pub struct TunnelClient {
    forwarded_tx: mpsc::UnboundedSender<ForwardedTcp>,
}

impl TunnelClient {
    fn new(forwarded_tx: mpsc::UnboundedSender<ForwardedTcp>) -> Self {
        Self { forwarded_tx }
    }
}

impl client::Handler for TunnelClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: ForwardedChannel,
        connected_address: &str,
        connected_port: u32,
        originator_address: &str,
        originator_port: u32,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        let _ = self.forwarded_tx.send(ForwardedTcp {
            channel,
            connected_address: connected_address.to_string(),
            connected_port,
            originator_address: originator_address.to_string(),
            originator_port,
        });
        Ok(())
    }
}

pub struct SshSession {
    handle: SharedSshHandle,
    forwarded_rx: Option<mpsc::UnboundedReceiver<ForwardedTcp>>,
    _bastion: Option<Box<SshSession>>,
}

impl SshSession {
    pub async fn connect(
        host: &str,
        port: u16,
        user: &str,
        identity_file: Option<&str>,
        password: Option<&str>,
        passphrase: Option<&str>,
        _known_hosts_policy: &str,
        jump_host_config: Option<&Tunnel>,
    ) -> Result<Self, String> {
        let store = ConfigStore::new();
        let timeout_secs = store
            .load_config()
            .ok()
            .and_then(|cfg| cfg.settings)
            .map(|s| s.connect_timeout)
            .unwrap_or(15) as u64;
        let keep_alive = store
            .load_config()
            .ok()
            .and_then(|cfg| cfg.settings)
            .map(|s| s.keep_alive_interval)
            .unwrap_or(30);

        let inactivity_timeout = if keep_alive > 0 {
            Some(Duration::from_secs(keep_alive as u64 * 3))
        } else {
            None
        };

        let config = Arc::new(Config {
            nodelay: true,
            inactivity_timeout,
            keepalive_interval: if keep_alive > 0 {
                Some(Duration::from_secs(keep_alive as u64))
            } else {
                None
            },
            ..Default::default()
        });

        let (forwarded_tx, forwarded_rx) = mpsc::unbounded_channel();
        let handler = TunnelClient::new(forwarded_tx);

        let (mut handle, bastion) = if let Some(jump) = jump_host_config {
            let jump_session = Box::new(
                Box::pin(SshSession::connect(
                    &jump.ssh_host,
                    jump.ssh_port,
                    &jump.ssh_user,
                    jump.ssh_identity_file.as_deref(),
                    jump.ssh_password.as_deref(),
                    passphrase,
                    _known_hosts_policy,
                    None,
                ))
                .await?,
            );
            let channel = jump_session
                .handle
                .lock()
                .await
                .channel_open_direct_tcpip(host.to_string(), port as u32, "127.0.0.1", 0)
                .await
                .map_err(|e| format!("Jump Host channel failed: {}", e))?;
            let stream = channel.into_stream();
            let handle = client::connect_stream(config, stream, handler)
                .await
                .map_err(|e| format!("SSH connection via Jump Host failed: {}", e))?;
            (handle, Some(jump_session))
        } else {
            let handle = tokio::time::timeout(
                Duration::from_secs(timeout_secs.max(1)),
                client::connect(config, (host, port), handler),
            )
            .await
            .map_err(|_| format!("SSH connection to {}:{} timed out", host, port))?
            .map_err(|e| format!("SSH connection failed: {}", e))?;
            (handle, None)
        };

        authenticate_handle(&mut handle, user, identity_file, password, passphrase).await?;

        Ok(Self {
            handle: Arc::new(Mutex::new(handle)),
            forwarded_rx: Some(forwarded_rx),
            _bastion: bastion,
        })
    }

    pub fn handle(&self) -> SharedSshHandle {
        self.handle.clone()
    }

    pub fn take_forwarded_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<ForwardedTcp>> {
        self.forwarded_rx.take()
    }

    pub async fn is_alive(&self) -> bool {
        !self.handle.lock().await.is_closed()
    }

    pub async fn disconnect(&mut self) {
        let _ = self
            .handle
            .lock()
            .await
            .disconnect(Disconnect::ByApplication, "Tunnel stopped by user", "en")
            .await;
        if let Some(bastion) = self._bastion.as_mut() {
            Box::pin(bastion.disconnect()).await;
        }
    }
}

async fn authenticate_handle(
    handle: &mut SshHandle,
    user: &str,
    identity_file: Option<&str>,
    password: Option<&str>,
    passphrase: Option<&str>,
) -> Result<(), String> {
    if let Some(path) = identity_file {
        authenticate_key_file(handle, user, PathBuf::from(path), passphrase).await?;
        return Ok(());
    }

    if let Some(password) = password.filter(|value| !value.is_empty()) {
        authenticate_password(handle, user, password).await?;
        return Ok(());
    }

    if try_agent_auth(handle, user).await? {
        return Ok(());
    }

    for key_path in default_key_candidates() {
        if key_path.exists()
            && authenticate_key_file(handle, user, key_path, passphrase)
                .await
                .is_ok()
        {
            return Ok(());
        }
    }

    Err("All authentication methods failed. Please provide a valid SSH password, SSH Agent, or Private Key configuration.".to_string())
}

async fn authenticate_password(
    handle: &mut SshHandle,
    user: &str,
    password: &str,
) -> Result<(), String> {
    let auth = handle
        .authenticate_password(user, password)
        .await
        .map_err(|e| format!("Password authentication failed: {}", e))?;

    if auth.success() {
        Ok(())
    } else {
        Err("Password authentication failed".to_string())
    }
}

async fn authenticate_key_file(
    handle: &mut SshHandle,
    user: &str,
    key_path: PathBuf,
    passphrase: Option<&str>,
) -> Result<(), String> {
    if !key_path.exists() {
        return Err(format!(
            "Private key file does not exist: {}",
            key_path.display()
        ));
    }

    let key = load_secret_key(&key_path, passphrase).map_err(|e| {
        let message = e.to_string();
        if message.to_lowercase().contains("passphrase")
            || message.to_lowercase().contains("encrypted")
        {
            "PASSPHRASE_REQUIRED".to_string()
        } else {
            format!(
                "Failed to load private key {}: {}",
                key_path.display(),
                message
            )
        }
    })?;

    let hash_alg = handle
        .best_supported_rsa_hash()
        .await
        .map_err(|e| format!("Failed to negotiate RSA signature algorithm: {}", e))?
        .flatten();
    let auth = handle
        .authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg))
        .await
        .map_err(|e| format!("Private key authentication failed: {}", e))?;

    if auth.success() {
        Ok(())
    } else {
        Err("Private key authentication failed".to_string())
    }
}

async fn try_agent_auth(handle: &mut SshHandle, user: &str) -> Result<bool, String> {
    #[cfg(unix)]
    {
        let mut agent = match AgentClient::connect_env().await {
            Ok(agent) => agent,
            Err(_) => return Ok(false),
        };

        let identities = match agent.request_identities().await {
            Ok(ids) => ids,
            Err(_) => return Ok(false),
        };

        let hash_alg = handle
            .best_supported_rsa_hash()
            .await
            .map_err(|e| format!("Failed to negotiate RSA signature algorithm: {}", e))?
            .flatten();

        for identity in identities {
            let public_key = identity.public_key().into_owned();
            match handle
                .authenticate_publickey_with(user, public_key, hash_alg, &mut agent)
                .await
            {
                Ok(auth) if auth.success() => return Ok(true),
                Ok(_) => {}
                Err(_) => {}
            }
        }
    }

    #[cfg(not(unix))]
    {
        let _ = handle;
        let _ = user;
    }

    Ok(false)
}

pub fn default_key_candidates() -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };

    ["id_ed25519", "id_rsa", "id_ecdsa", "id_dsa"]
        .iter()
        .map(|name| home.join(".ssh").join(name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_key_candidates_keep_ssh_order() {
        let names: Vec<_> = default_key_candidates()
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .collect();

        assert_eq!(names, ["id_ed25519", "id_rsa", "id_ecdsa", "id_dsa"]);
    }
}
