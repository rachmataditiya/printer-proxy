# 🚀 Deployment Guide

## Overview

Guide untuk deploy Printer Proxy ke Raspberry Pi atau Linux server dengan systemd service.

## 📋 Prerequisites

### Development Machine (Mac)
- Rust toolchain installed
- Docker Desktop (untuk cross-compilation)
- SSH access ke target device

### Target Device (Raspberry Pi/Linux)
- Raspbian OS atau Linux distro dengan systemd
- SSH server enabled
- sudo privileges

## 🎯 Supported Architectures

| Device | Architecture | Rust Target |
|--------|-------------|-------------|
| Raspberry Pi 4+ (64-bit) | `aarch64` | `aarch64-unknown-linux-musl` |
| Raspberry Pi 3 and older | `armv7` | `armv7-unknown-linux-musleabihf` |
| x86_64 Linux | `x86_64` | `x86_64-unknown-linux-musl` |

## 🛠️ Cross-Compilation Options

### Option 1: Docker Cross-Compilation (Recommended)

```bash
# Build for ARM64 (Pi 4+)
./build-docker.sh aarch64

# Build for ARMv7 (Pi 3 and older)
./build-docker.sh armv7
```

### Option 2: Native Cross-Compilation

Requires cross-compilation toolchain:

```bash
# Install target
rustup target add aarch64-unknown-linux-musl

# Install zig (as linker)
brew install zig

# Set environment
export CC_aarch64_unknown_linux_musl="zig cc -target aarch64-linux-musl"

# Build
cargo build --target aarch64-unknown-linux-musl --release
```

### Option 3: Build on Target Device

```bash
# Copy source to target
scp -r . pi@192.168.1.100:~/printer-proxy/

# SSH and build natively
ssh pi@192.168.1.100
cd ~/printer-proxy
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
cargo build --release
```

## 📦 Deployment Process

### Automated Deployment

```bash
# Deploy to Raspberry Pi
./deploy.sh pi@192.168.1.100 aarch64

# Deploy to custom host
./deploy.sh user@hostname armv7
```

### Manual Deployment

1. **Build binary** (choose one method above)

2. **Create deployment package:**
   ```bash
   mkdir deploy
   cp target/aarch64-unknown-linux-musl/release/printer-proxy deploy/
   cp printer-proxy.service deploy/
   cp printers.yaml deploy/
   ```

3. **Copy to target:**
   ```bash
   scp -r deploy/ pi@192.168.1.100:~/printer-proxy-deploy/
   ```

4. **Install on target:**
   ```bash
   ssh pi@192.168.1.100
   cd ~/printer-proxy-deploy
   sudo ./install.sh
   ```

## 🔧 Manual Installation Steps

### 1. Create User and Directories

```bash
# Create system user
sudo useradd --system --shell /usr/sbin/nologin --home-dir /var/lib/printer-proxy printer-proxy

# Create directories
sudo mkdir -p /usr/local/bin
sudo mkdir -p /etc/printer-proxy
sudo mkdir -p /var/lib/printer-proxy/logs
sudo mkdir -p /var/log/printer-proxy
```

### 2. Install Binary

```bash
# Copy binary
sudo cp printer-proxy /usr/local/bin/
sudo chmod +x /usr/local/bin/printer-proxy
sudo chown root:root /usr/local/bin/printer-proxy
```

### 3. Install Configuration

```bash
# Copy config
sudo cp printers.yaml /etc/printer-proxy/
sudo chown root:printer-proxy /etc/printer-proxy/printers.yaml
sudo chmod 640 /etc/printer-proxy/printers.yaml
```

### 4. Set Permissions

```bash
sudo chown -R printer-proxy:printer-proxy /var/lib/printer-proxy
sudo chown -R printer-proxy:printer-proxy /var/log/printer-proxy
```

### 5. Install Systemd Service

```bash
# Install service file
sudo cp printer-proxy.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable printer-proxy
```

## ⚙️ Configuration

### Edit Printer Configuration

```bash
sudo nano /etc/printer-proxy/printers.yaml
```

Example configuration:
```yaml
printers:
  - name: "Printer Kasir 1"
    id: "kasir_1"
    backend:
      type: "tcp9100"
      host: "192.168.1.201"
      port: 9100
      
  - name: "Printer Dapur"
    id: "dapur"
    backend:
      type: "tcp9100"
      host: "192.168.1.202"
      port: 9100
```

### Environment Variables

Edit service file if needed:
```bash
sudo systemctl edit printer-proxy
```

Override configuration:
```ini
[Service]
Environment=RUST_LOG=debug
Environment=LISTEN_ADDR=0.0.0.0:3000
Environment=PRINTERS_CONFIG=/etc/printer-proxy/custom-printers.yaml
```

## 🎮 Service Management

### Start Service

```bash
sudo systemctl start printer-proxy
```

### Stop Service

```bash
sudo systemctl stop printer-proxy
```

### Restart Service

```bash
sudo systemctl restart printer-proxy
```

### Check Status

```bash
sudo systemctl status printer-proxy
```

### Enable Auto-start

```bash
sudo systemctl enable printer-proxy
```

### Disable Auto-start

```bash
sudo systemctl disable printer-proxy
```

## 📊 Monitoring and Logs

### View Logs

```bash
# Follow logs real-time
sudo journalctl -u printer-proxy -f

# View recent logs
sudo journalctl -u printer-proxy -n 100

# View logs with timestamps
sudo journalctl -u printer-proxy --since "1 hour ago"
```

### Log Files

- **Service logs**: `journalctl -u printer-proxy`
- **Application logs**: `/var/lib/printer-proxy/logs/printer-proxy.log.YYYY-MM-DD`

### Health Check

```bash
# Application health
curl http://localhost:8080/healthz

# All printers health
curl -s http://localhost:8080/health/printers | jq .

# Specific printer health
curl -s http://localhost:8080/health/printer/kasir_1 | jq .
```

## 🔒 Security Considerations

### Firewall Configuration

```bash
# Allow printer proxy port
sudo ufw allow 8080/tcp

# Restrict to specific network
sudo ufw allow from 192.168.1.0/24 to any port 8080
```

### Service Security Features

- Runs as dedicated `printer-proxy` user
- No shell access (`/usr/sbin/nologin`)
- Limited capabilities (`CAP_NET_BIND_SERVICE` only)
- Protected directories (`ProtectSystem=strict`)
- Private devices (`PrivateDevices=yes`)

## 🚨 Troubleshooting

### Service Won't Start

```bash
# Check service status
sudo systemctl status printer-proxy

# Check configuration
sudo /usr/local/bin/printer-proxy --help

# Test configuration
sudo -u printer-proxy /usr/local/bin/printer-proxy
```

### Permission Issues

```bash
# Fix permissions
sudo chown -R printer-proxy:printer-proxy /var/lib/printer-proxy
sudo chmod 640 /etc/printer-proxy/printers.yaml
```

### Network Issues

```bash
# Test printer connectivity
telnet 192.168.1.201 9100

# Check listening ports
sudo netstat -tlnp | grep printer-proxy
```

### Log Issues

```bash
# Check log directory permissions
ls -la /var/lib/printer-proxy/logs/

# Check systemd journal
sudo journalctl -u printer-proxy --no-pager
```

## 🗑️ Uninstallation

Run the uninstall script:

```bash
sudo ./uninstall.sh
```

Or manually:

```bash
# Stop and disable service
sudo systemctl stop printer-proxy
sudo systemctl disable printer-proxy

# Remove files
sudo rm -f /usr/local/bin/printer-proxy
sudo rm -f /etc/systemd/system/printer-proxy.service
sudo rm -rf /etc/printer-proxy
sudo rm -rf /var/lib/printer-proxy
sudo rm -rf /var/log/printer-proxy

# Remove user
sudo userdel printer-proxy

# Reload systemd
sudo systemctl daemon-reload
```

## 📈 Performance Tuning

### Service Limits

Edit service file to adjust limits:

```ini
[Service]
LimitNOFILE=65536
TasksMax=4096
MemoryMax=512M
CPUQuota=200%
```

### Logging Performance

For high-volume deployments:

```ini
[Service]
Environment=RUST_LOG=warn
StandardOutput=null
StandardError=journal
```

## 🔄 Updates

### Update Binary

```bash
# Stop service
sudo systemctl stop printer-proxy

# Replace binary
sudo cp new-printer-proxy /usr/local/bin/printer-proxy

# Start service
sudo systemctl start printer-proxy
```

### Update Configuration

```bash
# Edit config
sudo nano /etc/printer-proxy/printers.yaml

# Reload service
sudo systemctl reload printer-proxy
```

## 🌐 Network Configuration

### Reverse Proxy (nginx)

```nginx
upstream printer_proxy {
    server 127.0.0.1:8080;
}

server {
    listen 80;
    server_name printer-proxy.local;
    
    location / {
        proxy_pass http://printer_proxy;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### SSL/TLS Setup

```bash
# Generate self-signed certificate
sudo openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
    -keyout /etc/ssl/private/printer-proxy.key \
    -out /etc/ssl/certs/printer-proxy.crt
```

Note: For production, consider using a reverse proxy like nginx for SSL termination.
