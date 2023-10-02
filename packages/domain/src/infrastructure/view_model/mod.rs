use std::collections::HashMap;

use time::PrimitiveDateTime;
use tokio_postgres::Row;

use crate::domain::{crypto::U256, event_source::Event, Ulid};

pub mod customer;
pub mod farming;
pub mod launchpad;
pub mod portfolio;
pub mod project;

#[derive(Debug)]
pub struct DomainEventViewModel {
    pub id: Ulid,
    pub event_id: String,
    pub block_number: U256,
    pub block_hash: String,
    pub metadata: HashMap<String, String>,
    pub payload: HashMap<String, String>,
    pub r#type: Event,
    pub recorded_at: PrimitiveDateTime,
}

impl From<Row> for DomainEventViewModel {
    fn from(value: Row) -> Self {
        let metadata = json_value_to_hash_map(
            value.get(4),
            &[
                "block_hash",
                "block_number",
                "from_address",
                "timestamp",
                "tx_hash",
            ],
        );
        let payload: HashMap<String, String> = serde_json::from_value(value.get(5))
            .expect("failed to deserialize payload to hash_map");
        Self {
            id: value.get(0),
            event_id: value.get(1),
            block_number: value.get(2),
            block_hash: value.get(3),
            metadata,
            payload,
            r#type: value.get(6),
            recorded_at: value.get(7),
        }
    }
}

fn json_value_to_hash_map(value: serde_json::Value, keys: &[&str]) -> HashMap<String, String> {
    let mut into = HashMap::new();
    for key in keys {
        if let Some(v) = value.get(key) {
            let inner = match v {
                serde_json::Value::Null => "".to_owned(),
                serde_json::Value::Bool(b) => {
                    if *b {
                        "true".to_owned()
                    } else {
                        "false".to_owned()
                    }
                }
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.to_owned(),
                serde_json::Value::Array(_) => panic!("cannot convert array to string"),
                serde_json::Value::Object(_) => panic!("cannot convert object to string"),
            };
            into.insert(key.to_string(), inner);
        }
    }
    into
}
