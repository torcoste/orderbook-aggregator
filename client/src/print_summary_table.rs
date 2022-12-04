use std::cmp::min;

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, Color, Table};

use crate::orderbook::Summary;

fn clear_terminal() {
    println!("{}c", 27 as char);
}

pub fn print_summary_as_table(summary: Summary) {
    clear_terminal();

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Bid Exchange"),
            Cell::new("Bid Amount"),
            Cell::new("Bid Price").fg(Color::Green),
            Cell::new("Ask Price").fg(Color::Green),
            Cell::new("Ask Amount"),
            Cell::new("Ask Exchange"),
        ]);

    let length = min(summary.bids.len(), summary.asks.len());

    for i in 0..length {
        let bid = summary.bids[i].clone();

        let ask = summary.asks[i].clone();

        table.add_row(vec![
            bid.exchange,
            bid.amount.to_string(),
            bid.price.to_string(),
            ask.price.to_string(),
            ask.amount.to_string(),
            ask.exchange,
        ]);
    }

    println!("Current spread: {}", summary.spread);
    println!("{}", table);
}
