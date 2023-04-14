use std::collections::HashMap;

use apibara_core::starknet::v1alpha2::Block;

use crate::domain::event_source::{project::ProjectEvents, DomainEvent, Event};

impl From<Block> for DomainEvent {
    fn from(value: Block) -> Self {
        DomainEvent {
            id: "".into(),
            metadata: HashMap::new(),
            payload: HashMap::new(),
            r#type: Event::Project(ProjectEvents::Transfer),
        }
    }
}
