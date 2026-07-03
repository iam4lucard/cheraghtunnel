use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, Mutex};
use std::collections::HashMap;

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

/// Pipes data bidirectionally between two streams, counting bytes in real-time.
/// Uses a select loop so that when either direction closes (EOF or error),
/// the relay terminates immediately and both streams are dropped — preventing
/// connection and task leaks.
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

    let mut buf1 = [0u8; 16384];
    let mut buf2 = [0u8; 16384];

    loop {
        tokio::select! {
            result = r1.read(&mut buf1) => {
                match result {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        tracker.rx_bytes.fetch_add(n as u64, Ordering::SeqCst);
                        if w2.write_all(&buf1[..n]).await.is_err() {
                            break;
                        }
                        let _ = w2.flush().await;
                    }
                }
            }
            result = r2.read(&mut buf2) => {
                match result {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        tracker.tx_bytes.fetch_add(n as u64, Ordering::SeqCst);
                        if w1.write_all(&buf2[..n]).await.is_err() {
                            break;
                        }
                        let _ = w1.flush().await;
                    }
                }
            }
        }
    }

    // Explicitly shut down both write halves so the remote peers get a FIN
    // and don't hang waiting for data that will never arrive.
    let _ = w1.shutdown().await;
    let _ = w2.shutdown().await;
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
