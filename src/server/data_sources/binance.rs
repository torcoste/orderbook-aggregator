use serde::Deserialize;
use tungstenite::{connect, Message};
use url::Url;

// Maximal age of connection is 24 hours
// TODO: reconnect if connection is lost

#[derive(Deserialize, Debug)]
pub struct BinanceApiOrderBookMessage {
    // lastUpdateId: u64,
    pub bids: Vec<(String, String)>,
    pub asks: Vec<(String, String)>,
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

use super::output_data_format::ExchangeOrderbookData;
use tokio::sync::mpsc;

pub fn spawn_thread(
    symbol: String,
    depth: u32,
    tx: mpsc::Sender<ExchangeOrderbookData>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let url = format!(
            "wss://stream.binance.com:9443/ws/{}@depth{}@100ms",
            symbol, depth
        );
        let url = Url::parse(&url).expect("Failed to parse URL");

        let (mut socket, _) = connect(url).expect("Can't connect to Binance API");

        loop {
            let message = socket.read_message().expect("Error reading message");

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
                        break;
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
