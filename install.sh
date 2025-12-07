#!/usr/bin/env bash
set -euo pipefail

# waybar-crypto-ticker installer
# Usage: curl -fsSL https://raw.githubusercontent.com/BurgessTG/waybar-crypto-ticker/main/install.sh | bash

REPO="BurgessTG/waybar-crypto-ticker"
INSTALL_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.local/share/waybar-crypto-ticker"
CONFIG_DIR="$HOME/.config/waybar-crypto-ticker"

info() { echo -e "\033[1;34m==>\033[0m $1"; }
success() { echo -e "\033[1;32m==>\033[0m $1"; }
error() { echo -e "\033[1;31mError:\033[0m $1" >&2; exit 1; }

# Check dependencies
check_deps() {
    local missing=()

    command -v cargo &>/dev/null || missing+=("rust/cargo")
    pkg-config --exists gtk4 2>/dev/null || missing+=("gtk4")
    pkg-config --exists gtk4-layer-shell-0 2>/dev/null || missing+=("gtk4-layer-shell")

    if [[ ${#missing[@]} -gt 0 ]]; then
        error "Missing dependencies: ${missing[*]}

Install on Arch:   sudo pacman -S gtk4 gtk4-layer-shell rust
Install on Fedora: sudo dnf install gtk4-devel gtk4-layer-shell-devel rust cargo
Install on Ubuntu: sudo apt install libgtk-4-dev libgtk4-layer-shell-dev rustc cargo"
    fi
}

main() {
    info "Installing waybar-crypto-ticker..."

    check_deps

    # Create temp directory
    TEMP_DIR=$(mktemp -d)
    trap "rm -rf $TEMP_DIR" EXIT

    # Clone repo
    info "Downloading source..."
    git clone --depth 1 "https://github.com/$REPO.git" "$TEMP_DIR/src" 2>/dev/null || \
        error "Failed to clone repository"

    cd "$TEMP_DIR/src"

    # Build
    info "Building (this may take a few minutes)..."
    cargo build --release 2>/dev/null || error "Build failed"

    # Install binary
    info "Installing binary..."
    mkdir -p "$INSTALL_DIR"
    cp target/release/waybar-crypto-ticker "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/waybar-crypto-ticker"

    # Install icons
    info "Installing icons..."
    mkdir -p "$DATA_DIR/icons"
    if [[ -d icons ]]; then
        cp icons/* "$DATA_DIR/icons/" 2>/dev/null || true
    fi

    # Create example config if none exists
    if [[ ! -f "$CONFIG_DIR/config.toml" ]]; then
        info "Creating example config..."
        mkdir -p "$CONFIG_DIR"
        cat > "$CONFIG_DIR/config.toml" << 'EOF'
# waybar-crypto-ticker configuration
# See: https://github.com/BurgessTG/waybar-crypto-ticker

# Monitor to display on (run `hyprctl monitors` to find name)
# monitor = "DP-3"

[position]
anchor = "top-right"
margin_right = 200
width = 320
height = 26

[appearance]
font_family = "monospace"
font_size = 11.0
color_up = "#4ec970"
color_down = "#e05555"
color_neutral = "#888888"

[animation]
scroll_speed = 30.0
fps = 60

[[coins]]
symbol = "BTC/USD"
name = "BTC"
icon = "btc.svg"

[[coins]]
symbol = "ETH/USD"
name = "ETH"
icon = "eth.svg"

[[coins]]
symbol = "SOL/USD"
name = "SOL"
icon = "sol.svg"
EOF
    fi

    success "Installation complete!"
    echo ""
    echo "Binary:  $INSTALL_DIR/waybar-crypto-ticker"
    echo "Icons:   $DATA_DIR/icons/"
    echo "Config:  $CONFIG_DIR/config.toml"
    echo ""
    echo "To start: waybar-crypto-ticker"
    echo "To autostart, add to ~/.config/hypr/hyprland.conf:"
    echo "  exec-once = ~/.local/bin/waybar-crypto-ticker"
}

main "$@"
