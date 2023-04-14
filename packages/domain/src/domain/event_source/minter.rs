use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MinterEvents {
    Upgraded,
    Airdrop,
    Buy,
    SoldOut,
    Migration,
    PreSaleOpen,
    PreSaleClosed,
    PublicSaleOpen,
    PublicSaleClosed,
}
