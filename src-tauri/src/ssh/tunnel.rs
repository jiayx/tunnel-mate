use crate::config::{Tunnel, TunnelType};
use crate::ssh::engine::{ForwardedTcp, SharedSshHandle};
use crate::ssh::socks5::negotiate_socks5;
use std::net::TcpListener as StdTcpListener;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};

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
        log_sender: tauri::ipc::Channel<String>,
    ) -> Result<Self, String> {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let tunnel_type = tunnel.tunnel_type.clone();
        let local_host = tunnel
            .local_host
            .clone()
            .unwrap_or_else(|| "127.0.0.1".to_string());
        let local_port = tunnel.local_port;
        let remote_host = tunnel.remote_host.clone().unwrap_or_default();
        let remote_port = tunnel.remote_port.unwrap_or(0);

        let (task, remote_forward) = match tunnel_type {
            TunnelType::Local => {
                let bind_addr = format!("{}:{}", local_host, local_port);
                let listener = bind_tcp_listener(&bind_addr).await?;
                let task = tokio::spawn(run_local_forward(
                    listener,
                    handle,
                    remote_host,
                    remote_port,
                    shutdown_rx,
                    log_sender,
                ));
                (task, None)
            }
            TunnelType::Socks5 => {
                let bind_addr = format!("{}:{}", local_host, local_port);
                let listener = bind_tcp_listener(&bind_addr).await?;
                let task = tokio::spawn(run_socks5_forward(
                    listener,
                    handle,
                    shutdown_rx,
                    log_sender,
                ));
                (task, None)
            }
            TunnelType::Remote => {
                let bind_addr = "127.0.0.1".to_string();
                let requested_port = remote_port as u32;
                send_log(
                    &log_sender,
                    format!(
                        "[INFO] Requesting Remote Forward to listen on remote port {}...",
                        requested_port
                    ),
                );

                let allocated_port = handle
                    .lock()
                    .await
                    .tcpip_forward(bind_addr.clone(), requested_port)
                    .await
                    .map_err(|e| format!("Remote forward listen request failed: {}", e))?;
                let port = if requested_port == 0 {
                    allocated_port
                } else {
                    requested_port
                };
                let forwarded_rx = forwarded_rx
                    .ok_or_else(|| "Remote forward receiver is unavailable".to_string())?;
                let task = tokio::spawn(run_remote_forward(
                    forwarded_rx,
                    local_host,
                    local_port,
                    port,
                    shutdown_rx,
                    log_sender,
                ));
                (
                    task,
                    Some(RemoteForward {
                        handle,
                        bind_addr,
                        port,
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
        .map_err(|e| format!("Failed to bind local port {}: {}", bind_addr, e))?;
    std_listener
        .set_nonblocking(true)
        .map_err(|e| format!("Failed to set listener nonblocking: {}", e))?;
    TcpListener::from_std(std_listener).map_err(|e| format!("Failed to create listener: {}", e))
}

async fn run_local_forward(
    listener: TcpListener,
    handle: SharedSshHandle,
    remote_host: String,
    remote_port: u16,
    mut shutdown_rx: watch::Receiver<bool>,
    log: tauri::ipc::Channel<String>,
) {
    send_log(
        &log,
        format!(
            "[INFO] Starting Local Forward to {}:{}...",
            remote_host, remote_port
        ),
    );

    loop {
        tokio::select! {
            res = listener.accept() => {
                match res {
                    Ok((socket, addr)) => {
                        send_log(&log, format!("[INFO] Accepted connection from {}", addr));
                        tokio::spawn(pipe_local_connection(socket, handle.clone(), remote_host.clone(), remote_port, log.clone()));
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
    remote_host: String,
    remote_port: u16,
    log: tauri::ipc::Channel<String>,
) {
    match handle
        .lock()
        .await
        .channel_open_direct_tcpip(remote_host.clone(), remote_port as u32, "127.0.0.1", 0)
        .await
    {
        Ok(channel) => {
            send_log(
                &log,
                format!(
                    "[INFO] Forwarding to {}:{} via SSH",
                    remote_host, remote_port
                ),
            );
            let mut stream = channel.into_stream();
            if let Err(e) = copy_bidirectional(&mut socket, &mut stream).await {
                send_log(&log, format!("[ERROR] Forwarding stream failed: {}", e));
            }
        }
        Err(e) => send_log(
            &log,
            format!("[ERROR] SSH channel connection failed: {}", e),
        ),
    }
}

async fn run_socks5_forward(
    listener: TcpListener,
    handle: SharedSshHandle,
    mut shutdown_rx: watch::Receiver<bool>,
    log: tauri::ipc::Channel<String>,
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
    local_host: String,
    local_port: u16,
    remote_port: u32,
    mut shutdown_rx: watch::Receiver<bool>,
    log: tauri::ipc::Channel<String>,
) {
    send_log(
        &log,
        "[INFO] Remote Forward listener started on remote SSH server".to_string(),
    );

    loop {
        tokio::select! {
            Some(forwarded) = forwarded_rx.recv() => {
                let target = format!("{}:{}", local_host, local_port);
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
                send_log(&log, format!("[INFO] Remote Forward on port {} closed", remote_port));
                break;
            }
        }
    }
}

async fn pipe_remote_connection(
    forwarded: ForwardedTcp,
    target: String,
    log: tauri::ipc::Channel<String>,
) {
    match TcpStream::connect(&target).await {
        Ok(mut local_stream) => {
            send_log(
                &log,
                format!("[INFO] Connected to local forward target {}", target),
            );
            let mut ssh_stream = forwarded.channel.into_stream();
            if let Err(e) = copy_bidirectional(&mut ssh_stream, &mut local_stream).await {
                send_log(
                    &log,
                    format!("[ERROR] Remote forwarding stream failed: {}", e),
                );
            }
        }
        Err(e) => send_log(
            &log,
            format!(
                "[ERROR] Failed to connect to local target {}: {}",
                target, e
            ),
        ),
    }
}

fn send_log(log: &tauri::ipc::Channel<String>, message: String) {
    let _ = log.send(message);
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

        assert!(err.contains("Failed to bind local port"));
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
}
