use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::infrastructure::postgres::entity::ErcImplementation;

#[derive(Debug, Serialize, Deserialize)]
pub struct UriViewModel {
    pub uri: Uuid,
    pub data: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct Project {
    id: Uuid,
    address: String,
    name: String,
    slug: String,
    uri: UriViewModel,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProjectViewModel {
    Erc721(Project),
    Erc3525(Project),
}

impl From<tokio_postgres::Row> for ProjectViewModel {
    fn from(value: tokio_postgres::Row) -> Self {
        let erc_implementation: ErcImplementation = value.get(4);
        match erc_implementation {
            ErcImplementation::Enum => panic!("should not fall into this case"),
            ErcImplementation::Erc721 => Self::Erc721(Project {
                id: value.get(0),
                address: value.get(1),
                name: value.get(2),
                slug: value.get(3),
                uri: UriViewModel {
                    uri: value.get(5),
                    data: value.get(6),
                },
            }),
            ErcImplementation::Erc3525 => todo!(),
        }
    }
}
