use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;

/// Obfuscates payload by adding random padding bytes.
/// Format: [2 Bytes Data Length (BigEndian)] + [Actual Data] + [Random Padding to multiple of 16 or random size]
#[allow(dead_code)]
pub fn add_padding(data: &[u8]) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let data_len = data.len();
    
    // Determine random padding size to make it dynamic
    let target_size = ((data_len + 2 + 15) / 16) * 16 + rng.gen_range(0..4) * 16;
    let padding_len = target_size - (data_len + 2);
    
    let mut padded = Vec::with_capacity(target_size);
    // Write 2-byte data length
    padded.extend_from_slice(&(data_len as u16).to_be_bytes());
    // Write actual data
    padded.extend_from_slice(data);
    // Write random bytes for padding
    for _ in 0..padding_len {
        padded.push(rng.gen::<u8>());
    }
    padded
}

/// Strips random padding bytes from obfuscated payload.
#[allow(dead_code)]
pub fn remove_padding(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 2 {
        return Err("Payload too short to extract length".to_string());
    }
    
    // Read 2-byte data length
    let data_len = u16::from_be_bytes([data[0], data[1]]) as usize;
    
    if data_len + 2 > data.len() {
        return Err(format!("Extracted length ({}) exceeds payload size ({})", data_len, data.len()));
    }
    
    // Extract actual data
    Ok(data[2..2 + data_len].to_vec())
}

/// Dynamic Jitter: Introduces random micro-delays to shape packet timing signature.
/// Mimics real user interaction flow.
pub async fn apply_jitter() {
    // Determine delay without holding ThreadRng across await point
    let delay_opt = {
        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.35) {
            Some(rng.gen_range(1..6))
        } else {
            None
        }
    };
    
    if let Some(delay) = delay_opt {
        sleep(Duration::from_millis(delay)).await;
    }
}
