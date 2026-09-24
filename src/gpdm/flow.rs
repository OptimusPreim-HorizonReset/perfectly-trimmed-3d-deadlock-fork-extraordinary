use crate::gpdm::accounts::{Account, AccountMap};
use std::collections::HashMap;

/// A single flow entry represents a delta applied to an account. Positive
/// amounts are credits, negative amounts are debits.
#[derive(Clone, Debug)]
pub struct FlowEntry {
    pub account_id: u64,
    pub amount: f64,
}

/// A transaction is a list of entries that must be atomically applied. The
/// commit algorithm performs a preflight on a copy and only mutates the
/// authoritative accounts if all balances remain valid.
#[derive(Clone, Debug)]
pub struct FlowTransaction {
    pub entries: Vec<FlowEntry>,
}

impl FlowTransaction {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn add_entry(&mut self, account_id: u64, amount: f64) {
        self.entries.push(FlowEntry { account_id, amount });
    }

    /// Attempt to commit this transaction against the provided accounts.
    /// Returns Ok(()) on success and applies the changes; otherwise returns
    /// Err(String) and leaves `accounts` unchanged.
    pub fn commit(&self, accounts: &mut AccountMap) -> Result<(), String> {
        // Preflight: clone relevant balances into a temp map
        let mut temp: HashMap<u64, f64> = HashMap::new();
        for entry in &self.entries {
            let bal = temp
                .entry(entry.account_id)
                .or_insert_with(|| accounts.get(&entry.account_id).map(|a| a.balance).unwrap_or(0.0));
            *bal += entry.amount;
        }

        // Validate: no negative balances
        for (&id, &balance) in temp.iter() {
            if balance < -1e-9 {
                return Err(format!("account {} would go negative ({})", id, balance));
            }
        }

        // Apply: ensure accounts exist and mutate
        for (&id, &balance) in temp.iter() {
            if let Some(acc) = accounts.get_mut(&id) {
                acc.balance = balance;
            } else {
                // create new account with final balance
                accounts.insert(id, Account::new(id, balance));
            }
        }

        Ok(())
    }
}
