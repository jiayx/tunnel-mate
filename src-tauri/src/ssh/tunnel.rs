use crate::config::{Tunnel, TunnelType};
use crate::ssh::socks5::negotiate_socks5;
use ssh2::{Channel, Session};
use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener as StdTcpListener;
use std::net::TcpStream as StdTcpStream;
use std::sync::Arc;
use tokio::net::TcpListener as TokioTcpListener;
use tokio::sync::broadcast;

pub struct TunnelWorker {
    shutdown_tx: broadcast::Sender<()>,
}

impl TunnelWorker {
    pub fn start(
        tunnel: Tunnel,
        session: Session,
        log_sender: tauri::ipc::Channel<String>, // We'll use this to stream logs to frontend
    ) -> Result<Self, String> {
        let (shutdown_tx, _) = broadcast::channel(1);
        let worker = Self {
            shutdown_tx: shutdown_tx.clone(),
        };

        let tunnel_type = tunnel.tunnel_type.clone();
        let local_host = tunnel
            .local_host
            .clone()
            .unwrap_or_else(|| "127.0.0.1".to_string());
        let local_port = tunnel.local_port;
        let remote_host = tunnel.remote_host.clone().unwrap_or_default();
        let remote_port = tunnel.remote_port.unwrap_or(0);
        let mut shutdown_rx = shutdown_tx.subscribe();

        // Spawn based on tunnel type
        match tunnel_type {
            TunnelType::Local => {
                let bind_addr = format!("{}:{}", local_host, local_port);
                let std_listener = StdTcpListener::bind(&bind_addr)
                    .map_err(|e| format!("Failed to bind local port {}: {}", bind_addr, e))?;
                std_listener
                    .set_nonblocking(true)
                    .map_err(|e| format!("Failed to set local listener nonblocking: {}", e))?;
                let listener = TokioTcpListener::from_std(std_listener)
                    .map_err(|e| format!("Failed to create local listener: {}", e))?;

                tokio::spawn(async move {
                    let log = log_sender.clone();
                    let _ = log.send(format!(
                        "[INFO] Starting Local Forward on {}:{}...",
                        local_host, local_port
                    ));

                    let session_arc = Arc::new(session);

                    loop {
                        tokio::select! {
                            accept_res = listener.accept() => {
                                match accept_res {
                                    Ok((socket, addr)) => {
                                        let _ = log.send(format!("[INFO] Accepted connection from {}", addr));

                                        let std_socket = match socket.into_std() {
                                            Ok(s) => s,
                                            Err(e) => {
                                                let _ = log.send(format!("[ERROR] Failed to convert socket: {}", e));
                                                continue;
                                            }
                                        };

                                        let sess = session_arc.clone();
                                        let r_host = remote_host.clone();
                                        let r_port = remote_port;
                                        let l_log = log.clone();

                                        std::thread::spawn(move || {
                                            match sess.channel_direct_tcpip(&r_host, r_port, None) {
                                                Ok(channel) => {
                                                    let _ = l_log.send(format!("[INFO] Forwarding to {}:{} via SSH", r_host, r_port));
                                                    pipe_bidirectional(std_socket, channel);
                                                }
                                                Err(e) => {
                                                    let _ = l_log.send(format!("[ERROR] SSH channel connection failed: {}", e));
                                                }
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        let _ = log.send(format!("[ERROR] Accept error: {}", e));
                                    }
                                }
                            }
                            _ = shutdown_rx.recv() => {
                                let _ = log.send("[INFO] Local Forward worker shutting down...".to_string());
                                break;
                            }
                        }
                    }
                });
            }
            TunnelType::Socks5 => {
                let bind_addr = format!("{}:{}", local_host, local_port);
                let std_listener = StdTcpListener::bind(&bind_addr)
                    .map_err(|e| format!("Failed to bind local port {}: {}", bind_addr, e))?;
                std_listener
                    .set_nonblocking(true)
                    .map_err(|e| format!("Failed to set SOCKS5 listener nonblocking: {}", e))?;
                let listener = TokioTcpListener::from_std(std_listener)
                    .map_err(|e| format!("Failed to create SOCKS5 listener: {}", e))?;

                tokio::spawn(async move {
                    let log = log_sender.clone();
                    let _ = log.send(format!(
                        "[INFO] Starting SOCKS5 Dynamic Proxy on {}:{}...",
                        local_host, local_port
                    ));

                    let session_arc = Arc::new(session);

                    loop {
                        tokio::select! {
                            accept_res = listener.accept() => {
                                match accept_res {
                                    Ok((mut socket, addr)) => {
                                        let _ = log.send(format!("[INFO] SOCKS5 connection from {}", addr));

                                        let sess = session_arc.clone();
                                        let l_log = log.clone();

                                        tokio::spawn(async move {
                                            match negotiate_socks5(&mut socket).await {
                                                Ok((dest_host, dest_port)) => {
                                                    let _ = l_log.send(format!("[INFO] SOCKS5 target: {}:{}", dest_host, dest_port));

                                                    if let Ok(std_socket) = socket.into_std() {
                                                        std::thread::spawn(move || {
                                                            match sess.channel_direct_tcpip(&dest_host, dest_port, None) {
                                                                Ok(channel) => {
                                                                    pipe_bidirectional(std_socket, channel);
                                                                }
                                                                Err(e) => {
                                                                    let _ = l_log.send(format!("[ERROR] SOCKS5 failed to open channel: {}", e));
                                                                }
                                                            }
                                                        });
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = l_log.send(format!("[ERROR] SOCKS5 negotiation failed: {}", e));
                                                }
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        let _ = log.send(format!("[ERROR] Accept error: {}", e));
                                    }
                                }
                            }
                            _ = shutdown_rx.recv() => {
                                let _ = log.send("[INFO] SOCKS5 worker shutting down...".to_string());
                                break;
                            }
                        }
                    }
                });
            }
            TunnelType::Remote => {
                let log = log_sender.clone();
                let _ = log.send(format!(
                    "[INFO] Requesting Remote Forward to listen on remote port {}...",
                    remote_port
                ));

                // Request remote forward
                let (mut listener, _) =
                    match session.channel_forward_listen(remote_port, None, None) {
                        Ok(l) => l,
                        Err(e) => {
                            let _ = log.send(format!(
                                "[ERROR] Remote forward listen request failed: {}",
                                e
                            ));
                            return Err(format!("Remote forward listen request failed: {}", e));
                        }
                    };

                session.set_blocking(false);
                let session_arc = Arc::new(session);

                std::thread::spawn(move || {
                    let _ = log.send(
                        "[INFO] Remote Forward listener started on remote SSH server".to_string(),
                    );

                    loop {
                        // Check shutdown signal
                        if shutdown_rx.try_recv().is_ok() {
                            let _ = log
                                .send("[INFO] Remote Forward worker shutting down...".to_string());
                            break;
                        }

                        match listener.accept() {
                            Ok(channel) => {
                                let _ = log.send(format!(
                                    "[INFO] Received remote connection request on port {}",
                                    remote_port
                                ));

                                let l_host = local_host.clone();
                                let l_port = local_port;
                                let l_log = log.clone();

                                std::thread::spawn(move || {
                                    match StdTcpStream::connect(format!("{}:{}", l_host, l_port)) {
                                        Ok(local_stream) => {
                                            let _ = l_log.send(format!(
                                                "[INFO] Connected to local forward target {}:{}",
                                                l_host, l_port
                                            ));
                                            pipe_bidirectional(local_stream, channel);
                                        }
                                        Err(e) => {
                                            let _ = l_log.send(format!("[ERROR] Failed to connect to local target {}:{}: {}", l_host, l_port, e));
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                let err_msg = e.to_string();
                                let io_err: std::io::Error = e.into();
                                if io_err.kind() == ErrorKind::WouldBlock {
                                    std::thread::sleep(std::time::Duration::from_millis(50));
                                    continue;
                                }

                                // If the session is disconnected, break
                                if !session_arc.authenticated() {
                                    let _ = log.send("[INFO] SSH Session disconnected. Remote Forward worker exiting.".to_string());
                                    break;
                                }
                                let _ = log.send(format!("[ERROR] Remote Forward accept error: {}", err_msg));
                                // Small sleep to prevent tight looping on non-fatal errors
                                std::thread::sleep(std::time::Duration::from_millis(100));
                            }
                        }
                    }
                });
            }
        }

        Ok(worker)
    }

    pub fn stop(self) {
        let _ = self.shutdown_tx.send(());
    }
}

fn pipe_bidirectional(mut socket: StdTcpStream, mut channel: Channel) {
    let mut socket_clone = match socket.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };
    let mut channel_clone = channel.clone();

    // Spawn thread for one direction: socket -> channel
    std::thread::spawn(move || {
        let mut buffer = [0u8; 16384];
        loop {
            match socket_clone.read(&mut buffer) {
                Ok(0) => {
                    if channel_clone.eof() {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Ok(n) => {
                    if write_all_retry(&mut channel_clone, &buffer[..n]).is_err() {
                        break;
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
        channel_clone.close().ok();
    });

    // Handle other direction in current thread: channel -> socket
    let mut buffer = [0u8; 16384];
    loop {
        match channel.read(&mut buffer) {
            Ok(0) => {
                if channel.eof() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Ok(n) => {
                if write_all_retry(&mut socket, &buffer[..n]).is_err() {
                    break;
                }
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(_) => break,
        }
    }
    socket.shutdown(std::net::Shutdown::Both).ok();
}

fn write_all_retry<W: Write>(writer: &mut W, mut buf: &[u8]) -> std::io::Result<()> {
    let mut zero_writes = 0;
    while !buf.is_empty() {
        match writer.write(buf) {
            Ok(0) => {
                zero_writes += 1;
                if zero_writes > 1000 {
                    return Err(std::io::Error::new(
                        ErrorKind::WriteZero,
                        "failed to write forwarded data",
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Ok(n) => {
                zero_writes = 0;
                buf = &buf[n..];
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
