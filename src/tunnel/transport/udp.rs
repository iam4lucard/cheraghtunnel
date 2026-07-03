use std::collections::{HashMap, VecDeque};
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, Mutex};

// Packet types for our custom reliable UDP layer
const PKT_SYN: u8 = 1;
const PKT_SYN_ACK: u8 = 2;
const PKT_DATA: u8 = 3;
const PKT_ACK: u8 = 4;
const PKT_FIN: u8 = 5;

// Helper function to send data over a UDP socket. Handles both connected and unconnected sockets.
async fn send_msg(socket: &UdpSocket, data: &[u8], peer: SocketAddr) -> io::Result<()> {
    match socket.send(data).await {
        Ok(_) => Ok(()),
        Err(_) => socket.send_to(data, peer).await.map(|_| ()),
    }
}

// Protocol styles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdpMode {
    Ray,       // Raw best-effort UDP
    Flash,     // Reliable sliding-window UDP
    Photon,    // Reliable UDP + FEC
    Halo,      // Reliable UDP + WebRTC/STUN framing
    Hysteria,  // High-speed paced reliable UDP
    Lantern,   // Reliable UDP + L3/TUN IP packet framing
}

struct SentPacket {
    seq: u32,
    data: Vec<u8>,
    sent_time: Instant,
    retries: u32,
}

struct FecEncoder {
    packet_counter: usize,
    buffer: Vec<Vec<u8>>,
}

impl FecEncoder {
    fn new() -> Self {
        Self {
            packet_counter: 0,
            buffer: Vec::new(),
        }
    }

    fn add_packet(&mut self, data: &[u8]) -> Option<Vec<u8>> {
        self.buffer.push(data.to_vec());
        self.packet_counter += 1;
        if self.packet_counter >= 4 {
            let max_len = self.buffer.iter().map(|v| v.len()).max().unwrap_or(0);
            let mut parity = vec![0u8; max_len];
            for pkt in &self.buffer {
                for (i, &b) in pkt.iter().enumerate() {
                    if i < parity.len() {
                        parity[i] ^= b;
                    }
                }
            }
            self.buffer.clear();
            self.packet_counter = 0;
            Some(parity)
        } else {
            None
        }
    }
}

struct FecDecoder {
    buffer: HashMap<u32, Vec<u8>>,
}

impl FecDecoder {
    fn new() -> Self {
        Self {
            buffer: HashMap::new(),
        }
    }

    fn add_and_recover(&mut self, seq: u32, data: &[u8]) -> Option<(u32, Vec<u8>)> {
        self.buffer.insert(seq, data.to_vec());
        let block_start = (seq / 5) * 5;
        let mut present = Vec::new();
        let mut missing = Vec::new();
        for i in 0..5 {
            let s = block_start + i;
            if self.buffer.contains_key(&s) {
                present.push(s);
            } else {
                missing.push(s);
            }
        }

        if missing.len() == 1 {
            let missing_seq = missing[0];
            let max_len = present.iter().map(|s| self.buffer.get(s).unwrap().len()).max().unwrap_or(0);
            let mut recovered = vec![0u8; max_len];
            for s in &present {
                let pkt = self.buffer.get(s).unwrap();
                for (i, &b) in pkt.iter().enumerate() {
                    if i < recovered.len() {
                        recovered[i] ^= b;
                    }
                }
            }
            self.buffer.insert(missing_seq, recovered.clone());
            Some((missing_seq, recovered))
        } else {
            None
        }
    }

    fn clean_old(&mut self, current_seq: u32) {
        if current_seq > 50 {
            let limit = current_seq - 50;
            self.buffer.retain(|&k, _| k >= limit);
        }
    }
}

pub struct UdpVirtualStreamInner {
    socket: Arc<UdpSocket>,
    peer: SocketAddr,
    mode: UdpMode,
    
    pub handshake_done: bool,
    
    rx_buf: VecDeque<u8>,
    rx_waker: Option<Waker>,
    next_expected_seq: u32,
    rx_out_of_order: HashMap<u32, Vec<u8>>,
    fec_decoder: FecDecoder,
    
    tx_waker: Option<Waker>,
    next_seq: u32,
    last_acked_seq: u32,
    unacked_packets: VecDeque<SentPacket>,
    fec_encoder: FecEncoder,
    
    is_closed: bool,
    
    last_sent_time: Instant,
    tokens: f64,
    max_tokens: f64,
}

pub struct UdpVirtualStream {
    pub inner: Arc<Mutex<UdpVirtualStreamInner>>,
    manager_handle: Option<tokio::task::JoinHandle<()>>,
}

impl Drop for UdpVirtualStream {
    fn drop(&mut self) {
        if let Some(h) = self.manager_handle.take() {
            h.abort();
        }
    }
}

impl UdpVirtualStream {
    pub fn new(
        socket: Arc<UdpSocket>,
        peer: SocketAddr,
        mode: UdpMode,
        rx: mpsc::Receiver<Vec<u8>>,
        handshake_done: bool,
    ) -> Self {
        let inner = Arc::new(Mutex::new(UdpVirtualStreamInner {
            socket,
            peer,
            mode,
            handshake_done,
            rx_buf: VecDeque::new(),
            rx_waker: None,
            next_expected_seq: 1,
            rx_out_of_order: HashMap::new(),
            fec_decoder: FecDecoder::new(),
            tx_waker: None,
            next_seq: 1,
            last_acked_seq: 0,
            unacked_packets: VecDeque::new(),
            fec_encoder: FecEncoder::new(),
            is_closed: false,
            last_sent_time: Instant::now(),
            tokens: 1000.0,
            max_tokens: 1000.0,
        }));

        let inner_clone = inner.clone();
        let manager_handle = tokio::spawn(async move {
            Self::manager_loop(inner_clone, rx).await;
        });

        Self {
            inner,
            manager_handle: Some(manager_handle),
        }
    }

    async fn manager_loop(inner: Arc<Mutex<UdpVirtualStreamInner>>, mut rx: mpsc::Receiver<Vec<u8>>) {
        let mut interval = tokio::time::interval(Duration::from_millis(15));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let mut lock = inner.lock().await;
                    if lock.is_closed {
                        break;
                    }
                    if lock.mode != UdpMode::Ray {
                        lock.handle_retransmissions().await;
                    }
                }
                pkt_opt = rx.recv() => {
                    match pkt_opt {
                        Some(pkt) => {
                            let mut lock = inner.lock().await;
                            lock.process_packet(&pkt).await;
                        }
                        None => {
                            let mut lock = inner.lock().await;
                            lock.is_closed = true;
                            if let Some(w) = lock.rx_waker.take() {
                                w.wake();
                            }
                            if let Some(w) = lock.tx_waker.take() {
                                w.wake();
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
}

impl UdpVirtualStreamInner {
    fn frame_packet(&self, pkt_type: u8, seq: u32, ack: u32, payload: &[u8]) -> Vec<u8> {
        let mut raw = Vec::with_capacity(32 + payload.len());
        
        if self.mode == UdpMode::Halo {
            raw.extend_from_slice(&[0x00, pkt_type, 0x00, payload.len() as u8]);
            raw.extend_from_slice(&[0x21, 0x12, 0xA4, 0x42]);
            raw.extend_from_slice(&seq.to_be_bytes());
            raw.extend_from_slice(&ack.to_be_bytes());
            raw.extend_from_slice(payload);
            return raw;
        }

        if self.mode == UdpMode::Lantern {
            let total_len = (20 + 9 + payload.len()) as u16;
            raw.extend_from_slice(&[0x45, 0x00]);
            raw.extend_from_slice(&total_len.to_be_bytes());
            raw.extend_from_slice(&[0x00, 0x00, 0x40, 0x00, 0x40, 0x11, 0x00, 0x00]);
            raw.extend_from_slice(&[10, 0, 0, 1]);
            raw.extend_from_slice(&[10, 0, 0, 2]);
            raw.push(pkt_type);
            raw.extend_from_slice(&seq.to_be_bytes());
            raw.extend_from_slice(&ack.to_be_bytes());
            raw.extend_from_slice(payload);
            return raw;
        }

        raw.push(pkt_type);
        raw.extend_from_slice(&seq.to_be_bytes());
        raw.extend_from_slice(&ack.to_be_bytes());
        raw.extend_from_slice(payload);
        raw
    }

    fn deframe_packet(&self, raw: &[u8]) -> Option<(u8, u32, u32, Vec<u8>)> {
        if self.mode == UdpMode::Halo {
            if raw.len() < 16 {
                return None;
            }
            let pkt_type = raw[1];
            let seq = u32::from_be_bytes([raw[8], raw[9], raw[10], raw[11]]);
            let ack = u32::from_be_bytes([raw[12], raw[13], raw[14], raw[15]]);
            let payload = raw[16..].to_vec();
            return Some((pkt_type, seq, ack, payload));
        }

        if self.mode == UdpMode::Lantern {
            if raw.len() < 29 {
                return None;
            }
            let pkt_type = raw[20];
            let seq = u32::from_be_bytes([raw[21], raw[22], raw[23], raw[24]]);
            let ack = u32::from_be_bytes([raw[25], raw[26], raw[27], raw[28]]);
            let payload = raw[29..].to_vec();
            return Some((pkt_type, seq, ack, payload));
        }

        if raw.len() < 9 {
            return None;
        }
        let pkt_type = raw[0];
        let seq = u32::from_be_bytes([raw[1], raw[2], raw[3], raw[4]]);
        let ack = u32::from_be_bytes([raw[5], raw[6], raw[7], raw[8]]);
        let payload = raw[9..].to_vec();
        Some((pkt_type, seq, ack, payload))
    }

    async fn send_packet_paced(&mut self, data: &[u8]) -> io::Result<()> {
        if self.mode == UdpMode::Hysteria {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_sent_time).as_secs_f64();
            self.last_sent_time = now;
            self.tokens = (self.tokens + elapsed * 2000.0).min(self.max_tokens);
            if self.tokens < 1.0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
                self.tokens += 0.001 * 2000.0;
            }
            self.tokens -= 1.0;
        }
        send_msg(&self.socket, data, self.peer).await
    }

    async fn handle_retransmissions(&mut self) {
        let now = Instant::now();
        let mut to_resend = Vec::new();
        
        for pkt in &mut self.unacked_packets {
            if now.duration_since(pkt.sent_time) > Duration::from_millis(100) {
                pkt.sent_time = now;
                pkt.retries += 1;
                to_resend.push(pkt.data.clone());
                if pkt.retries > 30 {
                    self.is_closed = true;
                    if let Some(w) = self.rx_waker.take() { w.wake(); }
                    if let Some(w) = self.tx_waker.take() { w.wake(); }
                    return;
                }
            }
        }

        for data in to_resend {
            let _ = self.send_packet_paced(&data).await;
        }
    }

    async fn process_packet(&mut self, raw: &[u8]) {
        if self.mode == UdpMode::Ray {
            self.rx_buf.extend(raw);
            if let Some(w) = self.rx_waker.take() {
                w.wake();
            }
            return;
        }

        let (pkt_type, seq, ack, payload) = match self.deframe_packet(raw) {
            Some(res) => res,
            None => return,
        };

        while let Some(pkt) = self.unacked_packets.front() {
            if pkt.seq <= ack {
                self.unacked_packets.pop_front();
            } else {
                break;
            }
        }
        self.last_acked_seq = self.last_acked_seq.max(ack);
        if let Some(w) = self.tx_waker.take() {
            w.wake();
        }

        match pkt_type {
            PKT_SYN => {
                let resp = self.frame_packet(PKT_SYN_ACK, 0, 0, &[]);
                let _ = send_msg(&self.socket, &resp, self.peer).await;
            }
            PKT_SYN_ACK => {
                self.handshake_done = true;
            }
            PKT_DATA => {
                if seq < self.next_expected_seq {
                    self.send_ack().await;
                    return;
                }

                if seq == self.next_expected_seq {
                    self.rx_buf.extend(&payload);
                    self.next_expected_seq += 1;

                    while let Some(buffered) = self.rx_out_of_order.remove(&self.next_expected_seq) {
                        self.rx_buf.extend(&buffered);
                        self.next_expected_seq += 1;
                    }

                    self.send_ack().await;
                    if let Some(w) = self.rx_waker.take() {
                        w.wake();
                    }
                } else if seq < self.next_expected_seq + 64 {
                    self.rx_out_of_order.insert(seq, payload.clone());
                    
                    if self.mode == UdpMode::Photon {
                        self.fec_decoder.clean_old(seq);
                        if let Some((recovered_seq, recovered_data)) = self.fec_decoder.add_and_recover(seq, &payload) {
                            if recovered_seq == self.next_expected_seq {
                                self.rx_buf.extend(&recovered_data);
                                self.next_expected_seq += 1;
                                while let Some(buffered) = self.rx_out_of_order.remove(&self.next_expected_seq) {
                                    self.rx_buf.extend(&buffered);
                                    self.next_expected_seq += 1;
                                }
                                if let Some(w) = self.rx_waker.take() {
                                    w.wake();
                                }
                            } else {
                                self.rx_out_of_order.insert(recovered_seq, recovered_data);
                            }
                        }
                    }
                    self.send_ack().await;
                }
            }
            PKT_ACK => {
                // Queue clean handled above
            }
            PKT_FIN => {
                self.is_closed = true;
                if let Some(w) = self.rx_waker.take() {
                    w.wake();
                }
            }
            _ => {}
        }
    }

    async fn send_ack(&mut self) {
        let ack_pkt = self.frame_packet(PKT_ACK, 0, self.next_expected_seq - 1, &[]);
        let _ = send_msg(&self.socket, &ack_pkt, self.peer).await;
    }

    pub async fn send_syn(&mut self) {
        let syn_pkt = self.frame_packet(PKT_SYN, 0, 0, &[]);
        let _ = send_msg(&self.socket, &syn_pkt, self.peer).await;
    }

    #[allow(dead_code)]
    pub async fn send_fin(&mut self) {
        let fin_pkt = self.frame_packet(PKT_FIN, 0, 0, &[]);
        let _ = send_msg(&self.socket, &fin_pkt, self.peer).await;
    }
}

impl AsyncRead for UdpVirtualStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let mut inner = match self.inner.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };

        if !inner.rx_buf.is_empty() {
            let n = std::cmp::min(buf.remaining(), inner.rx_buf.len());
            for _ in 0..n {
                if let Some(b) = inner.rx_buf.pop_front() {
                    buf.put_slice(&[b]);
                }
            }
            return Poll::Ready(Ok(()));
        }

        if inner.is_closed {
            return Poll::Ready(Ok(()));
        }

        inner.rx_waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl AsyncWrite for UdpVirtualStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut inner = match self.inner.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };

        if inner.is_closed {
            return Poll::Ready(Err(io::Error::new(io::ErrorKind::WriteZero, "Stream closed")));
        }

        if inner.mode == UdpMode::Ray {
            let socket = inner.socket.clone();
            let peer = inner.peer;
            let data = buf.to_vec();
            tokio::spawn(async move {
                let _ = send_msg(&socket, &data, peer).await;
            });
            return Poll::Ready(Ok(buf.len()));
        }

        if inner.next_seq - inner.last_acked_seq > 64 {
            inner.tx_waker = Some(cx.waker().clone());
            return Poll::Pending;
        }

        let seq = inner.next_seq;
        inner.next_seq += 1;

        let framed = inner.frame_packet(PKT_DATA, seq, inner.next_expected_seq - 1, buf);
        let socket = inner.socket.clone();
        let peer = inner.peer;
        let framed_clone = framed.clone();

        inner.unacked_packets.push_back(SentPacket {
            seq,
            data: framed,
            sent_time: Instant::now(),
            retries: 0,
        });

        if inner.mode == UdpMode::Photon {
            if let Some(parity) = inner.fec_encoder.add_packet(buf) {
                let parity_seq = seq + 1;
                let parity_framed = inner.frame_packet(PKT_DATA, parity_seq, inner.next_expected_seq - 1, &parity);
                let socket_fec = socket.clone();
                tokio::spawn(async move {
                    let _ = send_msg(&socket_fec, &parity_framed, peer).await;
                });
            }
        }

        let pacing_mode = inner.mode;
        let last_sent = inner.last_sent_time;
        let mut tokens = inner.tokens;
        let max_tokens = inner.max_tokens;

        tokio::spawn(async move {
            if pacing_mode == UdpMode::Hysteria {
                let now = Instant::now();
                let elapsed = now.duration_since(last_sent).as_secs_f64();
                tokens = (tokens + elapsed * 2000.0).min(max_tokens);
                if tokens < 1.0 {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            }
            let _ = send_msg(&socket, &framed_clone, peer).await;
        });

        inner.last_sent_time = Instant::now();
        inner.tokens = tokens;

        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut inner = match self.inner.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };

        if inner.is_closed {
            return Poll::Ready(Ok(()));
        }

        inner.is_closed = true;
        let socket = inner.socket.clone();
        let peer = inner.peer;
        let fin_pkt = inner.frame_packet(PKT_FIN, 0, 0, &[]);

        tokio::spawn(async move {
            let _ = send_msg(&socket, &fin_pkt, peer).await;
        });

        Poll::Ready(Ok(()))
    }
}

#[allow(dead_code)]
pub struct UdpMultiplexer {
    socket: Arc<UdpSocket>,
    sessions: Arc<Mutex<HashMap<SocketAddr, mpsc::Sender<Vec<u8>>>>>,
}

impl UdpMultiplexer {
    pub fn new(socket: UdpSocket, mode: UdpMode, new_conn_tx: mpsc::Sender<UdpVirtualStream>) -> Self {
        let socket = Arc::new(socket);
        let sessions: Arc<Mutex<HashMap<SocketAddr, mpsc::Sender<Vec<u8>>>>> = Arc::new(Mutex::new(HashMap::new()));
        
        let socket_clone = socket.clone();
        let sessions_clone = sessions.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 65535];
            loop {
                match socket_clone.recv_from(&mut buf).await {
                    Ok((n, addr)) => {
                        let data = buf[..n].to_vec();
                        let mut map = sessions_clone.lock().await;
                        
                        if let Some(tx) = map.get(&addr) {
                            if tx.send(data).await.is_err() {
                                map.remove(&addr);
                            }
                        } else {
                            let is_syn = if mode == UdpMode::Ray {
                                true
                            } else {
                                let pkt_type = match mode {
                                    UdpMode::Halo => data.get(1).copied(),
                                    UdpMode::Lantern => data.get(20).copied(),
                                    _ => data.first().copied(),
                                };
                                pkt_type.map(|b| b == PKT_SYN).unwrap_or(false)
                            };

                            if is_syn {
                                let (tx, rx) = mpsc::channel::<Vec<u8>>(1024);
                                map.insert(addr, tx);
                                
                                let virtual_stream = UdpVirtualStream::new(
                                    socket_clone.clone(),
                                    addr,
                                    mode,
                                    rx,
                                    false
                                );
                                
                                let _ = map.get(&addr).unwrap().send(data).await;
                                
                                if new_conn_tx.send(virtual_stream).await.is_err() {
                                    map.remove(&addr);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[UDP Multiplexer] Demux error: {}", e);
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                }
            }
        });

        Self { socket, sessions }
    }

    #[allow(dead_code)]
    pub async fn create_session(
        &self,
        peer: SocketAddr,
        mode: UdpMode,
        handshake_done: bool,
    ) -> UdpVirtualStream {
        let (tx, rx) = mpsc::channel::<Vec<u8>>(1024);
        let mut map = self.sessions.lock().await;
        map.insert(peer, tx);
        
        UdpVirtualStream::new(self.socket.clone(), peer, mode, rx, handshake_done)
    }

    #[allow(dead_code)]
    pub async fn remove_session(&self, peer: &SocketAddr) {
        let mut map = self.sessions.lock().await;
        map.remove(peer);
    }
}
