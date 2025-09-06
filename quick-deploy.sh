#!/bin/bash

# Quick deployment script for printer-proxy
# Usage: ./quick-deploy.sh [host] [arch]

set -e

HOST="${1:-pi@raspberrypi.local}"
ARCH="${2:-aarch64}"

echo "🚀 Quick Deploy to $HOST ($ARCH)"

# Build natively on target (most reliable method)
echo "📦 Building natively on target device..."

ssh "$HOST" << 'ENDSSH'
# Install Rust if not present
if ! command -v cargo &> /dev/null; then
    echo "🦀 Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
fi

# Create project directory
mkdir -p ~/printer-proxy-build
cd ~/printer-proxy-build
ENDSSH

# Copy source code
echo "📤 Copying source code..."
rsync -av --exclude=target --exclude=logs --exclude=deploy . "$HOST:~/printer-proxy-build/"

# Build and install
ssh "$HOST" << 'ENDSSH'
cd ~/printer-proxy-build
source ~/.cargo/env

echo "🔨 Building..."
cargo build --release

echo "🔧 Installing..."

# Create user if not exists
if ! id "printer-proxy" &>/dev/null; then
    echo "👤 Creating printer-proxy user..."
    sudo useradd --system --shell /usr/sbin/nologin --home-dir /var/lib/printer-proxy printer-proxy
fi

# Create directories
sudo mkdir -p /usr/local/bin
sudo mkdir -p /etc/printer-proxy
sudo mkdir -p /var/lib/printer-proxy/logs

# Install binary
sudo cp target/release/printer-proxy /usr/local/bin/
sudo chmod +x /usr/local/bin/printer-proxy

# Install config
sudo cp printers.yaml /etc/printer-proxy/
sudo chown root:printer-proxy /etc/printer-proxy/printers.yaml
sudo chmod 640 /etc/printer-proxy/printers.yaml

# Set permissions
sudo chown -R printer-proxy:printer-proxy /var/lib/printer-proxy

# Install service
sudo cp printer-proxy.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable printer-proxy

# Start service
sudo systemctl restart printer-proxy

echo "✅ Installation complete!"
echo "🌐 Service running on: http://$(hostname -I | awk '{print $1}'):8080"
ENDSSH

echo ""
echo "🎉 Deployment complete!"
echo ""
echo "📊 Check status:"
echo "ssh $HOST 'sudo systemctl status printer-proxy'"
echo ""
echo "📝 View logs:"
echo "ssh $HOST 'sudo journalctl -u printer-proxy -f'"
echo ""
echo "🌐 Test endpoints:"
IP=$(ssh "$HOST" "hostname -I | awk '{print \$1}'")
echo "curl http://$IP:8080/healthz"
echo "curl -s http://$IP:8080/health/printers | jq ."
