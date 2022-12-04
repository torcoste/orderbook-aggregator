// Unified output data format
pub mod output_data_format;
use output_data_format::ExchangeOrderbookData;

// Exchanges
mod binance;
mod bitstamp;

pub fn get_data_rx(symbol: String, depth: u16) -> flume::Receiver<ExchangeOrderbookData> {
    let (tx, rx) = flume::bounded::<ExchangeOrderbookData>(0);

    binance::spawn_thread(symbol.clone(), depth, tx.clone());

    bitstamp::spawn_thread(symbol, depth, tx);

    rx
}
