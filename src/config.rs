//! Configuration management for waybar-crypto-ticker.
//!
//! Loads settings from `~/.config/waybar-crypto-ticker/config.toml` if present,
//! otherwise uses sensible defaults.

use serde::Deserialize;
use std::path::PathBuf;

/// Runtime configuration for the ticker.
#[derive(Debug, Clone)]
pub struct Config {
    pub monitor: Option<String>,
    pub position: Position,
    pub appearance: Appearance,
    pub animation: Animation,
    pub coins: Vec<CoinConfig>,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub anchor: Anchor,
    pub margin_top: i32,
    pub margin_right: i32,
    pub margin_bottom: i32,
    pub margin_left: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Anchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone)]
pub struct Appearance {
    pub font_family: String,
    pub font_size: f64,
    pub color_up: (f64, f64, f64),
    pub color_down: (f64, f64, f64),
    pub color_neutral: (f64, f64, f64),
    pub icon_size: u32,
}

#[derive(Debug, Clone)]
pub struct Animation {
    pub scroll_speed: f64,
    pub fps: u32,
}

#[derive(Debug, Clone)]
pub struct CoinConfig {
    pub symbol: String,
    pub name: String,
    pub icon: String,
}

/// TOML file structure for deserialization.
#[derive(Deserialize, Default)]
#[serde(default)]
struct ConfigFile {
    monitor: Option<String>,
    position: PositionFile,
    appearance: AppearanceFile,
    animation: AnimationFile,
    coins: Option<Vec<CoinFile>>,
}

#[derive(Deserialize)]
#[serde(default)]
struct PositionFile {
    anchor: String,
    margin_top: i32,
    margin_right: i32,
    margin_bottom: i32,
    margin_left: i32,
    width: i32,
    height: i32,
}

impl Default for PositionFile {
    fn default() -> Self {
        Self {
            anchor: "top-right".to_string(),
            margin_top: 0,
            margin_right: 200,
            margin_bottom: 0,
            margin_left: 0,
            width: 320,
            height: 26,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct AppearanceFile {
    font_family: String,
    font_size: f64,
    color_up: String,
    color_down: String,
    color_neutral: String,
    icon_size: u32,
}

impl Default for AppearanceFile {
    fn default() -> Self {
        Self {
            font_family: "monospace".to_string(),
            font_size: 11.0,
            color_up: "#4ec970".to_string(),
            color_down: "#e05555".to_string(),
            color_neutral: "#888888".to_string(),
            icon_size: 16,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct AnimationFile {
    scroll_speed: f64,
    fps: u32,
}

impl Default for AnimationFile {
    fn default() -> Self {
        Self {
            scroll_speed: 30.0,
            fps: 60,
        }
    }
}

#[derive(Deserialize, Clone)]
struct CoinFile {
    symbol: String,
    name: String,
    icon: String,
}

impl Config {
    /// Load configuration from file or use defaults.
    pub fn load() -> Self {
        let config_path = Self::config_path();

        let file_config = if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(contents) => toml::from_str(&contents).unwrap_or_else(|e| {
                    eprintln!("Warning: Failed to parse config: {}", e);
                    ConfigFile::default()
                }),
                Err(e) => {
                    eprintln!("Warning: Failed to read config: {}", e);
                    ConfigFile::default()
                }
            }
        } else {
            ConfigFile::default()
        };

        Self::from_file(file_config)
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("waybar-crypto-ticker/config.toml")
    }

    fn from_file(f: ConfigFile) -> Self {
        let coins = f.coins.unwrap_or_else(Self::default_coins);

        Self {
            monitor: f.monitor,
            position: Position {
                anchor: match f.position.anchor.as_str() {
                    "top-left" => Anchor::TopLeft,
                    "top-right" => Anchor::TopRight,
                    "bottom-left" => Anchor::BottomLeft,
                    "bottom-right" => Anchor::BottomRight,
                    _ => Anchor::TopRight,
                },
                margin_top: f.position.margin_top,
                margin_right: f.position.margin_right,
                margin_bottom: f.position.margin_bottom,
                margin_left: f.position.margin_left,
                width: f.position.width,
                height: f.position.height,
            },
            appearance: Appearance {
                font_family: f.appearance.font_family,
                font_size: f.appearance.font_size,
                color_up: parse_hex_color(&f.appearance.color_up).unwrap_or((0.31, 0.79, 0.44)),
                color_down: parse_hex_color(&f.appearance.color_down).unwrap_or((0.88, 0.33, 0.33)),
                color_neutral: parse_hex_color(&f.appearance.color_neutral).unwrap_or((0.53, 0.53, 0.53)),
                icon_size: f.appearance.icon_size,
            },
            animation: Animation {
                scroll_speed: f.animation.scroll_speed,
                fps: f.animation.fps.max(1).min(120),
            },
            coins: coins.into_iter().map(|c| CoinConfig {
                symbol: c.symbol,
                name: c.name,
                icon: c.icon,
            }).collect(),
        }
    }

    fn default_coins() -> Vec<CoinFile> {
        vec![
            CoinFile { symbol: "BTC/USD".into(), name: "BTC".into(), icon: "btc.svg".into() },
            CoinFile { symbol: "ETH/USD".into(), name: "ETH".into(), icon: "eth.svg".into() },
            CoinFile { symbol: "SOL/USD".into(), name: "SOL".into(), icon: "sol.svg".into() },
            CoinFile { symbol: "ADA/USD".into(), name: "ADA".into(), icon: "ada.svg".into() },
            CoinFile { symbol: "XRP/USD".into(), name: "XRP".into(), icon: "xrp.svg".into() },
        ]
    }

    /// Get the icons directory path.
    pub fn icons_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("waybar-crypto-ticker/icons")
    }
}

/// Parse a hex color string like "#4ec970" into RGB floats (0.0-1.0).
fn parse_hex_color(hex: &str) -> Option<(f64, f64, f64)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;

    Some((r, g, b))
}
