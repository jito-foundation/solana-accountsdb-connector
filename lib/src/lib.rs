pub mod chain_data;
pub mod grpc_plugin_source;
pub mod metrics;

pub use chain_data::SlotStatus;
use serde_derive::Deserialize;
use solana_sdk::clock::Slot;
use solana_sdk::pubkey::Pubkey;

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

#[derive(Clone, Debug)]
pub struct SlotUpdate {
    pub slot: u64,
    pub parent: Option<u64>,
    pub status: chain_data::SlotStatus,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PostgresConfig {
    pub connection_string: String,
    /// Number of parallel postgres connections used for account write insertions
    pub account_write_connection_count: u64,
    /// Maximum batch size for account write inserts over one connection
    pub account_write_max_batch_size: usize,
    /// Max size of account write queues
    pub account_write_max_queue_size: usize,
    /// Number of parallel postgres connections used for slot insertions
    pub slot_update_connection_count: u64,
    /// Number of queries retries before fatal error
    pub retry_query_max_count: u64,
    /// Seconds to sleep between query retries
    pub retry_query_sleep_secs: u64,
    /// Seconds to sleep between connection attempts
    pub retry_connection_sleep_secs: u64,
    /// Fatal error when the connection can't be reestablished this long
    pub fatal_connection_timeout_secs: u64,
    /// Allow invalid TLS certificates, passed to native_tls danger_accept_invalid_certs
    pub allow_invalid_certs: bool,
    /// Name key to use in the monitoring table
    pub monitoring_name: String,
    /// Time between updates to the monitoring table
    pub monitoring_update_interval_secs: u64,
    /// Time between cleanup jobs (0 to disable)
    pub cleanup_interval_secs: u64,
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
