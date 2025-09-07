#!/bin/bash
set -euo pipefail

# =============================================================================
# Printer Proxy Installation Script for Raspberry Pi
# =============================================================================
# This script installs the printer-proxy service with all latest features:
# - High-performance connection pooling
# - Health check caching
# - Admin management endpoints
# - Printer CRUD API
# - SSL certificate management
# =============================================================================

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SERVICE_NAME="printer-proxy"
SERVICE_USER="printer-proxy"
BINARY_NAME="printer-proxy"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/printer-proxy"
DATA_DIR="/var/lib/printer-proxy"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

# Default configuration
DEFAULT_ADMIN_TOKEN=""
DEFAULT_LISTEN_ADDR="0.0.0.0:8080"
DEFAULT_LOG_LEVEL="info"

# Functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root (use sudo)"
        exit 1
    fi
}

# Check if binary exists
check_binary() {
    if [[ ! -f "./target/release/${BINARY_NAME}" ]]; then
        log_error "Binary not found at ./target/release/${BINARY_NAME}"
        log_error "Please build the project first with: cargo build --release"
        exit 1
    fi
    log_success "Binary found at ./target/release/${BINARY_NAME}"
}

# Generate secure admin token
generate_admin_token() {
    if command -v openssl >/dev/null 2>&1; then
        openssl rand -hex 32
    elif command -v /dev/urandom >/dev/null 2>&1; then
        head -c 32 /dev/urandom | base64 | tr -d "=+/" | cut -c1-32
    else
        # Fallback to date-based token
        date +%s | sha256sum | cut -c1-32
    fi
}

# Create service user
create_service_user() {
    log_info "Creating service user: ${SERVICE_USER}"
    
    if id "${SERVICE_USER}" &>/dev/null; then
        log_warning "User ${SERVICE_USER} already exists"
    else
        useradd --system --no-create-home --shell /bin/false "${SERVICE_USER}"
        log_success "Created user: ${SERVICE_USER}"
    fi
}

# Create directories
create_directories() {
    log_info "Creating directories..."
    
    # Create config directory
    mkdir -p "${CONFIG_DIR}"
    chmod 755 "${CONFIG_DIR}"
    
    # Create data directory
    mkdir -p "${DATA_DIR}/logs"
    chmod 755 "${DATA_DIR}"
    chmod 755 "${DATA_DIR}/logs"
    
    # Set ownership
    chown -R "${SERVICE_USER}:${SERVICE_USER}" "${CONFIG_DIR}"
    chown -R "${SERVICE_USER}:${SERVICE_USER}" "${DATA_DIR}"
    
    log_success "Created directories: ${CONFIG_DIR}, ${DATA_DIR}"
}

# Install binary
install_binary() {
    log_info "Installing binary to ${INSTALL_DIR}/${BINARY_NAME}"
    
    # Stop service if running
    if systemctl is-active --quiet "${SERVICE_NAME}"; then
        log_info "Stopping existing service..."
        systemctl stop "${SERVICE_NAME}"
    fi
    
    # Copy binary
    cp "./target/release/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod 755 "${INSTALL_DIR}/${BINARY_NAME}"
    chown root:root "${INSTALL_DIR}/${BINARY_NAME}"
    
    log_success "Binary installed successfully"
}

# Create default configuration
create_default_config() {
    local config_file="${CONFIG_DIR}/printers.yaml"
    
    if [[ -f "${config_file}" ]]; then
        log_warning "Configuration file already exists: ${config_file}"
        return
    fi
    
    log_info "Creating default configuration: ${config_file}"
    
    cat > "${config_file}" << 'EOF'
# Printer Proxy Configuration
# Add your printers here using the CRUD API or edit this file directly

printers:
  # Example printer configuration
  - name: "Example Office Printer"
    id: "office-001"
    backend:
      type: "tcp9100"
      host: "192.168.1.100"
      port: 9100
  
  # Add more printers as needed
  # - name: "Kitchen Printer"
  #   id: "kitchen-001"
  #   backend:
  #     type: "tcp9100"
  #     host: "192.168.1.101"
  #     port: 9100
EOF
    
    chmod 644 "${config_file}"
    chown "${SERVICE_USER}:${SERVICE_USER}" "${config_file}"
    
    log_success "Default configuration created"
}

# Install systemd service
install_service() {
    log_info "Installing systemd service..."
    
    # Generate admin token if not provided
    if [[ -z "${DEFAULT_ADMIN_TOKEN}" ]]; then
        DEFAULT_ADMIN_TOKEN=$(generate_admin_token)
        log_info "Generated admin token: ${DEFAULT_ADMIN_TOKEN}"
    fi
    
    # Create service file
    cat > "${SERVICE_FILE}" << EOF
[Unit]
Description=Printer Proxy Server (ESC/POS) - High Performance Rust Implementation
Documentation=https://github.com/rachmataditiya/printer-proxy
After=network.target
Wants=network.target

[Service]
Type=exec
User=${SERVICE_USER}
Group=${SERVICE_USER}
ExecStart=${INSTALL_DIR}/${BINARY_NAME}
ExecReload=/bin/kill -HUP \$MAINPID
KillMode=mixed
KillSignal=SIGTERM
TimeoutStopSec=30
RestartSec=5
Restart=on-failure

# Environment variables
Environment=RUST_LOG=${DEFAULT_LOG_LEVEL}
Environment=LISTEN_ADDR=${DEFAULT_LISTEN_ADDR}
Environment=PRINTERS_CONFIG=${CONFIG_DIR}/printers.yaml
Environment=ADMIN_TOKEN=${DEFAULT_ADMIN_TOKEN}

# Working directory and file permissions
WorkingDirectory=${DATA_DIR}
ExecStartPre=/bin/mkdir -p ${DATA_DIR}/logs
ExecStartPre=/bin/chown -R ${SERVICE_USER}:${SERVICE_USER} ${DATA_DIR}
ExecStartPre=/bin/chown -R ${SERVICE_USER}:${SERVICE_USER} ${CONFIG_DIR}

# Security settings
PrivateDevices=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=${DATA_DIR} ${CONFIG_DIR}
NoNewPrivileges=yes
CapabilityBoundingSet=CAP_NET_BIND_SERVICE

# Limits
LimitNOFILE=65536
TasksMax=4096
MemoryMax=512M

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=${SERVICE_NAME}

[Install]
WantedBy=multi-user.target
EOF
    
    # Reload systemd and enable service
    systemctl daemon-reload
    systemctl enable "${SERVICE_NAME}"
    
    log_success "Systemd service installed and enabled"
}

# Install SSL script if available
install_ssl_script() {
    if [[ -f "./ssl.sh" ]]; then
        log_info "Installing SSL management script..."
        cp "./ssl.sh" "${CONFIG_DIR}/ssl.sh"
        chmod 755 "${CONFIG_DIR}/ssl.sh"
        chown root:root "${CONFIG_DIR}/ssl.sh"
        log_success "SSL script installed to ${CONFIG_DIR}/ssl.sh"
    else
        log_warning "SSL script not found, skipping..."
    fi
}

# Start service
start_service() {
    log_info "Starting ${SERVICE_NAME} service..."
    
    if systemctl start "${SERVICE_NAME}"; then
        log_success "Service started successfully"
        
        # Wait a moment and check status
        sleep 2
        if systemctl is-active --quiet "${SERVICE_NAME}"; then
            log_success "Service is running"
        else
            log_error "Service failed to start"
            systemctl status "${SERVICE_NAME}" --no-pager
            exit 1
        fi
    else
        log_error "Failed to start service"
        systemctl status "${SERVICE_NAME}" --no-pager
        exit 1
    fi
}

# Show installation summary
show_summary() {
    echo
    echo "============================================================================="
    echo "🎉 PRINTER PROXY INSTALLATION COMPLETED SUCCESSFULLY!"
    echo "============================================================================="
    echo
    echo "📋 Installation Summary:"
    echo "  • Binary: ${INSTALL_DIR}/${BINARY_NAME}"
    echo "  • Config: ${CONFIG_DIR}/printers.yaml"
    echo "  • Data: ${DATA_DIR}"
    echo "  • Service: ${SERVICE_NAME}"
    echo "  • User: ${SERVICE_USER}"
    echo
    echo "🔧 Service Management:"
    echo "  • Start:   sudo systemctl start ${SERVICE_NAME}"
    echo "  • Stop:    sudo systemctl stop ${SERVICE_NAME}"
    echo "  • Restart: sudo systemctl restart ${SERVICE_NAME}"
    echo "  • Status:  sudo systemctl status ${SERVICE_NAME}"
    echo "  • Logs:    sudo journalctl -u ${SERVICE_NAME} -f"
    echo
    echo "🌐 Service Endpoints:"
    echo "  • Health Check: http://localhost:8080/healthz"
    echo "  • Printers Health: http://localhost:8080/health/printers"
    echo "  • Print Endpoint: http://localhost:8080/{printer_id}/cgi-bin/epos/service.cgi"
    echo
    echo "🔒 Admin Endpoints (Token Required):"
    echo "  • Admin Status: http://localhost:8080/admin/status?token=TOKEN"
    echo "  • Service Shutdown: http://localhost:8080/admin/shutdown?token=TOKEN"
    echo "  • Service Restart: http://localhost:8080/admin/restart?token=TOKEN"
    echo "  • SSL Renewal: http://localhost:8080/admin/ssl/renew?token=TOKEN"
    echo
    echo "🖨️ Printer Management API:"
    echo "  • List Printers: GET /api/printers?token=TOKEN"
    echo "  • Create Printer: POST /api/printers?token=TOKEN"
    echo "  • Update Printer: PUT /api/printers/{id}?token=TOKEN"
    echo "  • Delete Printer: DELETE /api/printers/{id}?token=TOKEN"
    echo "  • Reload Config: GET /api/printers/reload?token=TOKEN"
    echo
    echo "🔑 Admin Token: ${DEFAULT_ADMIN_TOKEN}"
    echo "   ⚠️  IMPORTANT: Change this token in production!"
    echo "   Edit: ${SERVICE_FILE}"
    echo
    echo "📚 Documentation:"
    echo "  • ADMIN.md - Admin endpoints guide"
    echo "  • PRINTERS_API.md - Printer CRUD API guide"
    echo "  • PERFORMANCE.md - Performance optimizations"
    echo
    echo "🚀 Next Steps:"
    echo "  1. Update admin token in ${SERVICE_FILE}"
    echo "  2. Configure printers in ${CONFIG_DIR}/printers.yaml"
    echo "  3. Test endpoints with the provided URLs"
    echo "  4. Set up SSL certificates if needed"
    echo
    echo "============================================================================="
}

# Main installation function
main() {
    echo "============================================================================="
    echo "🚀 PRINTER PROXY INSTALLATION SCRIPT"
    echo "============================================================================="
    echo "Installing high-performance printer proxy with:"
    echo "  • Connection pooling & health caching"
    echo "  • Admin management endpoints"
    echo "  • Printer CRUD API"
    echo "  • SSL certificate management"
    echo "============================================================================="
    echo
    
    # Run installation steps
    check_root
    check_binary
    create_service_user
    create_directories
    install_binary
    create_default_config
    install_service
    install_ssl_script
    start_service
    show_summary
}

# Handle command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --admin-token)
            DEFAULT_ADMIN_TOKEN="$2"
            shift 2
            ;;
        --listen-addr)
            DEFAULT_LISTEN_ADDR="$2"
            shift 2
            ;;
        --log-level)
            DEFAULT_LOG_LEVEL="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo
            echo "Options:"
            echo "  --admin-token TOKEN    Set admin token (default: auto-generated)"
            echo "  --listen-addr ADDR     Set listen address (default: 0.0.0.0:8080)"
            echo "  --log-level LEVEL      Set log level (default: info)"
            echo "  --help                 Show this help message"
            echo
            echo "Examples:"
            echo "  $0                                    # Install with defaults"
            echo "  $0 --admin-token my-secure-token     # Install with custom token"
            echo "  $0 --listen-addr 127.0.0.1:8080     # Install with custom address"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Run main installation
main
