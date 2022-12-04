use std::error::Error;

use tonic::{transport::Channel, Request};

use orderbook::orderbook_aggregator_client::OrderbookAggregatorClient;
use orderbook::Empty;

pub mod orderbook {
    tonic::include_proto!("orderbook"); // The string specified here must match the proto package name
}

mod print_summary_table;
use print_summary_table::print_summary_as_table;

async fn print_summaries(
    client: &mut OrderbookAggregatorClient<Channel>,
) -> Result<(), Box<dyn Error>> {
    let mut stream = client
        .book_summary(Request::new(Empty {}))
        .await?
        .into_inner();

    loop {
        let message_result = stream.message().await;

        if let Ok(message) = message_result {
            if let Some(message) = message {
                print_summary_as_table(message);
            }
        } else {
            eprint!("Error: {:?}", message_result);
            break;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = env!("PORT");
    let url = format!("http://[::1]:{}", port);
    let mut client = OrderbookAggregatorClient::connect(url).await?;

    print_summaries(&mut client).await?;

    Ok(())
}
