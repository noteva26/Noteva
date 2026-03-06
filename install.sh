#!/usr/bin/env bash
# ============================================================
# Noteva Install Script (Linux / macOS)
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/noteva26/Noteva/main/install.sh | bash
#   or: bash install.sh
# ============================================================
set -euo pipefail

# ─── Constants ───────────────────────────────────────────────
REPO="noteva26/Noteva"
API_URL="https://api.github.com/repos/${REPO}/releases/latest"
DOWNLOAD_BASE="https://github.com/${REPO}/releases/download"
DEFAULT_INSTALL_DIR="/opt/noteva"

# ─── Colors ──────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

info()    { echo -e "${CYAN}[INFO]${NC} $*"; }
success() { echo -e "${GREEN}[✓]${NC} $*"; }
warn()    { echo -e "${YELLOW}[!]${NC} $*"; }
error()   { echo -e "${RED}[✗]${NC} $*"; exit 1; }

# ─── Helper: prompt with default value ───────────────────────
# Usage: result=$(ask "Prompt text" "default_value")
ask() {
    local prompt="$1"
    local default="$2"
    local input
    echo -en "${BOLD}${prompt}${NC} [${GREEN}${default}${NC}]: " >&2
    read -r input
    echo "${input:-$default}"
}

# ─── Helper: prompt for choice ──────────────────────────────
# Usage: result=$(ask_choice "Prompt" "option1|option2" "default")
ask_choice() {
    local prompt="$1"
    local options="$2"
    local default="$3"
    local input
    echo -en "${BOLD}${prompt}${NC} (${options}) [${GREEN}${default}${NC}]: " >&2
    read -r input
    input="${input:-$default}"
    echo "$input"
}

# ============================================================
# Step 1: Detect OS & Architecture
# ============================================================
detect_platform() {
    info "Detecting platform..."

    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux*)  OS="linux" ;;
        Darwin*) OS="macos" ;;
        *)       error "Unsupported OS: $os. This script supports Linux and macOS only." ;;
    esac

    case "$arch" in
        x86_64|amd64)  ARCH="x86_64" ;;
        aarch64|arm64) ARCH="arm64" ;;
        *)             error "Unsupported architecture: $arch" ;;
    esac

    # Determine file extension
    EXT="tar.gz"

    ASSET_NAME="noteva-${OS}-${ARCH}.${EXT}"
    success "Platform: ${OS} ${ARCH} → ${ASSET_NAME}"
}

# ============================================================
# Step 2: Get latest version tag
# ============================================================
get_latest_version() {
    info "Fetching latest release version..."

    if command -v curl &>/dev/null; then
        VERSION=$(curl -fsSL "$API_URL" | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/')
    elif command -v wget &>/dev/null; then
        VERSION=$(wget -qO- "$API_URL" | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/')
    else
        error "curl or wget is required to download files."
    fi

    if [ -z "$VERSION" ]; then
        error "Failed to fetch latest version. Check your network connection."
    fi

    DOWNLOAD_URL="${DOWNLOAD_BASE}/${VERSION}/${ASSET_NAME}"
    success "Latest version: ${VERSION}"
}

# ============================================================
# Step 3: Download & Verify
# ============================================================
download_and_extract() {
    local install_dir="$1"
    local tmp_dir
    tmp_dir=$(mktemp -d)
    local tmp_file="${tmp_dir}/${ASSET_NAME}"

    info "Downloading ${ASSET_NAME}..."
    if command -v curl &>/dev/null; then
        curl -fSL --progress-bar -o "$tmp_file" "$DOWNLOAD_URL"
    else
        wget --show-progress -qO "$tmp_file" "$DOWNLOAD_URL"
    fi
    success "Download complete."

    # Extract
    info "Extracting to ${install_dir}..."
    mkdir -p "$install_dir"
    tar -xzf "$tmp_file" -C "$install_dir"
    chmod +x "${install_dir}/noteva"
    success "Extraction complete."

    # Cleanup
    rm -rf "$tmp_dir"
}

# ============================================================
# Step 4: Interactive Configuration
# ============================================================
configure() {
    local install_dir="$1"
    local config_file="${install_dir}/config.yml"

    echo ""
    echo -e "${BOLD}${CYAN}════════════════════════════════════════${NC}"
    echo -e "${BOLD}${CYAN}        Noteva Configuration Setup      ${NC}"
    echo -e "${BOLD}${CYAN}════════════════════════════════════════${NC}"
    echo ""
    echo -e "Press ${GREEN}Enter${NC} to accept the default value shown in [green]."
    echo ""

    # ─── Server ──────────────────────────────────────────────
    echo -e "${YELLOW}── Server ──${NC}"
    local host port cors_origin
    host=$(ask "  Host" "0.0.0.0")
    port=$(ask "  Port" "8080")
    cors_origin=$(ask "  CORS Origin" "*")

    # ─── Database ────────────────────────────────────────────
    echo ""
    echo -e "${YELLOW}── Database ──${NC}"
    local db_driver db_url
    db_driver=$(ask_choice "  Driver" "sqlite|mysql" "sqlite")

    if [ "$db_driver" = "mysql" ]; then
        local db_host db_port db_user db_pass db_name
        db_host=$(ask "  MySQL Host" "127.0.0.1")
        db_port=$(ask "  MySQL Port" "3306")
        db_user=$(ask "  MySQL Username" "noteva")
        db_pass=$(ask "  MySQL Password" "noteva")
        db_name=$(ask "  MySQL Database" "noteva")
        db_url="mysql://${db_user}:${db_pass}@${db_host}:${db_port}/${db_name}"
    else
        db_url="data/noteva.db"
    fi

    # ─── Cache ───────────────────────────────────────────────
    echo ""
    echo -e "${YELLOW}── Cache ──${NC}"
    local cache_driver redis_url ttl_seconds
    cache_driver=$(ask_choice "  Driver" "memory|redis" "memory")
    redis_url="null"

    if [ "$cache_driver" = "redis" ]; then
        redis_url=$(ask "  Redis URL" "redis://127.0.0.1:6379")
    fi

    ttl_seconds=$(ask "  Cache TTL (seconds)" "3600")

    # ─── Upload ──────────────────────────────────────────────
    echo ""
    echo -e "${YELLOW}── Upload ──${NC}"
    local upload_path max_file_mb max_plugin_mb max_file_size max_plugin_size
    upload_path=$(ask "  Upload directory" "uploads")
    max_file_mb=$(ask "  Max image size (MB)" "10")
    max_plugin_mb=$(ask "  Max plugin file size (MB)" "50")
    max_file_size=$((max_file_mb * 1024 * 1024))
    max_plugin_size=$((max_plugin_mb * 1024 * 1024))

    # ─── Write config.yml ────────────────────────────────────
    info "Generating config.yml..."

    # Handle redis_url quoting
    local redis_url_yaml
    if [ "$redis_url" = "null" ]; then
        redis_url_yaml="null"
    else
        redis_url_yaml="\"${redis_url}\""
    fi

    cat > "$config_file" << EOF
# Noteva Configuration
# Generated by install.sh on $(date '+%Y-%m-%d %H:%M:%S')

server:
  host: "${host}"
  port: ${port}
  cors_origin: "${cors_origin}"

database:
  driver: ${db_driver}
  url: "${db_url}"

cache:
  driver: ${cache_driver}
  redis_url: ${redis_url_yaml}
  ttl_seconds: ${ttl_seconds}

theme:
  active: "default"
  path: "themes"

upload:
  path: "${upload_path}"
  max_file_size: ${max_file_size}
  max_plugin_file_size: ${max_plugin_size}
  allowed_types:
    - "image/jpeg"
    - "image/png"
    - "image/gif"
    - "image/webp"
    - "image/svg+xml"

store_url: "https://store.noteva.org"
EOF

    success "config.yml generated."
}

# ============================================================
# Step 5: Create required directories
# ============================================================
create_directories() {
    local install_dir="$1"
    info "Creating directories..."
    mkdir -p "${install_dir}/data"
    mkdir -p "${install_dir}/uploads"
    mkdir -p "${install_dir}/themes"
    mkdir -p "${install_dir}/plugins"
    success "Directories created."
}

# ============================================================
# Step 6: Register system service
# ============================================================
setup_service() {
    local install_dir="$1"

    echo ""
    echo -e "${YELLOW}── System Service ──${NC}"
    local setup_svc
    setup_svc=$(ask_choice "  Register as system service for auto-start?" "yes|no" "yes")

    if [ "$setup_svc" != "yes" ]; then
        warn "Skipped service registration. You can start Noteva manually:"
        echo "  cd ${install_dir} && ./noteva"
        return
    fi

    if [ "$OS" = "linux" ]; then
        setup_systemd "$install_dir"
    elif [ "$OS" = "macos" ]; then
        setup_launchd "$install_dir"
    fi
}

# ─── Linux: systemd ─────────────────────────────────────────
setup_systemd() {
    local install_dir="$1"
    local service_file="/etc/systemd/system/noteva.service"

    info "Creating systemd service..."

    sudo tee "$service_file" > /dev/null << EOF
[Unit]
Description=Noteva Blog System
After=network.target

[Service]
Type=simple
WorkingDirectory=${install_dir}
ExecStart=${install_dir}/noteva
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

    sudo systemctl daemon-reload
    sudo systemctl enable noteva
    sudo systemctl start noteva

    success "systemd service registered and started."
    info "Useful commands:"
    echo "  sudo systemctl status noteva   # Check status"
    echo "  sudo systemctl restart noteva  # Restart"
    echo "  sudo systemctl stop noteva     # Stop"
    echo "  sudo journalctl -u noteva -f   # View logs"
}

# ─── macOS: launchd ──────────────────────────────────────────
setup_launchd() {
    local install_dir="$1"
    local plist_file="/Library/LaunchDaemons/org.noteva.plist"

    info "Creating launchd service..."

    sudo tee "$plist_file" > /dev/null << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>org.noteva</string>
    <key>ProgramArguments</key>
    <array>
        <string>${install_dir}/noteva</string>
    </array>
    <key>WorkingDirectory</key>
    <string>${install_dir}</string>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>${install_dir}/logs/noteva.log</string>
    <key>StandardErrorPath</key>
    <string>${install_dir}/logs/noteva-error.log</string>
</dict>
</plist>
EOF

    mkdir -p "${install_dir}/logs"
    sudo launchctl load -w "$plist_file"

    success "launchd service registered and started."
    info "Useful commands:"
    echo "  sudo launchctl list | grep noteva        # Check status"
    echo "  sudo launchctl unload ${plist_file}      # Stop"
    echo "  sudo launchctl load -w ${plist_file}     # Start"
    echo "  tail -f ${install_dir}/logs/noteva.log   # View logs"
}

# ============================================================
# Main
# ============================================================
main() {
    echo ""
    echo -e "${BOLD}${CYAN}╔═══════════════════════════════════════════╗${NC}"
    echo -e "${BOLD}${CYAN}║                                           ║${NC}"
    echo -e "${BOLD}${CYAN}║       🚀  Noteva Installer  🚀           ║${NC}"
    echo -e "${BOLD}${CYAN}║                                           ║${NC}"
    echo -e "${BOLD}${CYAN}╚═══════════════════════════════════════════╝${NC}"
    echo ""

    # Step 1: Detect platform
    detect_platform

    # Step 2: Get latest version
    get_latest_version

    # Step 3: Choose install directory
    echo ""
    local install_dir
    install_dir=$(ask "Install directory" "$DEFAULT_INSTALL_DIR")

    # Check if already installed
    if [ -f "${install_dir}/noteva" ]; then
        warn "Noteva is already installed at ${install_dir}."
        local action
        action=$(ask_choice "  Upgrade binary and keep config?" "yes|no" "yes")
        if [ "$action" != "yes" ]; then
            info "Installation cancelled."
            exit 0
        fi
        # Upgrade mode: only replace binary
        info "Upgrading binary..."
        local tmp_dir
        tmp_dir=$(mktemp -d)
        local tmp_file="${tmp_dir}/${ASSET_NAME}"
        if command -v curl &>/dev/null; then
            curl -fSL --progress-bar -o "$tmp_file" "$DOWNLOAD_URL"
        else
            wget --show-progress -qO "$tmp_file" "$DOWNLOAD_URL"
        fi
        # Extract only the binary
        tar -xzf "$tmp_file" -C "$tmp_dir"
        # Stop service before replacing binary
        if [ "$OS" = "linux" ] && systemctl is-active noteva &>/dev/null; then
            sudo systemctl stop noteva
        fi
        cp "${tmp_dir}/noteva" "${install_dir}/noteva"
        chmod +x "${install_dir}/noteva"
        rm -rf "$tmp_dir"
        # Restart service
        if [ "$OS" = "linux" ] && systemctl is-enabled noteva &>/dev/null; then
            sudo systemctl start noteva
        fi
        success "Upgrade complete! Noteva ${VERSION} is now running."
        exit 0
    fi

    # Step 4: Download & extract
    download_and_extract "$install_dir"

    # Step 5: Create directories
    create_directories "$install_dir"

    # Step 6: Interactive configuration
    configure "$install_dir"

    # Step 7: Setup system service
    setup_service "$install_dir"

    # Done!
    echo ""
    echo -e "${BOLD}${GREEN}════════════════════════════════════════${NC}"
    echo -e "${BOLD}${GREEN}    ✅  Noteva installed successfully!  ${NC}"
    echo -e "${BOLD}${GREEN}════════════════════════════════════════${NC}"
    echo ""
    echo -e "  📁 Install directory: ${BOLD}${install_dir}${NC}"
    echo -e "  📝 Configuration:     ${BOLD}${install_dir}/config.yml${NC}"

    # Read port from config to show correct URL
    local port
    port=$(grep 'port:' "${install_dir}/config.yml" | head -1 | awk '{print $2}')
    local host
    host=$(grep 'host:' "${install_dir}/config.yml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
    local display_host
    if [ "$host" = "0.0.0.0" ]; then
        display_host="localhost"
    else
        display_host="$host"
    fi
    echo -e "  🌐 Access URL:        ${BOLD}http://${display_host}:${port}${NC}"
    echo ""
}

main "$@"
