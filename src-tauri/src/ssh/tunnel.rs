use crate::config::{ForwardSpec, Tunnel};
use crate::ssh::engine::{ForwardedTcp, SharedSshHandle};
use crate::ssh::socks5::negotiate_socks5;
use std::future::Future;
use std::net::TcpListener as StdTcpListener;
use tauri::ipc::Channel;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};
use tokio::time::{timeout, Duration};

const FORWARD_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub enum LogSink {
    Channel {
        channel: Channel<String>,
        session_id: String,
    },
    Silent,
}

impl LogSink {
    pub fn send(&self, message: String) {
        if let LogSink::Channel {
            channel,
            session_id,
        } = self
        {
            let short_session_id = session_id.get(..8).unwrap_or(session_id);
            let _ = channel.send(format!("[session:{}] {}", short_session_id, message));
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
        forwarded_rx: Option<mpsc::UnboundedReceiver<ForwardedTcp>>,
        log_sender: LogSink,
    ) -> Result<Self, String> {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let (task, remote_forward) = match &tunnel.forward {
            ForwardSpec::Local { listen, target } => {
                let bind_addr = format!("{}:{}", listen.host, listen.port);
                let listener = bind_tcp_listener(&bind_addr).await?;
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
                let bind_addr = format!("{}:{}", listen.host, listen.port);
                let listener = bind_tcp_listener(&bind_addr).await?;
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
                let allocated_port = handle
                    .lock()
                    .await
                    .tcpip_forward(listen.host.clone(), requested_port)
                    .await
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
        if let Some(remote) = self.remote_forward {
            let _ = remote
                .handle
                .lock()
                .await
                .cancel_tcpip_forward(remote.bind_addr, remote.port)
                .await;
        }
        let _ = self.shutdown_tx.send(true);
        self.task.abort();
        let _ = self.task.await;
    }
}

async fn bind_tcp_listener(bind_addr: &str) -> Result<TcpListener, String> {
    let std_listener = StdTcpListener::bind(bind_addr)
        .map_err(|e| format!("Failed to bind local listener {}: {}", bind_addr, e))?;
    std_listener
        .set_nonblocking(true)
        .map_err(|e| format!("Failed to set listener nonblocking: {}", e))?;
    TcpListener::from_std(std_listener).map_err(|e| format!("Failed to create listener: {}", e))
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

    loop {
        tokio::select! {
            res = listener.accept() => {
                match res {
                    Ok((socket, addr)) => {
                        send_log(&log, format!("[INFO] Accepted connection from {}", addr));
                        tokio::spawn(pipe_local_connection(socket, handle.clone(), target_host.clone(), target_port, log.clone()));
                    }
                    Err(e) => send_log(&log, format!("[ERROR] Accept error: {}", e)),
                }
            }
            _ = shutdown_rx.changed() => {
                send_log(&log, "[INFO] Local Forward worker shutting down...".to_string());
                break;
            }
        }
    }
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
            .lock()
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

    loop {
        tokio::select! {
            res = listener.accept() => {
                match res {
                    Ok((mut socket, addr)) => {
                        send_log(&log, format!("[INFO] SOCKS5 connection from {}", addr));
                        let task_handle = handle.clone();
                        let task_log = log.clone();
                        tokio::spawn(async move {
                            match negotiate_socks5(&mut socket).await {
                                Ok((dest_host, dest_port)) => {
                                    pipe_local_connection(socket, task_handle, dest_host, dest_port, task_log).await;
                                }
                                Err(e) => send_log(&task_log, format!("[ERROR] SOCKS5 negotiation failed: {}", e)),
                            }
                        });
                    }
                    Err(e) => send_log(&log, format!("[ERROR] Accept error: {}", e)),
                }
            }
            _ = shutdown_rx.changed() => {
                send_log(&log, "[INFO] SOCKS5 worker shutting down...".to_string());
                break;
            }
        }
    }
}

async fn run_remote_forward(
    mut forwarded_rx: mpsc::UnboundedReceiver<ForwardedTcp>,
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

    loop {
        tokio::select! {
            Some(forwarded) = forwarded_rx.recv() => {
                let target = format!("{}:{}", target_host, target_port);
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
                tokio::spawn(pipe_remote_connection(forwarded, target, log.clone()));
            }
            _ = shutdown_rx.changed() => {
                send_log(&log, "[INFO] Remote Forward worker shutting down...".to_string());
                break;
            }
            else => {
                send_log(&log, format!("[INFO] Remote Forward on port {} closed", remote_listen_port));
                break;
            }
        }
    }
}

async fn pipe_remote_connection(forwarded: ForwardedTcp, target: String, log: LogSink) {
    match timeout_result(
        FORWARD_CONNECT_TIMEOUT,
        TcpStream::connect(&target),
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

    async fn assert_send<F: Future + Send>(future: F) -> F::Output {
        future.await
    }

    #[tokio::test]
    async fn bind_tcp_listener_reports_occupied_port() {
        let occupied = StdTcpListener::bind("127.0.0.1:0").unwrap();
        let addr = occupied.local_addr().unwrap().to_string();

        let err = bind_tcp_listener(&addr).await.unwrap_err();

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
