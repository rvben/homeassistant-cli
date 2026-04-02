use crate::api::{HaClient, HaError};
use crate::output::OutputConfig;

pub async fn get(_out: &OutputConfig, _client: &HaClient, _entity_id: &str) -> Result<(), HaError> {
    unimplemented!()
}

pub async fn list(_out: &OutputConfig, _client: &HaClient, _domain: Option<&str>) -> Result<(), HaError> {
    unimplemented!()
}

pub async fn watch(_out: &OutputConfig, _client: &HaClient, _entity_id: &str) -> Result<(), HaError> {
    unimplemented!()
}
