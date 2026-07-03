pub mod multiplex;
pub mod transport;

use std::error::Error;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::tunnel::multiplex::{connect_to_local, pipe_streams_monitored};

struct LoopGuard {
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl Drop for LoopGuard {
    fn drop(&mut self) {
        if let Some(h) = self.handle.take() {
            h.abort();
        }
    }
}

async fn relay_control_channel(mut control: TcpStream, peer: TcpStream, tunnel_id: i64) {
    let _ = control.write_u8(1).await;
    let _ = control.flush().await;
    let _ = pipe_streams_monitored(peer, control, tunnel_id).await;
}

pub async fn run_server(
    control_port: u16,
    public_port: u16,
    token: &str,
    protocol: &str,
    decoy: Option<String>,
    tunnel_id: i64,
) -> Result<(), Box<dyn Error>> {
    println!(
        "[SERVER] Launching protocol: '{}' on control port: {}, public port: {}",
        protocol, control_port, public_port
    );

    let control_addr: std::net::SocketAddr = format!("0.0.0.0:{}", control_port).parse()?;
    let public_addr: std::net::SocketAddr = format!("0.0.0.0:{}", public_port).parse()?;

    let control_listener = crate::common::network::bind_listener(control_addr)?;
    let public_listener = Arc::new(crate::common::network::bind_listener(public_addr)?);
    println!("[SERVER] Listening for public user traffic on port: {}", public_port);

    // Use an mpsc channel to queue authenticated control sockets from client nodes.
    // This solves the single-connection bottleneck: the client connects multiple times,
    // each authenticated connection is pushed into the channel, and the public accept loop
    // pulls one out per incoming user connection.
    let (control_tx, mut control_rx) = tokio::sync::mpsc::channel::<TcpStream>(64);

    // Spawn task to accept and authenticate control connections from client nodes.
    // Wrap in LoopGuard so the task is auto-aborted when run_server returns.
    let token_owned = token.to_string();
    let protocol_owned = protocol.to_string();
    let decoy_owned = decoy.clone();
    let _accept_guard = LoopGuard {
        handle: Some(tokio::spawn(async move {
            loop {
                match control_listener.accept().await {
                    Ok((control_socket, addr)) => {
                        println!("[SERVER] Client node connected from: {}", addr);

                        let control_socket = match transport::server_handshake(
                            control_socket,
                            &protocol_owned,
                            &token_owned,
                            decoy_owned.clone(),
                        )
                        .await
                        {
                            Ok(s) => s,
                            Err(e) => {
                                eprintln!("[SERVER] Handshake failed: {}", e);
                                continue;
                            }
                        };

                        // Push authenticated control socket into the channel
                        if control_tx.send(control_socket).await.is_err() {
                            eprintln!("[SERVER] Control channel closed, stopping accept loop");
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("[SERVER] Control listener error: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }
            }
        })),
    };

    // Main loop: accept public user connections and pair each with a queued control socket
    while let Ok((user_socket, user_addr)) = public_listener.accept().await {
        let _ = crate::common::network::optimize_socket(&user_socket);
        println!("[SERVER] User connected from {} to public port, waiting for control socket...", user_addr);

        // Wait for a control socket from the channel with a timeout
        let control_socket = match tokio::time::timeout(
            tokio::time::Duration::from_secs(10),
            control_rx.recv(),
        )
        .await
        {
            Ok(Some(cs)) => cs,
            Ok(None) => {
                eprintln!("[SERVER] Control channel closed, no more client nodes available");
                break;
            }
            Err(_) => {
                eprintln!("[SERVER] Timeout waiting for control socket, dropping user connection from {}", user_addr);
                continue;
            }
        };

        // Spawn relay in a separate task so we can immediately accept the next user
        let tid = tunnel_id;
        tokio::spawn(async move {
            relay_control_channel(control_socket, user_socket, tid).await;
        });
    }

    Ok(())
}

pub async fn run_client(
    server_ips: &str,
    control_port: u16,
    _public_port: u16,
    local_service: &str,
    token: &str,
    protocol: &str,
    tunnel_id: i64,
) -> Result<(), Box<dyn Error>> {
    let ips: Vec<&str> = server_ips
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if ips.is_empty() {
        return Err("No server IPs provided".into());
    }

    let mut ip_index = 0;

    loop {
        let current_ip = ips[ip_index % ips.len()];
        println!(
            "[CLIENT] Connecting to Iran Server {}:{} via '{}' (Failover index: {})...",
            current_ip, control_port, protocol, ip_index
        );

        let control_socket = match TcpStream::connect(format!("{}:{}", current_ip, control_port)).await
        {
            Ok(s) => {
                let _ = crate::common::network::optimize_socket(&s);
                s
            }
            Err(e) => {
                eprintln!(
                    "[CLIENT] Connection to {} failed: {}. Trying next IP in 3s...",
                    current_ip, e
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                ip_index += 1;
                continue;
            }
        };

        println!("[CLIENT] Connected to Iran control port successfully");

        let mut control_socket =
            match transport::client_handshake(control_socket, protocol, token).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!(
                        "[CLIENT] Handshake failed on {}: {}. Trying next IP in 3s...",
                        current_ip, e
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    ip_index += 1;
                    continue;
                }
            };

        println!("[CLIENT] Handshake succeeded");
        println!("[CLIENT] Waiting for tunnel relay signal...");

        let signal = match control_socket.read_u8().await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[CLIENT] Failed to read relay signal: {}", e);
                ip_index += 1;
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                continue;
            }
        };

        if signal != 1 {
            eprintln!("[CLIENT] Unexpected relay signal byte: {}", signal);
            ip_index += 1;
            continue;
        }

        println!(
            "[CLIENT] Relay signal received, connecting to local service: {}...",
            local_service
        );

        let local_conn = match connect_to_local(local_service).await {
            Ok(s) => {
                let _ = crate::common::network::optimize_socket(&s);
                s
            }
            Err(e) => {
                eprintln!(
                    "[CLIENT] Failed to connect to local service ({}): {}",
                    local_service, e
                );
                ip_index += 1;
                continue;
            }
        };

        // Spawn relay in a separate task so we can immediately reconnect
        // to the server for the next user connection, enabling true concurrency.
        let tid = tunnel_id;
        tokio::spawn(async move {
            pipe_streams_monitored(control_socket, local_conn, tid).await;
            println!("[CLIENT] Relay task finished for tunnel_id={}", tid);
        });

        // Reset ip_index on success (we got a valid connection from this IP)
        ip_index = 0;

        // Immediately loop back to connect a new control socket for the next user
        println!("[CLIENT] Relay spawned, reconnecting for next user...");
    }
}
