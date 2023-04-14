use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum YielderEvents {
    Upgraded,
    Deposit,
    Withdraw,
    Snapshot,
    Vesting,
}
