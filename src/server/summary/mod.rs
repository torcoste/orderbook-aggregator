use std::collections::HashMap;

use tokio::sync::mpsc::{self, Receiver};

use crate::api::orderbook::Summary;
use crate::data_sources::output_data_format::ExchangeOrderbookData;

mod calculate;
use calculate::calculate_summary;

pub fn get_summary_rx(
    mut data_rx: Receiver<ExchangeOrderbookData>,
    depth: u16,
    data_lifetime_ms: u64,
) -> Receiver<Summary> {
    let (tx, rx) = mpsc::channel::<Summary>(1);

    tokio::spawn(async move {
        let mut orderbook_data: HashMap<String, ExchangeOrderbookData> = HashMap::new();

        while let Some(data) = data_rx.recv().await {
            orderbook_data.insert(data.exchange.clone(), data);

            let summary = calculate_summary(orderbook_data.clone(), depth, data_lifetime_ms);

            if let Some(summary) = summary {
                println!("Summary calculated. Spread: {}", summary.spread);
                tx.send(summary).await.expect("Failed to send summary");
            } else {
                // if all data is too old, or there is not enough data
                println!("Failed to calculate summary");
            }
        }
    });

    rx
}
