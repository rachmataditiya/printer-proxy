# 🔒 Admin Endpoints Documentation

## Overview

Aplikasi printer proxy menyediakan admin endpoints yang secured untuk management operasi seperti shutdown, restart, dan SSL renewal. Semua admin endpoints dilindungi dengan authentication token untuk keamanan.

## 🛡️ Security Configuration

### Admin Token Setup

Set environment variable `ADMIN_TOKEN` dengan token yang strong (minimum 16 karakter):

```bash
# Generate secure token (32 characters recommended)
export ADMIN_TOKEN="your-super-secure-admin-token-here-32chars"

# Or dalam systemd service
Environment=ADMIN_TOKEN=your-super-secure-admin-token-here-32chars
```

**⚠️ Security Requirements:**
- Token minimum 16 karakter
- Gunakan random characters yang strong
- Jangan commit token ke version control
- Rotate token secara berkala

## 📡 Available Admin Endpoints

### 1. 🛑 Shutdown Service

**Endpoint**: `GET /admin/shutdown?token=TOKEN`

**Description**: Gracefully shutdown service dengan delay 2 detik untuk response.

**Usage**:
```bash
curl "http://localhost:8080/admin/shutdown?token=your-admin-token"
```

**Response**:
```json
{
  "success": true,
  "message": "Graceful shutdown initiated - server will stop in 2 seconds",
  "timestamp": "2024-01-20T10:30:00Z"
}
```

### 2. 🔄 Restart Service

**Endpoint**: `GET /admin/restart?token=TOKEN`

**Description**: Restart service menggunakan systemctl atau fallback ke graceful shutdown.

**Usage**:
```bash
curl "http://localhost:8080/admin/restart?token=your-admin-token"
```

**Response**:
```json
{
  "success": true,
  "message": "Service restart initiated - attempting systemctl restart",
  "timestamp": "2024-01-20T10:30:00Z"
}
```

**Behavior**:
- Pertama coba `systemctl restart printer-proxy`
- Jika gagal, fallback ke `exit(1)` (systemd auto-restart)

### 3. 🔐 SSL Certificate Renewal

**Endpoint**: `GET /admin/ssl/renew?token=TOKEN&domain=DOMAIN&port=PORT`

**Description**: Execute SSL renewal script untuk update certificates.

**Parameters**:
- `token` (required): Admin authentication token
- `domain` (optional): Target domain (default: localhost)
- `port` (optional): Target port (default: 8080)

**Usage**:
```bash
# Basic renewal
curl "http://localhost:8080/admin/ssl/renew?token=your-admin-token"

# Custom domain and port
curl "http://localhost:8080/admin/ssl/renew?token=your-admin-token&domain=printer.local&port=8080"
```

**Response**:
```json
{
  "success": true,
  "message": "SSL renewal initiated for domain 'printer.local' on port '8080' - check logs for progress",
  "timestamp": "2024-01-20T10:30:00Z"
}
```

**Requirements**:
- File `ssl.sh` harus ada di working directory
- User harus punya sudo permissions
- nginx dan openssl installed

### 4. 📊 Service Status

**Endpoint**: `GET /admin/status?token=TOKEN`

**Description**: Get comprehensive status information tentang service.

**Usage**:
```bash
curl "http://localhost:8080/admin/status?token=your-admin-token"
```

**Response**:
```json
{
  "success": true,
  "timestamp": "2024-01-20T10:30:00Z",
  "service": {
    "name": "printer-proxy",
    "version": "0.3.0",
    "uptime_seconds": 3600,
    "printers_configured": 3
  },
  "system": {
    "pid": 12345,
    "memory_usage": "45120 kB"
  },
  "endpoints": {
    "health": "/healthz",
    "printers_health": "/health/printers",
    "admin_shutdown": "/admin/shutdown?token=TOKEN",
    "admin_restart": "/admin/restart?token=TOKEN",
    "admin_ssl_renew": "/admin/ssl/renew?token=TOKEN&domain=DOMAIN&port=PORT",
    "admin_status": "/admin/status?token=TOKEN"
  }
}
```

## 🚨 Error Responses

### Unauthorized Access

**Status**: `401 Unauthorized`

```json
{
  "success": false,
  "message": "Invalid or missing admin token",
  "timestamp": "2024-01-20T10:30:00Z"
}
```

### SSL Script Not Found

**Status**: `500 Internal Server Error`

```json
{
  "success": false,
  "message": "SSL renewal script not found",
  "timestamp": "2024-01-20T10:30:00Z"
}
```

## 🛠️ Deployment Integration

### Systemd Service Configuration

Untuk enable restart functionality, pastikan service configured dengan restart policy:

```ini
[Unit]
Description=Printer Proxy Service
After=network.target

[Service]
Type=simple
User=printer-proxy
WorkingDirectory=/opt/printer-proxy
ExecStart=/opt/printer-proxy/printer-proxy
Environment=ADMIN_TOKEN=your-secure-token-here
Environment=PRINTERS_CONFIG=/opt/printer-proxy/printers.yaml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

### Nginx Integration

Admin endpoints dapat di-expose melalui reverse proxy dengan additional security:

```nginx
# Admin endpoints - restrict by IP
location /admin/ {
    allow 127.0.0.1;
    allow 192.168.1.0/24;  # Local network only
    deny all;
    
    proxy_pass http://127.0.0.1:8080;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
}
```

## 📝 Usage Examples

### Complete Management Workflow

```bash
#!/bin/bash
ADMIN_TOKEN="your-admin-token"
BASE_URL="http://localhost:8080"

# Check service status
echo "Checking service status..."
curl -s "${BASE_URL}/admin/status?token=${ADMIN_TOKEN}" | jq .

# Renew SSL certificates
echo "Renewing SSL certificates..."
curl -s "${BASE_URL}/admin/ssl/renew?token=${ADMIN_TOKEN}&domain=printer.local" | jq .

# Restart service to apply new certificates
echo "Restarting service..."
curl -s "${BASE_URL}/admin/restart?token=${ADMIN_TOKEN}" | jq .
```

### Monitoring Script

```bash
#!/bin/bash
# health-check.sh
ADMIN_TOKEN="your-admin-token"

STATUS=$(curl -s "http://localhost:8080/admin/status?token=${ADMIN_TOKEN}")
SUCCESS=$(echo $STATUS | jq -r .success)

if [ "$SUCCESS" = "true" ]; then
    echo "✅ Service healthy"
    echo $STATUS | jq .service
else
    echo "❌ Service unhealthy"
    # Restart if needed
    curl -s "http://localhost:8080/admin/restart?token=${ADMIN_TOKEN}"
fi
```

## 🔐 Security Best Practices

1. **Token Management**:
   - Use long, random tokens (32+ characters)
   - Store dalam environment variables, not config files
   - Rotate tokens regularly

2. **Network Security**:
   - Restrict admin endpoints ke trusted IPs only
   - Use HTTPS dalam production
   - Consider VPN atau private networks

3. **Logging**:
   - Monitor admin endpoint access
   - Set up alerts untuk unauthorized attempts
   - Regular audit logs

4. **Permissions**:
   - Run service dengan minimal privileges
   - Configure sudo permissions carefully untuk SSL script
   - Use dedicated service user

## 🚧 Troubleshooting

### Admin Token Issues

```bash
# Check if token is set
echo $ADMIN_TOKEN

# Test token validation
curl -s "http://localhost:8080/admin/status?token=wrong-token"
# Should return 401 Unauthorized
```

### SSL Renewal Issues

```bash
# Check ssl.sh exists and executable
ls -la ssl.sh

# Test manual execution
sudo ./ssl.sh printer.local 8080

# Check nginx configuration
sudo nginx -t
```

### Service Restart Issues

```bash
# Check systemd configuration
systemctl status printer-proxy

# Check service logs
journalctl -u printer-proxy -f

# Manual restart
sudo systemctl restart printer-proxy
```

## 📞 Emergency Procedures

### Manual Service Control

```bash
# Emergency stop
sudo systemctl stop printer-proxy

# Emergency restart
sudo systemctl restart printer-proxy

# Check service status
sudo systemctl status printer-proxy
```

### SSL Certificate Issues

```bash
# Check certificate validity
openssl x509 -in /etc/ssl/localcerts/printer.local.crt -text -noout

# Manual certificate renewal
sudo ./ssl.sh printer.local 8080

# Reload nginx with new certificates
sudo systemctl reload nginx
```

---

## 🎯 Integration Tips

Admin endpoints dirancang untuk integration dengan:
- **Monitoring systems** (Prometheus, Grafana)
- **Configuration management** (Ansible, Puppet)
- **CI/CD pipelines** untuk automated deployments
- **Load balancers** untuk health checking
- **Service discovery** systems

Gunakan admin endpoints sebagai building blocks untuk automated infrastructure management! 🚀
