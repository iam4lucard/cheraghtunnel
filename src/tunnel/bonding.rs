use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::net::TcpStream;
use crate::common::network::optimize_socket;

/// Multi-Path Tunnel Bonding Pool
/// Manages multiple redundant transport channels to aggregate bandwidth and provide seamless failover.
#[derive(Clone)]
pub struct MultiPathBondingPool {
    active_index: Arc<AtomicUsize>,
    channel_count: usize,
}

impl MultiPathBondingPool {
    pub fn new(channel_count: usize) -> Self {
        Self {
            active_index: Arc::new(AtomicUsize::new(0)),
            channel_count: channel_count.max(1),
        }
    }

    /// Selects the next active channel index for packet bonding / load distribution
    pub fn next_channel(&self) -> usize {
        let idx = self.active_index.fetch_add(1, Ordering::Relaxed);
        idx % self.channel_count
    }

    /// Optimizes a bonded socket stream with MTU clamping and socket options
    pub fn optimize_bonded_stream(&self, stream: &TcpStream, mss_opt: Option<u32>) -> std::io::Result<()> {
        optimize_socket(stream)?;
        if let Some(mss) = mss_opt {
            let _ = crate::common::network::set_tcp_mss_clamp(stream, mss);
        }
        Ok(())
    }
}
