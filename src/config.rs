use crate::api::HaError;

pub struct Config {
    pub url: String,
    pub token: String,
}

impl Config {
    pub fn load(_profile: Option<String>) -> Result<Self, HaError> {
        unimplemented!()
    }
}
