use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use super::binance::BinanceApiOrderBookMessage;
use super::bitstamp::BitstampApiOrderBookData;

/// Unified output data format
#[derive(Deserialize, Debug, Clone)]
pub struct ExchangeOrderbookData {
    pub exchange: String,
    pub asks: Vec<(f64, f64)>, // price, amount
    pub bids: Vec<(f64, f64)>, // price, amount
    pub timestamp: u64,
}

impl ExchangeOrderbookData {
    pub fn new(exchange: String, asks: Vec<(f64, f64)>, bids: Vec<(f64, f64)>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            exchange,
            asks,
            bids,
            timestamp,
        }
    }

    /// If exchanges API reeturns timestamp, we use it to calculate data age more accurately.
    /// Otherwise we use current time (see `new` method).
    pub fn new_with_timestamp(
        exchange: String,
        asks: Vec<(f64, f64)>,
        bids: Vec<(f64, f64)>,
        timestamp: u64,
    ) -> Self {
        Self {
            exchange,
            asks,
            bids,
            timestamp,
        }
    }
}

impl From<BinanceApiOrderBookMessage> for ExchangeOrderbookData {
    fn from(binance_orderbook_message: BinanceApiOrderBookMessage) -> Self {
        let exchange = "binance".to_string();

        let asks = parse_price_amount_tuples(&binance_orderbook_message.asks)
            .expect("Failed to parse Binance asks");

        let bids = parse_price_amount_tuples(&binance_orderbook_message.bids)
            .expect("Failed to parse Binance bids");

        Self::new(exchange, asks, bids)
    }
}

impl From<BitstampApiOrderBookData> for ExchangeOrderbookData {
    fn from(bitstamp_orderbook_message: BitstampApiOrderBookData) -> Self {
        let exchange = "bitstamp".to_string();

        let asks = parse_price_amount_tuples(&bitstamp_orderbook_message.asks)
            .expect("Failed to parse Bitstamp asks");

        let bids = parse_price_amount_tuples(&bitstamp_orderbook_message.bids)
            .expect("Failed to parse Bitstamp bids");

        let timestamp_in_seconds: u64 = bitstamp_orderbook_message.timestamp.parse().unwrap();
        let timestamp = timestamp_in_seconds * 1000;

        Self::new_with_timestamp(exchange, asks, bids, timestamp)
    }
}

fn parse_price_amount_tuples(
    vec: &Vec<(String, String)>,
) -> Result<Vec<(f64, f64)>, Box<dyn std::error::Error>> {
    let mut result = Vec::new();

    for tuple in vec {
        let price = tuple.0.parse::<f64>()?;
        let amount = tuple.1.parse::<f64>()?;
        result.push((price, amount));
    }

    Ok(result)
}
