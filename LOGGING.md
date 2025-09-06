# 📝 Logging System Documentation

## Overview

Aplikasi Printer Proxy dilengkapi dengan sistem logging yang komprehensif untuk monitoring dan debugging. Logging menggunakan `tracing` framework dengan output ke console dan file.

## 🗂️ File Structure

```
logs/
└── printer-proxy.log.YYYY-MM-DD    # Daily rotation
```

## 🔧 Configuration

### Environment Variables

- `RUST_LOG`: Level logging (default: `printer_proxy=info,axum=info,tower_http=info`)
- `PRINTERS_CONFIG`: Path ke config file (default: `printers.yaml`)
- `LISTEN_ADDR`: Server address (default: `0.0.0.0:8080`)

### Log Levels

- `error`: Error critical yang memerlukan perhatian
- `warn`: Warning dan situasi yang perlu diperhatikan
- `info`: Informasi operasional penting
- `debug`: Detail debugging untuk development
- `trace`: Level terendah untuk debugging mendalam

## 📊 Log Examples

### Server Startup
```
🚀 Starting Printer Proxy (ESC/POS) v0.3.0
📝 Logs akan disimpan di folder: ./logs/
📄 Loading config dari: printers.yaml
✅ Loaded 2 printer(s) dari printers.yaml
🖨️  Printer 'printer_kasir_1' -> Tcp9100 { host: "192.168.10.21", port: 9100 }
🌐 Server akan listen di: http://0.0.0.0:8080
✅ Server siap menerima koneksi di 0.0.0.0:8080
```

### Request Processing
```
📥 Incoming request: POST printer_kasir_1
✅ Printer 'printer_kasir_1' ditemukan: Tcp9100 { host: "192.168.10.21", port: 9100 }
📄 Content-Type: application/json
📊 Body size: 85 bytes
🔄 Processing JSON job mode
🔄 Processing 3 operations
  Op 0: Init
  Op 1: Text { data: "Test Print!", newline: None }
  Op 2: Cut { mode: None }
📦 Generated 17 ESC/POS bytes from JSON
```

### Printer Communication
```
🔌 Connecting to printer at 192.168.10.21:9100
✅ Connected to 192.168.10.21:9100, sending 17 bytes
📦 Payload preview: [1B, 40, 54, 65, 73, 74, 20, 50, 72, 69, 6E, 74, 21, 0A, 1D, 56, 00]
🎯 Successfully sent 17 bytes to 192.168.10.21:9100
✅ Successfully sent JSON job to printer 'printer_kasir_1'
```

### Error Handling
```
❌ Printer 'unknown_printer' tidak ditemukan
❌ Invalid method: DELETE (only POST/PUT allowed)
❌ JSON parsing error: expected value at line 1 column 1
❌ TCP connect to 192.168.1.100:9100 failed: Connection refused
```

## 🎯 Features

### 1. **Dual Output**
- **Console**: Colored output dengan emoji untuk readability
- **File**: Plain text dengan timestamp dan thread info

### 2. **Daily Rotation**
- File log baru setiap hari
- Format: `printer-proxy.log.YYYY-MM-DD`
- Otomatis rotation tanpa restart aplikasi

### 3. **Structured Logging**
- Thread ID tracking
- Request correlation via tracing spans
- File dan line number untuk debugging

### 4. **Performance Monitoring**
- Request latency tracking
- Payload size monitoring
- Connection timing information

### 5. **Detailed Instrumentation**
- Method-level tracing dengan `#[instrument]`
- Payload preview untuk debugging
- Operation-by-operation logging untuk JSON jobs

## 🚀 Usage Examples

### Basic Startup
```bash
cargo run
```

### Debug Mode
```bash
RUST_LOG=debug cargo run
```

### Custom Log Level
```bash
RUST_LOG="printer_proxy=trace,axum=info" cargo run
```

### Production Mode (Info only)
```bash
RUST_LOG=info cargo run
```

### Monitoring Logs
```bash
# Real-time monitoring
tail -f logs/printer-proxy.log.$(date +%Y-%m-%d)

# Filter errors only
grep "❌\|ERROR" logs/printer-proxy.log.$(date +%Y-%m-%d)

# Filter specific printer
grep "printer_kasir_1" logs/printer-proxy.log.$(date +%Y-%m-%d)
```

## 🔍 Troubleshooting

### Common Log Patterns

**Connection Issues:**
```
❌ TCP connect to 192.168.1.100:9100 failed: Connection refused
```

**Parsing Errors:**
```
❌ JSON parsing error: expected value at line 1 column 1
❌ Base64 decode error: Invalid byte 61, offset 1
```

**Configuration Issues:**
```
❌ Config tidak berisi printer apa pun
❌ Printer 'unknown_id' tidak ditemukan
```

### Log Analysis

```bash
# Count requests per printer
grep "📥 Incoming request" logs/*.log | cut -d' ' -f7 | sort | uniq -c

# Find connection errors
grep "❌.*connect" logs/*.log

# Monitor request volumes
grep "📥 Incoming request" logs/*.log | cut -d'T' -f2 | cut -d'.' -f1 | sort | uniq -c
```

## 📈 Log Retention

- **Default**: File per hari, manual cleanup
- **Recommended**: Setup logrotate atau cleanup script untuk production
- **Size**: Typical ~1-5MB per hari depending on volume

## 🛠️ Customization

Untuk modify logging behavior, edit `src/main.rs`:

```rust
// Modify log format
.with_ansi(true)
.with_target(true)
.with_thread_ids(true)
.with_file(true)
.with_line_number(true)

// Add custom fields
tracing::info_span!("custom_span", custom_field = %value)
```
