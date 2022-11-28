use tokio::sync::mpsc::{self, Receiver};

// Unified output data format
pub mod output_data_format;
use output_data_format::ExchangeOrderbookData;

// Exchanges
mod binance;

pub fn get_data_rx(symbol: String, deepth: u32) -> Receiver<ExchangeOrderbookData> {
    let (tx, rx) = mpsc::channel::<ExchangeOrderbookData>(1);

    binance::spawn_thread(symbol, deepth, tx);

    rx
}
