use serde::Deserialize;
use tungstenite::{connect, Message};
use url::Url;

use super::output_data_format::ExchangeOrderbookData;
use crate::helpers::get_env_var_or_default;

#[derive(Deserialize, Debug)]
pub struct BitstampApiOrderBookData {
    /// timestamp in seconds
    pub timestamp: String,
    // microtimestamp: String,
    pub bids: Vec<(String, String)>,
    pub asks: Vec<(String, String)>,
}

impl BitstampApiOrderBookData {
    pub fn trim(&mut self, depth: u16) {
        self.bids.truncate(depth as usize);
        self.asks.truncate(depth as usize);
    }
}

#[derive(Deserialize, Debug)]
struct BitstampApiIncomingMessage {
    event: String,
    // it also contains `channel` field, but it's not used in this app.

    // it also has `data` field, but it's not always present and it's not always the same type
    // so we use `BitstampApiOrderBookDataMessage` or `BitstampApiErrorMessage` depending on `event` field value
}

#[derive(Deserialize, Debug)]
struct BitstampApiOrderBookDataMessage {
    data: BitstampApiOrderBookData,
}

#[derive(Deserialize, Debug)]
struct BitstampApiErrorData {
    code: Option<i32>,
    message: String,
}

#[derive(Deserialize, Debug)]
struct BitstampApiErrorMessage {
    data: BitstampApiErrorData,
}

const DEFAULT_BITSTAMP_API_URL: &str = "wss://ws.bitstamp.net";
const BITSTAMP_DEPTH_LIMIT: u16 = 100;
const BITSTAMP_EVENT_SUBSCRIBE: &str = "bts:subscribe";
const BITSTAMP_ORDERBOOK_CHANNEL_PREFIX: &str = "order_book_";

// Maximal age of connection is 90 days
const BITSTAMP_CONNECTION_AGE_LIMIT_SECONDS: u64 = 90 * 24 * 60 * 60;
const BITSTAMP_RECONNECTION_FREQUENCY_SECONDS: u64 = BITSTAMP_CONNECTION_AGE_LIMIT_SECONDS - 60;

pub fn spawn_thread(
    symbol: String,
    depth: u16,
    tx: flume::Sender<ExchangeOrderbookData>,
) -> tokio::task::JoinHandle<()> {
    if depth > BITSTAMP_DEPTH_LIMIT {
        println!(
            "[WARNING] Bitstamp API supports only depth <= {}. Only first {} levels will be used.",
            BITSTAMP_DEPTH_LIMIT, BITSTAMP_DEPTH_LIMIT
        );
        // "Unlimited" depth support may be implemented with requesting initial orderbook and then using `diff_order_book` channel
        // Docs: https://www.bitstamp.net/websocket/v2/
        // But it's not the goal of this app.
    }

    let url = get_env_var_or_default("BITSTAMP_API_URL", DEFAULT_BITSTAMP_API_URL.to_string());
    let url = Url::parse(&url).expect("Failed to parse Bitstamp API URL");

    let channel = format!("{}{}", BITSTAMP_ORDERBOOK_CHANNEL_PREFIX, symbol);
    let subscribe_message = format!(
        r#"{{"event": "{}", "data": {{"channel": "{}"}}}}"#,
        BITSTAMP_EVENT_SUBSCRIBE, channel
    );

    tokio::spawn(async move {
        let mut should_reconnect = true;

        while should_reconnect {
            let (mut socket, _) = connect(url.clone()).expect("Can't connect to Bitstamp API");

            socket
                .write_message(Message::Text(subscribe_message.clone()))
                .expect("Error writing subscribe message");

            let connection_time = std::time::Instant::now();

            loop {
                if !socket.can_read() {
                    println!("Bitstamp API connection is closed by server. Reconnecting...");
                    should_reconnect = true;
                    break;
                }

                if connection_time.elapsed().as_secs() > BITSTAMP_RECONNECTION_FREQUENCY_SECONDS {
                    println!("Bitstamp API connection is too old. Reconnecting...");
                    should_reconnect = true;
                    break;
                }

                let message = socket
                    .read_message()
                    .expect("Error reading message from Bitstamp API");

                if message.is_ping() {
                    match socket.write_message(Message::Pong(message.into_data())) {
                        Ok(_) => {
                            continue;
                        }
                        Err(error) => {
                            println!(
                                "Error sending PONG message to Bitstamp API: {}. Reconnecting...",
                                error
                            );
                            // if we don't send PONG, the connection will be closed by server immediately
                            should_reconnect = true;
                            break;
                        }
                    }
                }

                let data = message.into_data();
                let response: BitstampApiIncomingMessage = serde_json::from_slice(&data)
                    .expect("Error parsing BitstampApiIncomingMessage JSON");

                if response.event == "bts:request_reconnect" {
                    println!("Bitstamp API requested reconnect. Reconnecting...");
                    should_reconnect = true;
                    break;
                }

                if response.event == "bts:error" {
                    match serde_json::from_slice::<BitstampApiErrorMessage>(&data) {
                        Ok(bitstamp_error_message) => {
                            let data = bitstamp_error_message.data;
                            let message = data.message;
                            let code = data.code.unwrap_or(-1);
                            println!("Error from Bitstamp API: {} (code: {})", message, code);
                        }
                        Err(error) => {
                            println!("Error parsing Bitstamp API error message: {}", error);
                        }
                    }

                    continue;
                }

                // It would make sense to check if response.channel == channel, but we don't need it in this app, because we only subscribe to one channel
                if response.event == "data" {
                    if tx.is_disconnected() {
                        println!("Channel is closed. Unsubscribing from Bitstamp API...");
                        should_reconnect = false;
                        break;
                    }

                    let orderbook_data_message: BitstampApiOrderBookDataMessage =
                        serde_json::from_slice(&data)
                            .expect("Error parsing BitstampApiOrderBookDataMessage JSON");

                    let mut orderbook_data = orderbook_data_message.data;
                    if depth < BITSTAMP_DEPTH_LIMIT {
                        // It will reduce further processing time
                        orderbook_data.trim(depth);
                    }

                    let orderbook_data = ExchangeOrderbookData::from(orderbook_data);

                    tx.send_async(orderbook_data)
                        .await
                        .expect("Failed to send orderbook data from Bitstamp API");
                }
            }

            socket
                .close(None)
                .expect("Error closing connection to Bitstamp API");
        }
    })
}
