use std::sync::Arc;

use tokio::sync::mpsc::{self, Sender};
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};

use orderbook::orderbook_aggregator_server::{OrderbookAggregator, OrderbookAggregatorServer};
use orderbook::{Empty, Summary};

pub mod orderbook {
    tonic::include_proto!("orderbook");
}

mod data_sources;
mod summary;

type ClientSender = Sender<Result<Summary, Status>>;

struct OrderbookAggregatorService {
    clients: Arc<Mutex<Vec<ClientSender>>>,
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
        println!(
            "New client connected. Total of {} clients connected",
            clients.len()
        );
        drop(clients);

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:10000".parse()?;
    let symbol = "ethbtc".to_string();
    let depth = 20;
    let data_lifetime_ms = 2000; // 2 seconds

    let clients: Arc<Mutex<Vec<ClientSender>>> = Arc::new(Mutex::new(vec![]));
    let data_rx = data_sources::get_data_rx(symbol, depth);
    let mut summary_rx = summary::get_summary_rx(data_rx, data_lifetime_ms);

    let _main_server_thread = {
        let clients = clients.clone();
        tokio::spawn(async move {
            let clients = clients.clone();

            while let Some(summary) = summary_rx.recv().await {
                println!("Current spread: {}", summary.spread);

                let mut clients = clients.lock().await;
                let mut clients_to_remove = vec![];

                for (i, client) in clients.iter().enumerate() {
                    if client.is_closed() {
                        clients_to_remove.push(i);
                        continue;
                    }

                    match client.send(Ok(summary.clone())).await {
                        Ok(_) => (),
                        Err(e) => {
                            println!("Error sending summary to client: {}", e);
                            clients_to_remove.push(i);
                        }
                    }
                }

                if !clients_to_remove.is_empty() {
                    for i in clients_to_remove.iter().rev() {
                        clients.remove(*i);
                    }
                    println!(
                        "{} clients disconnected. Clients left: {}",
                        clients_to_remove.len(),
                        clients.len()
                    );
                }
            }
        })
    };

    let orderbook_aggregator = OrderbookAggregatorService { clients };

    let svc = OrderbookAggregatorServer::new(orderbook_aggregator);

    Server::builder().add_service(svc).serve(addr).await?;

    Ok(())
}
