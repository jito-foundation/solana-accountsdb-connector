use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};

use crossbeam::channel::Sender;
use geyser_proto::geyser_client::GeyserClient;
use tokio::runtime::Runtime;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};

use crate::{
    geyser_proto,
    types::{AccountUpdate, PartialAccountUpdate, SlotUpdate},
};

pub struct GeyserConsumer<T> {
    inner: Arc<Mutex<GeyserClient<T>>>,
    runtime: Runtime,
}

impl<T> GeyserConsumer<T> {
    pub fn new(client: GeyserClient<T>, runtime: Runtime) -> Self {
        let inner = Arc::new(Mutex::new(client));
        Self { inner, runtime }
    }

    pub fn subscribe_account_updates(&self, tx: Sender<AccountUpdate>) {
        let mut inner = self.inner.lock().unwrap();
        let stream = self.runtime.block_on(async {
            inner
                .subscribe_account_updates(EmptyRequest {})
                .await?
                .into_inner()
        });
    }

    pub fn subscribe_partial_account_updates(
        &self,
        tx: Sender<PartialAccountUpdate>,
        skip_vote_accounts: bool,
    ) {
        let mut inner = self.inner.lock().unwrap();
        let stream = self.runtime.block_on(async {
            inner
                .subscribe_partial_account_updates(SubscribePartialAccountUpdates {
                    skip_vote_accounts,
                })
                .await?
                .into_inner()
        });
    }

    pub fn subscribe_slot_updates(self, tx: Sender<SlotUpdate>) {
        let mut inner = self.inner.lock().unwrap();
        let stream = self.runtime.block_on(async {
            inner
                .subscribe_slot_updates(EmptyRequest {})
                .await?
                .into_inner()
        });
    }
}
