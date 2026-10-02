#[cfg(test)]
mod tests {
    use russh::server::{self, Auth, ChannelOpenHandle, Msg, Session};
    use russh::{
        keys::{ssh_key, PrivateKey},
        Channel,
    };
    use std::net::{Ipv6Addr, SocketAddr};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::{mpsc, Notify};
    use tokio::time::timeout;
    use tunnel_core::config::{Endpoint, ForwardSpec, Tunnel};
    use tunnel_core::ssh::engine::{ConnectOptions, KnownHostsPolicy, SshSession};
    use tunnel_core::ssh::socks5::negotiate_socks5;
    use tunnel_core::{LogSink, TunnelWorker};

    async fn greeting<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(client: &mut S) {
        client.write_all(&[5, 1, 0]).await.unwrap();
        let mut reply = [0; 2];
        timeout(Duration::from_secs(2), client.read_exact(&mut reply))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(reply, [5, 0]);
    }

    #[tokio::test]
    async fn socks_must_not_claim_success_before_opening_target() {
        let (mut client, mut server) = tokio::io::duplex(1024);
        let negotiation = tokio::spawn(async move { negotiate_socks5(&mut server).await });
        greeting(&mut client).await;
        client
            .write_all(&[5, 1, 0, 1, 192, 0, 2, 1, 0, 80])
            .await
            .unwrap();
        let mut reply = [0; 10];
        let result = timeout(Duration::from_millis(200), client.read_exact(&mut reply)).await;
        let parsed = negotiation.await.unwrap().unwrap();
        assert_eq!(parsed, ("192.0.2.1".into(), 80));
        assert!(
            !matches!(result, Ok(Ok(_))) || reply[1] != 0,
            "received SOCKS success {:?} although this process has not opened any destination connection", reply
        );
    }

    #[tokio::test]
    async fn ipv6_destination_must_be_a_bare_address_for_ssh() {
        let (mut client, mut server) = tokio::io::duplex(1024);
        let negotiation = tokio::spawn(async move { negotiate_socks5(&mut server).await });
        greeting(&mut client).await;
        let mut request = vec![5, 1, 0, 4];
        request.extend_from_slice(&Ipv6Addr::LOCALHOST.octets());
        request.extend_from_slice(&443_u16.to_be_bytes());
        client.write_all(&request).await.unwrap();
        let (host, port) = negotiation.await.unwrap().unwrap();
        assert_eq!(port, 443);
        assert!(
            host.parse::<Ipv6Addr>().is_ok(),
            "SSH destination host contains socket-address brackets: {host}"
        );
    }

    #[derive(Clone)]
    struct TestServer {
        seen: mpsc::UnboundedSender<String>,
        release_slow: Arc<Notify>,
        forward_target: Option<SocketAddr>,
    }

    impl server::Handler for TestServer {
        type Error = russh::Error;

        async fn auth_password(&mut self, _: &str, _: &str) -> Result<Auth, Self::Error> {
            Ok(Auth::Accept)
        }

        async fn channel_open_direct_tcpip(
            &mut self,
            channel: Channel<Msg>,
            host: &str,
            _: u32,
            _: &str,
            _: u32,
            reply: ChannelOpenHandle,
            _: &mut Session,
        ) -> Result<(), Self::Error> {
            let host = host.to_string();
            let _ = self.seen.send(host.clone());
            let release = self.release_slow.clone();
            let forward_target = self.forward_target;
            tokio::spawn(async move {
                if host == "reject.example" {
                    // Dropping the reply rejects the SSH channel.
                    return;
                }
                if let Some(address) = forward_target {
                    let mut destination = TcpStream::connect(address).await.unwrap();
                    reply.accept().await;
                    let mut stream = channel.into_stream();
                    let _ = tokio::io::copy_bidirectional(&mut stream, &mut destination).await;
                    return;
                }
                if host == "slow.example" {
                    release.notified().await;
                }
                reply.accept().await;
                let mut stream = channel.into_stream();
                let _ = stream.write_all(b"ok").await;
                if host == "hold.example" {
                    release.notified().await;
                }
                let _ = stream.shutdown().await;
            });
            Ok(())
        }
    }

    async fn spawn_test_server(
        forward_target: Option<SocketAddr>,
    ) -> (
        SocketAddr,
        String,
        mpsc::UnboundedReceiver<String>,
        Arc<Notify>,
    ) {
        let (seen, received) = mpsc::unbounded_channel();
        let release_slow = Arc::new(Notify::new());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = TestServer {
            seen,
            release_slow: release_slow.clone(),
            forward_target,
        };
        let key = PrivateKey::random(&mut rand::rng(), ssh_key::Algorithm::Ed25519).unwrap();
        let fingerprint = key
            .public_key()
            .fingerprint(ssh_key::HashAlg::Sha256)
            .to_string();
        let config = Arc::new(server::Config {
            keys: vec![key],
            ..Default::default()
        });
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let config = config.clone();
                let server = server.clone();
                tokio::spawn(async move {
                    if let Ok(session) = server::run_stream(config, stream, server).await {
                        let _ = session.await;
                    }
                });
            }
        });
        (address, fingerprint, received, release_slow)
    }

    async fn test_session() -> (SshSession, mpsc::UnboundedReceiver<String>, Arc<Notify>) {
        let (address, _, received, release_slow) = spawn_test_server(None).await;
        let session = SshSession::connect(ConnectOptions {
            host: "127.0.0.1",
            port: address.port(),
            user: "review",
            password: Some("test"),
            identity_file: None,
            passphrase: None,
            known_hosts_policy: KnownHostsPolicy::TrustOnce,
            jump_host_config: None,
        })
        .await
        .unwrap();
        (session, received, release_slow)
    }

    fn socks_tunnel(port: u16) -> Tunnel {
        Tunnel {
            id: "review".into(),
            name: "review".into(),
            description: None,
            group_id: None,
            ssh_host: "127.0.0.1".into(),
            ssh_port: 22,
            ssh_user: "review".into(),
            ssh_identity_file: None,
            ssh_password: None,
            jump_host_enabled: false,
            jump_host_id: None,
            jump_host: None,
            jump_port: None,
            jump_user: None,
            jump_identity_file: None,
            jump_password: None,
            forward: ForwardSpec::Socks5 {
                listen: Endpoint {
                    host: "127.0.0.1".into(),
                    port,
                },
            },
            start_with_app: false,
            auto_reconnect: false,
            retry_count: 0,
            retry_interval: 1,
        }
    }

    async fn socks_worker(session: &SshSession) -> (TunnelWorker, u16) {
        let reservation = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = reservation.local_addr().unwrap().port();
        drop(reservation);
        let worker =
            TunnelWorker::start(socks_tunnel(port), session.handle(), None, LogSink::Silent)
                .await
                .unwrap();
        (worker, port)
    }

    async fn request_domain(client: &mut TcpStream, name: &str) {
        greeting(client).await;
        let mut request = vec![5, 1, 0, 3, name.len() as u8];
        request.extend_from_slice(name.as_bytes());
        request.extend_from_slice(&80_u16.to_be_bytes());
        client.write_all(&request).await.unwrap();
    }

    #[tokio::test]
    async fn stopping_worker_must_close_half_finished_socks_handshakes() {
        let (mut session, _, _) = test_session().await;
        let (worker, port) = socks_worker(&session).await;
        let mut client = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        greeting(&mut client).await;
        worker.stop().await;
        session.disconnect().await;
        let mut byte = [0];
        let closed = timeout(Duration::from_millis(500), client.read(&mut byte)).await;
        assert!(
            matches!(closed, Ok(Ok(0)) | Ok(Err(_))),
            "SOCKS connection is still open after worker.stop() and SSH disconnect(): {closed:?}"
        );
    }

    #[tokio::test]
    async fn slow_destination_must_not_block_an_unrelated_destination() {
        let (mut session, mut seen, release) = test_session().await;
        let (worker, port) = socks_worker(&session).await;
        let mut slow = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        request_domain(&mut slow, "slow.example").await;
        assert_eq!(
            timeout(Duration::from_secs(2), seen.recv())
                .await
                .unwrap()
                .as_deref(),
            Some("slow.example")
        );
        let mut fast = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        request_domain(&mut fast, "fast.example").await;
        let fast_received = timeout(Duration::from_millis(500), seen.recv()).await;
        release.notify_one();
        worker.stop().await;
        session.disconnect().await;
        assert!(
            matches!(fast_received, Ok(Some(ref host)) if host == "fast.example"),
            "second SSH open request was blocked by the first destination: {fast_received:?}"
        );
    }

    #[tokio::test]
    async fn control_socks_connection_can_forward_when_server_replies() {
        let (mut session, mut seen, _) = test_session().await;
        let (worker, port) = socks_worker(&session).await;
        let mut client = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        request_domain(&mut client, "fast.example").await;
        assert_eq!(
            timeout(Duration::from_secs(2), seen.recv())
                .await
                .unwrap()
                .as_deref(),
            Some("fast.example")
        );
        let mut reply = [0; 12];
        timeout(Duration::from_secs(2), client.read_exact(&mut reply))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&reply[10..], b"ok");
        worker.stop().await;
        session.disconnect().await;
    }

    #[tokio::test]
    async fn failed_ssh_destination_returns_socks_failure() {
        let (mut session, _, _) = test_session().await;
        let (worker, port) = socks_worker(&session).await;
        let mut client = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        request_domain(&mut client, "reject.example").await;
        let mut reply = [0; 10];
        timeout(Duration::from_secs(2), client.read_exact(&mut reply))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(reply[0], 5);
        assert_ne!(
            reply[1], 0,
            "a rejected destination must not report success"
        );
        worker.stop().await;
        session.disconnect().await;
    }

    #[tokio::test]
    async fn stopping_worker_closes_active_streams_without_closing_shared_ssh() {
        let (mut session, _, _) = test_session().await;
        let (worker, port) = socks_worker(&session).await;
        let mut client = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        request_domain(&mut client, "hold.example").await;
        let mut reply = [0; 12];
        timeout(Duration::from_secs(2), client.read_exact(&mut reply))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&reply[10..], b"ok");
        worker.stop().await;
        let closed = timeout(Duration::from_secs(2), client.read(&mut [0]))
            .await
            .unwrap();
        assert!(matches!(closed, Ok(0) | Err(_)));
        assert!(!session.handle().read().await.is_closed());
        session.disconnect().await;
    }

    #[tokio::test]
    async fn incomplete_socks_handshake_has_a_deadline() {
        let (mut session, _, _) = test_session().await;
        let (worker, port) = socks_worker(&session).await;
        let mut client = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        greeting(&mut client).await;
        let closed = timeout(Duration::from_secs(12), client.read(&mut [0]))
            .await
            .unwrap();
        assert!(matches!(closed, Ok(0) | Err(_)));
        worker.stop().await;
        session.disconnect().await;
    }

    #[test]
    fn host_key_confirmation_uses_the_jump_route_and_checks_fingerprint() {
        const CHILD: &str = "TUNNEL_MATE_HOST_KEY_TEST_CHILD";
        if std::env::var_os(CHILD).is_none() {
            // Set profile paths before starting any child threads. Never write
            // to the developer's known_hosts or mutate HOME in a parallel test.
            let profile = tempfile::tempdir().unwrap();
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "tests::host_key_confirmation_uses_the_jump_route_and_checks_fingerprint",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .env("TUNNEL_MATE_CONFIG_DIR", profile.path())
                .env(
                    "TUNNEL_MATE_KNOWN_HOSTS_PATH",
                    profile.path().join("known_hosts"),
                )
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let (destination, fingerprint, _, _) = spawn_test_server(None).await;
            let (jump_address, jump_fingerprint, _, _) = spawn_test_server(Some(destination)).await;
            SshSession::trust_host_key_via(
                "127.0.0.1",
                jump_address.port(),
                &jump_fingerprint,
                None,
                None,
            )
            .await
            .unwrap();
            let mut jump = socks_tunnel(1080);
            jump.ssh_host = "127.0.0.1".into();
            jump.ssh_port = jump_address.port();
            jump.ssh_password = Some("test".into());
            // The mock jump maps this otherwise unreachable endpoint to the
            // destination; a direct connection is refused.
            let options = || ConnectOptions {
                host: "127.0.0.2",
                port: destination.port(),
                user: "review",
                password: Some("test"),
                identity_file: None,
                passphrase: None,
                known_hosts_policy: KnownHostsPolicy::RequireKnown,
                jump_host_config: Some(&jump),
            };
            let error = SshSession::connect(options())
                .await
                .err()
                .expect("target must initially be untrusted");
            assert!(
                error.starts_with("HOST_KEY_NOT_TRUSTED|127.0.0.2|"),
                "{error}"
            );
            SshSession::trust_host_key_via(
                "127.0.0.2",
                destination.port(),
                &fingerprint,
                Some(&jump),
                None,
            )
            .await
            .unwrap();
            let mut session = SshSession::connect(options()).await.unwrap();
            session.disconnect().await;
            let error = SshSession::trust_host_key_via(
                "127.0.0.2",
                destination.port(),
                "SHA256:unexpected",
                Some(&jump),
                None,
            )
            .await
            .unwrap_err();
            assert!(error.contains("changed again"), "{error}");
            let mut session = SshSession::connect(options()).await.unwrap();
            session.disconnect().await;
        });
    }
}
