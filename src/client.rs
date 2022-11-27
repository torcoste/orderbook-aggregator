use std::error::Error;
use tonic::{transport::Channel, Request};

use orderbook::orderbook_aggregator_client::OrderbookAggregatorClient;
use orderbook::{Empty};

pub mod orderbook {
    tonic::include_proto!("orderbook"); // The string specified here must match the proto package name
}

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
                println!("Received summary. Spread: {}", message.spread);
                println!("Full summary: {:?}", message);
                println!("-----------------------------------")
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
    let mut client = OrderbookAggregatorClient::connect("http://[::1]:10000").await?;

    print_summaries(&mut client).await?;

    Ok(())
}
