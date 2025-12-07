//! Ticker state and display segment management.

use crate::config::{CoinConfig, Config};
use std::collections::HashMap;

/// Price movement direction for coloring.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Neutral,
}

/// A rendered segment of the ticker display.
#[derive(Clone)]
pub struct Segment {
    pub text: String,
    pub direction: Direction,
    pub icon: Option<String>,
}

/// Price data for a single coin.
#[derive(Clone)]
pub struct CoinData {
    pub price: f64,
    pub open_24h: f64,
}

/// Manages price state and generates display segments.
pub struct TickerState {
    prices: HashMap<String, CoinData>,
    coins: Vec<CoinConfig>,
    pub segments: Vec<Segment>,
}

const SEPARATOR: &str = "     ·     ";

impl TickerState {
    pub fn new(config: &Config) -> Self {
        Self {
            prices: HashMap::new(),
            coins: config.coins.clone(),
            segments: Vec::new(),
        }
    }

    /// Update the current price for a symbol.
    pub fn update_price(&mut self, symbol: &str, price: f64) {
        if let Some(data) = self.prices.get_mut(symbol) {
            data.price = price;
        } else {
            self.prices.insert(symbol.to_string(), CoinData {
                price,
                open_24h: price,
            });
        }
        self.rebuild_segments();
    }

    /// Set the 24h open price for calculating change percentage.
    pub fn set_open_price(&mut self, symbol: &str, open: f64) {
        if let Some(data) = self.prices.get_mut(symbol) {
            data.open_24h = open;
            self.rebuild_segments();
        }
    }

    fn get_change(&self, symbol: &str) -> (String, Direction) {
        if let Some(data) = self.prices.get(symbol) {
            if data.open_24h > 0.0 {
                let change = ((data.price - data.open_24h) / data.open_24h) * 100.0;
                if change > 0.01 {
                    return (format!("+{:.1}%▲", change), Direction::Up);
                } else if change < -0.01 {
                    return (format!("{:.1}%▼", change), Direction::Down);
                } else {
                    return (format!("{:.1}%", change), Direction::Neutral);
                }
            }
        }
        ("--".to_string(), Direction::Neutral)
    }

    fn format_price(price: f64) -> String {
        if price >= 1000.0 {
            format!("${:.0}", price)
        } else if price >= 1.0 {
            format!("${:.2}", price)
        } else if price >= 0.01 {
            format!("${:.4}", price)
        } else {
            format!("${:.6}", price)
        }
    }

    fn rebuild_segments(&mut self) {
        self.segments.clear();

        let active_count = self.coins.iter()
            .filter(|c| self.prices.contains_key(&c.symbol))
            .count();

        for coin in &self.coins {
            if let Some(data) = self.prices.get(&coin.symbol) {
                let (change_str, direction) = self.get_change(&coin.symbol);
                let price_str = Self::format_price(data.price);

                self.segments.push(Segment {
                    text: format!("{} {}", price_str, change_str),
                    direction,
                    icon: Some(coin.icon.clone()),
                });

                if active_count > 1 {
                    self.segments.push(Segment {
                        text: SEPARATOR.to_string(),
                        direction: Direction::Neutral,
                        icon: None,
                    });
                }
            }
        }
    }
}
