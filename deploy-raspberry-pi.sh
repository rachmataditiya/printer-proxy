#!/bin/bash
set -euo pipefail

# =============================================================================
# Complete Raspberry Pi Deployment Script for Printer Proxy
# =============================================================================
# This script performs a complete deployment including:
# - Building the application
# - Installing the service
# - Setting up SSL certificates
# - Configuring nginx reverse proxy
# - Generating secure admin tokens
# =============================================================================

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Configuration
SERVICE_NAME="printer-proxy"
DOMAIN="${1:-localhost}"
ADMIN_TOKEN=""
SKIP_BUILD=false
SKIP_SSL=false
SKIP_SERVICE=false

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

log_step() {
    echo -e "${PURPLE}[STEP]${NC} $1"
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root (use sudo)"
        exit 1
    fi
}

# Check system requirements
check_requirements() {
    log_step "Checking system requirements..."
    
    # Check if we're on Raspberry Pi OS or similar
    if [[ ! -f /etc/os-release ]]; then
        log_error "Cannot determine OS version"
        exit 1
    fi
    
    # Check for Rust
    if ! command -v cargo >/dev/null 2>&1; then
        log_error "Rust/Cargo not found. Please install Rust first:"
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
    
    # Check Rust version
    local rust_version
    rust_version=$(cargo --version | cut -d' ' -f2)
    log_success "Rust version: $rust_version"
    
    # Check available memory
    local mem_gb
    mem_gb=$(free -g | awk '/^Mem:/{print $2}')
    if [[ $mem_gb -lt 1 ]]; then
        log_warning "Low memory detected (${mem_gb}GB). Build may be slow or fail."
    else
        log_success "Memory: ${mem_gb}GB"
    fi
    
    # Check disk space
    local disk_gb
    disk_gb=$(df -BG . | awk 'NR==2{print $4}' | sed 's/G//')
    if [[ $disk_gb -lt 2 ]]; then
        log_warning "Low disk space (${disk_gb}GB). Ensure sufficient space for build."
    else
        log_success "Disk space: ${disk_gb}GB available"
    fi
}

# Build the application
build_application() {
    if [[ "$SKIP_BUILD" == true ]]; then
        log_info "Skipping build (--skip-build specified)"
        return
    fi
    
    log_step "Building printer-proxy application..."
    
    # Clean previous build
    log_info "Cleaning previous build..."
    cargo clean
    
    # Build with optimizations
    log_info "Building release version (this may take several minutes)..."
    if cargo build --release; then
        log_success "Build completed successfully"
    else
        log_error "Build failed"
        exit 1
    fi
    
    # Check binary size
    local binary_size
    binary_size=$(du -h target/release/printer-proxy | cut -f1)
    log_success "Binary size: $binary_size"
}

# Generate admin token
generate_admin_token() {
    log_step "Generating secure admin token..."
    
    if [[ -n "$ADMIN_TOKEN" ]]; then
        log_info "Using provided admin token"
    else
        if command -v openssl >/dev/null 2>&1; then
            ADMIN_TOKEN=$(openssl rand -hex 32)
        else
            ADMIN_TOKEN=$(head -c 32 /dev/urandom | base64 | tr -d "=+/" | cut -c1-32)
        fi
        log_success "Generated admin token: $ADMIN_TOKEN"
    fi
}

# Install the service
install_service() {
    if [[ "$SKIP_SERVICE" == true ]]; then
        log_info "Skipping service installation (--skip-service specified)"
        return
    fi
    
    log_step "Installing printer-proxy service..."
    
    # Run install script with generated token
    if ./install.sh --admin-token "$ADMIN_TOKEN"; then
        log_success "Service installed successfully"
    else
        log_error "Service installation failed"
        exit 1
    fi
}

# Setup SSL certificates
setup_ssl() {
    if [[ "$SKIP_SSL" == true ]]; then
        log_info "Skipping SSL setup (--skip-ssl specified)"
        return
    fi
    
    log_step "Setting up SSL certificates and nginx..."
    
    # Run SSL setup script
    if ./setup-ssl.sh "$DOMAIN" 8080; then
        log_success "SSL setup completed successfully"
    else
        log_error "SSL setup failed"
        exit 1
    fi
}

# Test the installation
test_installation() {
    log_step "Testing installation..."
    
    # Wait for service to start
    sleep 3
    
    # Test health endpoint
    log_info "Testing health endpoint..."
    if curl -s -f "http://localhost:8080/healthz" >/dev/null; then
        log_success "Health endpoint responding"
    else
        log_warning "Health endpoint not responding (service may still be starting)"
    fi
    
    # Test admin endpoint
    log_info "Testing admin endpoint..."
    if curl -s -f "http://localhost:8080/admin/status?token=$ADMIN_TOKEN" >/dev/null; then
        log_success "Admin endpoint responding"
    else
        log_warning "Admin endpoint not responding"
    fi
    
    # Test SSL if enabled
    if [[ "$SKIP_SSL" != true ]]; then
        log_info "Testing SSL endpoint..."
        if curl -s -k -f "https://$DOMAIN/healthz" >/dev/null; then
            log_success "SSL endpoint responding"
        else
            log_warning "SSL endpoint not responding"
        fi
    fi
}

# Show deployment summary
show_summary() {
    echo
    echo "============================================================================="
    echo "🎉 RASPBERRY PI DEPLOYMENT COMPLETED SUCCESSFULLY!"
    echo "============================================================================="
    echo
    echo "📋 Deployment Summary:"
    echo "  • Service: $SERVICE_NAME (installed and running)"
    echo "  • Domain: $DOMAIN"
    echo "  • Admin Token: $ADMIN_TOKEN"
    echo "  • SSL: $([ "$SKIP_SSL" == true ] && echo "Disabled" || echo "Enabled")"
    echo
    echo "🌐 Access URLs:"
    if [[ "$SKIP_SSL" != true ]]; then
        echo "  • HTTPS: https://$DOMAIN"
        echo "  • Health: https://$DOMAIN/healthz"
        echo "  • Print: https://$DOMAIN/{printer_id}/cgi-bin/epos/service.cgi"
    else
        echo "  • HTTP: http://localhost:8080"
        echo "  • Health: http://localhost:8080/healthz"
        echo "  • Print: http://localhost:8080/{printer_id}/cgi-bin/epos/service.cgi"
    fi
    echo
    echo "🔒 Admin Management:"
    echo "  • Status: curl \"http://localhost:8080/admin/status?token=$ADMIN_TOKEN\""
    echo "  • Shutdown: curl \"http://localhost:8080/admin/shutdown?token=$ADMIN_TOKEN\""
    echo "  • Restart: curl \"http://localhost:8080/admin/restart?token=$ADMIN_TOKEN\""
    echo
    echo "🖨️ Printer Management:"
    echo "  • List: curl \"http://localhost:8080/api/printers?token=$ADMIN_TOKEN\""
    echo "  • Add: curl -X POST \"http://localhost:8080/api/printers?token=$ADMIN_TOKEN\" -H \"Content-Type: application/json\" -d '{\"name\":\"Test\",\"id\":\"test\",\"backend\":{\"type\":\"tcp9100\",\"host\":\"192.168.1.100\",\"port\":9100}}'"
    echo
    echo "🔧 Service Management:"
    echo "  • Status: sudo systemctl status $SERVICE_NAME"
    echo "  • Logs: sudo journalctl -u $SERVICE_NAME -f"
    echo "  • Restart: sudo systemctl restart $SERVICE_NAME"
    echo
    echo "📁 Important Files:"
    echo "  • Config: /etc/printer-proxy/printers.yaml"
    echo "  • Service: /etc/systemd/system/$SERVICE_NAME.service"
    echo "  • Binary: /usr/local/bin/printer-proxy"
    echo "  • Logs: /var/lib/printer-proxy/logs/"
    echo
    echo "🚀 Next Steps:"
    echo "  1. Configure printers in /etc/printer-proxy/printers.yaml"
    echo "  2. Test printer connectivity"
    echo "  3. Set up monitoring and backups"
    echo "  4. Consider firewall rules for external access"
    echo
    echo "⚠️  Security Notes:"
    echo "  • Change the admin token in production"
    echo "  • Configure firewall rules appropriately"
    echo "  • Monitor logs for suspicious activity"
    echo "  • Keep the system updated"
    echo
    echo "============================================================================="
}

# Show usage information
show_usage() {
    echo "Usage: $0 [OPTIONS] [DOMAIN]"
    echo
    echo "Arguments:"
    echo "  DOMAIN    Domain name for SSL setup (default: localhost)"
    echo
    echo "Options:"
    echo "  --admin-token TOKEN    Use specific admin token"
    echo "  --skip-build          Skip building the application"
    echo "  --skip-ssl            Skip SSL certificate setup"
    echo "  --skip-service        Skip service installation"
    echo "  --help                Show this help message"
    echo
    echo "Examples:"
    echo "  $0                                    # Full deployment for localhost"
    echo "  $0 printer.local                     # Full deployment for printer.local"
    echo "  $0 --skip-ssl printer.local          # Deploy without SSL"
    echo "  $0 --admin-token my-token            # Use custom admin token"
    echo "  $0 --skip-build --skip-ssl           # Install service only"
    echo
    echo "This script will:"
    echo "  • Check system requirements"
    echo "  • Build the printer-proxy application"
    echo "  • Generate secure admin token"
    echo "  • Install systemd service"
    echo "  • Setup SSL certificates and nginx"
    echo "  • Test the installation"
    echo
    echo "Prerequisites:"
    echo "  • Rust/Cargo installed"
    echo "  • Root privileges (sudo)"
    echo "  • Internet connection for dependencies"
}

# Main deployment function
main() {
    echo "============================================================================="
    echo "🚀 RASPBERRY PI PRINTER PROXY DEPLOYMENT"
    echo "============================================================================="
    echo "Complete deployment with:"
    echo "  • High-performance Rust application"
    echo "  • Connection pooling & health caching"
    echo "  • Admin management endpoints"
    echo "  • Printer CRUD API"
    echo "  • SSL certificates & nginx reverse proxy"
    echo "============================================================================="
    echo
    
    check_root
    check_requirements
    build_application
    generate_admin_token
    install_service
    setup_ssl
    test_installation
    show_summary
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --admin-token)
            ADMIN_TOKEN="$2"
            shift 2
            ;;
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --skip-ssl)
            SKIP_SSL=true
            shift
            ;;
        --skip-service)
            SKIP_SERVICE=true
            shift
            ;;
        --help)
            show_usage
            exit 0
            ;;
        -*)
            log_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
        *)
            if [[ -z "${1:-}" ]] || [[ "$1" != "localhost" ]]; then
                DOMAIN="$1"
            fi
            shift
            ;;
    esac
done

# Run main deployment
main
