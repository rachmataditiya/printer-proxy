# 🏥 Health Check System Documentation

## Overview

Printer Proxy dilengkapi dengan sistem health check yang memverifikasi status printer sebelum memproses request print. Sistem ini mencegah request dikirim ke printer yang sedang offline.

## 🎯 Features

### 1. **Automatic Health Check**
- Setiap print request otomatis memverifikasi printer status
- Request ditolak jika printer offline
- Timeout 2 detik untuk health check

### 2. **Health Check Endpoints**
- **`/health/printers`**: Status semua printer
- **`/health/printer/{printer_id}`**: Status printer individual
- **`/healthz`**: Basic application health

### 3. **Concurrent Health Checks**
- Multiple printer checks berjalan parallel
- Efficient untuk monitoring banyak printer

## 📡 API Endpoints

### All Printers Health Check

```http
GET /health/printers
```

**Response Example:**
```json
{
  "status": "degraded",
  "timestamp": "2025-09-06T13:31:08.870257+00:00",
  "summary": {
    "total": 2,
    "online": 0,
    "offline": 2
  },
  "printers": {
    "printer_kasir_1": {
      "status": "offline",
      "message": "🔴 Offline"
    },
    "printer_kasir_2": {
      "status": "offline", 
      "message": "🔴 Offline"
    }
  }
}
```

**Status Values:**
- `healthy`: Semua printer online
- `degraded`: Ada printer offline
- `unhealthy`: Mayoritas/semua printer offline

### Individual Printer Health Check

```http
GET /health/printer/{printer_id}
```

**Response Example:**
```json
{
  "printer_id": "printer_kasir_1",
  "status": "offline",
  "message": "🔴 Offline",
  "backend": {
    "type": "tcp9100",
    "host": "192.168.10.21",
    "port": 9100
  },
  "timestamp": "2025-09-06T13:31:15.070322+00:00"
}
```

**Printer Status Values:**
- `online`: 🟢 Printer tersedia dan ready
- `offline`: 🔴 Printer tidak dapat dijangkau
- `unknown`: 🟡 Status tidak dapat ditentukan

### Print Request Flow

```http
POST /{printer_id}/cgi-bin/epos/service.cgi
```

**Flow:**
1. Validate printer exists
2. **Health check printer** ⬅️ NEW
3. Process request (jika online)
4. Send to printer backend

**Error Response (Offline):**
```xml
<?xml version="1.0"?><response success="false" code="1"/>
```

## 🔍 Health Check Logic

### TCP9100 Backend
```rust
// Attempt TCP connection dengan timeout 2 detik
match TcpStream::connect("192.168.10.21:9100").await {
    Ok(_) => PrinterStatus::Online,
    Err(_) => PrinterStatus::Offline,
}
```

### Timeout Configuration
- **Health Check**: 2 seconds
- **Quick Check**: 500ms (untuk bulk operations)
- **Connection retry**: None (fail fast)

## 📝 Logging Examples

### Successful Health Check
```
🔍 Checking printer 'printer_kasir_1' health status...
✅ TCP health check passed for 192.168.10.21:9100
✅ Printer 'printer_kasir_1' is online and ready
```

### Failed Health Check
```
🔍 Checking printer 'printer_kasir_1' health status...
⏰ TCP health check timeout for 192.168.10.21:9100
❌ Printer 'printer_kasir_1' is offline, rejecting request
```

### Bulk Health Check
```
🏥 Checking health status of all printers
⏰ TCP health check timeout for 192.168.10.21:9100
⏰ TCP health check timeout for 192.168.10.22:9100
🏥 Health check completed: 0 online, 2 offline
```

## 💡 Usage Examples

### Monitor All Printers
```bash
curl -s http://localhost:8080/health/printers | jq .
```

### Check Specific Printer
```bash
curl -s http://localhost:8080/health/printer/printer_kasir_1 | jq .
```

### Print Request (dengan auto health check)
```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -d '{"ops": [{"type": "text", "data": "Hello World"}]}' \
  http://localhost:8080/printer_kasir_1/cgi-bin/epos/service.cgi
```

### Monitor Health in Loop
```bash
while true; do
  echo "$(date): Checking printer health..."
  curl -s http://localhost:8080/health/printers | jq '.summary'
  sleep 30
done
```

## 🎯 Benefits

### 1. **Improved Reliability**
- Prevents print jobs ke printer offline
- Fast failure detection (2s timeout)
- Clear error messages

### 2. **Better Monitoring**
- Real-time printer status
- Aggregate health metrics
- Historical logging

### 3. **Efficient Operations**
- Concurrent health checks
- Minimal overhead (<2s per request)
- Non-blocking for online printers

### 4. **Enhanced Debugging**
- Detailed health check logs
- Connection timing information
- Error classification

## 🔧 Configuration

### Environment Variables
```bash
# Logging level untuk health checks
RUST_LOG="printer_proxy=info,printer_proxy::health=debug"

# Server configuration
LISTEN_ADDR="0.0.0.0:8080"
PRINTERS_CONFIG="printers.yaml"
```

### Custom Timeouts
Untuk modify timeout, edit `src/health.rs`:
```rust
// Health check timeout
timeout(Duration::from_secs(2), TcpStream::connect(&addr))

// Quick check timeout  
timeout(Duration::from_millis(500), TcpStream::connect(&addr))
```

## 🚨 Error Handling

### Print Request ke Offline Printer
```
Request error: Printer 'printer_kasir_1' sedang offline dan tidak dapat menerima request
```

### Network Issues
```
TCP health check timeout for 192.168.10.21:9100
TCP connect to 192.168.10.21:9100 failed: Connection refused
```

### Invalid Printer ID
```
Request error: Printer 'unknown_printer' tidak ditemukan
```

## 📊 Performance Impact

- **Health Check Overhead**: ~2ms for online printers
- **Timeout Cost**: 2s for offline printers
- **Memory Usage**: Minimal (shared connections)
- **CPU Impact**: Low (async operations)

## 🔮 Future Enhancements

### Planned Features
- [ ] Health check caching (reduce repeated checks)
- [ ] Configurable timeout per printer
- [ ] Health check history/trends
- [ ] Webhook notifications for status changes
- [ ] Bulk printer configuration updates

### Integration Ideas
- [ ] Prometheus metrics export
- [ ] Grafana dashboard templates
- [ ] Slack/Discord notifications
- [ ] Auto-retry with exponential backoff
