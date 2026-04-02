use crate::api::{HaClient, HaError};
use crate::output::OutputConfig;

pub async fn call(
    _out: &OutputConfig,
    _client: &HaClient,
    _service: &str,
    _entity: Option<&str>,
    _data: Option<&str>,
) -> Result<(), HaError> {
    unimplemented!()
}

pub async fn list(_out: &OutputConfig, _client: &HaClient, _domain: Option<&str>) -> Result<(), HaError> {
    unimplemented!()
}
