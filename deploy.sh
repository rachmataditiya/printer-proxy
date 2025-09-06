#!/bin/bash

# Printer Proxy Deployment Script
# Usage: ./deploy.sh [target-host] [target-arch]
# Example: ./deploy.sh pi@192.168.1.100 aarch64

set -e

# Configuration
TARGET_HOST="${1:-pi@raspberrypi.local}"
TARGET_ARCH="${2:-aarch64}"
SERVICE_NAME="printer-proxy"
BINARY_NAME="printer-proxy"

# Build configuration
case $TARGET_ARCH in
    "aarch64")
        RUST_TARGET="aarch64-unknown-linux-musl"
        echo "🎯 Building for ARM64 (Raspberry Pi 4+)"
        ;;
    "armv7")
        RUST_TARGET="armv7-unknown-linux-musleabihf"
        echo "🎯 Building for ARMv7 (Raspberry Pi 3 and older)"
        ;;
    *)
        echo "❌ Unsupported architecture: $TARGET_ARCH"
        echo "Supported: aarch64, armv7"
        exit 1
        ;;
esac

echo "🚀 === PRINTER PROXY DEPLOYMENT ==="
echo "Target Host: $TARGET_HOST"
echo "Target Arch: $TARGET_ARCH ($RUST_TARGET)"
echo ""

# Check if target is installed
if ! rustup target list --installed | grep -q "$RUST_TARGET"; then
    echo "📦 Installing Rust target: $RUST_TARGET"
    rustup target add "$RUST_TARGET"
fi

# Build release binary
echo "🔨 Building release binary..."
if cargo build --target "$RUST_TARGET" --release; then
    echo "✅ Build successful"
else
    echo "❌ Build failed"
    echo ""
    echo "🔧 Cross-compilation troubleshooting:"
    echo "1. Install cross-compilation tools:"
    echo "   - Option A: Install Docker and use cross: cargo install cross"
    echo "   - Option B: Install zig: brew install zig (then set CC_aarch64_unknown_linux_musl=zig cc -target aarch64-linux-musl)"
    echo "   - Option C: Build natively on target device"
    echo ""
    echo "2. Alternative: Build on target device directly:"
    echo "   scp -r . $TARGET_HOST:~/printer-proxy/"
    echo "   ssh $TARGET_HOST 'cd ~/printer-proxy && cargo build --release'"
    exit 1
fi

BINARY_PATH="target/$RUST_TARGET/release/$BINARY_NAME"

if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Binary not found: $BINARY_PATH"
    exit 1
fi

echo "📦 Binary size: $(du -h $BINARY_PATH | cut -f1)"
echo ""

# Create deployment package
echo "📦 Creating deployment package..."
DEPLOY_DIR="deploy"
rm -rf "$DEPLOY_DIR"
mkdir -p "$DEPLOY_DIR"

# Copy files
cp "$BINARY_PATH" "$DEPLOY_DIR/"
cp "printer-proxy.service" "$DEPLOY_DIR/"
cp "printers.yaml" "$DEPLOY_DIR/"
cp "DEPLOYMENT.md" "$DEPLOY_DIR/" 2>/dev/null || echo "# Deployment" > "$DEPLOY_DIR/README.md"

# Create installation script
cat > "$DEPLOY_DIR/install.sh" << 'EOF'
#!/bin/bash

# Printer Proxy Installation Script
set -e

SERVICE_NAME="printer-proxy"
BINARY_NAME="printer-proxy"

echo "🚀 Installing Printer Proxy..."

# Create user and group
if ! id "printer-proxy" &>/dev/null; then
    echo "👤 Creating printer-proxy user..."
    sudo useradd --system --shell /usr/sbin/nologin --home-dir /var/lib/printer-proxy printer-proxy
fi

# Create directories
echo "📁 Creating directories..."
sudo mkdir -p /usr/local/bin
sudo mkdir -p /etc/printer-proxy
sudo mkdir -p /var/lib/printer-proxy/logs
sudo mkdir -p /var/log/printer-proxy

# Install binary
echo "📦 Installing binary..."
sudo cp "$BINARY_NAME" /usr/local/bin/
sudo chmod +x /usr/local/bin/"$BINARY_NAME"
sudo chown root:root /usr/local/bin/"$BINARY_NAME"

# Install configuration
echo "⚙️ Installing configuration..."
sudo cp printers.yaml /etc/printer-proxy/
sudo chown root:printer-proxy /etc/printer-proxy/printers.yaml
sudo chmod 640 /etc/printer-proxy/printers.yaml

# Set permissions
sudo chown -R printer-proxy:printer-proxy /var/lib/printer-proxy
sudo chown -R printer-proxy:printer-proxy /var/log/printer-proxy

# Install systemd service
echo "🔧 Installing systemd service..."
sudo cp printer-proxy.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable "$SERVICE_NAME"

echo ""
echo "✅ Installation complete!"
echo ""
echo "🎯 Next steps:"
echo "1. Edit configuration: sudo nano /etc/printer-proxy/printers.yaml"
echo "2. Start service: sudo systemctl start $SERVICE_NAME"
echo "3. Check status: sudo systemctl status $SERVICE_NAME"
echo "4. View logs: sudo journalctl -u $SERVICE_NAME -f"
echo ""
echo "🌐 Default endpoints:"
echo "- Health check: http://localhost:8080/healthz"
echo "- Printers health: http://localhost:8080/health/printers"
echo "- Print endpoint: http://localhost:8080/{printer_id}/cgi-bin/epos/service.cgi"
EOF

chmod +x "$DEPLOY_DIR/install.sh"

# Create uninstall script
cat > "$DEPLOY_DIR/uninstall.sh" << 'EOF'
#!/bin/bash

# Printer Proxy Uninstallation Script
set -e

SERVICE_NAME="printer-proxy"
BINARY_NAME="printer-proxy"

echo "🗑️ Uninstalling Printer Proxy..."

# Stop and disable service
echo "⏹️ Stopping service..."
sudo systemctl stop "$SERVICE_NAME" 2>/dev/null || true
sudo systemctl disable "$SERVICE_NAME" 2>/dev/null || true

# Remove files
echo "📦 Removing files..."
sudo rm -f /usr/local/bin/"$BINARY_NAME"
sudo rm -f /etc/systemd/system/printer-proxy.service
sudo rm -rf /etc/printer-proxy
sudo rm -rf /var/lib/printer-proxy
sudo rm -rf /var/log/printer-proxy

# Remove user
echo "👤 Removing user..."
sudo userdel printer-proxy 2>/dev/null || true

# Reload systemd
sudo systemctl daemon-reload

echo "✅ Uninstallation complete!"
EOF

chmod +x "$DEPLOY_DIR/uninstall.sh"

echo "✅ Deployment package created in: $DEPLOY_DIR/"
echo ""

# Ask for deployment
read -p "🚀 Deploy to $TARGET_HOST now? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "📤 Deploying to $TARGET_HOST..."
    
    # Copy deployment package
    scp -r "$DEPLOY_DIR" "$TARGET_HOST:~/printer-proxy-deploy/"
    
    echo "✅ Files copied to target host"
    echo ""
    echo "🔧 To complete installation on target host, run:"
    echo "ssh $TARGET_HOST 'cd ~/printer-proxy-deploy && sudo ./install.sh'"
    echo ""
    echo "📊 Remote management commands:"
    echo "ssh $TARGET_HOST 'sudo systemctl status printer-proxy'"
    echo "ssh $TARGET_HOST 'sudo journalctl -u printer-proxy -f'"
    echo "ssh $TARGET_HOST 'curl -s http://localhost:8080/health/printers | jq .'"
else
    echo "📦 Deployment package ready in: $DEPLOY_DIR/"
    echo ""
    echo "🔧 Manual deployment commands:"
    echo "scp -r $DEPLOY_DIR $TARGET_HOST:~/printer-proxy-deploy/"
    echo "ssh $TARGET_HOST 'cd ~/printer-proxy-deploy && sudo ./install.sh'"
fi

echo ""
echo "🎉 Deployment script complete!"
