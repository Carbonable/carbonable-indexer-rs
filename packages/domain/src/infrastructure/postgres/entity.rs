use bigdecimal::BigDecimal;
use postgres_types::{FromSql, ToSql};
use sea_query::{enum_def, Iden};
use time::PrimitiveDateTime;
use uuid::Uuid;

#[derive(Debug, ToSql, Iden)]
pub enum ErcImplementation {
    #[iden = "erc_implementation"]
    Enum,
    #[iden = "erc_721"]
    Erc721,
    #[iden = "erc_3525"]
    Erc3525,
}

impl<'a> FromSql<'a> for ErcImplementation {
    fn from_sql(
        _ty: &postgres_types::Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let s = std::str::from_utf8(raw)?;
        match s {
            "erc_721" => Ok(ErcImplementation::Erc721),
            "erc_3525" => Ok(ErcImplementation::Erc3525),
            _ => Err("Unrecognized enum variant".into()),
        }
    }

    fn accepts(ty: &postgres_types::Type) -> bool {
        ty.name() == "erc_implementation"
    }
}

impl From<ErcImplementation> for &str {
    fn from(value: ErcImplementation) -> &'static str {
        match value {
            ErcImplementation::Erc721 => "erc_721",
            ErcImplementation::Erc3525 => "erc_3525",
            ErcImplementation::Enum => panic!("Not a valid erc implementation"),
        }
    }
}

// These structs are only table definition structs
// Not domain business entities
#[enum_def]
pub struct Project {
    pub id: Uuid,
    pub address: String,
    pub slug: String,
    pub name: String,
    pub slot: Option<i64>,
    pub symbol: Option<String>,
    pub total_supply: i64,
    pub owner: String,
    pub ton_equivalent: i64,
    pub times: Vec<PrimitiveDateTime>,
    pub absorptions: Vec<i64>,
    pub setup: bool,
    pub erc_implementation: ErcImplementation,
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
            slot: value.get(4),
            symbol: value.get(5),
            total_supply: value.get(6),
            owner: value.get(7),
            ton_equivalent: value.get(8),
            times: value.get(9),
            absorptions: value.get(10),
            setup: value.get(11),
            erc_implementation: value.get(12),
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
    pub max_supply: Option<u64>,
    // Can be reserved value in case of an erc3525
    pub reserved_supply: u64,
    pub pre_sale_open: bool,
    pub public_sale_open: bool,
    pub max_buy_per_tx: Option<u64>,
    pub max_value_per_tx: Option<u64>,
    pub min_value_per_tx: Option<u64>,
    pub unit_price: BigDecimal,
    pub whitelist_merkle_root: Option<String>,
    pub sold_out: bool,
    pub total_value: Option<BigDecimal>,
    pub whitelist: Option<serde_json::Value>,
    pub erc_implementation: ErcImplementation,
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
    pub previous_project_absorption: i64,
    pub previous_offseter_absorption: i64,
    pub previous_yielder_absorption: i64,
    pub current_project_absorption: i64,
    pub current_offseter_absorption: i64,
    pub current_yielder_absorption: i64,
    pub project_absorption: i64,
    pub offseter_absorption: i64,
    pub yielder_absorption: i64,
    pub time: PrimitiveDateTime,
    pub yielder_id: Option<Uuid>,
}
impl From<tokio_postgres::Row> for Snapshot {
    fn from(value: tokio_postgres::Row) -> Self {
        Self {
            id: value.get(0),
            previous_time: value.get(1),
            previous_project_absorption: value.get(2),
            previous_offseter_absorption: value.get(3),
            previous_yielder_absorption: value.get(4),
            current_project_absorption: value.get(5),
            current_offseter_absorption: value.get(6),
            current_yielder_absorption: value.get(7),
            project_absorption: value.get(8),
            offseter_absorption: value.get(9),
            yielder_absorption: value.get(10),
            time: value.get(11),
            yielder_id: None,
        }
    }
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
    pub amount: f64,
    pub time: PrimitiveDateTime,
    pub yielder_id: Option<Uuid>,
}

impl From<tokio_postgres::Row> for Vesting {
    fn from(value: tokio_postgres::Row) -> Self {
        Self {
            id: value.get(0),
            amount: value.get(1),
            time: value.get(2),
            yielder_id: None,
        }
    }
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
