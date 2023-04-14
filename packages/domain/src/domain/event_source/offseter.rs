use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OffseterEvents {
    Upgraded,
    Deposit,
    Withdraw,
    Claim,
}
