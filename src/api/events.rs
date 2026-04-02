use futures_util::StreamExt;

use crate::api::{HaClient, HaError, HaEvent};

pub async fn fire_event(
    client: &HaClient,
    event_type: &str,
    data: Option<&serde_json::Value>,
) -> Result<serde_json::Value, HaError> {
    let req = client.post(&format!("/api/events/{event_type}"));
    let req = if let Some(d) = data { req.json(d) } else { req };
    let resp = req.send().await?;
    match resp.status().as_u16() {
        200 => Ok(resp.json().await?),
        401 | 403 => Err(HaError::Auth("Unauthorized".into())),
        404 => Err(HaError::NotFound(format!(
            "Event type '{event_type}' not found"
        ))),
        status => Err(HaError::Api {
            status,
            message: resp.text().await.unwrap_or_default(),
        }),
    }
}

/// Parse a single SSE line of the form `data: <json>` into an HaEvent.
pub(crate) fn parse_sse_data(line: &str) -> Option<HaEvent> {
    let json = line.strip_prefix("data: ")?;
    serde_json::from_str(json).ok()
}

/// Stream SSE events from /api/stream, calling `on_event` for each.
/// Returns when `on_event` returns false or the stream ends.
pub async fn watch_stream(
    client: &HaClient,
    restrict: Option<&str>,
    mut on_event: impl FnMut(HaEvent) -> bool,
) -> Result<(), HaError> {
    let url = match restrict {
        Some(r) => format!("{}/api/stream?restrict={}", client.base_url, r),
        None => format!("{}/api/stream", client.base_url),
    };

    let resp = client
        .client
        .get(&url)
        .bearer_auth(client.token())
        .send()
        .await?;

    match resp.status().as_u16() {
        200 => {}
        401 | 403 => return Err(HaError::Auth("Unauthorized".into())),
        status => {
            return Err(HaError::Api {
                status,
                message: resp.text().await.unwrap_or_default(),
            })
        }
    }

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim_end_matches('\r').to_owned();
            buffer.drain(..=pos);
            if let Some(event) = parse_sse_data(&line) && !on_event(event) {
                return Ok(());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::HaClient;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn fire_event_sends_post() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/events/my_event"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"message": "Event my_event fired."})
            ))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        let result = fire_event(&client, "my_event", None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn fire_event_with_data_includes_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/events/custom"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"message": "Event custom fired."})
            ))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        let data = serde_json::json!({"key": "value"});
        let result = fire_event(&client, "custom", Some(&data)).await;
        assert!(result.is_ok());
    }

    #[test]
    fn parse_sse_line_extracts_data() {
        let line = r#"data: {"event_type":"state_changed","data":{},"time_fired":"2026-01-01T00:00:00Z"}"#;
        let event = parse_sse_data(line).unwrap();
        assert_eq!(event.event_type, "state_changed");
    }

    #[test]
    fn parse_sse_line_returns_none_for_non_data_lines() {
        assert!(parse_sse_data("").is_none());
        assert!(parse_sse_data(": ping").is_none());
        assert!(parse_sse_data("event: state_changed").is_none());
    }
}
