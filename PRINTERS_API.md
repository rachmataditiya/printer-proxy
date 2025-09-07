# 🖨️ Printer CRUD API Documentation

## Overview

Aplikasi printer proxy menyediakan REST API untuk mengelola konfigurasi printer secara dinamis. Semua endpoints dilindungi dengan admin token authentication dan mendukung hot-reload tanpa restart service.

## 🛡️ Authentication

Semua printer management endpoints memerlukan `ADMIN_TOKEN` yang sama dengan admin endpoints:

```bash
export ADMIN_TOKEN="your-super-secure-admin-token-32chars"
```

## 📡 Available Endpoints

### 1. 📋 List All Printers

**Endpoint**: `GET /api/printers?token=TOKEN`

**Description**: Mendapatkan daftar semua printer yang dikonfigurasi.

**Usage**:
```bash
curl "http://localhost:8080/api/printers?token=your-admin-token"
```

**Response**:
```json
{
  "success": true,
  "message": "Printers retrieved successfully",
  "data": {
    "printers": [
      {
        "name": "Main Office Printer",
        "id": "printer-001",
        "backend": {
          "type": "tcp9100",
          "host": "192.168.1.100",
          "port": 9100
        }
      },
      {
        "name": "Kitchen Printer",
        "id": "printer-002", 
        "backend": {
          "type": "tcp9100",
          "host": "192.168.1.101",
          "port": 9100
        }
      }
    ],
    "total": 2,
    "timestamp": "2024-01-20T10:30:00Z"
  },
  "timestamp": "2024-01-20T10:30:00Z"
}
```

### 2. 🔍 Get Specific Printer

**Endpoint**: `GET /api/printers/{printer_id}?token=TOKEN`

**Description**: Mendapatkan detail printer berdasarkan ID.

**Usage**:
```bash
curl "http://localhost:8080/api/printers/printer-001?token=your-admin-token"
```

**Response**:
```json
{
  "success": true,
  "message": "Printer retrieved successfully",
  "data": {
    "name": "Main Office Printer",
    "id": "printer-001",
    "backend": {
      "type": "tcp9100",
      "host": "192.168.1.100",
      "port": 9100
    }
  },
  "timestamp": "2024-01-20T10:30:00Z"
}
```

**Error Response (404)**:
```json
{
  "success": false,
  "message": "Printer 'printer-999' not found",
  "data": null,
  "timestamp": "2024-01-20T10:30:00Z"
}
```

### 3. ➕ Create New Printer

**Endpoint**: `POST /api/printers?token=TOKEN`

**Description**: Menambahkan printer baru ke konfigurasi.

**Request Body**:
```json
{
  "name": "New Office Printer",
  "id": "printer-003",
  "backend": {
    "type": "tcp9100",
    "host": "192.168.1.102",
    "port": 9100
  }
}
```

**Usage**:
```bash
curl -X POST "http://localhost:8080/api/printers?token=your-admin-token" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "New Office Printer",
    "id": "printer-003",
    "backend": {
      "type": "tcp9100",
      "host": "192.168.1.102",
      "port": 9100
    }
  }'
```

**Response (201 Created)**:
```json
{
  "success": true,
  "message": "Printer created successfully",
  "data": {
    "name": "New Office Printer",
    "id": "printer-003",
    "backend": {
      "type": "tcp9100",
      "host": "192.168.1.102",
      "port": 9100
    }
  },
  "timestamp": "2024-01-20T10:30:00Z"
}
```

**Error Response (409 Conflict)**:
```json
{
  "success": false,
  "message": "Printer 'printer-003' already exists",
  "data": null,
  "timestamp": "2024-01-20T10:30:00Z"
}
```

### 4. ✏️ Update Existing Printer

**Endpoint**: `PUT /api/printers/{printer_id}?token=TOKEN`

**Description**: Mengupdate konfigurasi printer yang sudah ada.

**Request Body** (semua field optional):
```json
{
  "name": "Updated Printer Name",
  "backend": {
    "type": "tcp9100",
    "host": "192.168.1.103",
    "port": 9100
  }
}
```

**Usage**:
```bash
curl -X PUT "http://localhost:8080/api/printers/printer-001?token=your-admin-token" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Updated Main Office Printer",
    "backend": {
      "type": "tcp9100",
      "host": "192.168.1.103",
      "port": 9100
    }
  }'
```

**Response**:
```json
{
  "success": true,
  "message": "Printer updated successfully",
  "data": {
    "name": "Updated Main Office Printer",
    "id": "printer-001",
    "backend": {
      "type": "tcp9100",
      "host": "192.168.1.103",
      "port": 9100
    }
  },
  "timestamp": "2024-01-20T10:30:00Z"
}
```

### 5. 🗑️ Delete Printer

**Endpoint**: `DELETE /api/printers/{printer_id}?token=TOKEN`

**Description**: Menghapus printer dari konfigurasi.

**Usage**:
```bash
curl -X DELETE "http://localhost:8080/api/printers/printer-003?token=your-admin-token"
```

**Response**:
```json
{
  "success": true,
  "message": "Printer 'printer-003' deleted successfully",
  "data": null,
  "timestamp": "2024-01-20T10:30:00Z"
}
```

### 6. 🔄 Reload Configuration

**Endpoint**: `GET /api/printers/reload?token=TOKEN`

**Description**: Memuat ulang konfigurasi printer dari file tanpa restart service.

**Usage**:
```bash
curl "http://localhost:8080/api/printers/reload?token=your-admin-token"
```

**Response**:
```json
{
  "success": true,
  "message": "Configuration reloaded successfully with 3 printers",
  "data": null,
  "timestamp": "2024-01-20T10:30:00Z"
}
```

## 🚨 Error Responses

### Unauthorized Access (401)

```json
{
  "success": false,
  "message": "Invalid or missing admin token",
  "data": null,
  "timestamp": "2024-01-20T10:30:00Z"
}
```

### Bad Request (400)

```json
{
  "success": false,
  "message": "ID and name are required",
  "data": null,
  "timestamp": "2024-01-20T10:30:00Z"
}
```

### Internal Server Error (500)

```json
{
  "success": false,
  "message": "Failed to save config: Permission denied",
  "data": null,
  "timestamp": "2024-01-20T10:30:00Z"
}
```

## 🔧 Backend Configuration

### TCP9100 Backend

```json
{
  "type": "tcp9100",
  "host": "192.168.1.100",
  "port": 9100
}
```

**Parameters**:
- `host`: IP address atau hostname printer
- `port`: Port printer (biasanya 9100 untuk raw printing)

## 📝 Usage Examples

### Complete Printer Management Workflow

```bash
#!/bin/bash
ADMIN_TOKEN="your-admin-token"
BASE_URL="http://localhost:8080"

# List all printers
echo "Current printers:"
curl -s "${BASE_URL}/api/printers?token=${ADMIN_TOKEN}" | jq '.data.printers[] | {id, name}'

# Add new printer
echo "Adding new printer..."
curl -s -X POST "${BASE_URL}/api/printers?token=${ADMIN_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Printer",
    "id": "test-printer",
    "backend": {
      "type": "tcp9100",
      "host": "192.168.1.200",
      "port": 9100
    }
  }' | jq .

# Update printer
echo "Updating printer..."
curl -s -X PUT "${BASE_URL}/api/printers/test-printer?token=${ADMIN_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Updated Test Printer",
    "backend": {
      "type": "tcp9100",
      "host": "192.168.1.201",
      "port": 9100
    }
  }' | jq .

# Test printer health
echo "Testing printer health..."
curl -s "${BASE_URL}/health/printer/test-printer" | jq .

# Delete printer
echo "Deleting printer..."
curl -s -X DELETE "${BASE_URL}/api/printers/test-printer?token=${ADMIN_TOKEN}" | jq .
```

### Bulk Printer Management

```bash
#!/bin/bash
# bulk-printers.sh
ADMIN_TOKEN="your-admin-token"
BASE_URL="http://localhost:8080"

# Add multiple printers from JSON file
printers=(
  '{"name":"Kitchen Printer","id":"kitchen","backend":{"type":"tcp9100","host":"192.168.1.10","port":9100}}'
  '{"name":"Bar Printer","id":"bar","backend":{"type":"tcp9100","host":"192.168.1.11","port":9100}}'
  '{"name":"Office Printer","id":"office","backend":{"type":"tcp9100","host":"192.168.1.12","port":9100}}'
)

for printer in "${printers[@]}"; do
  echo "Adding printer: $(echo $printer | jq -r '.name')"
  curl -s -X POST "${BASE_URL}/api/printers?token=${ADMIN_TOKEN}" \
    -H "Content-Type: application/json" \
    -d "$printer" | jq '.message'
done

# List all printers
echo "All printers:"
curl -s "${BASE_URL}/api/printers?token=${ADMIN_TOKEN}" | jq '.data.printers[] | {id, name, host: .backend.host}'
```

### Configuration Backup & Restore

```bash
#!/bin/bash
# backup-printers.sh
ADMIN_TOKEN="your-admin-token"
BASE_URL="http://localhost:8080"
BACKUP_FILE="printers-backup-$(date +%Y%m%d-%H%M%S).json"

# Backup current configuration
echo "Backing up printer configuration..."
curl -s "${BASE_URL}/api/printers?token=${ADMIN_TOKEN}" | jq '.data.printers' > "$BACKUP_FILE"
echo "Backup saved to: $BACKUP_FILE"

# Restore from backup
restore_printers() {
  local backup_file="$1"
  echo "Restoring from backup: $backup_file"
  
  # Clear existing printers (optional - be careful!)
  # curl -s "${BASE_URL}/api/printers?token=${ADMIN_TOKEN}" | jq -r '.data.printers[].id' | while read id; do
  #   curl -s -X DELETE "${BASE_URL}/api/printers/$id?token=${ADMIN_TOKEN}"
  # done
  
  # Add printers from backup
  jq -r '.[] | @json' "$backup_file" | while read printer; do
    curl -s -X POST "${BASE_URL}/api/printers?token=${ADMIN_TOKEN}" \
      -H "Content-Type: application/json" \
      -d "$printer" | jq '.message'
  done
}

# Usage: restore_printers "printers-backup-20240120-103000.json"
```

## 🔄 Hot Reload Features

### Automatic Configuration Reload

- **File Monitoring**: Konfigurasi otomatis reload saat file `printers.yaml` berubah
- **Zero Downtime**: Tidak perlu restart service
- **Atomic Updates**: File di-update secara atomic untuk mencegah corruption
- **Memory Sync**: In-memory configuration langsung sync dengan file

### Configuration File Format

File `printers.yaml` format:

```yaml
printers:
  - name: "Main Office Printer"
    id: "printer-001"
    backend:
      type: "tcp9100"
      host: "192.168.1.100"
      port: 9100
  
  - name: "Kitchen Printer"
    id: "printer-002"
    backend:
      type: "tcp9100"
      host: "192.168.1.101"
      port: 9100
```

## 🛠️ Integration Examples

### Ansible Playbook

```yaml
---
- name: Configure Printer Proxy
  hosts: printer-proxy
  tasks:
    - name: Add office printer
      uri:
        url: "http://{{ inventory_hostname }}:8080/api/printers"
        method: POST
        body_format: json
        body:
          name: "Office Printer"
          id: "office"
          backend:
            type: "tcp9100"
            host: "{{ office_printer_ip }}"
            port: 9100
        status_code: 201
      vars:
        office_printer_ip: "192.168.1.100"
```

### Terraform Provider

```hcl
resource "printer_proxy_printer" "office" {
  name = "Office Printer"
  id   = "office"
  backend = {
    type = "tcp9100"
    host = "192.168.1.100"
    port = 9100
  }
}
```

### Docker Compose Integration

```yaml
version: '3.8'
services:
  printer-proxy:
    image: printer-proxy:latest
    environment:
      - ADMIN_TOKEN=your-secure-token
      - PRINTERS_CONFIG=/config/printers.yaml
    volumes:
      - ./printers.yaml:/config/printers.yaml
    ports:
      - "8080:8080"
```

## 🚧 Troubleshooting

### Common Issues

1. **Permission Denied**:
   ```bash
   # Check file permissions
   ls -la printers.yaml
   chmod 644 printers.yaml
   ```

2. **Invalid YAML**:
   ```bash
   # Validate YAML syntax
   python -c "import yaml; yaml.safe_load(open('printers.yaml'))"
   ```

3. **Token Issues**:
   ```bash
   # Check token is set
   echo $ADMIN_TOKEN
   
   # Test token
   curl "http://localhost:8080/api/printers?token=wrong-token"
   # Should return 401
   ```

### Health Check Integration

```bash
#!/bin/bash
# health-check-printers.sh
ADMIN_TOKEN="your-admin-token"

# Check if all printers are accessible
curl -s "http://localhost:8080/health/printers" | jq -r '.printers | to_entries[] | select(.value.status == "offline") | .key' | while read printer_id; do
  echo "⚠️ Printer $printer_id is offline"
done
```

---

## 🎯 Best Practices

1. **Backup Configuration**: Regular backup printer configuration
2. **Validate Changes**: Test printer connectivity after changes
3. **Monitor Health**: Use health endpoints untuk monitoring
4. **Secure Tokens**: Use strong, unique admin tokens
5. **Atomic Operations**: Changes are atomic - either all succeed or all fail
6. **Hot Reload**: Use reload endpoint instead of service restart

Printer CRUD API memberikan full control untuk mengelola printer configuration secara dinamis! 🚀
