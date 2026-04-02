use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EntityState {
    pub entity_id: String,
    pub state: String,
    pub attributes: serde_json::Value,
    pub last_changed: String,
    pub last_updated: String,
}

#[derive(Debug, Deserialize)]
pub struct ServiceDomain {
    pub domain: String,
    pub services: BTreeMap<String, ServiceInfo>,
}

#[derive(Debug, Deserialize)]
pub struct ServiceInfo {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HaEvent {
    pub event_type: String,
    pub data: serde_json::Value,
    pub origin: Option<String>,
    pub time_fired: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StateChangedData {
    pub entity_id: String,
    pub new_state: Option<EntityState>,
    pub old_state: Option<EntityState>,
}
