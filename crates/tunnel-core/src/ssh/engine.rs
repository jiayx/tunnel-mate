use super::known_hosts::{
    append_known_host, check_known_host_key, host_key_error, known_hosts_path,
    remove_known_host_entries, HostKeyStatus,
};
use crate::config::{ConfigStore, Tunnel};
use russh::client::{self, Config, Handle, Msg};
use russh::keys::agent::client::AgentClient;
use russh::keys::agent::client::AgentStream;
use russh::keys::key::PrivateKeyWithHashAlg;
use russh::keys::{load_secret_key, ssh_key};
use russh::{Channel, Disconnect};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
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
    host: String,
    port: u16,
    known_hosts_policy: KnownHostsPolicy,
    host_key_error: Arc<StdMutex<Option<String>>>,
    expected_fingerprint: Option<String>,
}

impl TunnelClient {
    fn new(
        forwarded_tx: mpsc::UnboundedSender<ForwardedTcp>,
        host: String,
        port: u16,
        known_hosts_policy: KnownHostsPolicy,
        host_key_error: Arc<StdMutex<Option<String>>>,
        expected_fingerprint: Option<String>,
    ) -> Self {
        Self {
            forwarded_tx,
            host,
            port,
            known_hosts_policy,
            host_key_error,
            expected_fingerprint,
        }
    }
}

impl client::Handler for TunnelClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        let actual_fingerprint = server_public_key
            .fingerprint(ssh_key::HashAlg::Sha256)
            .to_string();
        if let Some(expected) = &self.expected_fingerprint {
            if expected != &actual_fingerprint {
                self.set_host_key_error(format!(
                    "The SSH host key changed again while confirming it (expected {expected}, received {actual_fingerprint}). Connection blocked."
                ));
                return Ok(false);
            }
        }
        match self.known_hosts_policy {
            KnownHostsPolicy::TrustOnce => Ok(true),
            KnownHostsPolicy::TrustAndRemember => {
                match check_known_host_key(&self.host, self.port, server_public_key) {
                    HostKeyStatus::Trusted => Ok(true),
                    HostKeyStatus::Unknown => {
                        match append_known_host(&self.host, self.port, server_public_key) {
                            Ok(()) => Ok(true),
                            Err(message) => {
                                self.set_host_key_error(message);
                                Ok(false)
                            }
                        }
                    }
                    HostKeyStatus::Changed => {
                        self.set_host_key_error(host_key_error(
                            "HOST_KEY_CHANGED",
                            &self.host,
                            self.port,
                            server_public_key,
                        ));
                        Ok(false)
                    }
                    HostKeyStatus::Revoked => {
                        self.set_host_key_error(host_key_error(
                            "HOST_KEY_REVOKED",
                            &self.host,
                            self.port,
                            server_public_key,
                        ));
                        Ok(false)
                    }
                }
            }
            KnownHostsPolicy::RequireKnown => {
                match check_known_host_key(&self.host, self.port, server_public_key) {
                    HostKeyStatus::Trusted => Ok(true),
                    HostKeyStatus::Unknown => {
                        self.set_host_key_error(host_key_error(
                            "HOST_KEY_NOT_TRUSTED",
                            &self.host,
                            self.port,
                            server_public_key,
                        ));
                        Ok(false)
                    }
                    HostKeyStatus::Changed => {
                        self.set_host_key_error(host_key_error(
                            "HOST_KEY_CHANGED",
                            &self.host,
                            self.port,
                            server_public_key,
                        ));
                        Ok(false)
                    }
                    HostKeyStatus::Revoked => {
                        self.set_host_key_error(host_key_error(
                            "HOST_KEY_REVOKED",
                            &self.host,
                            self.port,
                            server_public_key,
                        ));
                        Ok(false)
                    }
                }
            }
        }
    }

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: ForwardedChannel,
        connected_address: &str,
        connected_port: u32,
        originator_address: &str,
        originator_port: u32,
        reply: client::ChannelOpenHandle,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        if self.forwarded_tx.is_closed() {
            return Ok(());
        }

        reply.accept().await;
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

impl TunnelClient {
    fn set_host_key_error(&self, message: String) {
        if let Ok(mut error) = self.host_key_error.lock() {
            *error = Some(message);
        }
    }
}

pub struct SshSession {
    handle: SharedSshHandle,
    forwarded_rx: Option<mpsc::UnboundedReceiver<ForwardedTcp>>,
    _bastion: Option<Box<SshSession>>,
}

pub struct ConnectOptions<'a> {
    pub host: &'a str,
    pub port: u16,
    pub user: &'a str,
    pub identity_file: Option<&'a str>,
    pub password: Option<&'a str>,
    pub passphrase: Option<&'a str>,
    pub known_hosts_policy: KnownHostsPolicy,
    pub jump_host_config: Option<&'a Tunnel>,
}

#[derive(Clone, Copy)]
pub enum KnownHostsPolicy {
    TrustOnce,
    TrustAndRemember,
    RequireKnown,
}

fn client_config(keep_alive_interval: u32) -> Config {
    Config {
        nodelay: true,
        // Keepalive replies already prove that an otherwise idle SSH session is
        // healthy. Treating application inactivity as a failure caused valid
        // tunnels to be closed simply because no forwarded traffic was flowing.
        inactivity_timeout: None,
        keepalive_interval: if keep_alive_interval > 0 {
            Some(Duration::from_secs(keep_alive_interval as u64))
        } else {
            None
        },
        ..Default::default()
    }
}

fn session_end_message(result: Result<(), russh::Error>) -> String {
    match result {
        Ok(()) => "SSH session ended".to_string(),
        Err(russh::Error::KeepaliveTimeout) => "SSH keepalive timed out".to_string(),
        Err(russh::Error::InactivityTimeout) => "SSH session inactivity timed out".to_string(),
        Err(russh::Error::HUP) => "SSH connection closed by remote host".to_string(),
        Err(russh::Error::Disconnect) => "SSH session disconnected by remote host".to_string(),
        Err(error) => format!("SSH session disconnected: {error}"),
    }
}

impl SshSession {
    pub async fn connect(options: ConnectOptions<'_>) -> Result<Self, String> {
        let ConnectOptions {
            host,
            port,
            user,
            identity_file,
            password,
            passphrase,
            known_hosts_policy,
            jump_host_config,
        } = options;
        let settings = ConfigStore::new()
            .load_config()
            .map(|cfg| cfg.settings)
            .unwrap_or_default();
        let timeout_secs = settings.connect_timeout as u64;
        let keep_alive = settings.keep_alive_interval;
        let config = Arc::new(client_config(keep_alive));

        let (forwarded_tx, forwarded_rx) = mpsc::unbounded_channel();
        let host_key_error = Arc::new(StdMutex::new(None));
        let handler = TunnelClient::new(
            forwarded_tx,
            host.to_string(),
            port,
            known_hosts_policy,
            host_key_error.clone(),
            None,
        );

        let (mut handle, bastion) = if let Some(jump) = jump_host_config {
            let jump_session = Box::new(
                Box::pin(SshSession::connect(ConnectOptions {
                    host: &jump.ssh_host,
                    port: jump.ssh_port,
                    user: &jump.ssh_user,
                    identity_file: jump.ssh_identity_file.as_deref(),
                    password: jump.ssh_password.as_deref(),
                    passphrase,
                    known_hosts_policy,
                    jump_host_config: None,
                }))
                .await?,
            );
            let channel = jump_session
                .open_direct_tcpip_with_timeout(host.to_string(), port as u32, timeout_secs)
                .await?;
            let stream = channel.into_stream();
            let handle = tokio::time::timeout(
                Duration::from_secs(timeout_secs.max(1)),
                client::connect_stream(config, stream, handler),
            )
            .await
            .map_err(|_| {
                format!(
                    "SSH connection via Jump Host to {}:{} timed out",
                    host, port
                )
            })?
            .map_err(|e| {
                host_key_error
                    .lock()
                    .ok()
                    .and_then(|err| err.clone())
                    .unwrap_or_else(|| format!("SSH connection via Jump Host failed: {}", e))
            })?;
            (handle, Some(jump_session))
        } else {
            let handle = tokio::time::timeout(
                Duration::from_secs(timeout_secs.max(1)),
                client::connect(config, (host, port), handler),
            )
            .await
            .map_err(|_| format!("SSH connection to {}:{} timed out", host, port))?
            .map_err(|e| {
                host_key_error
                    .lock()
                    .ok()
                    .and_then(|err| err.clone())
                    .unwrap_or_else(|| format!("SSH connection failed: {}", e))
            })?;
            (handle, None)
        };

        tokio::time::timeout(
            Duration::from_secs(timeout_secs.max(1)),
            authenticate_handle(&mut handle, user, identity_file, password, passphrase),
        )
        .await
        .map_err(|_| format!("SSH authentication for {}@{} timed out", user, host))??;

        Ok(Self {
            handle: Arc::new(Mutex::new(handle)),
            forwarded_rx: Some(forwarded_rx),
            _bastion: bastion,
        })
    }

    pub async fn trust_host_key(
        host: &str,
        port: u16,
        expected_fingerprint: &str,
    ) -> Result<(), String> {
        let config = Arc::new(Config {
            nodelay: true,
            ..Default::default()
        });
        let (forwarded_tx, _) = mpsc::unbounded_channel();
        let host_key_error = Arc::new(StdMutex::new(None));
        let handler = TunnelClient::new(
            forwarded_tx,
            host.to_string(),
            port,
            KnownHostsPolicy::TrustAndRemember,
            host_key_error.clone(),
            Some(expected_fingerprint.to_string()),
        );
        let timeout_secs = ConfigStore::new()
            .load_config()
            .map(|config| config.settings.connect_timeout as u64)
            .unwrap_or(10)
            .max(1);
        let handle = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            client::connect(config, (host, port), handler),
        )
        .await
        .map_err(|_| format!("Timed out while reading host key from {}:{}", host, port))?
        .map_err(|e| {
            host_key_error
                .lock()
                .ok()
                .and_then(|error| error.clone())
                .unwrap_or_else(|| format!("Failed to trust host key: {e}"))
        })?;
        let _ = handle
            .disconnect(Disconnect::ByApplication, "Host key trusted", "en")
            .await;
        Ok(())
    }

    pub async fn replace_host_key(
        host: &str,
        port: u16,
        expected_fingerprint: &str,
    ) -> Result<(), String> {
        let path = known_hosts_path().ok_or_else(|| "No home directory found".to_string())?;
        let original =
            fs::read_to_string(&path).map_err(|e| format!("Failed to read known_hosts: {e}"))?;
        let filtered = remove_known_host_entries(&original, host, port);
        if filtered == original {
            return Err("No saved host key was found to replace".to_string());
        }
        fs::write(&path, &filtered).map_err(|e| format!("Failed to update known_hosts: {e}"))?;

        if let Err(error) = Self::trust_host_key(host, port, expected_fingerprint).await {
            if let Err(restore_error) = fs::write(&path, original) {
                return Err(format!(
                    "{error}; restoring the previous known_hosts file also failed: {restore_error}"
                ));
            }
            return Err(error);
        }
        Ok(())
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

    pub async fn closed_reason(handle: &SharedSshHandle) -> Option<String> {
        let mut handle = handle.lock().await;
        if !handle.is_closed() {
            return None;
        }

        Some(
            match tokio::time::timeout(Duration::from_secs(1), &mut *handle).await {
                Ok(result) => session_end_message(result),
                Err(_) => "SSH session closed without a reported reason".to_string(),
            },
        )
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

    async fn open_direct_tcpip_with_timeout(
        &self,
        host: String,
        port: u32,
        timeout_secs: u64,
    ) -> Result<Channel<Msg>, String> {
        tokio::time::timeout(
            Duration::from_secs(timeout_secs.max(1)),
            self.handle
                .lock()
                .await
                .channel_open_direct_tcpip(host.clone(), port, "127.0.0.1", 0),
        )
        .await
        .map_err(|_| format!("Jump Host channel to {}:{} timed out", host, port))?
        .map_err(|e| format!("Jump Host channel failed: {}", e))
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

    let mut last_key_error = match try_agent_auth(handle, user).await {
        Ok(true) => return Ok(()),
        Ok(false) => None,
        Err(error) => Some(error),
    };
    for key_path in default_key_candidates().into_iter().filter_map(|key_path| {
        if !key_path.exists() {
            return None;
        }

        Some(key_path)
    }) {
        match authenticate_key_file(handle, user, key_path, passphrase).await {
            Ok(()) => return Ok(()),
            Err(err) => {
                update_key_auth_error(&mut last_key_error, err)?;
            }
        }
    }

    Err(last_key_error.unwrap_or_else(|| {
        "All authentication methods failed. Please provide a valid SSH password, SSH Agent, or Private Key configuration.".to_string()
    }))
}

fn update_key_auth_error(last_key_error: &mut Option<String>, err: String) -> Result<(), String> {
    if err == "PASSPHRASE_REQUIRED" {
        return Err(err);
    }
    *last_key_error = Some(err);
    Ok(())
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

    let hash_alg = if key.algorithm().is_rsa() {
        negotiate_rsa_hash(handle).await?
    } else {
        None
    };
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

#[cfg(unix)]
async fn try_agent_auth(handle: &mut SshHandle, user: &str) -> Result<bool, String> {
    let agent = match AgentClient::connect_env().await {
        Ok(agent) => agent,
        Err(_) => return Ok(false),
    };

    authenticate_with_agent(handle, user, agent).await
}

#[cfg(windows)]
async fn try_agent_auth(handle: &mut SshHandle, user: &str) -> Result<bool, String> {
    const OPENSSH_AGENT_PIPE: &str = r"\\.\pipe\openssh-ssh-agent";

    if let Ok(agent) = AgentClient::connect_named_pipe(OPENSSH_AGENT_PIPE).await {
        if authenticate_with_agent(handle, user, agent).await? {
            return Ok(true);
        }
    }

    let agent = match AgentClient::connect_pageant().await {
        Ok(agent) => agent,
        Err(_) => return Ok(false),
    };

    authenticate_with_agent(handle, user, agent).await
}

#[cfg(not(any(unix, windows)))]
async fn try_agent_auth(_handle: &mut SshHandle, _user: &str) -> Result<bool, String> {
    Ok(false)
}

async fn authenticate_with_agent<S>(
    handle: &mut SshHandle,
    user: &str,
    mut agent: AgentClient<S>,
) -> Result<bool, String>
where
    S: AgentStream + Send + Unpin,
{
    let identities = match agent.request_identities().await {
        Ok(ids) => ids,
        Err(_) => return Ok(false),
    };

    let rsa_hash = if identities
        .iter()
        .any(|identity| identity.public_key().algorithm().is_rsa())
    {
        Some(negotiate_rsa_hash(handle).await)
    } else {
        None
    };
    let mut rsa_error = None;

    for identity in identities {
        let public_key = identity.public_key().into_owned();
        let hash_alg = if public_key.algorithm().is_rsa() {
            match rsa_hash.as_ref().expect("RSA identities negotiated a hash") {
                Ok(hash) => *hash,
                Err(error) => {
                    rsa_error = Some(error.clone());
                    continue;
                }
            }
        } else {
            None
        };
        match handle
            .authenticate_publickey_with(user, public_key, hash_alg, &mut agent)
            .await
        {
            Ok(auth) if auth.success() => return Ok(true),
            Ok(_) => {}
            Err(_) => {}
        }
    }

    match rsa_error {
        Some(error) => Err(error),
        None => Ok(false),
    }
}

async fn negotiate_rsa_hash(handle: &mut SshHandle) -> Result<Option<ssh_key::HashAlg>, String> {
    let advertised = handle
        .best_supported_rsa_hash()
        .await
        .map_err(|e| format!("Failed to negotiate RSA signature algorithm: {e}"))?;
    select_rsa_hash(advertised).map_err(str::to_string)
}

fn select_rsa_hash(
    advertised: Option<Option<ssh_key::HashAlg>>,
) -> Result<Option<ssh_key::HashAlg>, &'static str> {
    match advertised {
        Some(Some(hash)) => Ok(Some(hash)),
        // RFC 8308 explicitly tells us this server only accepts the deprecated
        // SHA-1 signature. Do not silently downgrade authentication security.
        Some(None) => Err(
            "The SSH server only supports legacy ssh-rsa (SHA-1). Enable RSA-SHA2 on the server or update its SSH software.",
        ),
        // Older servers may support RFC 8332 without advertising server-sig-algs.
        None => Ok(Some(ssh_key::HashAlg::Sha256)),
    }
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

    #[test]
    fn default_key_auth_returns_passphrase_required() {
        let mut last_key_error = None;
        update_key_auth_error(
            &mut last_key_error,
            "Private key authentication failed".to_string(),
        )
        .unwrap();

        let err = update_key_auth_error(&mut last_key_error, "PASSPHRASE_REQUIRED".to_string())
            .unwrap_err();

        assert_eq!(err, "PASSPHRASE_REQUIRED");
    }

    #[test]
    fn rsa_auth_prefers_sha2_without_legacy_sha1_fallback() {
        assert_eq!(
            select_rsa_hash(Some(Some(ssh_key::HashAlg::Sha512))).unwrap(),
            Some(ssh_key::HashAlg::Sha512)
        );
        assert_eq!(
            select_rsa_hash(None).unwrap(),
            Some(ssh_key::HashAlg::Sha256)
        );
        assert!(select_rsa_hash(Some(None)).is_err());
    }

    #[test]
    fn keepalive_does_not_turn_idle_sessions_into_failures() {
        let enabled = client_config(30);
        assert_eq!(enabled.keepalive_interval, Some(Duration::from_secs(30)));
        assert_eq!(enabled.keepalive_max, 3);
        assert_eq!(enabled.inactivity_timeout, None);

        let disabled = client_config(0);
        assert_eq!(disabled.keepalive_interval, None);
        assert_eq!(disabled.inactivity_timeout, None);
    }

    #[test]
    fn session_end_messages_preserve_the_failure_reason() {
        assert_eq!(
            session_end_message(Err(russh::Error::KeepaliveTimeout)),
            "SSH keepalive timed out"
        );
        assert_eq!(
            session_end_message(Err(russh::Error::HUP)),
            "SSH connection closed by remote host"
        );
    }
}
