use crate::api::{HaClient, HaError};
use crate::output::OutputConfig;

pub async fn fire(
    _out: &OutputConfig,
    _client: &HaClient,
    _event_type: &str,
    _data: Option<&str>,
) -> Result<(), HaError> {
    unimplemented!()
}

pub async fn watch(_out: &OutputConfig, _client: &HaClient, _event_type: Option<&str>) -> Result<(), HaError> {
    unimplemented!()
}
