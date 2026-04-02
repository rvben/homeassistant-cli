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
    Auth(String),
    NotFound(String),
    InvalidInput(String),
    Connection(String),
    Api { status: u16, message: String },
    Http(reqwest::Error),
    Other(String),
}

impl fmt::Display for HaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HaError::Auth(msg) => write!(f, "auth error: {msg}"),
            HaError::NotFound(msg) => write!(f, "not found: {msg}"),
            HaError::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            HaError::Connection(msg) => write!(f, "connection error: {msg}"),
            HaError::Api { status, message } => write!(f, "API error {status}: {message}"),
            HaError::Http(e) => write!(f, "HTTP error: {e}"),
            HaError::Other(msg) => write!(f, "error: {msg}"),
        }
    }
}

impl std::error::Error for HaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HaError::Http(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for HaError {
    fn from(e: reqwest::Error) -> Self {
        HaError::Http(e)
    }
}
