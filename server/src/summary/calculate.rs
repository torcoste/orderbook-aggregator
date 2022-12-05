use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::api::orderbook::{Level, Summary};
use crate::data_sources::output_data_format::ExchangeOrderbookData;

pub fn calculate_summary(
    orderbook_data: HashMap<String, ExchangeOrderbookData>,
    depth: u16,
    data_lifetime_ms: u64,
) -> Option<Summary> {
    let mut bids: Vec<Level> = Vec::new();
    let mut asks: Vec<Level> = Vec::new();

    for (exchange, orderbook) in orderbook_data.iter() {
        let current_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get current time")
            .as_millis();
        let data_age = current_timestamp as u64 - orderbook.timestamp;

        // println!("[DEBUG] {} data age: {}; ", exchange, data_age);

        if data_age > data_lifetime_ms {
            // Data is too old, skip it
            continue;
        }

        let mut exchange_bids: Vec<Level> = orderbook
            .bids
            .iter()
            .map(|(price, amount)| Level {
                exchange: exchange.to_string(),
                price: *price,
                amount: *amount,
            })
            .collect();
        bids.append(&mut exchange_bids);

        let mut exchange_asks: Vec<Level> = orderbook
            .asks
            .iter()
            .map(|(price, amount)| Level {
                exchange: exchange.to_string(),
                price: *price,
                amount: *amount,
            })
            .collect();
        asks.append(&mut exchange_asks);
    }

    // sort bids and asks by price and amount
    bids.sort_by(|a, b| {
        b.price
            .partial_cmp(&a.price)
            .expect("Failed to compare bid prices")
            .then(
                b.amount
                    .partial_cmp(&a.amount)
                    .expect("Failed to compare bid amounts"),
            )
    });
    asks.sort_by(|a, b| {
        a.price
            .partial_cmp(&b.price)
            .expect("Failed to compare ask prices")
            .then(
                b.amount
                    .partial_cmp(&a.amount)
                    .expect("Failed to compare ask amounts"),
            )
    });

    // Select only first `depth` levels
    {
        if bids.len() > depth as usize {
            bids.truncate(depth as usize);
        }

        if asks.len() > depth as usize {
            asks.truncate(depth as usize);
        }
    }

    if asks.is_empty() && bids.is_empty() {
        return None;
    }

    let spread = asks[0].price - bids[0].price;

    Some(Summary { spread, bids, asks })
}
