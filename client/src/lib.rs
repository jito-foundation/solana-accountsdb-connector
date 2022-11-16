pub mod geyer_consumer;
pub mod grpc_plugin_source;
pub mod metrics;
pub mod types;

use serde_derive::Deserialize;
use solana_sdk::{clock::Slot, pubkey::Pubkey};

pub(crate) mod geyser_proto {
    tonic::include_proto!("geyser");
}

trait AnyhowWrap {
    type Value;
    fn map_err_anyhow(self) -> anyhow::Result<Self::Value>;
}

impl<T, E: std::fmt::Debug> AnyhowWrap for Result<T, E> {
    type Value = T;
    fn map_err_anyhow(self) -> anyhow::Result<Self::Value> {
        self.map_err(|err| anyhow::anyhow!("{:?}", err))
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct AccountWrite {
    pub pubkey: Pubkey,
    pub slot: Slot,
}

impl AccountWrite {
    fn from(pubkey: Pubkey, slot: Slot) -> AccountWrite {
        AccountWrite { pubkey, slot }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SlotStatus {
    Rooted,
    Confirmed,
    Processed,
}

#[derive(Clone, Debug)]
pub struct SlotUpdate {
    pub slot: u64,
    pub parent: Option<u64>,
    pub status: SlotStatus,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TlsConfig {
    pub ca_cert_path: String,
    pub client_cert_path: String,
    pub client_key_path: String,
    pub domain_name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GrpcSourceConfig {
    pub name: String,
    pub connection_string: String,
    pub retry_connection_sleep_secs: u64,
    pub tls: Option<TlsConfig>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SourceConfig {
    pub dedup_queue_size: usize,
    pub grpc_sources: Vec<GrpcSourceConfig>,
    pub maybe_snapshot_config: Option<SnapshotSourceConfig>,
    pub rpc_ws_url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SnapshotSourceConfig {
    pub rpc_http_url: String,
    pub program_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub postgres_target: PostgresConfig,
    pub source: SourceConfig,
}
