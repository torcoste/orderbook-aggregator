use serde::Deserialize;
use tokio::sync::mpsc;
use tungstenite::{connect, Message};

use super::output_data_format::ExchangeOrderbookData;

// Maximal age of connection is 90 days
// TODO: reconnect if connection is lost

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

const BITSTAMP_API_URL: &str = "wss://ws.bitstamp.net";
const BITSTAMP_DEPTH_LIMIT: u16 = 100;
const BITSTAMP_EVENT_SUBSCRIBE: &str = "bts:subscribe";
const BITSTAMP_EVENT_UNSUBSCRIBE: &str = "bts:unsubscribe";
const BITSTAMP_ORDERBOOK_CHANNEL_PREFIX: &str = "order_book_";

pub fn spawn_thread(
    symbol: String,
    depth: u16,
    tx: mpsc::Sender<ExchangeOrderbookData>,
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

    tokio::spawn(async move {
        let channel = format!("{}{}", BITSTAMP_ORDERBOOK_CHANNEL_PREFIX, symbol);

        let (mut socket, _) = connect(BITSTAMP_API_URL).expect("Can't connect to Bitstamp API");

        let subscribe_message = format!(
            r#"{{"event": "{}", "data": {{"channel": "{}"}}}}"#,
            BITSTAMP_EVENT_SUBSCRIBE, channel
        );

        socket
            .write_message(Message::Text(subscribe_message.clone()))
            .expect("Error writing subscribe message");

        loop {
            let message = socket
                .read_message()
                .expect("Error reading message from Bitstamp API");

            // TODO: implement heartbeat

            let data = message.into_data();
            let response: BitstampApiIncomingMessage = serde_json::from_slice(&data).unwrap();

            if response.event == "bts:request_reconnect" {
                println!("Bitstamp API requested reconnect. Reconnecting...");
                socket
                    .write_message(Message::Text(subscribe_message.clone()))
                    .expect("Error resubscribing to Bitstamp API");
                continue;
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
                if tx.is_closed() {
                    println!("Channel is closed. Unsubscribing from Bitstamp API...");
                    let unsubscribe_message = format!(
                        r#"{{"event":"{}","data":{{"channel":"{}"}}}}"#,
                        BITSTAMP_EVENT_UNSUBSCRIBE, channel
                    );
                    socket
                        .write_message(Message::Text(unsubscribe_message))
                        .expect("Error unsubscribing from Bitstamp API");
                    socket
                        .close(None)
                        .expect("Error closing connection to Bitstamp API");
                    println!("Connection to Bitstamp API closed");
                    break;
                }

                let orderbook_data_message: BitstampApiOrderBookDataMessage =
                    serde_json::from_slice(&data).unwrap();

                let mut orderbook_data = orderbook_data_message.data;
                if depth < BITSTAMP_DEPTH_LIMIT {
                    // It will reduce further processing time
                    orderbook_data.trim(depth);
                }

                let orderbook_data = ExchangeOrderbookData::from(orderbook_data);

                tx.send(orderbook_data)
                    .await
                    .expect("Failed to send orderbook data from Bitstamp API");
            }
        }
    })
}
