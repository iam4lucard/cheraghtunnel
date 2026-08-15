// CheraghTunnel API - Deployment Submodule
use crate::db;

pub async fn run_ssh_command(
    node: &db::Node,
    command: &str,
    stdin_data: Option<&str>,
) -> Result<String, String> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    let _key_file = if let Some(pk) = &node.private_key {
        if pk.trim().is_empty() {
            None
        } else {
            let mut file = tempfile::Builder::new()
                .prefix("cheragh_key_")
                .tempfile()
                .map_err(|e| e.to_string())?;
            file.as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600))
                .map_err(|e| e.to_string())?;
            file.write_all(pk.as_bytes()).map_err(|e| e.to_string())?;
            file.flush().map_err(|e| e.to_string())?;
            Some(file)
        }
    } else {
        None
    };

    let key_path = _key_file.as_ref().map(|f| f.path().to_string_lossy().to_string());

    let mut ssh_cmd = tokio::process::Command::new(if key_path.is_none() { "sshpass" } else { "ssh" });

    if let Some(path) = &key_path {
        ssh_cmd.args([
            "-i", path,
            "-o", "StrictHostKeyChecking=no",
            "-o", "UserKnownHostsFile=/dev/null",
            "-o", "ConnectTimeout=10",
            "-o", "LogLevel=ERROR",
            "-p", &node.port.to_string(),
            &format!("{}@{}", node.username, node.host),
            command
        ]);
    } else {
        ssh_cmd.args([
            "-p", node.password.as_deref().unwrap_or_default(),
            "ssh",
            "-o", "StrictHostKeyChecking=no",
            "-o", "UserKnownHostsFile=/dev/null",
            "-o", "ConnectTimeout=10",
            "-o", "LogLevel=ERROR",
            "-p", &node.port.to_string(),
            &format!("{}@{}", node.username, node.host),
            command
        ]);
    }

    if stdin_data.is_some() {
        ssh_cmd.stdin(std::process::Stdio::piped());
    }
    ssh_cmd.stdout(std::process::Stdio::piped());
    ssh_cmd.stderr(std::process::Stdio::piped());

    let mut child = ssh_cmd.spawn().map_err(|e| e.to_string())?;

    if let Some(data) = stdin_data {
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.write_all(data.as_bytes()).await;
        }
    }

    let output = child.wait_with_output().await.map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

pub fn generate_server_script(tunnel: &db::Tunnel) -> String {
    let port_hop_flag = if tunnel.port_hopping.unwrap_or(0) == 1 { "--port-hopping" } else { "" };
    let decoy = tunnel.decoy_url.clone().unwrap_or_else(|| "google.com".to_string());
    let api_port = 18000 + tunnel.id.unwrap_or(0) as u16;
    let transport_opts_str = tunnel.transport_options.clone().unwrap_or_else(|| "{}".to_string());

    format!(
        r#"#!/bin/bash
set -e
mkdir -p /etc/cheraghtunnel

# Stop the existing service BEFORE replacing the binary to prevent ETXTBSY (Text file busy)
systemctl stop cheragh-server-{id} 2>/dev/null || true

# Detect architecture
ARCH=$(uname -m)
if [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
    BINARY_URL="https://github.com/iam4lucard/cheraghtunnel/releases/latest/download/cheraghtunnel-linux-arm64"
else
    BINARY_URL="https://github.com/iam4lucard/cheraghtunnel/releases/latest/download/cheraghtunnel-linux-amd64"
fi

# Download to a temp file on the SAME filesystem as the destination to ensure atomic rename
curl -sSfL -o /usr/local/bin/cheraghtunnel-{id}.tmp "$BINARY_URL" || curl -sSfL -o /usr/local/bin/cheraghtunnel-{id}.tmp "https://ghfast.top/$BINARY_URL" || true
if [ -f "/usr/local/bin/cheraghtunnel-{id}.tmp" ]; then
    chmod +x /usr/local/bin/cheraghtunnel-{id}.tmp
    # Atomic rename: replaces binary only after fully downloaded, avoids Text-file-busy
    mv /usr/local/bin/cheraghtunnel-{id}.tmp /usr/local/bin/cheraghtunnel-{id}
fi

if [ ! -f "/usr/local/bin/cheraghtunnel-{id}" ] && [ -f "/usr/local/bin/cheraghtunnel" ]; then
    cp /usr/local/bin/cheraghtunnel /usr/local/bin/cheraghtunnel-{id}
    chmod +x /usr/local/bin/cheraghtunnel-{id}
fi

cat << 'EOF' > /etc/systemd/system/cheragh-server-{id}.service
[Unit]
Description=CheraghTunnel Server {id}
After=network.target

[Service]
ExecStart=/usr/local/bin/cheraghtunnel-{id} server -c {control_port} -p {public_port} -t '{token}' --protocol {protocol} --decoy '{decoy}' {port_hop_flag} --api-port {api_port} --transport-options '{transport_options}'
Restart=always
RestartSec=2s
User=root

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable cheragh-server-{id}
systemctl start cheragh-server-{id}
"#,
        id = tunnel.id.unwrap_or(0),
        control_port = tunnel.control_port,
        public_port = tunnel.iran_port,
        token = tunnel.token,
        protocol = tunnel.protocol,
        decoy = decoy,
        port_hop_flag = port_hop_flag,
        api_port = api_port,
        transport_options = transport_opts_str,
    )
}

pub fn generate_client_script(tunnel: &db::Tunnel, iran_ip: &str) -> String {
    let port_hop_flag = if tunnel.port_hopping.unwrap_or(0) == 1 { "--port-hopping" } else { "" };
    let decoy = tunnel.decoy_url.clone().unwrap_or_else(|| "google.com".to_string());
    let transport_opts_str = tunnel.transport_options.clone().unwrap_or_else(|| "{}".to_string());
    
    format!(
        r#"#!/bin/bash
set -e
mkdir -p /etc/cheraghtunnel

# Stop the existing service BEFORE replacing the binary to prevent ETXTBSY (Text file busy)
systemctl stop cheragh-node-{id} 2>/dev/null || true

# Detect architecture
ARCH=$(uname -m)
if [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
    BINARY_URL="https://github.com/iam4lucard/cheraghtunnel/releases/latest/download/cheraghtunnel-linux-arm64"
else
    BINARY_URL="https://github.com/iam4lucard/cheraghtunnel/releases/latest/download/cheraghtunnel-linux-amd64"
fi

# Download to a temp file on the SAME filesystem as the destination to ensure atomic rename
curl -sSfL -o /usr/local/bin/cheraghtunnel-{id}.tmp "$BINARY_URL" || curl -sSfL -o /usr/local/bin/cheraghtunnel-{id}.tmp "https://ghfast.top/$BINARY_URL" || true
if [ -f "/usr/local/bin/cheraghtunnel-{id}.tmp" ]; then
    chmod +x /usr/local/bin/cheraghtunnel-{id}.tmp
    # Atomic rename: replaces binary only after fully downloaded, avoids Text-file-busy
    mv /usr/local/bin/cheraghtunnel-{id}.tmp /usr/local/bin/cheraghtunnel-{id}
fi

if [ ! -f "/usr/local/bin/cheraghtunnel-{id}" ] && [ -f "/usr/local/bin/cheraghtunnel" ]; then
    cp /usr/local/bin/cheraghtunnel /usr/local/bin/cheraghtunnel-{id}
    chmod +x /usr/local/bin/cheraghtunnel-{id}
fi

cat << 'EOF' > /etc/systemd/system/cheragh-node-{id}.service
[Unit]
Description=CheraghTunnel Client Node {id}
After=network.target

[Service]
ExecStart=/usr/local/bin/cheraghtunnel-{id} client -s {iran_ip} -c {control_port} -p {public_port} -l 127.0.0.1:{kharej_port} -t '{token}' --protocol {protocol} --tunnel-id {id} --decoy '{decoy}' {port_hop_flag} --transport-options '{transport_options}'
Restart=always
RestartSec=2s
User=root

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable cheragh-node-{id}
systemctl start cheragh-node-{id}
"#,
        id = tunnel.id.unwrap_or(0),
        iran_ip = iran_ip,
        control_port = tunnel.control_port,
        public_port = tunnel.iran_port,
        kharej_port = tunnel.kharej_port,
        token = tunnel.token,
        protocol = tunnel.protocol,
        decoy = decoy,
        port_hop_flag = port_hop_flag,
        transport_options = transport_opts_str,
    )
}
