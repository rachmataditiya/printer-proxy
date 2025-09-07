#!/bin/bash
set -euo pipefail

# =============================================================================
# Admin Token Generator for Printer Proxy
# =============================================================================
# This script generates a secure admin token for printer-proxy service
# =============================================================================

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

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

# Generate secure token
generate_token() {
    local length=${1:-32}
    
    if command -v openssl >/dev/null 2>&1; then
        # Use OpenSSL for cryptographically secure random
        openssl rand -hex "$length"
    elif [[ -r /dev/urandom ]]; then
        # Use /dev/urandom as fallback
        head -c "$length" /dev/urandom | base64 | tr -d "=+/" | cut -c1-"$length"
    else
        # Fallback to date-based token (less secure)
        date +%s | sha256sum | cut -c1-"$length"
    fi
}

# Validate token strength
validate_token() {
    local token="$1"
    local length=${#token}
    
    if [[ $length -lt 16 ]]; then
        log_warning "Token is shorter than recommended minimum (16 characters)"
        return 1
    fi
    
    if [[ $length -ge 32 ]]; then
        log_success "Token length is excellent ($length characters)"
    elif [[ $length -ge 24 ]]; then
        log_success "Token length is good ($length characters)"
    else
        log_success "Token length is acceptable ($length characters)"
    fi
    
    # Check for character diversity
    local unique_chars=$(echo "$token" | fold -w1 | sort -u | wc -l)
    if [[ $unique_chars -ge 10 ]]; then
        log_success "Token has good character diversity ($unique_chars unique characters)"
    else
        log_warning "Token has limited character diversity ($unique_chars unique characters)"
    fi
    
    return 0
}

# Update service file with new token
update_service_file() {
    local token="$1"
    local service_file="/etc/systemd/system/printer-proxy.service"
    
    if [[ ! -f "$service_file" ]]; then
        log_warning "Service file not found: $service_file"
        log_info "You can manually set the token in your service configuration"
        return 1
    fi
    
    if [[ $EUID -ne 0 ]]; then
        log_warning "Cannot update service file without root privileges"
        log_info "Run with sudo to update the service file automatically"
        return 1
    fi
    
    # Backup original file
    cp "$service_file" "${service_file}.backup.$(date +%Y%m%d-%H%M%S)"
    
    # Update token in service file
    sed -i "s/Environment=ADMIN_TOKEN=.*/Environment=ADMIN_TOKEN=${token}/" "$service_file"
    
    # Reload systemd
    systemctl daemon-reload
    
    log_success "Service file updated with new token"
    log_info "Restart the service to apply changes: sudo systemctl restart printer-proxy"
}

# Show usage information
show_usage() {
    echo "============================================================================="
    echo "🔑 PRINTER PROXY ADMIN TOKEN GENERATOR"
    echo "============================================================================="
    echo
    echo "Usage: $0 [OPTIONS]"
    echo
    echo "Options:"
    echo "  --length N        Generate token of length N (default: 32)"
    echo "  --update-service  Update service file with new token (requires sudo)"
    echo "  --help           Show this help message"
    echo
    echo "Examples:"
    echo "  $0                           # Generate 32-character token"
    echo "  $0 --length 48               # Generate 48-character token"
    echo "  $0 --update-service          # Generate and update service file"
    echo "  $0 --length 64 --update-service  # Generate 64-char token and update"
    echo
    echo "Security Recommendations:"
    echo "  • Use at least 32 characters for production"
    echo "  • Store token securely (not in version control)"
    echo "  • Rotate token regularly"
    echo "  • Use different tokens for different environments"
    echo
    echo "============================================================================="
}

# Main function
main() {
    local length=32
    local update_service=false
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --length)
                length="$2"
                if ! [[ "$length" =~ ^[0-9]+$ ]] || [[ $length -lt 8 ]]; then
                    echo "Error: Length must be a number >= 8"
                    exit 1
                fi
                shift 2
                ;;
            --update-service)
                update_service=true
                shift
                ;;
            --help)
                show_usage
                exit 0
                ;;
            *)
                echo "Error: Unknown option $1"
                echo "Use --help for usage information"
                exit 1
                ;;
        esac
    done
    
    echo "============================================================================="
    echo "🔑 GENERATING SECURE ADMIN TOKEN"
    echo "============================================================================="
    echo
    
    log_info "Generating ${length}-character admin token..."
    
    # Generate token
    local token
    token=$(generate_token "$length")
    
    # Validate token
    validate_token "$token"
    
    echo
    echo "============================================================================="
    echo "🎯 GENERATED ADMIN TOKEN"
    echo "============================================================================="
    echo
    echo "Token: ${token}"
    echo
    echo "📋 Usage Examples:"
    echo "  • Health Check: curl \"http://localhost:8080/healthz\""
    echo "  • Admin Status: curl \"http://localhost:8080/admin/status?token=${token}\""
    echo "  • List Printers: curl \"http://localhost:8080/api/printers?token=${token}\""
    echo
    echo "🔧 Service Configuration:"
    echo "  • Environment Variable: ADMIN_TOKEN=${token}"
    echo "  • Service File: /etc/systemd/system/printer-proxy.service"
    echo
    
    # Update service file if requested
    if [[ "$update_service" == true ]]; then
        echo "🔄 Updating service file..."
        if update_service_file "$token"; then
            echo "✅ Service file updated successfully"
        else
            echo "⚠️  Could not update service file automatically"
            echo "   Please manually set ADMIN_TOKEN=${token} in your service configuration"
        fi
    else
        echo "💡 To update service file automatically, run:"
        echo "   sudo $0 --update-service"
    fi
    
    echo
    echo "⚠️  SECURITY REMINDERS:"
    echo "  • Keep this token secure and private"
    echo "  • Do not commit to version control"
    echo "  • Rotate regularly in production"
    echo "  • Use different tokens for different environments"
    echo
    echo "============================================================================="
}

# Run main function
main "$@"
