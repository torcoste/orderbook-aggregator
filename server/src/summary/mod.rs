use std::collections::HashMap;

use crate::api::orderbook::Summary;
use crate::data_sources::output_data_format::ExchangeOrderbookData;

mod calculate;
use calculate::calculate_summary;

pub fn get_summary_rx(
    data_rx: flume::Receiver<ExchangeOrderbookData>,
    depth: u16,
    data_lifetime_ms: u64,
) -> flume::Receiver<Summary> {
    let (tx, rx) = flume::bounded::<Summary>(10);

    tokio::spawn(async move {
        let mut orderbook_data: HashMap<String, ExchangeOrderbookData> = HashMap::new();

        while !data_rx.is_disconnected() {
            let data_rx_drain = data_rx.drain();
            let data = data_rx_drain.last();

            if let Some(data) = data {
                orderbook_data.insert(data.exchange.clone(), data);

                let summary = calculate_summary(orderbook_data.clone(), depth, data_lifetime_ms);

                if let Some(summary) = summary {
                    // println!("Summary calculated. Spread: {}", summary.spread);
                    tx.send_async(summary)
                        .await
                        .expect("Failed to send summary");
                } else {
                    // if all data is too old, or there is not enough data
                    println!("Failed to calculate summary");
                }
            }
        }

        println!("Summary thread finished");
    });

    rx
}
