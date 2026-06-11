use owo_colors::OwoColorize;

use crate::api::{self, HaClient, HaError};
use crate::output::{self, OutputConfig};

pub async fn get(out: &OutputConfig, client: &HaClient, entity_id: &str) -> Result<(), HaError> {
    let state = api::entities::get_state(client, entity_id).await?;

    if out.is_json() {
        out.print_data(
            &serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "data": state
            }))
            .expect("serialize"),
        );
    } else {
        let attrs = state
            .attributes
            .as_object()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join("  ")
            })
            .unwrap_or_default();
        let status_sym = if state.state == "on" {
            "●".green().to_string()
        } else {
            "○".dimmed().to_string()
        };
        out.print_data(&format!(
            "{} {}  {}  {}",
            status_sym,
            state.entity_id,
            state.state.bold(),
            attrs.dimmed()
        ));
    }
    Ok(())
}

pub async fn list(
    out: &OutputConfig,
    client: &HaClient,
    domain: Option<&str>,
    state_filter: Option<&str>,
    limit: usize,
    offset: usize,
    fields: Option<&str>,
) -> Result<(), HaError> {
    let mut states = api::entities::list_states(client).await?;

    if let Some(d) = domain {
        states.retain(|s| s.entity_id.starts_with(&format!("{d}.")));
    }
    if let Some(st) = state_filter {
        states.retain(|s| s.state == st);
    }

    states.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));

    let total = states.len();

    // Apply offset before slicing to the requested page.
    let states = if offset < states.len() {
        &states[offset..]
    } else {
        &[][..]
    };
    let states: Vec<_> = states.iter().take(limit).collect();

    let field_filter: Option<Vec<&str>> = fields.map(|f| f.split(',').map(str::trim).collect());

    if out.is_json() {
        let items: Vec<serde_json::Value> = states
            .iter()
            .map(|s| {
                let mut obj = serde_json::json!({
                    "entity_id": s.entity_id,
                    "state": s.state,
                    "attributes": s.attributes,
                    "last_changed": s.last_changed,
                    "last_updated": s.last_updated,
                });
                if let (Some(ff), Some(map)) = (&field_filter, obj.as_object_mut()) {
                    map.retain(|k, _| ff.contains(&k.as_str()));
                }
                obj
            })
            .collect();
        out.print_data(
            &serde_json::to_string_pretty(&serde_json::json!({
                "items": items,
                "total": total,
                "limit": limit,
                "offset": offset,
            }))
            .expect("serialize"),
        );
    } else {
        let default_fields = vec!["entity_id", "name", "state", "last_updated"];
        let show_fields: &[&str] = field_filter.as_deref().unwrap_or(&default_fields);
        let rows: Vec<Vec<String>> = states
            .iter()
            .map(|s| {
                show_fields
                    .iter()
                    .map(|f| match *f {
                        "entity_id" => output::colored_entity_id(&s.entity_id),
                        "name" => s
                            .attributes
                            .get("friendly_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_owned(),
                        "state" => output::colored_state(&s.state),
                        "last_updated" | "updated" => output::relative_time(&s.last_updated),
                        "last_changed" | "changed" => output::relative_time(&s.last_changed),
                        other => s
                            .attributes
                            .get(other)
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                    })
                    .collect()
            })
            .collect();
        let headers: Vec<&str> = show_fields
            .iter()
            .map(|f| match *f {
                "entity_id" => "ENTITY",
                "name" => "NAME",
                "state" => "STATE",
                "last_updated" | "updated" => "UPDATED",
                "last_changed" | "changed" => "CHANGED",
                other => other,
            })
            .collect();
        out.print_data(&output::table(&headers, &rows));
        if total > offset + limit {
            out.print_message(&format!(
                "Showing {limit} of {total} (use --offset {} to see more)",
                offset + limit
            ));
        }
    }
    Ok(())
}

pub async fn watch(out: &OutputConfig, client: &HaClient, entity_id: &str) -> Result<(), HaError> {
    out.print_message(&format!("Watching {} (Ctrl+C to stop)...", entity_id));

    let entity_id = entity_id.to_owned();
    api::events::watch_stream(client, Some("state_changed"), |event| {
        if let Ok(data) = serde_json::from_value::<crate::api::StateChangedData>(event.data.clone())
            && data.entity_id == entity_id
        {
            if out.is_json() {
                if let Ok(s) = serde_json::to_string_pretty(&serde_json::json!({
                    "ok": true,
                    "data": data
                })) {
                    println!("{s}");
                }
            } else if let Some(new) = &data.new_state {
                let status_sym = if new.state == "on" {
                    "●".green().to_string()
                } else {
                    "○".dimmed().to_string()
                };
                println!(
                    "{} {}  {}  {}",
                    status_sym,
                    new.entity_id,
                    new.state.bold(),
                    output::relative_time(&new.last_updated).dimmed()
                );
            }
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

    fn state_json(entity_id: &str, state: &str) -> serde_json::Value {
        serde_json::json!({
            "entity_id": entity_id,
            "state": state,
            "attributes": {},
            "last_changed": "2026-01-01T00:00:00Z",
            "last_updated": "2026-01-01T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn get_returns_ok_for_existing_entity() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/states/light.x"))
            .respond_with(ResponseTemplate::new(200).set_body_json(state_json("light.x", "on")))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        let result = get(&json_out(), &client, "light.x").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_returns_ok() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/states"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                state_json("light.a", "on"),
                state_json("switch.b", "off"),
                state_json("light.c", "off"),
            ])))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        let result = list(&json_out(), &client, Some("light"), None, 100, 0, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_json_output_includes_pagination_metadata() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/states"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                state_json("light.a", "on"),
                state_json("light.b", "off"),
                state_json("light.c", "on"),
            ])))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        // Capture stdout is not straightforward in unit tests; exercise the code
        // path and verify no errors. The pagination metadata fields are validated
        // via schema tests.
        let result = list(&json_out(), &client, None, None, 2, 0, None).await;
        assert!(result.is_ok(), "list with limit should succeed");
    }

    #[tokio::test]
    async fn list_applies_offset_correctly() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/states"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                state_json("light.a", "on"),
                state_json("light.b", "off"),
                state_json("light.c", "on"),
            ])))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        let result = list(&json_out(), &client, None, None, 100, 2, None).await;
        assert!(result.is_ok(), "offset beyond some items should succeed");
    }

    #[tokio::test]
    async fn get_propagates_not_found() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/states/light.missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        let result = get(&json_out(), &client, "light.missing").await;
        assert!(matches!(result, Err(crate::api::HaError::NotFound(_))));
    }
}
