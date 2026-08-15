use crate::config::{ConfigStore, Tunnel, TunnelStatus};
use crate::event_logger::{EventLogger, EventType, LogEvent};
use crate::ssh::engine::{ConnectOptions, KnownHostsPolicy, SshSession};
use crate::ssh::tunnel::{LogSink, TunnelWorker};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusPayload {
    pub tunnel_id: String,
    pub status: TunnelStatus,
    pub message: Option<String>,
}

#[derive(Clone)]
pub enum RuntimeEvent {
    Status(StatusPayload),
    Activity(LogEvent),
}

pub type EventSink = Arc<dyn Fn(RuntimeEvent) + Send + Sync>;

pub struct ActiveTunnel {
    pub tunnel: Tunnel,
    pub worker: TunnelWorker,
    pub log_channel: LogSink,
    pub session_id: String,
    ssh_session: SshSession,
}

pub struct TunnelManager {
    active_tunnels: HashMap<String, ActiveTunnel>,
    reconnect_tasks: HashMap<String, tokio::task::JoinHandle<()>>,
    operations: HashMap<String, Uuid>,
    statuses: HashMap<String, TunnelStatus>,
    events: EventSink,
}

impl Default for TunnelManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TunnelManager {
    pub fn new() -> Self {
        Self::with_event_sink(Arc::new(|_| {}))
    }

    pub fn with_event_sink(events: EventSink) -> Self {
        Self {
            active_tunnels: HashMap::new(),
            reconnect_tasks: HashMap::new(),
            operations: HashMap::new(),
            statuses: HashMap::new(),
            events,
        }
    }

    pub fn get_status(&self, tunnel_id: &str) -> TunnelStatus {
        self.statuses
            .get(tunnel_id)
            .cloned()
            .unwrap_or(TunnelStatus::Stopped)
    }

    pub fn set_event_sink(&mut self, events: EventSink) {
        self.events = events;
    }

    pub async fn start_tunnel(
        manager_state: Arc<Mutex<Self>>,
        tunnel: Tunnel,
        passphrase: Option<String>,
        log_sink: LogSink,
    ) -> Result<(), String> {
        let operation_id = Self::begin_operation(&manager_state, &tunnel.id).await?;
        Self::start_tunnel_attempt(manager_state, tunnel, passphrase, log_sink, 0, operation_id)
            .await
    }

    pub async fn start_tunnel_silent(
        manager_state: Arc<Mutex<Self>>,
        tunnel: Tunnel,
    ) -> Result<(), String> {
        let operation_id = Self::begin_operation(&manager_state, &tunnel.id).await?;
        Self::start_tunnel_attempt(
            manager_state,
            tunnel,
            None,
            LogSink::Silent,
            0,
            operation_id,
        )
        .await
    }

    async fn begin_operation(
        manager_state: &Arc<Mutex<Self>>,
        tunnel_id: &str,
    ) -> Result<Uuid, String> {
        let mut manager = manager_state.lock().await;
        if manager.active_tunnels.contains_key(tunnel_id)
            || manager.operations.contains_key(tunnel_id)
        {
            return Err("Tunnel is already running or connecting".to_string());
        }
        if let Some(task) = manager.reconnect_tasks.remove(tunnel_id) {
            task.abort();
        }
        let operation_id = Uuid::new_v4();
        manager
            .operations
            .insert(tunnel_id.to_string(), operation_id);
        manager
            .statuses
            .insert(tunnel_id.to_string(), TunnelStatus::Connecting);
        Ok(operation_id)
    }

    async fn operation_is_current(
        manager_state: &Arc<Mutex<Self>>,
        tunnel_id: &str,
        operation_id: Uuid,
    ) -> bool {
        manager_state.lock().await.operations.get(tunnel_id) == Some(&operation_id)
    }

    async fn start_tunnel_attempt(
        manager_state: Arc<Mutex<Self>>,
        tunnel: Tunnel,
        passphrase: Option<String>,
        log_channel: LogSink,
        reconnect_attempt: u32,
        operation_id: Uuid,
    ) -> Result<(), String> {
        let tunnel_id = tunnel.id.clone();
        let session_id = Uuid::new_v4().to_string();
        let logger = EventLogger::new();
        let log_channel = log_channel.with_session(session_id.clone());
        let events = {
            let manager = manager_state.lock().await;
            manager.events.clone()
        };

        if !Self::operation_is_current(&manager_state, &tunnel_id, operation_id).await {
            return Err("Tunnel operation was cancelled".to_string());
        }
        {
            let mut manager = manager_state.lock().await;
            manager.reconnect_tasks.remove(&tunnel_id);
            manager
                .statuses
                .insert(tunnel_id.clone(), TunnelStatus::Connecting);
        }

        // Notify connecting status
        emit_status(
            &events,
            &tunnel_id,
            TunnelStatus::Connecting,
            Some("Connecting to SSH host...".to_string()),
        );
        let _ = emit_activity_event(
            &events,
            &logger,
            Some(session_id.clone()),
            Some(tunnel_id.clone()),
            Some(tunnel.name.clone()),
            EventType::Connecting,
            "Tunnel connection starting...".to_string(),
        );

        // Resolve Jump Host configuration if enabled
        let jump_config = if tunnel.jump_host_enabled {
            let config = ConfigStore::new().load_config()?;
            if let Some(jump_host_id) = tunnel.jump_host_id.as_deref() {
                let selected = config
                    .tunnels
                    .iter()
                    .find(|candidate| candidate.id == jump_host_id)
                    .cloned()
                    .ok_or_else(|| {
                        format!("Tunnel '{}' jump host reference was not found", tunnel.name)
                    })?;
                Some(selected)
            } else {
                let jump_host = tunnel.jump_host.clone().unwrap_or_default();
                Some(Tunnel {
                    id: format!("{}_manual_jump", tunnel.id),
                    name: jump_host.clone(),
                    description: None,
                    group_id: None,
                    ssh_host: jump_host,
                    ssh_port: tunnel
                        .jump_port
                        .ok_or_else(|| format!("Tunnel '{}' jump port is required", tunnel.name))?,
                    ssh_user: tunnel
                        .jump_user
                        .clone()
                        .ok_or_else(|| format!("Tunnel '{}' jump user is required", tunnel.name))?,
                    ssh_identity_file: tunnel.jump_identity_file.clone(),
                    ssh_password: tunnel.jump_password.clone(),
                    jump_host_enabled: false,
                    jump_host_id: None,
                    jump_host: None,
                    jump_port: None,
                    jump_user: None,
                    jump_identity_file: None,
                    jump_password: None,
                    forward: tunnel.forward.clone(),
                    start_with_app: false,
                    auto_reconnect: false,
                    retry_count: 0,
                    retry_interval: tunnel.retry_interval,
                })
            }
        } else {
            None
        };

        let t_clone = tunnel.clone();
        let m_state = manager_state.clone();
        let task_events = events.clone();
        let passphrase_conn = passphrase.clone();
        let session_id_task = session_id.clone();

        // Run connection in separate task to prevent blocking GUI
        tokio::spawn(async move {
            let log_ch = log_channel.clone();
            log_ch.send("[INFO] Establishing SSH connection...".to_string());

            let conn_res = SshSession::connect(ConnectOptions {
                host: &t_clone.ssh_host,
                port: t_clone.ssh_port,
                user: &t_clone.ssh_user,
                identity_file: t_clone.ssh_identity_file.as_deref(),
                password: t_clone.ssh_password.as_deref(),
                passphrase: passphrase_conn.as_deref(),
                known_hosts_policy: KnownHostsPolicy::RequireKnown,
                jump_host_config: jump_config.as_ref(),
            })
            .await;

            let ssh_session = match conn_res {
                Ok(sess) => sess,
                Err(err_msg) => {
                    if !Self::operation_is_current(&m_state, &tunnel_id, operation_id).await {
                        return;
                    }
                    log_ch.send(format!("[ERROR] Connection failed: {}", err_msg));
                    let _ = emit_activity_event(
                        &task_events,
                        &logger,
                        Some(session_id_task.clone()),
                        Some(tunnel_id.clone()),
                        Some(tunnel.name.clone()),
                        EventType::Failed,
                        format!("Connection failed: {}", err_msg),
                    );

                    if err_msg == "PASSPHRASE_REQUIRED"
                        || err_msg.starts_with("HOST_KEY_NOT_TRUSTED|")
                        || err_msg.starts_with("HOST_KEY_CHANGED|")
                        || err_msg.starts_with("HOST_KEY_REVOKED|")
                    {
                        {
                            let mut manager = m_state.lock().await;
                            if manager.operations.get(&tunnel_id) != Some(&operation_id) {
                                return;
                            }
                            manager
                                .statuses
                                .insert(tunnel_id.clone(), TunnelStatus::Failed);
                            manager.operations.remove(&tunnel_id);
                        }
                        emit_status(
                            &task_events,
                            &tunnel_id,
                            TunnelStatus::Failed,
                            Some(err_msg),
                        );
                    } else if tunnel.auto_reconnect {
                        // Spawn reconnect task
                        Self::spawn_reconnect_flow(
                            m_state,
                            task_events,
                            tunnel,
                            passphrase,
                            reconnect_attempt + 1,
                            log_ch,
                            operation_id,
                        );
                    } else {
                        {
                            let mut manager = m_state.lock().await;
                            if manager.operations.get(&tunnel_id) != Some(&operation_id) {
                                return;
                            }
                            manager
                                .statuses
                                .insert(tunnel_id.clone(), TunnelStatus::Failed);
                            manager.operations.remove(&tunnel_id);
                        }
                        emit_status(
                            &task_events,
                            &tunnel_id,
                            TunnelStatus::Failed,
                            Some(err_msg),
                        );
                    }
                    return;
                }
            };

            // Start forwarder worker
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
                        log_ch.send(format!("[ERROR] {}", err_msg));
                        let _ = emit_activity_event(
                            &task_events,
                            &logger,
                            Some(session_id_task.clone()),
                            Some(tunnel_id.clone()),
                            Some(tunnel.name.clone()),
                            EventType::Failed,
                            err_msg.clone(),
                        );
                        let is_current = {
                            let mut manager = m_state.lock().await;
                            if manager.operations.get(&tunnel_id) == Some(&operation_id) {
                                manager
                                    .statuses
                                    .insert(tunnel_id.clone(), TunnelStatus::Failed);
                                manager.operations.remove(&tunnel_id);
                                true
                            } else {
                                false
                            }
                        };
                        if is_current {
                            emit_status(
                                &task_events,
                                &tunnel_id,
                                TunnelStatus::Failed,
                                Some(err_msg),
                            );
                        }
                        ssh_session.disconnect().await;
                        return;
                    }
                };

            // Register active tunnel
            let active = ActiveTunnel {
                tunnel: tunnel.clone(),
                worker,
                log_channel: log_ch,
                session_id: session_id_task.clone(),
                ssh_session,
            };

            {
                let mut manager = m_state.lock().await;
                if manager.operations.get(&tunnel_id) != Some(&operation_id) {
                    drop(manager);
                    let mut active = active;
                    active.worker.stop().await;
                    active.ssh_session.disconnect().await;
                    return;
                }
                manager
                    .statuses
                    .insert(tunnel_id.clone(), TunnelStatus::Running);
                manager.active_tunnels.insert(tunnel_id.clone(), active);
            }

            emit_status(&task_events, &tunnel_id, TunnelStatus::Running, None);
            let _ = emit_activity_event(
                &task_events,
                &logger,
                Some(session_id_task.clone()),
                Some(tunnel_id.clone()),
                Some(tunnel.name.clone()),
                EventType::Started,
                "Tunnel is active".to_string(),
            );

            // Start background heartbeat monitor for this tunnel
            Self::spawn_monitor(
                m_state,
                task_events,
                tunnel_id,
                tunnel,
                passphrase,
                operation_id,
            );
        });

        Ok(())
    }

    pub async fn stop_tunnel(
        manager_state: Arc<Mutex<Self>>,
        tunnel_id: &str,
    ) -> Result<(), String> {
        let logger = EventLogger::new();
        let (events, active) = {
            let mut manager = manager_state.lock().await;
            manager.operations.remove(tunnel_id);
            if let Some(task) = manager.reconnect_tasks.remove(tunnel_id) {
                task.abort();
            }
            let active = manager.active_tunnels.remove(tunnel_id);
            manager
                .statuses
                .insert(tunnel_id.to_string(), TunnelStatus::Stopped);
            (manager.events.clone(), active)
        };

        if let Some(mut active) = active {
            let name = active.tunnel.name.clone();
            let session_id = active.session_id.clone();
            active
                .log_channel
                .send("[INFO] Stopping tunnel listeners...".to_string());

            active.worker.stop().await;
            active.ssh_session.disconnect().await;
            let _ = emit_activity_event(
                &events,
                &logger,
                Some(session_id),
                Some(tunnel_id.to_string()),
                Some(name),
                EventType::Stopped,
                "Tunnel stopped by user".to_string(),
            );
        }

        // Always emit stopped status so the frontend updates
        emit_status(&events, tunnel_id, TunnelStatus::Stopped, None);

        Ok(())
    }

    #[allow(dead_code)] // Used by the GPUI client through the shared core crate.
    pub async fn stop_all(manager_state: Arc<Mutex<Self>>) {
        let tunnel_ids = {
            let manager = manager_state.lock().await;
            manager.operations.keys().cloned().collect::<Vec<_>>()
        };
        for tunnel_id in tunnel_ids {
            let _ = Self::stop_tunnel(manager_state.clone(), &tunnel_id).await;
        }
    }

    fn spawn_reconnect_flow(
        manager_state: Arc<Mutex<Self>>,
        events: EventSink,
        tunnel: Tunnel,
        passphrase: Option<String>,
        attempt: u32,
        log_channel: LogSink,
        operation_id: Uuid,
    ) {
        let tunnel_id = tunnel.id.clone();
        let max_retries = tunnel.retry_count;
        let interval = tunnel.retry_interval;
        let logger = EventLogger::new();

        if attempt > max_retries {
            let m_state_fail = manager_state.clone();
            let t_id = tunnel_id.clone();
            let fail_events = events.clone();
            let fail_name = tunnel.name.clone();
            tokio::spawn(async move {
                let mut manager = m_state_fail.lock().await;
                if manager.operations.get(&t_id) == Some(&operation_id) {
                    manager.statuses.insert(t_id.clone(), TunnelStatus::Failed);
                    manager.operations.remove(&t_id);
                    drop(manager);
                    emit_status(
                        &fail_events,
                        &t_id,
                        TunnelStatus::Failed,
                        Some("Max reconnect attempts reached".to_string()),
                    );
                    let _ = emit_activity_event(
                        &fail_events,
                        &EventLogger::new(),
                        None,
                        Some(t_id),
                        Some(fail_name),
                        EventType::Failed,
                        "Max reconnect attempts reached. Giving up.".to_string(),
                    );
                }
            });
            return;
        }

        let m_state_reconn = manager_state.clone();
        let t_id_reconn = tunnel_id.clone();
        let reconnect_events = events.clone();
        tokio::spawn(async move {
            let mut manager = m_state_reconn.lock().await;
            if manager.operations.get(&t_id_reconn) == Some(&operation_id) {
                manager
                    .statuses
                    .insert(t_id_reconn.clone(), TunnelStatus::Reconnecting);
                drop(manager);
                emit_status(
                    &reconnect_events,
                    &t_id_reconn,
                    TunnelStatus::Reconnecting,
                    Some(format!(
                        "Reconnecting (attempt {}/{})...",
                        attempt, max_retries
                    )),
                );
            }
        });

        let tunnel_id_task = tunnel_id.clone();
        let tunnel_task = tunnel.clone();
        let manager_state_task = manager_state.clone();
        let task_events = events.clone();
        let passphrase_task = passphrase.clone();
        let log_channel_task = log_channel.clone();

        let task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(interval as u64)).await;

            if !Self::operation_is_current(&manager_state_task, &tunnel_id_task, operation_id).await
            {
                return;
            }

            // Log attempt
            let _ = emit_activity_event(
                &task_events,
                &logger,
                None,
                Some(tunnel_id_task.clone()),
                Some(tunnel_task.name.clone()),
                EventType::Reconnected,
                format!("Reconnecting attempt {}/{}...", attempt, max_retries),
            );

            // Try to connect again
            let start_res = Self::start_tunnel_attempt(
                manager_state_task.clone(),
                tunnel_task.clone(),
                passphrase_task.clone(),
                log_channel_task.clone(),
                attempt,
                operation_id,
            )
            .await;

            if let Err(_e) = start_res {
                // If start failed immediately, chain to next attempt
                Self::spawn_reconnect_flow(
                    manager_state_task,
                    task_events,
                    tunnel_task,
                    passphrase_task,
                    attempt + 1,
                    log_channel_task,
                    operation_id,
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
        events: EventSink,
        tunnel_id: String,
        tunnel: Tunnel,
        passphrase: Option<String>,
        operation_id: Uuid,
    ) {
        let m_state = manager_state.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(3)).await;

                if !Self::operation_is_current(&m_state, &tunnel_id, operation_id).await {
                    break;
                }
                let handle = {
                    let manager = m_state.lock().await;
                    if let Some(active) = manager.active_tunnels.get(&tunnel_id) {
                        active.ssh_session.handle()
                    } else {
                        // Tunnel was stopped by user, stop monitoring
                        break;
                    }
                };
                if let Some(disconnect_reason) = SshSession::closed_reason(&handle).await {
                    // Stop the disconnected tunnel
                    let active = m_state.lock().await.active_tunnels.remove(&tunnel_id);
                    let log_channel = if let Some(mut active) = active {
                        active.worker.stop().await;
                        active.ssh_session.disconnect().await;
                        Some(active.log_channel)
                    } else {
                        None
                    };

                    let logger = EventLogger::new();
                    let _ = emit_activity_event(
                        &events,
                        &logger,
                        None,
                        Some(tunnel_id.clone()),
                        Some(tunnel.name.clone()),
                        EventType::Failed,
                        disconnect_reason,
                    );

                    // Trigger reconnection flow
                    if tunnel.auto_reconnect {
                        if let Some(log_ch) = log_channel {
                            Self::spawn_reconnect_flow(
                                m_state.clone(),
                                events.clone(),
                                tunnel.clone(),
                                passphrase.clone(),
                                1,
                                log_ch,
                                operation_id,
                            );
                        }
                    } else {
                        let is_current = {
                            let mut manager = m_state.lock().await;
                            if manager.operations.get(&tunnel_id) == Some(&operation_id) {
                                manager
                                    .statuses
                                    .insert(tunnel_id.clone(), TunnelStatus::Failed);
                                manager.operations.remove(&tunnel_id);
                                true
                            } else {
                                false
                            }
                        };
                        if is_current {
                            emit_status(
                                &events,
                                &tunnel_id,
                                TunnelStatus::Failed,
                                Some("Session disconnected".to_string()),
                            );
                        }
                    }
                    break;
                }
            }
        });
    }
}

fn emit_status(events: &EventSink, tunnel_id: &str, status: TunnelStatus, message: Option<String>) {
    events(RuntimeEvent::Status(StatusPayload {
        tunnel_id: tunnel_id.to_string(),
        status,
        message,
    }));
}

fn emit_activity_event(
    events: &EventSink,
    logger: &EventLogger,
    session_id: Option<String>,
    tunnel_id: Option<String>,
    tunnel_name: Option<String>,
    event_type: EventType,
    message: String,
) -> Result<crate::event_logger::LogEvent, String> {
    let event = logger.log_with_session(session_id, tunnel_id, tunnel_name, event_type, message)?;
    events(RuntimeEvent::Activity(event.clone()));
    Ok(event)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancelling_an_operation_allows_a_fresh_start_generation() {
        let manager = Arc::new(Mutex::new(TunnelManager::new()));
        let first = TunnelManager::begin_operation(&manager, "tunnel-1")
            .await
            .unwrap();

        assert!(TunnelManager::begin_operation(&manager, "tunnel-1")
            .await
            .unwrap_err()
            .contains("already running or connecting"));

        TunnelManager::stop_tunnel(manager.clone(), "tunnel-1")
            .await
            .unwrap();
        assert!(!TunnelManager::operation_is_current(&manager, "tunnel-1", first).await);

        let second = TunnelManager::begin_operation(&manager, "tunnel-1")
            .await
            .unwrap();
        assert_ne!(first, second);
        assert!(TunnelManager::operation_is_current(&manager, "tunnel-1", second).await);
    }
}
