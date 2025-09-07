# 🚀 Raspberry Pi Deployment Guide

## Overview

Complete deployment guide for printer-proxy on Raspberry Pi with all latest features including high-performance connection pooling, admin management endpoints, printer CRUD API, and SSL certificate management.

## 📋 Prerequisites

### System Requirements
- **Raspberry Pi OS** (or compatible Linux distribution)
- **Rust/Cargo** installed
- **Root privileges** (sudo access)
- **Internet connection** for dependencies
- **Minimum 1GB RAM** (2GB+ recommended)
- **Minimum 2GB free disk space**

### Install Rust (if not already installed)
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
cargo --version
rustc --version
```

## 🚀 Quick Deployment

### Option 1: Complete Automated Deployment
```bash
# Clone repository
git clone https://github.com/rachmataditiya/printer-proxy.git
cd printer-proxy

# Run complete deployment
sudo ./deploy-raspberry-pi.sh
```

### Option 2: Step-by-Step Deployment
```bash
# 1. Build the application
cargo build --release

# 2. Install service
sudo ./install.sh

# 3. Setup SSL certificates
sudo ./setup-ssl.sh your-domain.local

# 4. Generate admin token
sudo ./generate-token.sh --update-service
```

## 📜 Deployment Scripts

### 1. `deploy-raspberry-pi.sh` - Complete Deployment
**Purpose**: One-command complete deployment with all features.

**Usage**:
```bash
# Full deployment for localhost
sudo ./deploy-raspberry-pi.sh

# Full deployment for custom domain
sudo ./deploy-raspberry-pi.sh printer.local

# Deploy without SSL
sudo ./deploy-raspberry-pi.sh --skip-ssl printer.local

# Use custom admin token
sudo ./deploy-raspberry-pi.sh --admin-token my-secure-token-32chars
```

**Features**:
- ✅ System requirements check
- ✅ Application build
- ✅ Service installation
- ✅ SSL certificate setup
- ✅ Nginx reverse proxy configuration
- ✅ Admin token generation
- ✅ Installation testing

### 2. `install.sh` - Service Installation
**Purpose**: Install printer-proxy as systemd service.

**Usage**:
```bash
# Install with auto-generated token
sudo ./install.sh

# Install with custom token
sudo ./install.sh --admin-token my-secure-token

# Install with custom configuration
sudo ./install.sh --listen-addr 127.0.0.1:8080 --log-level debug
```

**What it does**:
- Creates `printer-proxy` system user
- Installs binary to `/usr/local/bin/`
- Creates configuration directories
- Sets up systemd service
- Generates default `printers.yaml`
- Starts and enables service

### 3. `setup-ssl.sh` - SSL Certificate Setup
**Purpose**: Setup SSL certificates and nginx reverse proxy.

**Usage**:
```bash
# Setup for localhost
sudo ./setup-ssl.sh

# Setup for custom domain
sudo ./setup-ssl.sh printer.local

# Setup for custom domain and port
sudo ./setup-ssl.sh printer.local 8080
```

**Features**:
- Self-signed certificate generation
- Nginx reverse proxy configuration
- HTTP to HTTPS redirect
- Security headers
- Admin/API endpoint restrictions
- WebSocket support

### 4. `generate-token.sh` - Admin Token Generator
**Purpose**: Generate secure admin tokens for API access.

**Usage**:
```bash
# Generate 32-character token
sudo ./generate-token.sh

# Generate custom length token
sudo ./generate-token.sh --length 48

# Generate and update service file
sudo ./generate-token.sh --update-service
```

### 5. `uninstall.sh` - Service Removal
**Purpose**: Completely remove printer-proxy installation.

**Usage**:
```bash
# Interactive uninstallation
sudo ./uninstall.sh

# Force uninstallation
sudo ./uninstall.sh --force
```

## 🔧 Configuration

### Service Configuration
The service is configured via systemd environment variables in `/etc/systemd/system/printer-proxy.service`:

```ini
[Service]
Environment=RUST_LOG=info
Environment=LISTEN_ADDR=0.0.0.0:8080
Environment=PRINTERS_CONFIG=/etc/printer-proxy/printers.yaml
Environment=ADMIN_TOKEN=your-secure-token-here
```

### Printer Configuration
Configure printers in `/etc/printer-proxy/printers.yaml`:

```yaml
printers:
  - name: "Office Printer"
    id: "office-001"
    backend:
      type: "tcp9100"
      host: "192.168.1.100"
      port: 9100
  
  - name: "Kitchen Printer"
    id: "kitchen-001"
    backend:
      type: "tcp9100"
      host: "192.168.1.101"
      port: 9100
```

### Nginx Configuration
SSL and reverse proxy configuration in `/etc/nginx/sites-available/your-domain.conf`:

- HTTP to HTTPS redirect
- SSL certificate configuration
- Security headers
- Admin/API endpoint restrictions
- WebSocket support

## 🌐 Access URLs

### HTTP Endpoints (Direct Access)
- **Health Check**: `http://localhost:8080/healthz`
- **Printers Health**: `http://localhost:8080/health/printers`
- **Print Endpoint**: `http://localhost:8080/{printer_id}/cgi-bin/epos/service.cgi`

### HTTPS Endpoints (via Nginx)
- **Main Site**: `https://your-domain.local`
- **Health Check**: `https://your-domain.local/healthz`
- **Print Endpoint**: `https://your-domain.local/{printer_id}/cgi-bin/epos/service.cgi`

### Admin Endpoints (Token Required)
- **Service Status**: `GET /admin/status?token=TOKEN`
- **Service Shutdown**: `GET /admin/shutdown?token=TOKEN`
- **Service Restart**: `GET /admin/restart?token=TOKEN`
- **SSL Renewal**: `GET /admin/ssl/renew?token=TOKEN`

### Printer Management API (Token Required)
- **List Printers**: `GET /api/printers?token=TOKEN`
- **Create Printer**: `POST /api/printers?token=TOKEN`
- **Update Printer**: `PUT /api/printers/{id}?token=TOKEN`
- **Delete Printer**: `DELETE /api/printers/{id}?token=TOKEN`
- **Reload Config**: `GET /api/printers/reload?token=TOKEN`

## 🔒 Security Configuration

### Admin Token Security
```bash
# Generate secure token
sudo ./generate-token.sh --length 64

# Update service with new token
sudo ./generate-token.sh --update-service

# Restart service to apply changes
sudo systemctl restart printer-proxy
```

### Network Security
- Admin/API endpoints restricted to local networks
- SSL/TLS encryption for all HTTPS traffic
- Security headers (HSTS, X-Frame-Options, etc.)
- Firewall rules recommended

### File Permissions
```bash
# Check service file permissions
ls -la /etc/systemd/system/printer-proxy.service

# Check config directory permissions
ls -la /etc/printer-proxy/

# Check data directory permissions
ls -la /var/lib/printer-proxy/
```

## 🛠️ Service Management

### Basic Commands
```bash
# Check service status
sudo systemctl status printer-proxy

# Start service
sudo systemctl start printer-proxy

# Stop service
sudo systemctl stop printer-proxy

# Restart service
sudo systemctl restart printer-proxy

# Enable auto-start
sudo systemctl enable printer-proxy

# Disable auto-start
sudo systemctl disable printer-proxy
```

### Log Management
```bash
# View service logs
sudo journalctl -u printer-proxy -f

# View recent logs
sudo journalctl -u printer-proxy --since "1 hour ago"

# View logs with timestamps
sudo journalctl -u printer-proxy -o short-iso

# Clear old logs
sudo journalctl --vacuum-time=7d
```

### Configuration Reload
```bash
# Reload systemd configuration
sudo systemctl daemon-reload

# Reload nginx configuration
sudo nginx -t && sudo systemctl reload nginx

# Reload printer configuration (via API)
curl "http://localhost:8080/api/printers/reload?token=YOUR_TOKEN"
```

## 🔍 Troubleshooting

### Common Issues

#### 1. Service Won't Start
```bash
# Check service status
sudo systemctl status printer-proxy

# Check logs for errors
sudo journalctl -u printer-proxy --no-pager

# Check binary exists
ls -la /usr/local/bin/printer-proxy

# Check permissions
ls -la /etc/printer-proxy/printers.yaml
```

#### 2. SSL Certificate Issues
```bash
# Check certificate validity
openssl x509 -in /etc/ssl/localcerts/your-domain.crt -text -noout

# Regenerate certificate
sudo ./setup-ssl.sh your-domain

# Check nginx configuration
sudo nginx -t
```

#### 3. Admin Token Issues
```bash
# Check token in service file
grep ADMIN_TOKEN /etc/systemd/system/printer-proxy.service

# Generate new token
sudo ./generate-token.sh --update-service

# Test token
curl "http://localhost:8080/admin/status?token=YOUR_TOKEN"
```

#### 4. Printer Connection Issues
```bash
# Test printer connectivity
telnet 192.168.1.100 9100

# Check printer configuration
cat /etc/printer-proxy/printers.yaml

# Test printer health
curl "http://localhost:8080/health/printer/your-printer-id"
```

### Performance Monitoring
```bash
# Check memory usage
free -h

# Check CPU usage
top -p $(pgrep printer-proxy)

# Check disk usage
df -h

# Check network connections
netstat -tulpn | grep :8080
```

## 🔄 Updates and Maintenance

### Updating the Application
```bash
# Pull latest changes
git pull origin main

# Rebuild application
cargo build --release

# Restart service
sudo systemctl restart printer-proxy
```

### Backup Configuration
```bash
# Backup configuration
sudo cp /etc/printer-proxy/printers.yaml /backup/printers-$(date +%Y%m%d).yaml

# Backup service configuration
sudo cp /etc/systemd/system/printer-proxy.service /backup/service-$(date +%Y%m%d).service
```

### Certificate Renewal
```bash
# Manual renewal
sudo ./setup-ssl.sh your-domain

# Via admin API
curl "http://localhost:8080/admin/ssl/renew?token=YOUR_TOKEN&domain=your-domain"
```

## 📊 Monitoring and Alerts

### Health Check Monitoring
```bash
#!/bin/bash
# health-monitor.sh
TOKEN="your-admin-token"
BASE_URL="http://localhost:8080"

# Check service health
if ! curl -s -f "$BASE_URL/healthz" >/dev/null; then
    echo "❌ Service health check failed"
    exit 1
fi

# Check printer health
PRINTERS=$(curl -s "$BASE_URL/health/printers" | jq -r '.printers | to_entries[] | select(.value.status == "offline") | .key')
if [[ -n "$PRINTERS" ]]; then
    echo "⚠️ Offline printers: $PRINTERS"
fi

echo "✅ All checks passed"
```

### Log Monitoring
```bash
# Monitor for errors
sudo journalctl -u printer-proxy -f | grep -i error

# Monitor for failed connections
sudo journalctl -u printer-proxy -f | grep -i "failed\|error\|offline"
```

## 🎯 Production Recommendations

### Security
1. **Change default admin token** immediately
2. **Configure firewall** rules appropriately
3. **Use strong SSL certificates** (Let's Encrypt for production)
4. **Monitor logs** for suspicious activity
5. **Keep system updated** regularly

### Performance
1. **Monitor memory usage** during high load
2. **Configure log rotation** to prevent disk full
3. **Set up monitoring** for service health
4. **Use connection pooling** (already enabled)
5. **Optimize nginx** configuration for your use case

### Reliability
1. **Set up automated backups** of configuration
2. **Configure log monitoring** and alerts
3. **Test failover procedures** regularly
4. **Document your configuration** and procedures
5. **Plan for disaster recovery**

---

## 🆘 Support

For issues and questions:
- **GitHub Issues**: [Create an issue](https://github.com/rachmataditiya/printer-proxy/issues)
- **Documentation**: Check `ADMIN.md`, `PRINTERS_API.md`, `PERFORMANCE.md`
- **Logs**: Always check service logs first: `sudo journalctl -u printer-proxy -f`

Happy printing! 🖨️✨