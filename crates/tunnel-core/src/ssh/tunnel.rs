use crate::config::{ForwardSpec, Tunnel};
use crate::ssh::engine::{ForwardedTcp, SharedSshHandle};
use crate::ssh::socks5::{negotiate_socks5, send_reply, Socks5Reply};
use std::future::Future;
use std::sync::Arc;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinSet;
use tokio::time::{timeout, Duration};

const FORWARD_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const SOCKS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_CONNECTIONS: usize = 256;

#[derive(Clone)]
pub enum LogSink {
    Callback {
        callback: Arc<dyn Fn(String) + Send + Sync>,
        session_id: String,
    },
    Silent,
}

impl LogSink {
    pub fn callback(callback: impl Fn(String) + Send + Sync + 'static) -> Self {
        Self::Callback {
            callback: Arc::new(callback),
            session_id: String::new(),
        }
    }

    pub fn with_session(&self, session_id: String) -> Self {
        match self {
            Self::Callback { callback, .. } => Self::Callback {
                callback: callback.clone(),
                session_id,
            },
            Self::Silent => Self::Silent,
        }
    }

    pub fn send(&self, message: String) {
        if let LogSink::Callback {
            callback,
            session_id,
        } = self
        {
            let short_session_id = session_id.get(..8).unwrap_or(session_id);
            callback(format!("[session:{}] {}", short_session_id, message));
        }
    }
}

pub struct TunnelWorker {
    shutdown_tx: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
    remote_forward: Option<RemoteForward>,
}

#[derive(Clone)]
struct RemoteForward {
    handle: SharedSshHandle,
    bind_addr: String,
    port: u32,
}

impl TunnelWorker {
    pub async fn start(
        tunnel: Tunnel,
        handle: SharedSshHandle,
        forwarded_rx: Option<mpsc::Receiver<ForwardedTcp>>,
        log_sender: LogSink,
    ) -> Result<Self, String> {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let (task, remote_forward) = match &tunnel.forward {
            ForwardSpec::Local { listen, target } => {
                let listener = bind_tcp_listener(&listen.host, listen.port).await?;
                let task = tokio::spawn(run_local_forward(
                    listener,
                    handle,
                    target.host.clone(),
                    target.port,
                    shutdown_rx,
                    log_sender,
                ));
                (task, None)
            }
            ForwardSpec::Socks5 { listen } => {
                let listener = bind_tcp_listener(&listen.host, listen.port).await?;
                let task = tokio::spawn(run_socks5_forward(
                    listener,
                    handle,
                    shutdown_rx,
                    log_sender,
                ));
                (task, None)
            }
            ForwardSpec::Remote { listen, target } => {
                send_log(
                    &log_sender,
                    format!(
                        "[INFO] Requesting Remote Forward to listen on SSH server {}:{}...",
                        listen.host, listen.port
                    ),
                );
                send_log(
                    &log_sender,
                    format!(
                        "[INFO] Remote Forward will send traffic to target {}:{}",
                        target.host, target.port
                    ),
                );

                let requested_port = listen.port as u32;
                let allocated_port = timeout(FORWARD_CONNECT_TIMEOUT, async {
                    handle
                        .read()
                        .await
                        .tcpip_forward(listen.host.clone(), requested_port)
                        .await
                })
                .await
                .map_err(|_| "Remote forward listen request timed out".to_string())?
                .map_err(|e| format!("Remote forward listen request failed: {}", e))?;
                let active_port = if requested_port == 0 {
                    allocated_port
                } else {
                    requested_port
                };
                let forwarded_rx = forwarded_rx
                    .ok_or_else(|| "Remote forward receiver is unavailable".to_string())?;
                let task = tokio::spawn(run_remote_forward(
                    forwarded_rx,
                    target.host.clone(),
                    target.port,
                    active_port,
                    shutdown_rx,
                    log_sender,
                ));
                (
                    task,
                    Some(RemoteForward {
                        handle,
                        bind_addr: listen.host.clone(),
                        port: active_port,
                    }),
                )
            }
        };

        Ok(Self {
            shutdown_tx,
            task,
            remote_forward,
        })
    }

    pub async fn stop(self) {
        let _ = self.shutdown_tx.send(true);
        // Let each worker drop its listener and abort and join all child connections.
        let _ = self.task.await;
        if let Some(remote) = self.remote_forward {
            let _ = timeout(Duration::from_secs(2), async {
                remote
                    .handle
                    .read()
                    .await
                    .cancel_tcpip_forward(remote.bind_addr, remote.port)
                    .await
            })
            .await;
        }
    }
}

async fn bind_tcp_listener(host: &str, port: u16) -> Result<TcpListener, String> {
    TcpListener::bind((host, port))
        .await
        .map_err(|e| format!("Failed to bind local listener {host}:{port}: {e}"))
}

async fn run_local_forward(
    listener: TcpListener,
    handle: SharedSshHandle,
    target_host: String,
    target_port: u16,
    mut shutdown_rx: watch::Receiver<bool>,
    log: LogSink,
) {
    send_log(
        &log,
        format!(
            "[INFO] Starting Local Forward to target {}:{}...",
            target_host, target_port
        ),
    );

    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            biased;
            _ = shutdown_rx.changed() => break,
            Some(_) = connections.join_next(), if !connections.is_empty() => {},
            res = listener.accept(), if connections.len() < MAX_CONNECTIONS => {
                match res {
                    Ok((socket, addr)) => {
                        send_log(&log, format!("[INFO] Accepted connection from {}", addr));
                        connections.spawn(pipe_local_connection(socket, handle.clone(), target_host.clone(), target_port, log.clone()));
                    }
                    Err(e) => send_log(&log, format!("[ERROR] Accept error: {}", e)),
                }
            }
        }
    }
    drop(listener);
    connections.shutdown().await;
    send_log(&log, "[INFO] Local Forward worker stopped".to_string());
}

async fn pipe_local_connection(
    mut socket: TcpStream,
    handle: SharedSshHandle,
    target_host: String,
    target_port: u16,
    log: LogSink,
) {
    send_log(
        &log,
        format!(
            "[INFO] Opening SSH channel to {}:{}",
            target_host, target_port
        ),
    );
    let open_channel = async {
        handle
            .read()
            .await
            .channel_open_direct_tcpip(target_host.clone(), target_port as u32, "127.0.0.1", 0)
            .await
    };

    match timeout_result(
        FORWARD_CONNECT_TIMEOUT,
        open_channel,
        format!(
            "SSH channel connection timed out after {}s: {}:{}",
            FORWARD_CONNECT_TIMEOUT.as_secs(),
            target_host,
            target_port
        ),
    )
    .await
    {
        Ok(Ok(channel)) => {
            send_log(
                &log,
                format!(
                    "[INFO] Forwarding to target {}:{} via SSH",
                    target_host, target_port
                ),
            );
            let mut stream = channel.into_stream();
            if let Err(e) = copy_bidirectional(&mut socket, &mut stream).await {
                send_log(&log, format!("[ERROR] Forwarding stream failed: {}", e));
            }
        }
        Ok(Err(e)) => send_log(
            &log,
            format!("[ERROR] SSH channel connection failed: {}", e),
        ),
        Err(message) => send_log(&log, format!("[ERROR] {}", message)),
    }
}

async fn run_socks5_forward(
    listener: TcpListener,
    handle: SharedSshHandle,
    mut shutdown_rx: watch::Receiver<bool>,
    log: LogSink,
) {
    send_log(&log, "[INFO] Starting SOCKS5 Dynamic Proxy...".to_string());

    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            biased;
            _ = shutdown_rx.changed() => break,
            Some(_) = connections.join_next(), if !connections.is_empty() => {},
            res = listener.accept(), if connections.len() < MAX_CONNECTIONS => {
                match res {
                    Ok((socket, addr)) => {
                        send_log(&log, format!("[INFO] SOCKS5 connection from {}", addr));
                        let task_handle = handle.clone();
                        let task_log = log.clone();
                        connections.spawn(pipe_socks5_connection(socket, task_handle, task_log));
                    }
                    Err(e) => send_log(&log, format!("[ERROR] Accept error: {}", e)),
                }
            }
        }
    }
    drop(listener);
    connections.shutdown().await;
    send_log(&log, "[INFO] SOCKS5 worker stopped".to_string());
}

async fn pipe_socks5_connection(mut socket: TcpStream, handle: SharedSshHandle, log: LogSink) {
    let (host, port) = match timeout(SOCKS_HANDSHAKE_TIMEOUT, negotiate_socks5(&mut socket)).await {
        Ok(Ok(destination)) => destination,
        Ok(Err(error)) => {
            send_log(&log, format!("[ERROR] SOCKS5 negotiation failed: {error}"));
            return;
        }
        Err(_) => {
            send_log(&log, "[ERROR] SOCKS5 handshake timed out".to_string());
            return;
        }
    };
    let channel = timeout(FORWARD_CONNECT_TIMEOUT, async {
        handle
            .read()
            .await
            .channel_open_direct_tcpip(host, port as u32, "127.0.0.1", 0)
            .await
    })
    .await;
    match channel {
        Ok(Ok(channel)) => {
            if send_reply(&mut socket, Socks5Reply::Succeeded)
                .await
                .is_err()
            {
                return;
            }
            let mut stream = channel.into_stream();
            if let Err(error) = copy_bidirectional(&mut socket, &mut stream).await {
                send_log(&log, format!("[ERROR] SOCKS5 forwarding failed: {error}"));
            }
        }
        Ok(Err(error)) => {
            let _ = send_reply(&mut socket, Socks5Reply::GeneralFailure).await;
            send_log(&log, format!("[ERROR] SOCKS5 destination failed: {error}"));
        }
        Err(_) => {
            let _ = send_reply(&mut socket, Socks5Reply::HostUnreachable).await;
            send_log(&log, "[ERROR] SOCKS5 destination timed out".to_string());
        }
    }
}

async fn run_remote_forward(
    mut forwarded_rx: mpsc::Receiver<ForwardedTcp>,
    target_host: String,
    target_port: u16,
    remote_listen_port: u32,
    mut shutdown_rx: watch::Receiver<bool>,
    log: LogSink,
) {
    send_log(
        &log,
        format!(
            "[INFO] Remote Forward listener started on SSH server port {}",
            remote_listen_port
        ),
    );

    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            biased;
            _ = shutdown_rx.changed() => break,
            Some(_) = connections.join_next(), if !connections.is_empty() => {},
            forwarded = forwarded_rx.recv(), if connections.len() < MAX_CONNECTIONS => {
                let Some(forwarded) = forwarded else { break };
                send_log(
                    &log,
                    format!(
                        "[INFO] Received remote connection on {}:{} from {}:{}",
                        forwarded.connected_address,
                        forwarded.connected_port,
                        forwarded.originator_address,
                        forwarded.originator_port
                    ),
                );
                connections.spawn(pipe_remote_connection(forwarded, target_host.clone(), target_port, log.clone()));
            }
        }
    }
    drop(forwarded_rx);
    connections.shutdown().await;
    send_log(&log, "[INFO] Remote Forward worker stopped".to_string());
}

async fn pipe_remote_connection(forwarded: ForwardedTcp, host: String, port: u16, log: LogSink) {
    let target = format!("{host}:{port}");
    match timeout_result(
        FORWARD_CONNECT_TIMEOUT,
        TcpStream::connect((host.as_str(), port)),
        format!(
            "Target connection timed out after {}s: {}",
            FORWARD_CONNECT_TIMEOUT.as_secs(),
            target
        ),
    )
    .await
    {
        Ok(Ok(mut target_stream)) => {
            send_log(&log, format!("[INFO] Connected to target {}", target));
            let mut ssh_stream = forwarded.channel.into_stream();
            if let Err(e) = copy_bidirectional(&mut ssh_stream, &mut target_stream).await {
                send_log(
                    &log,
                    format!("[ERROR] Remote forwarding stream failed: {}", e),
                );
            }
        }
        Ok(Err(e)) => send_log(
            &log,
            format!("[ERROR] Failed to connect to target {}: {}", target, e),
        ),
        Err(message) => send_log(&log, format!("[ERROR] {}", message)),
    }
}

fn send_log(log: &LogSink, message: String) {
    log.send(message);
}

async fn timeout_result<T, E, F>(
    duration: Duration,
    future: F,
    timeout_message: String,
) -> Result<Result<T, E>, String>
where
    F: Future<Output = Result<T, E>>,
{
    timeout(duration, future).await.map_err(|_| timeout_message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::net::TcpListener as StdTcpListener;

    async fn assert_send<F: Future + Send>(future: F) -> F::Output {
        future.await
    }

    #[tokio::test]
    async fn bind_tcp_listener_reports_occupied_port() {
        let occupied = StdTcpListener::bind("127.0.0.1:0").unwrap();
        let address = occupied.local_addr().unwrap();
        let addr = address.to_string();

        let err = bind_tcp_listener(&address.ip().to_string(), address.port())
            .await
            .unwrap_err();

        assert!(err.contains("Failed to bind local listener"));
        assert!(err.contains(&addr));
    }

    #[tokio::test]
    async fn worker_stop_future_is_send() {
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);
        let worker = TunnelWorker {
            shutdown_tx,
            task: tokio::spawn(async {}),
            remote_forward: None,
        };

        assert_send(worker.stop()).await;
    }

    #[tokio::test]
    async fn timeout_result_returns_error_for_pending_future() {
        let err = timeout_result(
            std::time::Duration::from_millis(1),
            std::future::pending::<Result<(), &'static str>>(),
            "forward setup timed out".to_string(),
        )
        .await
        .unwrap_err();

        assert_eq!(err, "forward setup timed out");
    }
}
