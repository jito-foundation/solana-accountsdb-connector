use solana_sdk::{pubkey::Pubkey, slot_hashes::Slot};

pub struct AccountUpdate {
    pub pubkey: Pubkey,
    pub owner: Pubkey,
    pub data: Vec<u8>,
    pub tx_signature: Option<String>,
    pub slot: Slot,
    pub lamports: u64,
    pub rent_epoch: u64,
    pub seq: u8,
    pub is_executable: bool,
    pub is_startup: bool,
    pub is_selected: bool,
}

pub struct PartialAccountUpdate {
    pub pubkey: Pubkey,
    pub tx_signature: Option<String>,
    pub slot: Slot,
    pub seq: u8,
    pub is_startup: bool,
}

enum Status {
    Confirmed,
    Processed,
    Rooted,
}

pub struct SlotUpdate {
    pub parent_slot: Option<Slot>,
    pub slot: Slot,
    pub status: Status,
}
