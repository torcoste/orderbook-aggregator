mod api;
mod data_sources;
mod summary;

mod helpers;
use helpers::get_env_var_or_default;

const DEFAULT_SYMBOL: &str = "ethbtc";
const DEFAULT_DEPTH: u16 = 20;
const DEFAULT_DATA_LIFETIME_MS: u64 = 2000; // 2 seconds

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let symbol = get_env_var_or_default("SYMBOL", DEFAULT_SYMBOL.to_string());
    let depth: u16 = get_env_var_or_default("DEPTH", DEFAULT_DEPTH);
    let data_lifetime_ms = get_env_var_or_default("DATA_LIFETIME_MS", DEFAULT_DATA_LIFETIME_MS);

    let data_rx = data_sources::get_data_rx(symbol, depth);
    let summary_rx = summary::get_summary_rx(data_rx, depth, data_lifetime_ms);

    api::serve(summary_rx).await?;

    Ok(())
}
