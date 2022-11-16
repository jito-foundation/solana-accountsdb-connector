use std::{convert::TryInto, ptr::write};

use log::error;
use solana_program::pubkey::Pubkey;

use crate::geyser_plugin_grpc::PluginData;

pub mod accounts_selector;
pub mod active_accounts;
pub mod geyser_plugin_grpc;
pub mod server;

pub(crate) mod geyser_proto {
    tonic::include_proto!("geyser");
}

pub(crate) fn maybe_new_account_write(pubkey: Pubkey, data: &PluginData) -> Option<AccountWrite> {
    if pubkey.len() != 32 {
        error!(
            "bad account pubkey length: {}",
            bs58::encode(pubkey).into_string()
        );
        return None;
    }

    // Select only accounts configured to look at, plus writes to accounts
    // that were previously selected (to catch closures and account reuse)
    let is_selected = data.accounts_selector().is_account_selected(pubkey, owner);
    let previously_selected = data.active_accounts().contains(&pubkey[0..32]);
    let previously_selected = {
        let read = data.read_active_accounts();
        read.contains(&pubkey[0..32])
    };
    if !is_selected && !previously_selected {
        return None;
    }

    // If the account is newly selected, add it
    if !previously_selected {
        let mut write = data.read_active_accounts().write().unwrap();
        write.insert(pubkey.try_into().unwrap());
    }

    data.highest_write_slot.fetch_max(slot, Ordering::SeqCst);

    debug!(
        "Updating account {:?} with owner {:?} at slot {:?}",
        bs58::encode(pubkey).into_string(),
        bs58::encode(owner).into_string(),
        slot,
    );
}
