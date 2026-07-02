// Transport handshakes module
use std::error::Error;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::common::crypto::generate_auth_header;

/// Read data from socket until we get at least `min_bytes` or a delimiter pattern.
/// Returns accumulated bytes as String. Times out after 5 seconds.
async fn read_handshake(socket: &mut TcpStream, min_bytes: usize) -> Result<String, Box<dyn Error>> {
    let mut buf = vec![0u8; 4096];
    let mut accumulated = Vec::new();
    
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);
    
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        
        match tokio::time::timeout(remaining, socket.read(&mut buf)).await {
            Ok(Ok(0)) => break, // EOF
            Ok(Ok(n)) => {
                accumulated.extend_from_slice(&buf[..n]);
                // For HTTP-based handshakes, check for \r\n\r\n delimiter
                if accumulated.len() >= min_bytes || accumulated.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            Ok(Err(e)) => return Err(Box::new(e)),
            Err(_) => break, // timeout
        }
    }
    
    Ok(String::from_utf8_lossy(&accumulated).to_string())
}

/// Client handshake wrapper
pub async fn client_handshake(
    mut socket: TcpStream,
    protocol: &str,
    token: &str,
) -> Result<TcpStream, Box<dyn Error>> {
    match protocol {
        "beam" | "tcpmux" => {
            // Simple PSK handshake
            let header = generate_auth_header(token);
            socket.write_all(header.as_bytes()).await?;
            socket.flush().await?;
            
            // Read response ack
            let mut ack = [0u8; 3];
            socket.read_exact(&mut ack).await?;
            if &ack != b"ACK" {
                return Err("Server authentication failed".into());
            }
        }
        "aura" | "httpmux" => {
            // Send standard HTTP GET upgrade request
            let req = format!(
                "GET /chat HTTP/1.1\r\n\
                 Host: localhost\r\n\
                 Upgrade: websocket\r\n\
                 Connection: Upgrade\r\n\
                 Authorization: {}\r\n\r\n",
                generate_auth_header(token)
            );
            socket.write_all(req.as_bytes()).await?;
            socket.flush().await?;

            // Read response (with accumulation)
            let resp = read_handshake(&mut socket, 20).await?;
            if !resp.contains("101 Switching Protocols") {
                return Err("HTTP Upgrade failed on server".into());
            }
        }
        "glimmer" | "wsmux" => {
            // Websocket handshake
            let header = generate_auth_header(token);
            let req = format!(
                "GET /ws HTTP/1.1\r\n\
                 Host: localhost\r\n\
                 Upgrade: websocket\r\n\
                 Connection: Upgrade\r\n\
                 Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                 Authorization: {}\r\n\r\n",
                header
            );
            socket.write_all(req.as_bytes()).await?;
            socket.flush().await?;

            let resp = read_handshake(&mut socket, 20).await?;
            if !resp.contains("101 Switching Protocols") {
                return Err("WebSocket handshake failed".into());
            }
        }
        _ => {
            // Fallback to simple handshake for other protocol profiles
            let header = generate_auth_header(token);
            socket.write_all(header.as_bytes()).await?;
            socket.flush().await?;
            
            let mut ack = [0u8; 3];
            socket.read_exact(&mut ack).await?;
        }
    }
    Ok(socket)
}

/// Server handshake wrapper
pub async fn server_handshake(
    mut socket: TcpStream,
    protocol: &str,
    token: &str,
    decoy: Option<String>,
) -> Result<TcpStream, Box<dyn Error>> {
    match protocol {
        "beam" | "tcpmux" => {
            // Read authentication header (with accumulation for robustness)
            let header = read_handshake(&mut socket, 10).await?;
            
            let expected = generate_auth_header(token);
            if !header.contains(&expected) {
                return Err("Client authentication failed".into());
            }
            
            // Send ACK
            socket.write_all(b"ACK").await?;
            socket.flush().await?;
        }
        "aura" | "httpmux" => {
            // Read HTTP GET request (accumulate until \r\n\r\n)
            let req = read_handshake(&mut socket, 50).await?;
            
            let expected = generate_auth_header(token);
            if !req.contains(&expected) {
                // If not authenticated, return decoy website (Active-Probing Defense!)
                let decoy_resp = if let Some(ref d) = decoy {
                    if d.starts_with("http://") || d.starts_with("https://") {
                        format!(
                            "HTTP/1.1 302 Found\r\n\
                             Location: {}\r\n\
                             Content-Length: 0\r\n\
                             Connection: close\r\n\r\n",
                            d
                        )
                    } else {
                        format!(
                            "HTTP/1.1 200 OK\r\n\
                             Content-Type: text/html; charset=UTF-8\r\n\
                             Content-Length: {}\r\n\
                             Connection: close\r\n\r\n\
                             {}",
                            d.len(),
                            d
                        )
                    }
                } else {
                    let default_body = "<!DOCTYPE html><html><head><title>Welcome</title></head><body><h1>Under Construction</h1><p>This site is coming soon. Check back later.</p></body></html>";
                    format!(
                        "HTTP/1.1 200 OK\r\n\
                         Content-Type: text/html; charset=UTF-8\r\n\
                         Content-Length: {}\r\n\
                         Connection: close\r\n\r\n\
                         {}",
                        default_body.len(),
                        default_body
                    )
                };
                socket.write_all(decoy_resp.as_bytes()).await?;
                socket.flush().await?;
                return Err("Active probe detected, sent decoy response".into());
            }

            // Return 101 upgrade response
            let resp = "HTTP/1.1 101 Switching Protocols\r\n\
                        Upgrade: websocket\r\n\
                        Connection: Upgrade\r\n\r\n";
            socket.write_all(resp.as_bytes()).await?;
            socket.flush().await?;
        }
        "glimmer" | "wsmux" => {
            let req = read_handshake(&mut socket, 50).await?;
            
            let expected = generate_auth_header(token);
            if !req.contains(&expected) {
                return Err("Unauthorized WebSocket connection".into());
            }

            let resp = "HTTP/1.1 101 Switching Protocols\r\n\
                        Upgrade: websocket\r\n\
                        Connection: Upgrade\r\n\
                        Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n";
            socket.write_all(resp.as_bytes()).await?;
            socket.flush().await?;
        }
        _ => {
            // Default simple fallback verification (accumulate reads)
            let header = read_handshake(&mut socket, 10).await?;
            let expected = generate_auth_header(token);
            if !header.contains(&expected) {
                return Err("Client authentication failed".into());
            }
            socket.write_all(b"ACK").await?;
            socket.flush().await?;
        }
    }
    Ok(socket)
}
