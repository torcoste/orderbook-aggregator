use std::sync::Arc;

use tokio::sync::mpsc::{self, Sender};
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};

use orderbook::orderbook_aggregator_server::{OrderbookAggregator, OrderbookAggregatorServer};
use orderbook::{Empty, Level, Summary};

pub mod orderbook {
    tonic::include_proto!("orderbook");
}

struct OrderbookAggregatorService {
    clients: Arc<Mutex<Vec<Sender<Result<Summary, Status>>>>>,
}

#[tonic::async_trait]
impl OrderbookAggregator for OrderbookAggregatorService {
    type BookSummaryStream = ReceiverStream<Result<Summary, Status>>;

    async fn book_summary(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::BookSummaryStream>, Status> {
        let (tx, rx) = mpsc::channel(1);

        let mut clients = self.clients.lock().await;
        clients.push(tx.clone());
        println!("New client connected. {} clients connected", clients.len());
        drop(clients);

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:10000".parse()?;

    let clients: Arc<Mutex<Vec<Sender<Result<Summary, Status>>>>> = Arc::new(Mutex::new(vec![]));

    let _demo_server_thread = {
        let clients = clients.clone();
        tokio::spawn(async move {
            let mut i = 0;
            let clients = clients.clone();

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
                println!("Current spread: {}", spread);

                let mut clients = clients.lock().await;

                let mut client_index = 0;

                while client_index < clients.len() {
                    let client = &clients[client_index];

                    if client.is_closed() {
                        clients.remove(client_index);

                        println!(
                            "Client #{} disconnected. Clients left: {}",
                            client_index,
                            clients.len()
                        );

                        continue;
                    }

                    client.send(Ok(summary.clone())).await.expect(
                        format!("Failed to send summary to client #{}", client_index).as_str(),
                    );
                    println!("Sent summary to client #{}", client_index);

                    client_index += 1;
                }

                i += 1;
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        })
    };

    let orderbook_aggregator = OrderbookAggregatorService { clients };

    let svc = OrderbookAggregatorServer::new(orderbook_aggregator);

    Server::builder().add_service(svc).serve(addr).await?;

    Ok(())
}
