use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc, Arc,
};

use log::*;
use tokio::sync::broadcast;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Code, Request, Response, Status};

use crate::geyser_proto::geyser_server::Geyser;

#[derive(Clone, Debug, Deserialize)]
pub struct ServiceConfig {
    broadcast_buffer_size: usize,
    subscriber_buffer_size: usize,
}

#[derive(Debug)]
pub struct Service {
    pub sender: broadcast::Sender<Update>,
    pub config: ServiceConfig,
    pub highest_write_slot: Arc<AtomicU64>,
}

impl Service {
    pub fn new(config: ServiceConfig, highest_write_slot: Arc<AtomicU64>) -> Self {
        let (tx, _) = broadcast::channel(config.broadcast_buffer_size);
        Self {
            sender: tx,
            config,
            highest_write_slot,
        }
    }
}

#[tonic::async_trait]
impl Geyser for Service {
    type SubscribeStream = ReceiverStream<Result<Update, Status>>;

    async fn subscribe(
        &self,
        _request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        info!("new subscriber");
        let (tx, rx) = mpsc::channel(self.config.subscriber_buffer_size);
        let mut broadcast_rx = self.sender.subscribe();

        tx.send(Ok(Update {
            update_oneof: Some(UpdateOneof::SubscribeResponse(SubscribeResponse {
                highest_write_slot: self.highest_write_slot.load(Ordering::SeqCst),
            })),
        }))
        .await
        .unwrap();

        tokio::spawn(async move {
            let mut exit = false;
            while !exit {
                let fwd = broadcast_rx.recv().await.map_err(|err| {
                    // Note: If we can't keep up pulling from the broadcast
                    // channel here, there'll be a Lagged error, and we'll
                    // close the connection because data was lost.
                    warn!("error while receiving message to be broadcast: {:?}", err);
                    exit = true;
                    Status::new(Code::Internal, err.to_string())
                });
                if let Err(_err) = tx.send(fwd).await {
                    info!("subscriber stream closed");
                    exit = true;
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
