use super::project::Project;
use serde::Serialize;
use time::PrimitiveDateTime;

#[derive(Debug, Serialize)]
pub struct Launchpad {
    is_ready: bool,
    sale_date: Option<PrimitiveDateTime>,
    minter_contract: MinterContract,
    image: Option<String>,
    whitelisted_sale_open: bool,
    public_sale_open: bool,
    is_sold_out: bool,
}

#[derive(Debug, Serialize)]
pub struct MinterContract {
    address: String,
    abi: serde_json::Value,
}

#[derive(Serialize)]
pub struct LaunchpadProject {
    project: Project,
    launchpad: Launchpad,
    #[serde(skip_serializing_if = "Option::is_none")]
    whitelist: Option<serde_json::Value>,
}

impl From<tokio_postgres::Row> for LaunchpadProject {
    fn from(value: tokio_postgres::Row) -> Self {
        LaunchpadProject {
            project: Project {
                id: value.get(0),
                address: value.get(1),
                name: value.get(2),
                slug: value.get(3),
                uri: super::project::UriViewModel {
                    id: value.get(5),
                    uri: value.get(6),
                    data: value.get(7),
                },
            },
            launchpad: Launchpad {
                is_ready: value.get(4),
                sale_date: value.get(8),
                minter_contract: MinterContract {
                    address: value.get(9),
                    abi: value.get(13),
                },
                image: None,
                whitelisted_sale_open: value.get(10),
                public_sale_open: value.get(11),
                is_sold_out: value.get(12),
            },
            whitelist: match value.try_get(14) {
                Ok(w) => Some(w),
                Err(_) => None,
            },
        }
    }
}
