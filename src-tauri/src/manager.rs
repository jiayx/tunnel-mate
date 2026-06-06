use crate::config::{ConfigStore, Tunnel, TunnelStatus};
use crate::event_logger::{EventLogger, EventType};
use crate::ssh::engine::SshSession;
use crate::ssh::tunnel::TunnelWorker;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::{ipc::Channel, AppHandle, Emitter};
use tokio::sync::Mutex;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusPayload {
    pub tunnel_id: String,
    pub status: TunnelStatus,
    pub message: Option<String>,
}

pub struct ActiveTunnel {
    pub tunnel: Tunnel,
    pub worker: TunnelWorker,
    pub log_channel: Channel<String>,
    ssh_session: SshSession,
}

pub struct TunnelManager {
    active_tunnels: HashMap<String, ActiveTunnel>,
    reconnect_tasks: HashMap<String, tokio::task::JoinHandle<()>>,
    statuses: HashMap<String, TunnelStatus>,
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {
            active_tunnels: HashMap::new(),
            reconnect_tasks: HashMap::new(),
            statuses: HashMap::new(),
        }
    }

    pub fn get_status(&self, tunnel_id: &str) -> TunnelStatus {
        self.statuses.get(tunnel_id).cloned().unwrap_or(TunnelStatus::Stopped)
    }

    pub async fn start_tunnel(
        manager_state: Arc<Mutex<Self>>,
        app: AppHandle,
        tunnel: Tunnel,
        passphrase: Option<String>,
        log_channel: Channel<String>, // Passed from frontend!
    ) -> Result<(), String> {
        let tunnel_id = tunnel.id.clone();
        let logger = EventLogger::new();

        // 1. Check if already running
        {
            let mut manager = manager_state.lock().await;
            if manager.active_tunnels.contains_key(&tunnel_id) {
                return Err("Tunnel is already running".to_string());
            }
            // Cancel any pending reconnect tasks for this tunnel
            if let Some(task) = manager.reconnect_tasks.remove(&tunnel_id) {
                task.abort();
            }
            manager.statuses.insert(tunnel_id.clone(), TunnelStatus::Connecting);
        }

        // Notify connecting status
        emit_status(
            &app,
            &tunnel_id,
            TunnelStatus::Connecting,
            Some("Connecting to SSH host...".to_string()),
        );
        let _ = logger.log(
            Some(tunnel_id.clone()),
            Some(tunnel.name.clone()),
            EventType::Started,
            "Tunnel connection starting...".to_string(),
        );

        // Resolve Jump Host configuration if enabled
        let jump_config = if tunnel.jump_host_enabled {
            let config = ConfigStore::new().load_config()?;
            config
                .tunnels
                .iter()
                .find(|t| t.name == tunnel.jump_host.clone().unwrap_or_default())
                .cloned()
        } else {
            None
        };

        let t_clone = tunnel.clone();
        let m_state = manager_state.clone();
        let app_handle = app.clone();
        let passphrase_conn = passphrase.clone();

        // Run connection in separate task to prevent blocking GUI
        tokio::spawn(async move {
            let log_ch = log_channel.clone();
            let _ = log_ch.send("[INFO] Establishing SSH connection...".to_string());

            let conn_res = SshSession::connect(
                &t_clone.ssh_host,
                t_clone.ssh_port,
                &t_clone.ssh_user,
                t_clone.ssh_identity_file.as_deref(),
                passphrase_conn.as_deref(),
                "trustPermanently", // Default verify policy: auto-add on first connect
                jump_config.as_ref(),
            )
            .await;

            let ssh_session = match conn_res {
                Ok(sess) => sess,
                Err(err_msg) => {
                    let _ = log_ch.send(format!("[ERROR] Connection failed: {}", err_msg));
                    let _ = logger.log(
                        Some(tunnel_id.clone()),
                        Some(tunnel.name.clone()),
                        EventType::Failed,
                        format!("Connection failed: {}", err_msg),
                    );

                    if err_msg == "PASSPHRASE_REQUIRED" {
                        {
                            let mut manager = m_state.lock().await;
                            manager.statuses.insert(tunnel_id.clone(), TunnelStatus::Failed);
                        }
                        emit_status(
                            &app_handle,
                            &tunnel_id,
                            TunnelStatus::Failed,
                            Some("PASSPHRASE_REQUIRED".to_string()),
                        );
                    } else if tunnel.auto_reconnect {
                        // Spawn reconnect task
                        Self::spawn_reconnect_flow(
                            m_state, app_handle, tunnel, passphrase, 1, log_ch,
                        );
                    } else {
                        {
                            let mut manager = m_state.lock().await;
                            manager.statuses.insert(tunnel_id.clone(), TunnelStatus::Failed);
                        }
                        emit_status(&app_handle, &tunnel_id, TunnelStatus::Failed, Some(err_msg));
                    }
                    return;
                }
            };

            // Start forwarder worker
            let _ =
                log_ch.send("[INFO] SSH Session authenticated. Spawning listeners...".to_string());
            let ssh_handle = ssh_session.handle();
            let mut ssh_session = ssh_session;
            let forwarded_rx = ssh_session.take_forwarded_receiver();

            let worker =
                match TunnelWorker::start(tunnel.clone(), ssh_handle, forwarded_rx, log_ch.clone())
                    .await
                {
                    Ok(w) => w,
                    Err(e) => {
                        let err_msg = format!("Failed to start forwarding listeners: {}", e);
                        let _ = log_ch.send(format!("[ERROR] {}", err_msg));
                        let _ = logger.log(
                            Some(tunnel_id.clone()),
                            Some(tunnel.name.clone()),
                            EventType::Failed,
                            err_msg.clone(),
                        );
                        {
                            let mut manager = m_state.lock().await;
                            manager.statuses.insert(tunnel_id.clone(), TunnelStatus::Failed);
                        }
                        emit_status(&app_handle, &tunnel_id, TunnelStatus::Failed, Some(err_msg));
                        ssh_session.disconnect().await;
                        return;
                    }
                };

            // Register active tunnel
            let active = ActiveTunnel {
                tunnel: tunnel.clone(),
                worker,
                log_channel: log_ch,
                ssh_session,
            };

            {
                let mut manager = m_state.lock().await;
                manager.statuses.insert(tunnel_id.clone(), TunnelStatus::Running);
                manager.active_tunnels.insert(tunnel_id.clone(), active);
            }

            emit_status(&app_handle, &tunnel_id, TunnelStatus::Running, None);
            let _ = logger.log(
                Some(tunnel_id.clone()),
                Some(tunnel.name.clone()),
                EventType::Started,
                "Tunnel is active".to_string(),
            );

            // Start background heartbeat monitor for this tunnel
            Self::spawn_monitor(m_state, app_handle, tunnel_id, tunnel, passphrase);
        });

        Ok(())
    }

    pub async fn stop_tunnel(&mut self, app: &AppHandle, tunnel_id: &str) -> Result<(), String> {
        let logger = EventLogger::new();

        // Cancel reconnect task if exists
        if let Some(task) = self.reconnect_tasks.remove(tunnel_id) {
            task.abort();
        }

        if let Some(mut active) = self.active_tunnels.remove(tunnel_id) {
            let name = active.tunnel.name.clone();
            let _ = active
                .log_channel
                .send("[INFO] Stopping tunnel listeners...".to_string());

            active.worker.stop().await;
            active.ssh_session.disconnect().await;
            let _ = logger.log(
                Some(tunnel_id.to_string()),
                Some(name),
                EventType::Stopped,
                "Tunnel stopped by user".to_string(),
            );
        }
        
        self.statuses.insert(tunnel_id.to_string(), TunnelStatus::Stopped);

        // Always emit stopped status so the frontend updates
        emit_status(app, tunnel_id, TunnelStatus::Stopped, None);

        Ok(())
    }

    fn spawn_reconnect_flow(
        manager_state: Arc<Mutex<Self>>,
        app: AppHandle,
        tunnel: Tunnel,
        passphrase: Option<String>,
        attempt: u32,
        log_channel: Channel<String>,
    ) {
        let tunnel_id = tunnel.id.clone();
        let max_retries = tunnel.retry_count;
        let interval = tunnel.retry_interval;
        let logger = EventLogger::new();

        if attempt > max_retries {
            let m_state_fail = manager_state.clone();
            let t_id = tunnel_id.clone();
            tokio::spawn(async move {
                let mut manager = m_state_fail.lock().await;
                manager.statuses.insert(t_id, TunnelStatus::Failed);
            });
            emit_status(
                &app,
                &tunnel_id,
                TunnelStatus::Failed,
                Some("Max reconnect attempts reached".to_string()),
            );
            let _ = logger.log(
                Some(tunnel_id.clone()),
                Some(tunnel.name.clone()),
                EventType::Failed,
                "Max reconnect attempts reached. Giving up.".to_string(),
            );
            return;
        }

        let m_state_reconn = manager_state.clone();
        let t_id_reconn = tunnel_id.clone();
        tokio::spawn(async move {
            let mut manager = m_state_reconn.lock().await;
            manager.statuses.insert(t_id_reconn, TunnelStatus::Reconnecting);
        });

        emit_status(
            &app,
            &tunnel_id,
            TunnelStatus::Reconnecting,
            Some(format!(
                "Reconnecting (attempt {}/{})...",
                attempt, max_retries
            )),
        );

        let tunnel_id_task = tunnel_id.clone();
        let tunnel_task = tunnel.clone();
        let manager_state_task = manager_state.clone();
        let app_task = app.clone();
        let passphrase_task = passphrase.clone();
        let log_channel_task = log_channel.clone();

        let task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(interval as u64)).await;

            // Log attempt
            let _ = logger.log(
                Some(tunnel_id_task.clone()),
                Some(tunnel_task.name.clone()),
                EventType::Reconnected,
                format!("Reconnecting attempt {}/{}...", attempt, max_retries),
            );

            // Try to connect again
            let start_res = Self::start_tunnel(
                manager_state_task.clone(),
                app_task.clone(),
                tunnel_task.clone(),
                passphrase_task.clone(),
                log_channel_task.clone(),
            )
            .await;

            if let Err(_e) = start_res {
                // If start failed immediately, chain to next attempt
                Self::spawn_reconnect_flow(
                    manager_state_task,
                    app_task,
                    tunnel_task,
                    passphrase_task,
                    attempt + 1,
                    log_channel_task,
                );
            }
        });

        // Store task handle so we can abort it if user stops tunnel
        let m_state = manager_state.clone();
        tokio::spawn(async move {
            let mut manager = m_state.lock().await;
            manager.reconnect_tasks.insert(tunnel_id, task);
        });
    }

    fn spawn_monitor(
        manager_state: Arc<Mutex<Self>>,
        app: AppHandle,
        tunnel_id: String,
        tunnel: Tunnel,
        passphrase: Option<String>,
    ) {
        let m_state = manager_state.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(3)).await;

                let mut is_disconnected = false;
                {
                    let manager = m_state.lock().await;
                    if let Some(active) = manager.active_tunnels.get(&tunnel_id) {
                        if !active.ssh_session.is_alive().await {
                            is_disconnected = true;
                        }
                    } else {
                        // Tunnel was stopped by user, stop monitoring
                        break;
                    }
                }

                if is_disconnected {
                    // Stop the disconnected tunnel
                    let mut manager = m_state.lock().await;
                    let log_channel =
                        if let Some(mut active) = manager.active_tunnels.remove(&tunnel_id) {
                            active.worker.stop().await;
                            active.ssh_session.disconnect().await;
                            Some(active.log_channel)
                        } else {
                            None
                        };

                    let _ = EventLogger::new().log(
                        Some(tunnel_id.clone()),
                        Some(tunnel.name.clone()),
                        EventType::Failed,
                        "SSH session heartbeat timed out or disconnected".to_string(),
                    );

                    // Trigger reconnection flow
                    if tunnel.auto_reconnect {
                        if let Some(log_ch) = log_channel {
                            Self::spawn_reconnect_flow(
                                m_state.clone(),
                                app.clone(),
                                tunnel.clone(),
                                passphrase.clone(),
                                1,
                                log_ch,
                            );
                        }
                    } else {
                        {
                            let mut manager = m_state.lock().await;
                            manager.statuses.insert(tunnel_id.clone(), TunnelStatus::Failed);
                        }
                        emit_status(
                            &app,
                            &tunnel_id,
                            TunnelStatus::Failed,
                            Some("Session disconnected".to_string()),
                        );
                    }
                    break;
                }
            }
        });
    }
}

fn emit_status(app: &AppHandle, tunnel_id: &str, status: TunnelStatus, message: Option<String>) {
    app.emit(
        "tunnel-status-changed",
        StatusPayload {
            tunnel_id: tunnel_id.to_string(),
            status,
            message,
        },
    )
    .ok();

    super::update_tray_menu(app);
}
