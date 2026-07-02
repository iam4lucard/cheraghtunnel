#!/bin/bash
# CheraghTunnel One-Click Panel Installer
set -e

echo "=========================================================="
echo "      CheraghTunnel Web Panel Setup & Installer"
echo "=========================================================="

# Check if running as root
if [ "$EUID" -ne 0 ]; then
  echo "Please run as root (sudo)"
  exit 1
fi

# Install system dependencies
echo "Installing system package dependencies..."
apt-get update && apt-get install -y build-essential sqlite3 curl git sshpass || true

# Install Rust toolchain if cargo is missing
if ! command -v cargo &> /dev/null; then
    echo "Installing Rust compiler..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

# Build project in release mode
echo "Compiling CheraghTunnel Rust binary..."
cargo build --release

# Install binary to system path
cp target/release/cheraghtunnel /usr/local/bin/cheraghtunnel
chmod +x /usr/local/bin/cheraghtunnel

# Setup config and DB folders
mkdir -p /etc/cheraghtunnel
mkdir -p /var/lib/cheraghtunnel

# Initialize DB to generate default admin credentials
echo "Initializing SQLite Database..."
/usr/local/bin/cheraghtunnel panel --port 8000 --db-path /var/lib/cheraghtunnel/cheraghtunnel.db &
PID=$!
sleep 2
kill $PID

# Retrieve credentials
PASSWORD=$(sqlite3 /var/lib/cheraghtunnel/cheraghtunnel.db "SELECT value FROM settings WHERE key='admin_password';")
USERNAME=$(sqlite3 /var/lib/cheraghtunnel/cheraghtunnel.db "SELECT value FROM settings WHERE key='admin_username';")

# Setup systemd service
echo "Configuring systemd service daemon..."
cat <<EOF > /etc/systemd/system/cheraghtunnel.service
[Unit]
Description=CheraghTunnel Web Management Panel
After=network.target

[Service]
Type=simple
WorkingDirectory=/var/lib/cheraghtunnel
ExecStart=/usr/local/bin/cheraghtunnel panel --port 8000 --db-path /var/lib/cheraghtunnel/cheraghtunnel.db
Restart=always
User=root

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable cheraghtunnel
systemctl start cheraghtunnel

echo "=========================================================="
echo "  CheraghTunnel Web Panel successfully installed!"
echo "  Access Port: http://$(curl -s ifconfig.me):8000"
echo "  "
echo "  Credentials:"
echo "  Username: $USERNAME"
echo "  Password: $PASSWORD"
echo "=========================================================="
