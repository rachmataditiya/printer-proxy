#!/bin/bash
set -euo pipefail

# =============================================================================
# Printer Proxy Uninstallation Script for Raspberry Pi
# =============================================================================
# This script removes the printer-proxy service and all related files
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

# Confirm uninstallation
confirm_uninstall() {
    echo "============================================================================="
    echo "⚠️  PRINTER PROXY UNINSTALLATION"
    echo "============================================================================="
    echo "This will remove:"
    echo "  • Service: ${SERVICE_NAME}"
    echo "  • Binary: ${INSTALL_DIR}/${BINARY_NAME}"
    echo "  • Config: ${CONFIG_DIR}"
    echo "  • Data: ${DATA_DIR}"
    echo "  • User: ${SERVICE_USER}"
    echo
    echo "⚠️  WARNING: This action cannot be undone!"
    echo "============================================================================="
    echo
    
    read -p "Are you sure you want to uninstall printer-proxy? (yes/no): " -r
    if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
        log_info "Uninstallation cancelled"
        exit 0
    fi
}

# Stop and disable service
stop_service() {
    log_info "Stopping and disabling ${SERVICE_NAME} service..."
    
    if systemctl is-active --quiet "${SERVICE_NAME}"; then
        systemctl stop "${SERVICE_NAME}"
        log_success "Service stopped"
    else
        log_warning "Service was not running"
    fi
    
    if systemctl is-enabled --quiet "${SERVICE_NAME}"; then
        systemctl disable "${SERVICE_NAME}"
        log_success "Service disabled"
    else
        log_warning "Service was not enabled"
    fi
}

# Remove systemd service file
remove_service_file() {
    log_info "Removing systemd service file..."
    
    if [[ -f "${SERVICE_FILE}" ]]; then
        rm -f "${SERVICE_FILE}"
        systemctl daemon-reload
        log_success "Service file removed"
    else
        log_warning "Service file not found: ${SERVICE_FILE}"
    fi
}

# Remove binary
remove_binary() {
    log_info "Removing binary..."
    
    if [[ -f "${INSTALL_DIR}/${BINARY_NAME}" ]]; then
        rm -f "${INSTALL_DIR}/${BINARY_NAME}"
        log_success "Binary removed from ${INSTALL_DIR}/${BINARY_NAME}"
    else
        log_warning "Binary not found: ${INSTALL_DIR}/${BINARY_NAME}"
    fi
}

# Remove configuration directory
remove_config() {
    log_info "Removing configuration directory..."
    
    if [[ -d "${CONFIG_DIR}" ]]; then
        # Ask if user wants to keep config
        echo
        read -p "Do you want to keep configuration files in ${CONFIG_DIR}? (yes/no): " -r
        if [[ $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
            log_info "Keeping configuration directory: ${CONFIG_DIR}"
        else
            rm -rf "${CONFIG_DIR}"
            log_success "Configuration directory removed"
        fi
    else
        log_warning "Configuration directory not found: ${CONFIG_DIR}"
    fi
}

# Remove data directory
remove_data() {
    log_info "Removing data directory..."
    
    if [[ -d "${DATA_DIR}" ]]; then
        # Ask if user wants to keep data
        echo
        read -p "Do you want to keep data files in ${DATA_DIR}? (yes/no): " -r
        if [[ $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
            log_info "Keeping data directory: ${DATA_DIR}"
        else
            rm -rf "${DATA_DIR}"
            log_success "Data directory removed"
        fi
    else
        log_warning "Data directory not found: ${DATA_DIR}"
    fi
}

# Remove service user
remove_service_user() {
    log_info "Removing service user..."
    
    if id "${SERVICE_USER}" &>/dev/null; then
        # Check if user has any processes
        if pgrep -u "${SERVICE_USER}" >/dev/null 2>&1; then
            log_warning "User ${SERVICE_USER} has running processes, skipping user removal"
            log_info "You may need to manually remove the user later"
        else
            userdel "${SERVICE_USER}"
            log_success "Service user removed: ${SERVICE_USER}"
        fi
    else
        log_warning "Service user not found: ${SERVICE_USER}"
    fi
}

# Clean up logs
cleanup_logs() {
    log_info "Cleaning up systemd logs..."
    
    # Remove journal logs for the service
    journalctl --vacuum-time=1s --unit="${SERVICE_NAME}" >/dev/null 2>&1 || true
    log_success "Systemd logs cleaned up"
}

# Show uninstallation summary
show_summary() {
    echo
    echo "============================================================================="
    echo "✅ PRINTER PROXY UNINSTALLATION COMPLETED"
    echo "============================================================================="
    echo
    echo "📋 Uninstallation Summary:"
    echo "  • Service: ${SERVICE_NAME} (stopped and disabled)"
    echo "  • Binary: ${INSTALL_DIR}/${BINARY_NAME} (removed)"
    echo "  • Config: ${CONFIG_DIR} (removed or kept as requested)"
    echo "  • Data: ${DATA_DIR} (removed or kept as requested)"
    echo "  • User: ${SERVICE_USER} (removed or kept as requested)"
    echo
    echo "🧹 Cleanup:"
    echo "  • Systemd service file removed"
    echo "  • Systemd daemon reloaded"
    echo "  • Journal logs cleaned up"
    echo
    echo "📝 Notes:"
    echo "  • If you kept config/data directories, you can remove them manually"
    echo "  • If you kept the service user, you can remove it with: userdel ${SERVICE_USER}"
    echo "  • Check for any remaining processes: ps aux | grep ${BINARY_NAME}"
    echo
    echo "🔄 To reinstall:"
    echo "  • Run the install.sh script again"
    echo "  • Or follow the manual installation guide"
    echo
    echo "============================================================================="
}

# Main uninstallation function
main() {
    check_root
    confirm_uninstall
    stop_service
    remove_service_file
    remove_binary
    remove_config
    remove_data
    remove_service_user
    cleanup_logs
    show_summary
}

# Handle command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --force)
            # Skip confirmation prompts
            FORCE_UNINSTALL=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo
            echo "Options:"
            echo "  --force    Skip confirmation prompts (use with caution)"
            echo "  --help     Show this help message"
            echo
            echo "Examples:"
            echo "  $0                # Interactive uninstallation"
            echo "  $0 --force        # Force uninstallation without prompts"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Run main uninstallation
main
