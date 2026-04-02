pub mod entities;
pub mod events;
pub mod services;
pub mod types;

use std::fmt;

pub struct HaClient {
    pub url: String,
    pub token: String,
}

impl HaClient {
    pub fn new(url: &str, token: &str) -> Self {
        Self {
            url: url.to_string(),
            token: token.to_string(),
        }
    }
}

#[derive(Debug)]
pub enum HaError {
    Config(String),
    NotFound(String),
    Api(String),
    Http(String),
    Json(String),
}

impl fmt::Display for HaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HaError::Config(msg) => write!(f, "config error: {msg}"),
            HaError::NotFound(msg) => write!(f, "not found: {msg}"),
            HaError::Api(msg) => write!(f, "API error: {msg}"),
            HaError::Http(msg) => write!(f, "HTTP error: {msg}"),
            HaError::Json(msg) => write!(f, "JSON error: {msg}"),
        }
    }
}

impl std::error::Error for HaError {}
