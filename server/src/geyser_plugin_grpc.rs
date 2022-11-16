use std::{
    collections::HashSet,
    convert::TryInto,
    fs::File,
    io::Read,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock, RwLockReadGuard,
    },
};

use bs58;
use log::*;
use serde_derive::Deserialize;
use serde_json;
use solana_geyser_plugin_interface::geyser_plugin_interface::{
    GeyserPlugin, GeyserPluginError, ReplicaAccountInfoVersions, Result as PluginResult, SlotStatus,
};
use tokio::sync::{broadcast, mpsc};
use tonic::transport::Server;

use crate::{
    accounts_selector::AccountsSelector,
    active_accounts::ActiveAccounts,
    geyser_proto::{
        slot_update::Status as SlotUpdateStatus, update::UpdateOneof, AccountWrite, Ping,
        SlotUpdate, SubscribeRequest, SubscribeResponse, Update,
    },
};

pub struct PluginData {
    runtime: Option<tokio::runtime::Runtime>,
    server_broadcast: broadcast::Sender<Update>,
    server_exit_sender: Option<broadcast::Sender<()>>,
    accounts_selector: AccountsSelector,

    /// Largest slot that an account write was processed for
    highest_write_slot: Arc<AtomicU64>,

    /// Accounts that saw account writes
    ///
    /// Needed to catch writes that signal account closure, where
    /// lamports=0 and owner=system-program.
    active_accounts: ActiveAccounts,
}

#[derive(Default)]
pub struct Plugin {
    // initialized by on_load()
    data: Option<PluginData>,
}

impl std::fmt::Debug for Plugin {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct PluginConfig {
    pub bind_address: String,
    pub service_config: geyser_service::ServiceConfig,
}

impl PluginData {
    fn broadcast(&self, update: UpdateOneof) {
        // Don't care about the error that happens when there are no receivers.
        let _ = self.server_broadcast.send(Update {
            update_oneof: Some(update),
        });
    }

    pub(crate) fn accounts_selector(&self) -> &AccountsSelector {
        &self.accounts_selector
    }

    pub(crate) fn active_accounts(&self) -> &ActiveAccounts {
        &self.active_accounts
    }
}

impl GeyserPlugin for Plugin {
    fn name(&self) -> &'static str {
        "GeyserPluginGrpc"
    }

    fn on_load(&mut self, config_file: &str) -> PluginResult<()> {
        solana_logger::setup_with_default("info");
        info!(
            "Loading plugin {:?} from config_file {:?}",
            self.name(),
            config_file
        );

        let mut file = File::open(config_file)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let result: serde_json::Value = serde_json::from_str(&contents).unwrap();
        let accounts_selector = Self::create_accounts_selector_from_config(&result);

        let config: PluginConfig = serde_json::from_str(&contents).map_err(|err| {
            GeyserPluginError::ConfigFileReadError {
                msg: format!(
                    "The config file is not in the JSON format expected: {:?}",
                    err
                ),
            }
        })?;

        let addr =
            config
                .bind_address
                .parse()
                .map_err(|err| GeyserPluginError::ConfigFileReadError {
                    msg: format!("Error parsing the bind_address {:?}", err),
                })?;

        let highest_write_slot = Arc::new(AtomicU64::new(0));
        let service =
            geyser_service::Service::new(config.service_config, highest_write_slot.clone());
        let (server_exit_sender, mut server_exit_receiver) = broadcast::channel::<()>(1);
        let server_broadcast = service.sender.clone();

        let server = geyser_proto::accounts_db_server::AccountsDbServer::new(service);
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.spawn(Server::builder().add_service(server).serve_with_shutdown(
            addr,
            async move {
                let _ = server_exit_receiver.recv().await;
            },
        ));
        let server_broadcast_c = server_broadcast.clone();
        let mut server_exit_receiver = server_exit_sender.subscribe();
        runtime.spawn(async move {
            loop {
                // Don't care about the error if there are no receivers.
                let _ = server_broadcast_c.send(Update {
                    update_oneof: Some(UpdateOneof::Ping(Ping {})),
                });

                tokio::select! {
                    _ = server_exit_receiver.recv() => { break; },
                    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {},
                }
            }
        });

        self.data = Some(PluginData {
            runtime: Some(runtime),
            server_broadcast,
            server_exit_sender: Some(server_exit_sender),
            accounts_selector,
            highest_write_slot,
            active_accounts: RwLock::new(HashSet::new()),
        });

        Ok(())
    }

    fn on_unload(&mut self) {
        info!("Unloading plugin: {:?}", self.name());

        let mut data = self.data.take().expect("plugin must be initialized");
        data.server_exit_sender
            .take()
            .expect("on_unload can only be called once")
            .send(())
            .expect("sending grpc server termination should succeed");

        data.runtime
            .take()
            .expect("must exist")
            .shutdown_background();
    }

    fn update_account(
        &mut self,
        account: ReplicaAccountInfoVersions,
        slot: u64,
        is_startup: bool,
    ) -> PluginResult<()> {
        let data = self.data.as_ref().expect("plugin must be initialized");
        let (pubkey, owner, write_version, maybe_signature) = match account {
            ReplicaAccountInfoVersions::V0_0_1(account) => {
                (account.pubkey, account.owner, account.write_version, None)
            }
            ReplicaAccountInfoVersions::V0_0_2(account) => (
                account.pubkey,
                account.owner,
                account.write_version,
                account.txn_signature,
            ),
        };

        if pubkey.len() != 32 {
            error!(
                "bad account pubkey length: {}",
                bs58::encode(pubkey).into_string()
            );
            return Ok(());
        }

        // Select only accounts configured to look at, plus writes to accounts
        // that were previously selected (to catch closures and account reuse)
        let is_selected = data.accounts_selector.is_account_selected(pubkey, owner);
        let previously_selected = {
            let read = data.active_accounts.read().unwrap();
            read.contains(&pubkey[0..32])
        };
        if !is_selected && !previously_selected {
            return Ok(());
        }

        // If the account is newly selected, add it
        if !previously_selected {
            let mut write = data.active_accounts.write().unwrap();
            write.insert(pubkey.try_into().unwrap());
        }

        data.highest_write_slot.fetch_max(slot, Ordering::SeqCst);

        debug!(
            "Updating account {:?} with owner {:?} at slot {:?}",
            bs58::encode(pubkey).into_string(),
            bs58::encode(owner).into_string(),
            slot,
        );

        data.broadcast(UpdateOneof::AccountWrite(AccountWrite {
            pubkey: pubkey.to_vec(),
            tx_signature: maybe_signature.map(|sig| sig.to_string()),
            is_startup,
            slot,
            write_version,
        }));

        Ok(())
    }

    fn notify_end_of_startup(&mut self) -> PluginResult<()> {
        Ok(())
    }

    fn update_slot_status(
        &mut self,
        slot: u64,
        parent: Option<u64>,
        status: SlotStatus,
    ) -> PluginResult<()> {
        let data = self.data.as_ref().expect("plugin must be initialized");
        debug!("Updating slot {:?} at with status {:?}", slot, status);

        let status = match status {
            SlotStatus::Processed => SlotUpdateStatus::Processed,
            SlotStatus::Confirmed => SlotUpdateStatus::Confirmed,
            SlotStatus::Rooted => SlotUpdateStatus::Rooted,
        };
        data.broadcast(UpdateOneof::SlotUpdate(SlotUpdate {
            slot,
            parent,
            status: status as i32,
        }));

        Ok(())
    }
}

impl Plugin {
    fn create_accounts_selector_from_config(config: &serde_json::Value) -> AccountsSelector {
        let accounts_selector = &config["accounts_selector"];

        if accounts_selector.is_null() {
            AccountsSelector::default()
        } else {
            let accounts = &accounts_selector["accounts"];
            let accounts: Vec<String> = if accounts.is_array() {
                accounts
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|val| val.as_str().unwrap().to_string())
                    .collect()
            } else {
                Vec::default()
            };
            let owners = &accounts_selector["owners"];
            let owners: Vec<String> = if owners.is_array() {
                owners
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|val| val.as_str().unwrap().to_string())
                    .collect()
            } else {
                Vec::default()
            };
            AccountsSelector::new(&accounts, &owners)
        }
    }
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
/// # Safety
///
/// This function returns the Plugin pointer as trait GeyserPlugin.
pub unsafe extern "C" fn _create_plugin() -> *mut dyn GeyserPlugin {
    let plugin = Plugin::default();
    let plugin: Box<dyn GeyserPlugin> = Box::new(plugin);
    Box::into_raw(plugin)
}

#[cfg(test)]
pub(crate) mod tests {
    use serde_json;

    use super::*;

    #[test]
    fn test_accounts_selector_from_config() {
        let config = "{\"accounts_selector\" : { \
           \"owners\" : [\"9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin\"] \
        }}";

        let config: serde_json::Value = serde_json::from_str(config).unwrap();
        Plugin::create_accounts_selector_from_config(&config);
    }
}
