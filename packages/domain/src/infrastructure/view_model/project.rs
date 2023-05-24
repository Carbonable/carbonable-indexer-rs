use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::infrastructure::postgres::entity::ErcImplementation;

#[derive(Debug, Serialize, Deserialize)]
pub struct UriViewModel {
    pub id: Option<Uuid>,
    pub uri: String,
    pub data: serde_json::Value,
}

#[derive(Serialize)]
pub struct Project {
    pub(crate) id: Uuid,
    pub(crate) address: String,
    pub(crate) name: String,
    pub(crate) slug: String,
    pub uri: UriViewModel,
}

#[derive(Serialize)]
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
                    id: value.get(5),
                    uri: value.get(6),
                    data: value.get(7),
                },
            }),
            ErcImplementation::Erc3525 => Self::Erc3525(Project {
                id: value.get(0),
                address: value.get(1),
                name: value.get(2),
                slug: value.get(3),
                uri: UriViewModel {
                    id: value.get(5),
                    uri: value.get(6),
                    data: value.get(7),
                },
            }),
        }
    }
}
