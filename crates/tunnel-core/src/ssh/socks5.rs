use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Socks5Reply {
    Succeeded = 0x00,
    GeneralFailure = 0x01,
    HostUnreachable = 0x04,
}

pub async fn send_reply<S: AsyncWrite + Unpin>(
    stream: &mut S,
    reply: Socks5Reply,
) -> Result<(), String> {
    stream
        .write_all(&[0x05, reply as u8, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await
        .map_err(|e| format!("Failed to write SOCKS5 request reply: {e}"))
}

/// Parse a CONNECT request. The caller must open the destination before replying.
pub async fn negotiate_socks5<S>(stream: &mut S) -> Result<(String, u16), String>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut header = [0u8; 2];
    stream
        .read_exact(&mut header)
        .await
        .map_err(|e| format!("Failed to read SOCKS5 greeting: {}", e))?;

    if header[0] != 0x05 {
        return Err(format!("Unsupported SOCKS version: {}", header[0]));
    }

    let num_methods = header[1] as usize;
    let mut methods = vec![0u8; num_methods];
    stream
        .read_exact(&mut methods)
        .await
        .map_err(|e| format!("Failed to read SOCKS5 auth methods: {}", e))?;

    // We only support No Auth (0x00)
    if !methods.contains(&0x00) {
        // Send failure reply
        stream.write_all(&[0x05, 0xFF]).await.ok();
        return Err("No supported authentication methods".to_string());
    }

    // Send No Auth selected reply
    stream
        .write_all(&[0x05, 0x00])
        .await
        .map_err(|e| format!("Failed to write SOCKS5 greeting reply: {}", e))?;

    // Read request header
    let mut req_header = [0u8; 4];
    stream
        .read_exact(&mut req_header)
        .await
        .map_err(|e| format!("Failed to read SOCKS5 request header: {}", e))?;

    if req_header[0] != 0x05 {
        return Err(format!(
            "Unsupported SOCKS version in request: {}",
            req_header[0]
        ));
    }

    let cmd = req_header[1];
    if cmd != 0x01 {
        // Only CONNECT is supported
        stream
            .write_all(&[0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
            .await
            .ok();
        return Err(format!("Unsupported command: {}", cmd));
    }

    // req_header[2] is reserved 0x00
    let atyp = req_header[3];
    let host = match atyp {
        0x01 => {
            // IPv4: 4 bytes
            let mut ipv4 = [0u8; 4];
            stream
                .read_exact(&mut ipv4)
                .await
                .map_err(|e| format!("Failed to read SOCKS5 IPv4: {}", e))?;
            format!("{}.{}.{}.{}", ipv4[0], ipv4[1], ipv4[2], ipv4[3])
        }
        0x03 => {
            // Domain name: 1 byte length + string
            let len = stream
                .read_u8()
                .await
                .map_err(|e| format!("Failed to read SOCKS5 domain len: {}", e))?;
            let mut domain = vec![0u8; len as usize];
            stream
                .read_exact(&mut domain)
                .await
                .map_err(|e| format!("Failed to read SOCKS5 domain: {}", e))?;
            String::from_utf8(domain).map_err(|e| format!("Invalid UTF-8 domain name: {}", e))?
        }
        0x04 => {
            // IPv6: 16 bytes
            let mut ipv6 = [0u8; 16];
            stream
                .read_exact(&mut ipv6)
                .await
                .map_err(|e| format!("Failed to read SOCKS5 IPv6: {}", e))?;
            // SSH carries the host and port separately, so it needs a bare address.
            std::net::Ipv6Addr::from(ipv6).to_string()
        }
        _ => {
            stream
                .write_all(&[0x05, 0x08, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .await
                .ok();
            return Err(format!("Unsupported address type: {}", atyp));
        }
    };

    let port = stream
        .read_u16()
        .await
        .map_err(|e| format!("Failed to read SOCKS5 port: {}", e))?;

    Ok((host, port))
}
