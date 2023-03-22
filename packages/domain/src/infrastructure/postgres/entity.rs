use sea_query::enum_def;
use time::PrimitiveDateTime;
use uuid::Uuid;

// These structs are only table definition structs
// Not domain business entities
#[enum_def]
pub struct Project {
    pub id: Uuid,
    pub address: String,
    pub slug: String,
    pub name: String,
    pub symbol: String,
    pub total_supply: i64,
    pub owner: String,
    pub ton_equivalent: i64,
    pub times: Vec<PrimitiveDateTime>,
    pub absorptions: Vec<i64>,
    pub setup: bool,
    pub implementation_id: Option<Uuid>,
    pub uri_id: Option<Uuid>,
}

impl From<tokio_postgres::Row> for Project {
    fn from(value: tokio_postgres::Row) -> Self {
        Self {
            id: value.get(0),
            address: value.get(1),
            slug: value.get(2),
            name: value.get(3),
            symbol: value.get(4),
            total_supply: value.get(5),
            owner: value.get(6),
            ton_equivalent: value.get(7),
            times: value.get(8),
            absorptions: value.get(9),
            setup: value.get(10),
            implementation_id: None,
            uri_id: None,
        }
    }
}

#[enum_def]
pub struct Implementation {
    pub id: Uuid,
    pub address: String,
    pub abi: serde_json::Value,
}

impl From<tokio_postgres::Row> for Implementation {
    fn from(value: tokio_postgres::Row) -> Self {
        Self {
            id: value.get(0),
            address: value.get(1),
            abi: value.get(2),
        }
    }
}

#[enum_def]
pub struct Uri {
    pub id: Uuid,
    pub uri: String,
    pub data: serde_json::Value,
}

impl From<tokio_postgres::Row> for Uri {
    fn from(value: tokio_postgres::Row) -> Self {
        Self {
            id: value.get(0),
            uri: value.get(1),
            data: value.get(2),
        }
    }
}

#[enum_def]
pub struct Payment {
    pub id: Uuid,
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: i64,
    pub implementation_id: Option<Uuid>,
}

impl From<tokio_postgres::Row> for Payment {
    fn from(value: tokio_postgres::Row) -> Self {
        Self {
            id: value.get(0),
            address: value.get(1),
            name: value.get(2),
            symbol: value.get(3),
            decimals: value.get(4),
            implementation_id: None,
        }
    }
}

#[enum_def]
pub struct Minter {
    pub id: Uuid,
    pub address: String,
    pub max_supply: u64,
    pub reserved_supply: u64,
    pub pre_sale_open: bool,
    pub public_sale_open: bool,
    pub max_buy_per_tx: u64,
    pub unit_price: u64,
    pub whitelist_merkle_root: Option<String>,
    pub sold_out: bool,
    pub total_value: u64,
    pub whitelist: Option<serde_json::Value>,
    pub project_id: Option<Uuid>,
    pub payment_id: Option<Uuid>,
    pub implementation_id: Option<Uuid>,
}

#[enum_def]
pub struct Offseter {
    pub id: Uuid,
    pub address: String,
    pub total_deposited: u64,
    pub total_claimed: u64,
    pub total_claimable: u64,
    pub min_claimable: u64,
    pub project_id: Option<Uuid>,
    pub implementation_id: Option<Uuid>,
}

#[enum_def]
pub struct Snapshot {
    pub id: Uuid,
    pub previous_time: PrimitiveDateTime,
    pub previous_project_absorption: u64,
    pub previous_offseter_absorption: u64,
    pub previous_yielder_absorption: u64,
    pub current_project_absorption: u64,
    pub current_offseter_absorption: u64,
    pub current_yielder_absorption: u64,
    pub project_absorption: u64,
    pub offseter_absorption: u64,
    pub yielder_absorption: u64,
    pub time: PrimitiveDateTime,
    pub yielder_id: Option<Uuid>,
}

#[enum_def]
pub struct Yielder {
    pub id: Uuid,
    pub address: String,
    pub total_deposited: u64,
    pub total_absorption: u64,
    pub snapshot_time: PrimitiveDateTime,
    pub project_id: Option<Uuid>,
    pub vester_id: Option<Uuid>,
    pub implementation_id: Option<Uuid>,
}

#[enum_def]
pub struct Vesting {
    pub id: Uuid,
    pub amount: u64,
    pub time: PrimitiveDateTime,
    pub yielder_id: Option<Uuid>,
}

#[enum_def]
pub struct Vester {
    pub id: Uuid,
    pub address: String,
    pub total_amount: i64,
    pub withdrawable_amount: i64,
    pub implementation_id: Option<Uuid>,
}
impl From<tokio_postgres::Row> for Vester {
    fn from(value: tokio_postgres::Row) -> Self {
        Self {
            id: value.get(0),
            address: value.get(1),
            total_amount: value.get(2),
            withdrawable_amount: value.get(3),
            implementation_id: None,
        }
    }
}

#[enum_def]
pub struct Transfer {
    pub id: Uuid,
    pub hash: String,
    pub from: String,
    pub to: String,
    pub token_id: u64,
    pub time: PrimitiveDateTime,
    pub block_id: u64,
    pub project_id: Option<Uuid>,
}

#[enum_def]
pub struct Airdrop {
    pub id: Uuid,
    pub hash: String,
    pub address: String,
    pub quantity: u64,
    pub time: PrimitiveDateTime,
    pub block_id: u64,
    pub minter_id: Option<Uuid>,
}

#[enum_def]
pub struct Buy {
    pub id: Uuid,
    pub hash: String,
    pub address: String,
    pub quantity: u64,
    pub time: PrimitiveDateTime,
    pub block_id: u64,
    pub minter_id: Option<Uuid>,
}

#[enum_def]
pub struct TransferSingle {
    pub id: Uuid,
    pub hash: String,
    pub from: String,
    pub to: String,
    pub token_id: u64,
    pub time: PrimitiveDateTime,
    pub block_id: u64,
    pub badge_id: Option<Uuid>,
}

#[enum_def]
pub struct Badge {
    pub id: Uuid,
    pub address: String,
    pub name: String,
    pub owner: String,
    pub implementation_id: Option<Uuid>,
    pub uri_id: Option<Uuid>,
}
