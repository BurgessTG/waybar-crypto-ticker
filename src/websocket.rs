//! WebSocket connection to Kraken for real-time price updates.

use crate::config::Config;
use crate::ticker::TickerState;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const KRAKEN_WS: &str = "wss://ws.kraken.com/v2";
const KRAKEN_REST: &str = "https://api.kraken.com/0/public/Ticker";

/// Kraken REST API uses different symbol names than WebSocket.
fn ws_to_rest_symbol(ws_symbol: &str) -> String {
    match ws_symbol {
        "BTC/USD" => "XXBTZUSD".to_string(),
        "ETH/USD" => "XETHZUSD".to_string(),
        "XRP/USD" => "XXRPZUSD".to_string(),
        other => other.replace("/", "").to_uppercase(),
    }
}

#[derive(Serialize)]
struct SubscribeMessage {
    method: String,
    params: SubscribeParams,
}

#[derive(Serialize)]
struct SubscribeParams {
    channel: String,
    symbol: Vec<String>,
    snapshot: bool,
}

#[derive(Deserialize)]
struct WsMessage {
    channel: Option<String>,
    data: Option<Vec<TickerData>>,
}

#[derive(Deserialize)]
struct TickerData {
    symbol: Option<String>,
    last: Option<f64>,
}

#[derive(Deserialize)]
struct RestResponse {
    result: Option<HashMap<String, RestTicker>>,
}

#[derive(Deserialize)]
struct RestTicker {
    o: Option<String>,
}

/// Fetch 24h open prices from REST API for percentage calculation.
fn fetch_open_prices(state: &Arc<Mutex<TickerState>>, config: &Config) {
    let pairs: Vec<String> = config.coins.iter()
        .map(|c| ws_to_rest_symbol(&c.symbol))
        .collect();

    let url = format!("{}?pair={}", KRAKEN_REST, pairs.join(","));

    if let Ok(resp) = reqwest::blocking::get(&url) {
        if let Ok(data) = resp.json::<RestResponse>() {
            if let Some(result) = data.result {
                if let Ok(mut state) = state.lock() {
                    for coin in &config.coins {
                        let rest_sym = ws_to_rest_symbol(&coin.symbol);
                        if let Some(ticker) = result.get(&rest_sym) {
                            if let Some(open_str) = &ticker.o {
                                if let Ok(open) = open_str.parse::<f64>() {
                                    state.set_open_price(&coin.symbol, open);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Main WebSocket loop with automatic reconnection.
#[tokio::main]
pub async fn run(state: &Arc<Mutex<TickerState>>, config: &Config) {
    fetch_open_prices(state, config);

    let symbols: Vec<String> = config.coins.iter()
        .map(|c| c.symbol.clone())
        .collect();

    loop {
        if let Err(e) = connect_and_stream(state, &symbols).await {
            eprintln!("WebSocket error: {:?}", e);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn connect_and_stream(
    state: &Arc<Mutex<TickerState>>,
    symbols: &[String],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (ws_stream, _) = connect_async(KRAKEN_WS).await?;
    let (mut write, mut read) = ws_stream.split();

    let subscribe = SubscribeMessage {
        method: "subscribe".to_string(),
        params: SubscribeParams {
            channel: "ticker".to_string(),
            symbol: symbols.to_vec(),
            snapshot: true,
        },
    };

    write.send(Message::Text(serde_json::to_string(&subscribe)?)).await?;

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                    if ws_msg.channel.as_deref() == Some("ticker") {
                        if let Some(data) = ws_msg.data {
                            if let Ok(mut state) = state.lock() {
                                for ticker in data {
                                    if let (Some(symbol), Some(price)) = (ticker.symbol, ticker.last) {
                                        state.update_price(&symbol, price);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                let _ = write.send(Message::Pong(data)).await;
            }
            Err(e) => return Err(Box::new(e)),
            _ => {}
        }
    }

    Ok(())
}
