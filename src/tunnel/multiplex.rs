use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, Mutex};
use std::collections::HashMap;

use crate::common::obfuscate::apply_jitter;

pub struct TunnelTraffic {
    pub rx_bytes: AtomicU64,
    pub tx_bytes: AtomicU64,
}

/// Global thread-safe registry to accumulate traffic byte counts per tunnel ID.
pub static TRAFFIC_REGISTRY: OnceLock<Mutex<HashMap<i64, Arc<TunnelTraffic>>>> = OnceLock::new();

pub fn get_traffic_tracker(tunnel_id: i64) -> Arc<TunnelTraffic> {
    let registry = TRAFFIC_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = registry.lock().unwrap();
    map.entry(tunnel_id)
        .or_insert_with(|| {
            Arc::new(TunnelTraffic {
                rx_bytes: AtomicU64::new(0),
                tx_bytes: AtomicU64::new(0),
            })
        })
        .clone()
}

/// Pipes data bidirectionally between two streams, counting bytes in real-time
/// and applying dynamic AI Jitter on packet transfer.
pub async fn pipe_streams_monitored<S1, S2>(
    stream1: S1,
    stream2: S2,
    tunnel_id: i64,
) where
    S1: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    S2: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let tracker = get_traffic_tracker(tunnel_id);
    let (mut r1, mut w1) = tokio::io::split(stream1);
    let (mut r2, mut w2) = tokio::io::split(stream2);

    let tracker_rx = tracker.clone();
    let t1 = tokio::spawn(async move {
        let mut buf = [0u8; 16384];
        loop {
            // Apply AI Jitter on read path
            apply_jitter().await;
            
            match r1.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    tracker_rx.rx_bytes.fetch_add(n as u64, Ordering::SeqCst);
                    if w2.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                    let _ = w2.flush().await;
                }
                Err(_) => break,
            }
        }
    });

    let tracker_tx = tracker.clone();
    let t2 = tokio::spawn(async move {
        let mut buf = [0u8; 16384];
        loop {
            // Apply AI Jitter on write path
            apply_jitter().await;

            match r2.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    tracker_tx.tx_bytes.fetch_add(n as u64, Ordering::SeqCst);
                    if w1.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                    let _ = w1.flush().await;
                }
                Err(_) => break,
            }
        }
    });

    // Wait for either copy direction to complete, then terminate both
    let _ = tokio::join!(t1, t2);
}

/// Legacy/Direct pipe without monitoring (used for control connections)
#[allow(dead_code)]
pub async fn pipe_streams<S1, S2>(mut stream1: S1, mut stream2: S2)
where
    S1: AsyncRead + AsyncWrite + Unpin,
    S2: AsyncRead + AsyncWrite + Unpin,
{
    let _ = tokio::io::copy_bidirectional(&mut stream1, &mut stream2).await;
}

/// Helper to connect to local service
pub async fn connect_to_local(target: &str) -> Result<TcpStream, std::io::Error> {
    TcpStream::connect(target).await
}
