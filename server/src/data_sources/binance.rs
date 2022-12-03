use serde::Deserialize;
use tokio::sync::mpsc;
use tungstenite::{connect, Message};
use url::Url;

use crate::helpers::get_env_var_or_default;
use super::output_data_format::ExchangeOrderbookData;

// Maximal age of connection is 24 hours
// TODO: reconnect if connection is lost

#[derive(Deserialize, Debug)]
pub struct BinanceApiOrderBookMessage {
    // lastUpdateId: u64,
    pub bids: Vec<(String, String)>,
    pub asks: Vec<(String, String)>,
}

impl BinanceApiOrderBookMessage {
    pub fn trim(&mut self, depth: u16) {
        self.bids.truncate(depth as usize);
        self.asks.truncate(depth as usize);
    }
}

#[derive(Deserialize, Debug)]
struct BinanceApiError {
    code: i32,
    msg: String,
}

#[derive(Deserialize, Debug)]
struct BinanceApiErrorMessage {
    error: BinanceApiError,
}

const BINANCE_API_BASE_URL: &str = "wss://stream.binance.com:9443/ws";
const BINANCE_SUPPORTED_DEPTH_LIMITS: [u16; 3] = [5, 10, 20];

pub fn spawn_thread(
    symbol: String,
    depth: u16,
    tx: mpsc::Sender<ExchangeOrderbookData>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let effective_depth = {
            BINANCE_SUPPORTED_DEPTH_LIMITS
                .iter()
                .find(|&&limit| limit >= depth)
                .unwrap_or_else(|| {
                    println!(
                        "[WARNING] Binance API supports only depth <= {}. Only first {} levels will be used.",
                        BINANCE_SUPPORTED_DEPTH_LIMITS[2], BINANCE_SUPPORTED_DEPTH_LIMITS[2]
                    );

                    &BINANCE_SUPPORTED_DEPTH_LIMITS[2]
                }).to_owned()

            // "Unlimited" depth support may be implemented with more complex logic
            // Docs: https://github.com/binance/binance-spot-api-docs/blob/master/web-socket-streams.md#how-to-manage-a-local-order-book-correctly
            // But it's not the goal of this app.
        };

        let base_url = get_env_var_or_default("BINANCE_API_BASE_URL", BINANCE_API_BASE_URL.to_string());
        let url = format!("{}/{}@depth{}@100ms", base_url, symbol, depth);
        let url = Url::parse(&url).expect("Failed to parse Binance API URL");

        let (mut socket, _) = connect(url).expect("Can't connect to Binance API");

        loop {
            let message = socket
                .read_message()
                .expect("Error reading message from Binance API");

            if message.is_ping() {
                socket
                    .write_message(Message::Pong(message.into_data()))
                    .unwrap_or_else(|e| {
                        println!("Error sending PONG message: {}", e);
                    });
                continue;
            }

            let data = message.into_data();

            match serde_json::from_slice::<BinanceApiOrderBookMessage>(&data) {
                Ok(orderbook) => {
                    if tx.is_closed() {
                        println!("Channel is closed. Unsubscribing from Binance API...");
                        socket
                            .close(None)
                            .expect("Error closing connection to Binance API");
                        println!("Connection to Binance API closed.");
                        break;
                    }

                    let mut orderbook = orderbook;
                    if depth < effective_depth {
                        orderbook.trim(depth);
                    }

                    let orderbook_data = ExchangeOrderbookData::from(orderbook);

                    tx.send(orderbook_data)
                        .await
                        .expect("Failed to send orderbook data from Binance API");
                }
                Err(_) => match serde_json::from_slice::<BinanceApiErrorMessage>(&data) {
                    Ok(error_message) => {
                        println!(
                            "Error from Binance API: {} (code: {})",
                            error_message.error.msg, error_message.error.code
                        );
                    }
                    Err(error) => {
                        println!("Error parsing Binance API message: {}", error);
                    }
                },
            }
        }
    })
}
