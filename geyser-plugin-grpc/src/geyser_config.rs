use {log::*, std::collections::HashSet};

#[derive(Debug)]
pub(crate) struct GeyserConfig {
    /// Streams all accounts set here.
    pub accounts: HashSet<Vec<u8>>,

    /// Streams out accounts that have an owner set here.
    pub owners: HashSet<Vec<u8>>,

    /// Excludes vote accounts from being streamed out.
    pub exclude_vote_accounts: bool,

    /// Streams all accounts out except for vote accounts if exclude_vote_accounts is true.
    pub select_all_accounts: bool,
}

impl GeyserConfig {
    pub fn default() -> Self {
        GeyserConfig {
            accounts: HashSet::default(),
            owners: HashSet::default(),
            select_all_accounts: true,
            exclude_vote_accounts: true,
        }
    }

    pub fn new(accounts: &[String], owners: &[String], exclude_vote_accounts: bool) -> Self {
        info!(
            "Creating AccountsSelector from accounts: {:?}, owners: {:?}",
            accounts, owners
        );

        let select_all_accounts = accounts.iter().any(|key| key == "*");
        if select_all_accounts {
            return GeyserConfig {
                accounts: HashSet::default(),
                owners: HashSet::default(),
                select_all_accounts,
                exclude_vote_accounts,
            };
        }

        let accounts = accounts
            .iter()
            .map(|key| bs58::decode(key).into_vec().unwrap())
            .collect();
        let owners = owners
            .iter()
            .map(|key| {
                let decoded = bs58::decode(key).into_vec().unwrap();
                if exclude_vote_accounts {
                    assert_ne!(decoded, solana_program::vote::program::id(), "exclude_vote_accounts cannot be true while owners contains the vote program id");
                }
                decoded
            })
            .collect();
        GeyserConfig {
            accounts,
            owners,
            exclude_vote_accounts,
            select_all_accounts,
        }
    }

    pub fn is_account_selected(&self, account: &[u8], owner: &[u8]) -> bool {
        if self.exclude_vote_accounts && solana_program::vote::program::id() == owner {
            return false;
        }

        self.select_all_accounts || self.accounts.contains(account) || self.owners.contains(owner)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn test_create_accounts_selector() {
        GeyserConfig::new(
            &["9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin".to_string()],
            &[],
        );

        GeyserConfig::new(
            &[],
            &["9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin".to_string()],
        );
    }
}
