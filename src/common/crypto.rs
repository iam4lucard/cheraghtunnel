/// Simple cryptographic helper for connection handshakes.
pub fn generate_auth_header(token: &str) -> String {
    // Basic base64-like representation of token for HTTP headers
    let auth = format!("Cheragh-Auth {}", token);
    auth
}

#[allow(dead_code)]
pub fn verify_auth_header(header: &str, token: &str) -> bool {
    let expected = generate_auth_header(token);
    // Constant-time comparison to prevent timing side-channel attacks
    if header.len() != expected.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in header.as_bytes().iter().zip(expected.as_bytes().iter()) {
        diff |= a ^ b;
    }
    diff == 0
}
