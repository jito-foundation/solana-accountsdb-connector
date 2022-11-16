use std::collections::HashSet;
use std::sync::RwLock;

pub struct ActiveAccounts {
    active_accounts: RwLock<HashSet<[u8; 32]>>,
}

impl ActiveAccounts {}
