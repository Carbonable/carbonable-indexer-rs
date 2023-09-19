use std::sync::Arc;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::value::Value;

use crate::domain::{Contract, Erc3525, Erc721};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner_image_url: Option<String>,
    pub youtube_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<Attribute>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Erc3525Metadata {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub image: String,
    pub banner_image_url: String,
    pub external_url: String,
    pub youtube_url: Option<String>,
}

pub struct UriModel<C = Erc721> {
    client: Arc<Client>,
    ipfs_link: String,
    contract_type: std::marker::PhantomData<C>,
}

impl<C> UriModel<C>
where
    C: Contract,
{
    pub fn new(ipfs_link: String) -> Result<Self, ModelError> {
        Ok(Self {
            client: Arc::new(Client::new()),
            ipfs_link,
            contract_type: std::marker::PhantomData::<C>,
        })
    }
}

#[async_trait::async_trait]
impl StarknetModel<Metadata> for UriModel<Erc721> {
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

#[async_trait::async_trait]
impl StarknetModel<Erc3525Metadata> for UriModel<Erc3525> {
    async fn load(&self) -> Result<Erc3525Metadata, ModelError> {
        let metadata: Erc3525Metadata = self
            .client
            .get(self.ipfs_link.to_owned())
            .send()
            .await?
            .json()
            .await
            .expect("failed to parse metadata");

        Ok(metadata)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BadgeMetadata {
    pub name: String,
    pub description: String,
    pub image: String,
    pub external_link: String,
    pub banner_image_url: String,
}

pub struct BadgeUriModel {
    client: Arc<Client>,
    ipfs_link: String,
}
impl BadgeUriModel {
    pub fn new(ipfs_link: String) -> Result<Self, ModelError> {
        Ok(Self {
            client: Arc::new(Client::new()),
            ipfs_link,
        })
    }
}

#[async_trait::async_trait]
impl StarknetModel<BadgeMetadata> for BadgeUriModel {
    async fn load(&self) -> Result<BadgeMetadata, ModelError> {
        let gateway = std::env::var("GATEWAY")?;
        let ipfs_uri = self.ipfs_link.replace("ipfs://", &gateway);
        let metadata: BadgeMetadata = self
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
