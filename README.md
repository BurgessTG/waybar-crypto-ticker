# waybar-crypto-ticker

A sleek, scrolling cryptocurrency ticker overlay for Waybar on Hyprland/Wayland.

![Demo](https://raw.githubusercontent.com/BurgessTG/waybar-crypto-ticker/main/demo.gif)

## Features

- **Real-time prices** via Kraken WebSocket API
- **Smooth scrolling** animation at 60 FPS
- **24h change percentage** with color-coded arrows
- **Cryptocurrency icons** with circular clipping
- **Auto-hide on fullscreen** — disappears when you go fullscreen
- **Multi-monitor aware** — only shows on your configured display
- **Fully configurable** — position, colors, fonts, coins, and more

## Included Cryptocurrencies

Icons included out of the box:

| Coin | Symbol | Kraken Pair |
|------|--------|-------------|
| Bitcoin | BTC | `BTC/USD` |
| Ethereum | ETH | `ETH/USD` |
| Solana | SOL | `SOL/USD` |
| Cardano | ADA | `ADA/USD` |
| XRP | XRP | `XRP/USD` |
| Avalanche | AVAX | `AVAX/USD` |

### Adding More Coins

Any cryptocurrency available on [Kraken](https://www.kraken.com/) can be added. Simply:

1. Find the trading pair on Kraken (e.g., `DOGE/USD`, `MATIC/USD`, `LINK/USD`)
2. Download an icon (SVG or PNG) to `~/.local/share/waybar-crypto-ticker/icons/`
3. Add to your config:

```toml
[[coins]]
symbol = "DOGE/USD"
name = "Dogecoin"
icon = "doge.svg"
```

Popular pairs available: `DOT/USD`, `ATOM/USD`, `LINK/USD`, `MATIC/USD`, `UNI/USD`, `LTC/USD`, `SHIB/USD`, and [many more](https://api.kraken.com/0/public/AssetPairs).

## Requirements

- Hyprland (Wayland compositor)
- Waybar
- GTK4 + gtk4-layer-shell
- Rust toolchain (for building)

### Arch Linux

```bash
sudo pacman -S gtk4 gtk4-layer-shell rust
```

### Fedora

```bash
sudo dnf install gtk4-devel gtk4-layer-shell-devel rust cargo
```

### Ubuntu/Debian

```bash
sudo apt install libgtk-4-dev libgtk4-layer-shell-dev rustc cargo
```

## Installation

### One-liner install

```bash
curl -fsSL https://raw.githubusercontent.com/BurgessTG/waybar-crypto-ticker/main/install.sh | bash
```

### Manual build

```bash
git clone https://github.com/BurgessTG/waybar-crypto-ticker
cd waybar-crypto-ticker
cargo build --release
cp target/release/waybar-crypto-ticker ~/.local/bin/
mkdir -p ~/.local/share/waybar-crypto-ticker/icons
cp icons/* ~/.local/share/waybar-crypto-ticker/icons/
```

## Configuration

Create `~/.config/waybar-crypto-ticker/config.toml`:

```toml
# Monitor to display on (use `hyprctl monitors` to find name)
monitor = "DP-3"

[position]
anchor = "top-right"    # top-left, top-right, bottom-left, bottom-right
margin_top = 0
margin_right = 200      # Adjust to not overlap waybar modules
margin_bottom = 0
margin_left = 0
width = 320
height = 26

[appearance]
font_family = "monospace"
font_size = 11.0
color_up = "#4ec970"
color_down = "#e05555"
color_neutral = "#888888"
icon_size = 16

[animation]
scroll_speed = 30.0     # Pixels per second
fps = 60

# Coins to display (Kraken trading pairs)
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

[[coins]]
symbol = "ADA/USD"
name = "ADA"
icon = "ada.svg"

[[coins]]
symbol = "XRP/USD"
name = "XRP"
icon = "xrp.svg"
```

## Autostart

Add to `~/.config/hypr/hyprland.conf`:

```conf
exec-once = ~/.local/bin/waybar-crypto-ticker
```

Or create a separate autostart file:

```conf
# ~/.config/hypr/autostart.conf
exec-once = ~/.local/bin/waybar-crypto-ticker
```

## Icons

Place SVG or PNG icons in `~/.local/share/waybar-crypto-ticker/icons/`.

Icon filenames must match the `icon` field in your config. The ticker renders icons at the configured size with circular clipping.

### Getting icons

You can download cryptocurrency icons from:
- [CryptoCurrency Icons](https://github.com/spothq/cryptocurrency-icons)
- [Simple Icons](https://simpleicons.org/)

## How it works

1. Connects to Kraken's WebSocket API for real-time price feeds
2. Fetches 24h open prices from REST API for change calculation
3. Renders a smooth scrolling ticker using GTK4 + Cairo
4. Uses gtk4-layer-shell to overlay on Waybar
5. Monitors Hyprland IPC socket to hide during fullscreen

## Troubleshooting

### Ticker doesn't appear
- Check if the monitor name is correct: `hyprctl monitors`
- Verify GTK4 layer shell is installed
- Check logs: `waybar-crypto-ticker 2>&1 | head -50`

### Overlaps waybar modules
- Adjust `margin_right` in config to push ticker left

### Prices not updating
- Check internet connection
- Kraken may be experiencing issues
- Wait for WebSocket reconnection (5 second retry)

## License

MIT License - see [LICENSE](LICENSE)

## Contributing

Pull requests welcome! Please ensure code is formatted with `cargo fmt`.
