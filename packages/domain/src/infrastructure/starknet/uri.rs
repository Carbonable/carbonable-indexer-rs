use std::sync::Arc;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::value::Value;

use super::model::{ModelError, StarknetModel};

#[derive(Debug, Serialize, Deserialize)]
pub struct Attribute {
    pub display_type: Option<String>,
    pub trait_type: String,
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub description: String,
    pub image: String,
    pub external_url: String,
    pub banner_image_url: String,
    pub youtube_url: String,
    pub attributes: Vec<Attribute>,
}

pub struct UriModel {
    client: Arc<Client>,
    ipfs_link: String,
}

impl UriModel {
    pub fn new(ipfs_link: String) -> Result<Self, ModelError> {
        Ok(Self {
            client: Arc::new(Client::new()),
            ipfs_link,
        })
    }
}

#[async_trait::async_trait]
impl StarknetModel<Metadata> for UriModel {
    async fn load(&self) -> Result<Metadata, ModelError> {
        let gateway = std::env::var("GATEWAY")?;
        let ipfs_uri = self.ipfs_link.replace("ipfs://", &gateway);
        let metadata: Metadata = self
            .client
            .get(ipfs_uri)
            .send()
            .await?
            .json()
            .await
            .expect("failed to parse metadata");

        Ok(metadata)
    }
}
