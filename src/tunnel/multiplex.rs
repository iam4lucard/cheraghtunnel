use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, Mutex};
use std::collections::HashMap;

pub struct TunnelTraffic {
    pub rx_bytes: AtomicU64,
    pub tx_bytes: AtomicU64,
    pub quota_limit: AtomicU64,
    pub quota_used: AtomicU64,
    pub speed_limit: std::sync::atomic::AtomicU32, // in KB/s
    pub rtt_ms: std::sync::atomic::AtomicU32,
    pub last_time: Mutex<std::time::Instant>,
    pub bytes_this_sec: std::sync::atomic::AtomicU32,
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
                quota_limit: AtomicU64::new(0),
                quota_used: AtomicU64::new(0),
                speed_limit: std::sync::atomic::AtomicU32::new(0),
                rtt_ms: std::sync::atomic::AtomicU32::new(0),
                last_time: Mutex::new(std::time::Instant::now()),
                bytes_this_sec: std::sync::atomic::AtomicU32::new(0),
            })
        })
        .clone()
}

use std::pin::Pin;
use std::task::{Context, Poll};
use std::future::Future;
use tokio::io::ReadBuf;

pub struct MonitoredStream<S> {
    inner: S,
    tracker: Arc<TunnelTraffic>,
    delay: Option<Pin<Box<tokio::time::Sleep>>>,
}

impl<S> MonitoredStream<S> {
    pub fn new(inner: S, tracker: Arc<TunnelTraffic>) -> Self {
        Self { inner, tracker, delay: None }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for MonitoredStream<S> {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        // Enforce quota limit
        let limit = self.tracker.quota_limit.load(Ordering::Relaxed);
        if limit > 0 {
            let used = self.tracker.quota_used.load(Ordering::Relaxed);
            if used >= limit {
                return Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "Quota limit exceeded",
                )));
            }
        }

        let before = buf.filled().len();
        let res = Pin::new(&mut self.inner).poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = &res {
            let after = buf.filled().len();
            if after > before {
                let n = after - before;
                self.tracker.rx_bytes.fetch_add(n as u64, Ordering::Relaxed);
            }
        }
        res
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for MonitoredStream<S> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        // Poll active pacing delay if present
        if let Some(ref mut d) = self.delay {
            if d.as_mut().poll(cx).is_pending() {
                return Poll::Pending;
            }
            self.delay = None;
        }

        // Enforce quota limit
        let limit = self.tracker.quota_limit.load(Ordering::Relaxed);
        if limit > 0 {
            let used = self.tracker.quota_used.load(Ordering::Relaxed);
            if used >= limit {
                return Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "Quota limit exceeded",
                )));
            }
        }

        // Enforce speed limit with RTT-based BBR-like congestion pacing
        let conf_limit = self.tracker.speed_limit.load(Ordering::Relaxed);
        let speed_limit = if conf_limit > 0 {
            let rtt = self.tracker.rtt_ms.load(Ordering::Relaxed);
            if rtt > 400 && rtt < 999 {
                // High RTT detected! Restrict speed to 250 KB/s to drain queue buffers
                std::cmp::min(conf_limit, 250)
            } else {
                conf_limit
            }
        } else {
            0
        };

        if speed_limit > 0 {
            let need_sleep = {
                let now = std::time::Instant::now();
                let mut last = self.tracker.last_time.lock().unwrap();
                let elapsed = now.duration_since(*last).as_secs_f64();
                if elapsed >= 1.0 {
                    *last = now;
                    self.tracker.bytes_this_sec.store(0, Ordering::Relaxed);
                }
                let current = self.tracker.bytes_this_sec.load(Ordering::Relaxed);
                current >= (speed_limit as u32 * 1024)
            };

            if need_sleep {
                let mut d = Box::pin(tokio::time::sleep(tokio::time::Duration::from_millis(50)));
                if d.as_mut().poll(cx).is_pending() {
                    self.delay = Some(d);
                    return Poll::Pending;
                }
            }
        }

        let res = Pin::new(&mut self.inner).poll_write(cx, buf);
        if let Poll::Ready(Ok(n)) = &res {
            if speed_limit > 0 {
                self.tracker.bytes_this_sec.fetch_add(*n as u32, Ordering::Relaxed);
            }
            self.tracker.tx_bytes.fetch_add(*n as u64, Ordering::Relaxed);
        }
        res
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Pipes data bidirectionally between two streams, counting bytes in real-time.
/// Uses tokio::io::copy_bidirectional_with_sizes with 64KB buffers to minimize syscalls and guarantee ultra low-latency full-duplex transfer.
pub async fn pipe_streams_monitored<S1, S2>(
    stream1: S1,
    mut stream2: S2,
    tunnel_id: i64,
) where
    S1: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    S2: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let tracker = get_traffic_tracker(tunnel_id);
    let mut monitored_stream1 = MonitoredStream::new(stream1, tracker);
    
    let _ = tokio::io::copy_bidirectional_with_sizes(&mut monitored_stream1, &mut stream2, 65536, 65536).await;
}

/// Legacy/Direct pipe without monitoring (used for control connections)
#[allow(dead_code)]
pub async fn pipe_streams<S1, S2>(mut stream1: S1, mut stream2: S2)
where
    S1: AsyncRead + AsyncWrite + Unpin,
    S2: AsyncRead + AsyncWrite + Unpin,
{
    let _ = tokio::io::copy_bidirectional_with_sizes(&mut stream1, &mut stream2, 65536, 65536).await;
}

/// Helper to connect to local service
pub async fn connect_to_local(target: &str) -> Result<TcpStream, std::io::Error> {
    TcpStream::connect(target).await
}
