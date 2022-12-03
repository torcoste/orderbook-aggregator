use tokio::sync::mpsc::{self, Receiver};

// Unified output data format
pub mod output_data_format;
use output_data_format::ExchangeOrderbookData;

// Exchanges
mod binance;
mod bitstamp;

pub fn get_data_rx(symbol: String, depth: u16) -> Receiver<ExchangeOrderbookData> {
    let (tx, rx) = mpsc::channel::<ExchangeOrderbookData>(1);

    binance::spawn_thread(symbol.clone(), depth, tx.clone());

    bitstamp::spawn_thread(symbol, depth, tx);

    rx
}
