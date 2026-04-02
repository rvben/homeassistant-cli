pub mod entities;
pub mod events;
pub mod services;
pub mod types;

pub use types::*;

use std::fmt;

#[derive(Debug)]
pub enum HaError {
    /// 401/403 from HA API.
    Auth(String),
    /// 404 — entity, service, or resource not found.
    NotFound(String),
    /// Missing or invalid config/input.
    InvalidInput(String),
    /// Could not reach Home Assistant.
    Connection(String),
    /// Non-2xx response.
    Api { status: u16, message: String },
    /// Network/TLS error from reqwest.
    Http(reqwest::Error),
    /// Any other error.
    Other(String),
}

impl fmt::Display for HaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HaError::Auth(msg) => write!(
                f,
                "Authentication failed: {msg}\nCheck your token or run `ha init`."
            ),
            HaError::NotFound(msg) => write!(f, "Not found: {msg}"),
            HaError::InvalidInput(msg) => write!(f, "{msg}"),
            HaError::Connection(url) => write!(
                f,
                "Could not connect to Home Assistant at {url}\nCheck that HA is running and the URL is correct."
            ),
            HaError::Api { status, message } => write!(f, "API error {status}: {message}"),
            HaError::Http(e) => write!(f, "HTTP error: {e}"),
            HaError::Other(msg) => write!(f, "{msg}"),
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
        if e.is_connect() || e.is_timeout() {
            HaError::Connection(
                e.url().map(|u| u.to_string()).unwrap_or_else(|| "unknown".into()),
            )
        } else {
            HaError::Http(e)
        }
    }
}

/// HTTP client for the Home Assistant REST API.
pub struct HaClient {
    pub base_url: String,
    token: String,
    pub(crate) client: reqwest::Client,
}

impl HaClient {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            token: token.into(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("build reqwest client"),
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    /// Returns a GET request builder pre-configured with Bearer auth.
    pub fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .get(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.token)
    }

    /// Returns a POST request builder pre-configured with Bearer auth.
    pub fn post(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .post(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.token)
    }

    /// Validate the connection by calling GET /api/
    pub async fn validate(&self) -> Result<String, HaError> {
        let resp = self.get("/api/").send().await?;
        match resp.status().as_u16() {
            200 => {
                let body: serde_json::Value = resp.json().await?;
                Ok(body["message"]
                    .as_str()
                    .unwrap_or("API running.")
                    .to_owned())
            }
            401 | 403 => Err(HaError::Auth("Invalid token".into())),
            status => Err(HaError::Api {
                status,
                message: resp.text().await.unwrap_or_default(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn auth_error_display_includes_guidance() {
        let err = HaError::Auth("401 Unauthorized".into());
        let msg = err.to_string();
        assert!(msg.contains("Authentication failed"));
        assert!(msg.contains("ha init") || msg.contains("HA_TOKEN"));
    }

    #[test]
    fn not_found_display_includes_entity() {
        let err = HaError::NotFound("light.missing".into());
        assert!(err.to_string().contains("light.missing"));
    }

    #[test]
    fn connection_error_mentions_url() {
        let err = HaError::Connection("http://ha.local:8123".into());
        assert!(err.to_string().contains("http://ha.local:8123"));
    }

    #[test]
    fn http_error_source_is_reqwest() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let reqwest_err = rt.block_on(async {
            reqwest::Client::new()
                .get("http://127.0.0.1:1")
                .send()
                .await
                .unwrap_err()
        });
        let api_err = HaError::Http(reqwest_err);
        assert!(api_err.source().is_some());
    }

    #[test]
    fn ha_client_new_trims_trailing_slash() {
        let client = HaClient::new("http://ha.local:8123/", "token");
        assert_eq!(client.base_url, "http://ha.local:8123");
    }
}
