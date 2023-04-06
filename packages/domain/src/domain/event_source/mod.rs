use std::collections::HashMap;

use uuid::Uuid;

#[allow(unused)]
pub struct DomainEvent<T> {
    event_id: Uuid,
    metadata: HashMap<String, String>,
    payload: T,
}

pub enum DomainError {}
