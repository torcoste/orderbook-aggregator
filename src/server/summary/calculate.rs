use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::data_sources::output_data_format::ExchangeOrderbookData;
use crate::orderbook::{Level, Summary};

pub fn calculate_summary(
    orderbook_data: HashMap<String, ExchangeOrderbookData>,
    data_lifetime_ms: u64,
) -> Option<Summary> {
    let mut bids: Vec<Level> = Vec::new();
    let mut asks: Vec<Level> = Vec::new();

    for (exchange, orderbook) in orderbook_data.iter() {
        let data_age = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
            - orderbook.timestamp;

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
        a.price
            .partial_cmp(&b.price)
            .unwrap()
            .then(a.amount.partial_cmp(&b.amount).unwrap())
    });
    asks.sort_by(|a, b| {
        a.price
            .partial_cmp(&b.price)
            .unwrap()
            .then(a.amount.partial_cmp(&b.amount).unwrap())
    });

    if asks.is_empty() && bids.is_empty() {
        return None;
    }

    let spread = asks[0].price - bids[0].price;

    Some(Summary { spread, bids, asks })
}
