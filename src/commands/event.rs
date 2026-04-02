use crate::api::{self, HaClient, HaError};
use crate::output::OutputConfig;

pub async fn fire(
    out: &OutputConfig,
    client: &HaClient,
    event_type: &str,
    data: Option<&str>,
) -> Result<(), HaError> {
    let body = if let Some(d) = data {
        Some(
            serde_json::from_str::<serde_json::Value>(d)
                .map_err(|e| HaError::InvalidInput(format!("Invalid JSON data: {e}")))?,
        )
    } else {
        None
    };

    let result = api::events::fire_event(client, event_type, body.as_ref()).await?;

    if out.is_json() {
        out.print_data(
            &serde_json::to_string_pretty(&serde_json::json!({"ok": true, "data": result}))
                .expect("serialize"),
        );
    } else {
        out.print_data(&format!("✔ Fired event: {event_type}"));
    }
    Ok(())
}

pub async fn watch(
    out: &OutputConfig,
    client: &HaClient,
    event_type: Option<&str>,
) -> Result<(), HaError> {
    out.print_message(&format!(
        "Watching events{} (Ctrl+C to stop)...",
        event_type
            .map(|t| format!(": {t}"))
            .unwrap_or_default()
    ));

    api::events::watch_stream(client, event_type, |event| {
        if out.is_json() {
            if let Ok(s) = serde_json::to_string_pretty(
                &serde_json::json!({"ok": true, "data": event}),
            ) {
                println!("{s}");
            }
        } else {
            let time = event.time_fired.as_deref().unwrap_or("-");
            println!("{} {}  {}", time, event.event_type, event.data);
        }
        true
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::HaClient;
    use crate::output::{OutputConfig, OutputFormat};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn json_out() -> OutputConfig {
        OutputConfig::new(Some(OutputFormat::Json), false)
    }

    #[tokio::test]
    async fn fire_succeeds_on_200() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/events/my_event"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"message": "Event my_event fired."})
            ))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        let result = fire(&json_out(), &client, "my_event", None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn fire_with_invalid_json_returns_error() {
        let server = MockServer::start().await;
        let client = HaClient::new(server.uri(), "tok");
        let result = fire(&json_out(), &client, "my_event", Some("{invalid}")).await;
        assert!(matches!(result, Err(crate::api::HaError::InvalidInput(_))));
    }
}
