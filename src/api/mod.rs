pub mod entities;
pub mod events;
pub mod services;
pub mod types;
pub mod websocket;

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
    /// Destructive command requires --yes but stdin is not a TTY.
    ConfirmationRequired(String),
    /// Resource exists with a different configuration than requested.
    Conflict(String),
    /// Any other error.
    Other(String),
}

impl HaError {
    /// Stable, snake_case kind identifier used in the structured error envelope.
    /// Consumers branch on this field without parsing the message.
    pub fn error_kind(&self) -> &str {
        match self {
            HaError::Auth(_) => "auth",
            HaError::NotFound(_) => "not_found",
            HaError::InvalidInput(_) => "invalid_input",
            HaError::Connection(_) => "connection",
            HaError::Api { .. } => "api_error",
            HaError::Http(_) => "http_error",
            HaError::ConfirmationRequired(_) => "confirmation_required",
            HaError::Conflict(_) => "conflict",
            HaError::Other(_) => "error",
        }
    }

    /// Optional actionable hint for the error envelope. May be null in JSON.
    pub fn error_hint(&self) -> Option<&str> {
        match self {
            HaError::Auth(_) => Some("Run `ha init` to set up or refresh credentials."),
            HaError::ConfirmationRequired(_) => {
                Some("Re-run with --yes to bypass the confirmation prompt.")
            }
            HaError::Connection(_) => {
                Some("Check that Home Assistant is reachable and the URL is correct.")
            }
            _ => None,
        }
    }
}

impl fmt::Display for HaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HaError::Auth(msg) => write!(f, "Authentication failed: {msg}"),
            HaError::NotFound(msg) => write!(f, "Not found: {msg}"),
            HaError::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
            HaError::Connection(url) => {
                write!(f, "Could not connect to Home Assistant at {url}")
            }
            HaError::Api { status, message } => write!(f, "API error {status}: {message}"),
            HaError::Http(e) => write!(f, "HTTP error: {e}"),
            HaError::ConfirmationRequired(msg) => write!(f, "{msg}"),
            HaError::Conflict(msg) => write!(f, "Conflict: {msg}"),
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

impl From<std::io::Error> for HaError {
    fn from(e: std::io::Error) -> Self {
        HaError::Other(e.to_string())
    }
}

impl From<reqwest::Error> for HaError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_connect() || e.is_timeout() {
            HaError::Connection(
                e.url()
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| "unknown".into()),
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
    fn error_kind_returns_snake_case_identifiers() {
        assert_eq!(HaError::Auth("x".into()).error_kind(), "auth");
        assert_eq!(HaError::NotFound("x".into()).error_kind(), "not_found");
        assert_eq!(
            HaError::InvalidInput("x".into()).error_kind(),
            "invalid_input"
        );
        assert_eq!(HaError::Connection("x".into()).error_kind(), "connection");
        assert_eq!(
            HaError::Api {
                status: 500,
                message: "x".into()
            }
            .error_kind(),
            "api_error"
        );
        assert_eq!(HaError::Other("x".into()).error_kind(), "error");
        assert_eq!(
            HaError::ConfirmationRequired("x".into()).error_kind(),
            "confirmation_required"
        );
        assert_eq!(HaError::Conflict("x".into()).error_kind(), "conflict");
    }

    #[test]
    fn auth_error_hint_suggests_init() {
        let err = HaError::Auth("expired".into());
        assert!(
            err.error_hint().unwrap_or("").contains("ha init"),
            "auth hint must mention ha init"
        );
    }

    #[test]
    fn confirmation_required_hint_mentions_yes_flag() {
        let err = HaError::ConfirmationRequired("delete requires confirmation".into());
        assert!(
            err.error_hint().unwrap_or("").contains("--yes"),
            "confirmation_required hint must mention --yes"
        );
    }

    #[test]
    fn auth_error_display_includes_guidance() {
        let err = HaError::Auth("401 Unauthorized".into());
        let msg = err.to_string();
        assert!(msg.contains("Authentication failed"));
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
