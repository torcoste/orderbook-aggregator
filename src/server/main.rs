mod api;
mod data_sources;
mod summary;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let symbol = "ethbtc".to_string();
    let depth: u16 = 20;
    let data_lifetime_ms = 2000; // 2 seconds

    let data_rx = data_sources::get_data_rx(symbol, depth);
    let summary_rx = summary::get_summary_rx(data_rx, depth, data_lifetime_ms);

    api::serve(summary_rx).await?;

    Ok(())
}
