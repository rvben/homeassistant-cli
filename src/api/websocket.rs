//! WebSocket client for the Home Assistant WebSocket API.
//!
//! Unlike REST, the registry endpoints (`config/entity_registry/*`,
//! `config/device_registry/*`, `config/area_registry/*`) are only reachable
//! over the WebSocket API. This module provides a minimal id-multiplexed
//! request/response client that authenticates once and exchanges JSON
//! messages with Home Assistant.
//!
//! Protocol reference: <https://developers.home-assistant.io/docs/api/websocket>

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::api::HaError;

/// Derive the WebSocket URL from a REST base URL.
/// `http://host/` → `ws://host/api/websocket`, `https://host/` → `wss://host/api/websocket`.
/// Preserves any base path (e.g. `https://ha.example.com/ha` for reverse-proxied installs).
pub(crate) fn derive_ws_url(base_url: &str) -> Result<String, HaError> {
    let base = base_url.trim_end_matches('/');
    let (scheme, rest) = if let Some(rest) = base.strip_prefix("https://") {
        ("wss://", rest)
    } else if let Some(rest) = base.strip_prefix("http://") {
        ("ws://", rest)
    } else {
        return Err(HaError::InvalidInput(format!(
            "URL must start with http:// or https://: {base_url}"
        )));
    };
    Ok(format!("{scheme}{rest}/api/websocket"))
}

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Authenticated Home Assistant WebSocket client.
///
/// The connection is opened and authenticated in [`HaWs::connect`]; afterwards
/// [`HaWs::call`] sends a command and returns the matching `result` payload.
/// Ids are assigned monotonically per client.
pub struct HaWs {
    stream: WsStream,
    next_id: u64,
}

impl HaWs {
    /// Open a WebSocket connection and complete the auth handshake.
    pub async fn connect(base_url: &str, token: &str) -> Result<Self, HaError> {
        let ws_url = derive_ws_url(base_url)?;
        let (stream, _response) = tokio_tungstenite::connect_async(&ws_url)
            .await
            .map_err(|e| HaError::Connection(format!("{ws_url}: {e}")))?;
        let mut client = Self { stream, next_id: 1 };
        client.authenticate(token).await?;
        Ok(client)
    }

    async fn authenticate(&mut self, token: &str) -> Result<(), HaError> {
        let msg = self.recv_json().await?;
        match msg.get("type").and_then(|v| v.as_str()) {
            Some("auth_required") => {}
            Some(other) => {
                return Err(HaError::Other(format!(
                    "expected auth_required, got {other}"
                )));
            }
            None => return Err(HaError::Other("missing type on first message".into())),
        }

        self.send_json(&serde_json::json!({
            "type": "auth",
            "access_token": token,
        }))
        .await?;

        let msg = self.recv_json().await?;
        match msg.get("type").and_then(|v| v.as_str()) {
            Some("auth_ok") => Ok(()),
            Some("auth_invalid") => {
                let m = msg
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("invalid token");
                Err(HaError::Auth(m.to_owned()))
            }
            _ => Err(HaError::Other(format!("unexpected auth response: {msg}"))),
        }
    }

    /// Send a command and return its `result` payload.
    ///
    /// `extra` is merged into the command envelope alongside `id` and `type`
    /// (e.g. `{"entity_id": "light.x"}` for a `config/entity_registry/remove`).
    /// HA error codes map to [`HaError`]: `not_found` → `NotFound`, everything
    /// else → `Api { status: 0, message: "<code>: <message>" }`.
    pub async fn call(
        &mut self,
        msg_type: &str,
        extra: serde_json::Value,
    ) -> Result<serde_json::Value, HaError> {
        let id = self.next_id;
        self.next_id += 1;

        let mut cmd = serde_json::json!({ "id": id, "type": msg_type });
        if let serde_json::Value::Object(extras) = extra
            && let serde_json::Value::Object(ref mut obj) = cmd
        {
            for (k, v) in extras {
                obj.insert(k, v);
            }
        }
        self.send_json(&cmd).await?;

        loop {
            let msg = self.recv_json().await?;
            let is_result = msg.get("type").and_then(|v| v.as_str()) == Some("result");
            let matches_id = msg.get("id").and_then(|v| v.as_u64()) == Some(id);
            if !(is_result && matches_id) {
                continue;
            }
            if msg.get("success").and_then(|v| v.as_bool()) == Some(true) {
                return Ok(msg
                    .get("result")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null));
            }
            let err = msg.get("error").cloned().unwrap_or(serde_json::Value::Null);
            let code = err
                .get("code")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_owned();
            let message = err
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            return Err(match code.as_str() {
                "not_found" => HaError::NotFound(message),
                "unauthorized" => HaError::Auth(message),
                _ => HaError::Api {
                    status: 0,
                    message: format!("{code}: {message}"),
                },
            });
        }
    }

    /// Close the WebSocket cleanly. Errors on close are ignored.
    pub async fn close(mut self) {
        let _ = self.stream.close(None).await;
    }

    async fn send_json(&mut self, value: &serde_json::Value) -> Result<(), HaError> {
        let text = value.to_string();
        self.stream
            .send(Message::Text(text))
            .await
            .map_err(|e| HaError::Connection(format!("send failed: {e}")))
    }

    async fn recv_json(&mut self) -> Result<serde_json::Value, HaError> {
        loop {
            let msg = self
                .stream
                .next()
                .await
                .ok_or_else(|| HaError::Connection("connection closed".into()))?
                .map_err(|e| HaError::Connection(format!("recv failed: {e}")))?;
            match msg {
                Message::Text(text) => {
                    return serde_json::from_str(&text)
                        .map_err(|e| HaError::Other(format!("invalid JSON from server: {e}")));
                }
                Message::Binary(_) => {
                    return Err(HaError::Other("unexpected binary frame".into()));
                }
                Message::Close(_) => {
                    return Err(HaError::Connection("server closed connection".into()));
                }
                Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => continue,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_ws_url_http_to_ws() {
        assert_eq!(
            derive_ws_url("http://ha.local:8123").unwrap(),
            "ws://ha.local:8123/api/websocket"
        );
    }

    #[test]
    fn derive_ws_url_https_to_wss() {
        assert_eq!(
            derive_ws_url("https://ha.example.com").unwrap(),
            "wss://ha.example.com/api/websocket"
        );
    }

    #[test]
    fn derive_ws_url_strips_trailing_slash() {
        assert_eq!(
            derive_ws_url("http://ha.local:8123/").unwrap(),
            "ws://ha.local:8123/api/websocket"
        );
    }

    #[test]
    fn derive_ws_url_preserves_base_path() {
        assert_eq!(
            derive_ws_url("https://example.com/ha").unwrap(),
            "wss://example.com/ha/api/websocket"
        );
    }

    #[test]
    fn derive_ws_url_rejects_invalid_scheme() {
        assert!(matches!(
            derive_ws_url("ftp://ha.local").unwrap_err(),
            HaError::InvalidInput(_)
        ));
    }

    /// Spawn a tiny WebSocket server that runs `handler` against exactly one
    /// client connection, then returns the HTTP base URL the client should use.
    async fn spawn_mock_server<F, Fut>(handler: F) -> (String, tokio::task::JoinHandle<()>)
    where
        F: FnOnce(tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>) -> Fut
            + Send
            + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let base_url = format!("http://127.0.0.1:{port}");
        let handle = tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await
                && let Ok(ws) = tokio_tungstenite::accept_async(stream).await
            {
                handler(ws).await;
            }
        });
        (base_url, handle)
    }

    async fn recv_text(
        ws: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    ) -> serde_json::Value {
        let msg = ws.next().await.unwrap().unwrap();
        let text = match msg {
            Message::Text(t) => t.to_string(),
            other => panic!("expected text frame, got {other:?}"),
        };
        serde_json::from_str(&text).unwrap()
    }

    async fn send_text(
        ws: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        v: serde_json::Value,
    ) {
        ws.send(Message::Text(v.to_string())).await.unwrap();
    }

    #[tokio::test]
    async fn connect_completes_auth_handshake() {
        let (base_url, handle) = spawn_mock_server(|mut ws| async move {
            send_text(&mut ws, serde_json::json!({"type": "auth_required"})).await;
            let auth = recv_text(&mut ws).await;
            assert_eq!(auth["type"], "auth");
            assert_eq!(auth["access_token"], "tok");
            send_text(&mut ws, serde_json::json!({"type": "auth_ok"})).await;
        })
        .await;

        let client = HaWs::connect(&base_url, "tok").await.unwrap();
        client.close().await;
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn connect_auth_invalid_maps_to_auth_error() {
        let (base_url, handle) = spawn_mock_server(|mut ws| async move {
            send_text(&mut ws, serde_json::json!({"type": "auth_required"})).await;
            let _ = recv_text(&mut ws).await;
            send_text(
                &mut ws,
                serde_json::json!({"type": "auth_invalid", "message": "Invalid access token"}),
            )
            .await;
        })
        .await;

        let result = HaWs::connect(&base_url, "tok").await;
        match result {
            Err(HaError::Auth(_)) => {}
            Err(e) => panic!("expected Auth error, got {e:?}"),
            Ok(_) => panic!("expected Auth error, got Ok"),
        }
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn call_returns_result_payload() {
        let (base_url, handle) = spawn_mock_server(|mut ws| async move {
            send_text(&mut ws, serde_json::json!({"type": "auth_required"})).await;
            let _ = recv_text(&mut ws).await;
            send_text(&mut ws, serde_json::json!({"type": "auth_ok"})).await;

            let cmd = recv_text(&mut ws).await;
            assert_eq!(cmd["type"], "config/entity_registry/list");
            let id = cmd["id"].as_u64().unwrap();
            send_text(
                &mut ws,
                serde_json::json!({
                    "id": id,
                    "type": "result",
                    "success": true,
                    "result": [{"entity_id": "light.x"}]
                }),
            )
            .await;
        })
        .await;

        let mut client = HaWs::connect(&base_url, "tok").await.unwrap();
        let result = client
            .call("config/entity_registry/list", serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(result[0]["entity_id"], "light.x");
        client.close().await;
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn call_not_found_error_maps_to_not_found() {
        let (base_url, handle) = spawn_mock_server(|mut ws| async move {
            send_text(&mut ws, serde_json::json!({"type": "auth_required"})).await;
            let _ = recv_text(&mut ws).await;
            send_text(&mut ws, serde_json::json!({"type": "auth_ok"})).await;

            let cmd = recv_text(&mut ws).await;
            let id = cmd["id"].as_u64().unwrap();
            send_text(
                &mut ws,
                serde_json::json!({
                    "id": id,
                    "type": "result",
                    "success": false,
                    "error": {"code": "not_found", "message": "Entity not found"}
                }),
            )
            .await;
        })
        .await;

        let mut client = HaWs::connect(&base_url, "tok").await.unwrap();
        let err = client
            .call(
                "config/entity_registry/remove",
                serde_json::json!({"entity_id": "light.missing"}),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, HaError::NotFound(_)));
        client.close().await;
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn call_merges_extra_fields() {
        let (base_url, handle) = spawn_mock_server(|mut ws| async move {
            send_text(&mut ws, serde_json::json!({"type": "auth_required"})).await;
            let _ = recv_text(&mut ws).await;
            send_text(&mut ws, serde_json::json!({"type": "auth_ok"})).await;

            let cmd = recv_text(&mut ws).await;
            assert_eq!(cmd["type"], "config/entity_registry/remove");
            assert_eq!(cmd["entity_id"], "light.kitchen");
            let id = cmd["id"].as_u64().unwrap();
            send_text(
                &mut ws,
                serde_json::json!({
                    "id": id,
                    "type": "result",
                    "success": true,
                    "result": null
                }),
            )
            .await;
        })
        .await;

        let mut client = HaWs::connect(&base_url, "tok").await.unwrap();
        client
            .call(
                "config/entity_registry/remove",
                serde_json::json!({"entity_id": "light.kitchen"}),
            )
            .await
            .unwrap();
        client.close().await;
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn call_ignores_interleaved_unrelated_messages() {
        let (base_url, handle) = spawn_mock_server(|mut ws| async move {
            send_text(&mut ws, serde_json::json!({"type": "auth_required"})).await;
            let _ = recv_text(&mut ws).await;
            send_text(&mut ws, serde_json::json!({"type": "auth_ok"})).await;

            let cmd = recv_text(&mut ws).await;
            let id = cmd["id"].as_u64().unwrap();
            // Send a spurious event, then a result with a mismatched id, then the real result.
            send_text(&mut ws, serde_json::json!({"type": "event", "event": {}})).await;
            send_text(
                &mut ws,
                serde_json::json!({
                    "id": 9999,
                    "type": "result",
                    "success": true,
                    "result": "wrong"
                }),
            )
            .await;
            send_text(
                &mut ws,
                serde_json::json!({
                    "id": id,
                    "type": "result",
                    "success": true,
                    "result": "correct"
                }),
            )
            .await;
        })
        .await;

        let mut client = HaWs::connect(&base_url, "tok").await.unwrap();
        let result = client
            .call("config/entity_registry/list", serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(result, "correct");
        client.close().await;
        handle.await.unwrap();
    }
}
