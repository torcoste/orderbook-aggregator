use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use super::binance::BinanceApiOrderBookMessage;

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
}

impl From<BinanceApiOrderBookMessage> for ExchangeOrderbookData {
    fn from(binance_orderbook_message: BinanceApiOrderBookMessage) -> Self {
        let exchange = "binance".to_string();

        let asks = binance_orderbook_message
            .asks
            .iter()
            .map(|(price, amount)| {
                (
                    price.parse::<f64>().unwrap(),
                    amount.parse::<f64>().unwrap(),
                )
            })
            .collect();

        let bids = binance_orderbook_message
            .bids
            .iter()
            .map(|(price, amount)| {
                (
                    price.parse::<f64>().unwrap(),
                    amount.parse::<f64>().unwrap(),
                )
            })
            .collect();

        Self::new(exchange, asks, bids)
    }
}
