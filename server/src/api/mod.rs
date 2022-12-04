use std::sync::Arc;

use flume::{r#async::RecvStream, Receiver, Sender};
use tokio::sync::Mutex;
use tonic::{transport::Server, Request, Response, Status};

use orderbook::orderbook_aggregator_server::{OrderbookAggregator, OrderbookAggregatorServer};
use orderbook::{Empty, Summary};

use crate::helpers::get_env_var_or_default;

pub mod orderbook {
    tonic::include_proto!("orderbook");
}

type ClientSender = Sender<Result<Summary, Status>>;

struct OrderbookAggregatorService {
    clients: Arc<Mutex<Vec<ClientSender>>>,
}

#[tonic::async_trait]
impl OrderbookAggregator for OrderbookAggregatorService {
    type BookSummaryStream = RecvStream<'static, Result<Summary, Status>>;

    async fn book_summary(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::BookSummaryStream>, Status> {
        let (tx, rx) = flume::bounded(0);

        let mut clients = self.clients.lock().await;
        clients.push(tx.clone());
        println!(
            "New client connected. Total of {} clients connected",
            clients.len()
        );
        drop(clients);

        Ok(Response::new(rx.into_stream()))
    }
}

const DEFAULT_PORT: u16 = 10000;

pub async fn serve(summary_rx: Receiver<Summary>) -> Result<(), Box<dyn std::error::Error>> {
    let port = get_env_var_or_default("PORT", DEFAULT_PORT);
    let addr = format!("[::1]:{}", port).parse()?;

    let clients: Arc<Mutex<Vec<ClientSender>>> = Arc::new(Mutex::new(vec![]));

    let _main_server_thread = {
        let clients = clients.clone();
        tokio::spawn(async move {
            let mut iter = summary_rx.into_iter();

            while let Some(summary) = iter.next() {
                let mut clients = clients.lock().await;
                let mut clients_to_remove = vec![];

                for (i, client) in clients.iter().enumerate() {
                    if client.is_disconnected() {
                        clients_to_remove.push(i);
                        continue;
                    }

                    match client.send_async(Ok(summary.clone())).await {
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
