use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};

use orderbook::orderbook_aggregator_server::{OrderbookAggregator, OrderbookAggregatorServer};
use orderbook::{Empty, Level, Summary};

pub mod orderbook {
    tonic::include_proto!("orderbook");
}

struct OrderbookAggregatorService {
    // TODO:
    // here we can store stream reciever for data from the aggregator
    // then we will read it from book_summary and send values to the client
}

fn demo_summary_generator(tx: mpsc::Sender<Result<Summary, Status>>) {
    tokio::spawn(async move {
        let mut i = 0;

        loop {
            let spread = i as f64;
            let summary = Summary {
                spread: i as f64,
                bids: vec![Level {
                    exchange: "binance".to_string(),
                    price: 1.0 - (spread / 2.0),
                    amount: 1.0,
                }],
                asks: vec![Level {
                    exchange: "bitstamp".to_string(),
                    price: 1.0 + (spread / 2.0),
                    amount: 1.0,
                }],
            };

            tx.send(Ok(summary)).await.unwrap();
            println!("Sent summary. Spread: {}", spread);

            i += 1;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });
}

#[tonic::async_trait]
impl OrderbookAggregator for OrderbookAggregatorService {
    type BookSummaryStream = ReceiverStream<Result<Summary, Status>>;

    async fn book_summary(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::BookSummaryStream>, Status> {
        let (tx, rx) = mpsc::channel(1);

        demo_summary_generator(tx);

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:10000".parse().unwrap();

    let orderbook_aggregator = OrderbookAggregatorService {};

    let svc = OrderbookAggregatorServer::new(orderbook_aggregator);

    Server::builder().add_service(svc).serve(addr).await?;

    Ok(())
}
