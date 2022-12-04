use serde::Deserialize;
use tungstenite::{connect, Message};
use url::Url;

use super::output_data_format::ExchangeOrderbookData;
use crate::helpers::get_env_var_or_default;

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

// Maximal age of connection is 24 hours
const BINANCE_CONNECTION_AGE_LIMIT_SECONDS: u64 = 24 * 60 * 60;
const BINANCE_RECONNECTION_FREQUENCY_SECONDS: u64 = BINANCE_CONNECTION_AGE_LIMIT_SECONDS - 60;

pub fn spawn_thread(
    symbol: String,
    depth: u16,
    tx: flume::Sender<ExchangeOrderbookData>,
) -> tokio::task::JoinHandle<()> {
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

    tokio::spawn(async move {
        let mut should_reconnect = true;

        while should_reconnect {
            let (mut socket, _) = connect(url.clone()).expect("Can't connect to Binance API");

            let connection_time = std::time::Instant::now();

            loop {
                if !socket.can_read() {
                    println!("Binance API connection is closed by server. Reconnecting...");
                    should_reconnect = true;
                    break;
                }

                if connection_time.elapsed().as_secs() > BINANCE_RECONNECTION_FREQUENCY_SECONDS {
                    println!("Binance API connection is too old. Reconnecting...");
                    should_reconnect = true;
                    break;
                }

                let message = socket
                    .read_message()
                    .expect("Error reading message from Binance API");

                if message.is_ping() {
                    match socket.write_message(Message::Pong(message.into_data())) {
                        Ok(_) => {
                            continue;
                        }
                        Err(error) => {
                            println!(
                                "Error sending PONG message to Binance API: {}. Reconnecting...",
                                error
                            );
                            // if we don't send PONG, the connection will be closed by server 10 minutes later PING
                            should_reconnect = true;
                            break;
                        }
                    }
                }

                let data = message.into_data();

                match serde_json::from_slice::<BinanceApiOrderBookMessage>(&data) {
                    Ok(orderbook) => {
                        if tx.is_disconnected() {
                            println!("Channel is closed. Unsubscribing from Binance API...");
                            should_reconnect = false;
                            break;
                        }

                        let mut orderbook = orderbook;
                        if depth < effective_depth {
                            orderbook.trim(depth);
                        }

                        let orderbook_data = ExchangeOrderbookData::from(orderbook);

                        tx.send_async(orderbook_data)
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

            socket
                .close(None)
                .expect("Failed to close Binance API connection");
        }
    })
}
