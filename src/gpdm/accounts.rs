use std::collections::HashMap;

/// Simple conservation account with f64 balance. Designed for Phase 1
/// atomic commit semantics via FlowTransaction in flow.rs.
#[derive(Clone, Debug)]
pub struct Account {
    pub id: u64,
    pub balance: f64,
}

impl Account {
    pub fn new(id: u64, balance: f64) -> Self {
        Self { id, balance }
    }

    pub fn can_debit(&self, amount: f64) -> bool {
        // Allow tiny negative tolerance for floating-point noise
        self.balance - amount >= -1e-12
    }

    pub fn apply_debit(&mut self, amount: f64) -> Result<(), &'static str> {
        if self.can_debit(amount) {
            self.balance -= amount;
            Ok(())
        } else {
            Err("insufficient funds")
        }
    }

    pub fn apply_credit(&mut self, amount: f64) {
        self.balance += amount;
    }
}

/// Convenience: account map keyed by account id.
pub type AccountMap = HashMap<u64, Account>;

/// Ensure the given account exists in the map; if missing, insert with `initial` balance.
pub fn ensure_account(map: &mut AccountMap, id: u64, initial: f64) {
    if !map.contains_key(&id) {
        map.insert(id, Account::new(id, initial));
    }
}
