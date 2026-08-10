use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

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
            // Return bracketed IPv6 for standard socket parsing
            format!(
                "[{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}]",
                ipv6[0], ipv6[1], ipv6[2], ipv6[3], ipv6[4], ipv6[5], ipv6[6], ipv6[7],
                ipv6[8], ipv6[9], ipv6[10], ipv6[11], ipv6[12], ipv6[13], ipv6[14], ipv6[15]
            )
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

    // Respond with success. The client expects:
    // VER=0x05, REP=0x00 (success), RSV=0x00, ATYP=0x01 (IPv4), BND.ADDR=4 bytes, BND.PORT=2 bytes
    stream
        .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await
        .map_err(|e| format!("Failed to write SOCKS5 request reply: {}", e))?;

    Ok((host, port))
}
