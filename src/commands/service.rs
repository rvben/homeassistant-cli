use owo_colors::OwoColorize;

use crate::api::{self, HaClient, HaError};
use crate::output::{self, OutputConfig};

pub async fn call(
    out: &OutputConfig,
    client: &HaClient,
    service: &str,
    entity: Option<&str>,
    data: Option<&str>,
) -> Result<(), HaError> {
    let (domain, svc) = service.split_once('.').ok_or_else(|| {
        HaError::InvalidInput(format!(
            "Service must be in 'domain.service' format, got '{service}'"
        ))
    })?;

    let mut body = if let Some(d) = data {
        serde_json::from_str::<serde_json::Value>(d)
            .map_err(|e| HaError::InvalidInput(format!("Invalid JSON data: {e}")))?
    } else {
        serde_json::json!({})
    };

    if let Some(entity_id) = entity
        && let Some(obj) = body.as_object_mut()
    {
        obj.insert(
            "entity_id".into(),
            serde_json::Value::String(entity_id.to_owned()),
        );
    }

    let result = api::services::call_service(client, domain, svc, Some(&body)).await?;

    if out.is_json() {
        out.print_data(
            &serde_json::to_string_pretty(&serde_json::json!({"ok": true, "data": result}))
                .expect("serialize"),
        );
    } else {
        let affected: Vec<String> = result
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| {
                        let id = s.get("entity_id")?.as_str()?;
                        let state = s.get("state")?.as_str().unwrap_or("?");
                        Some(format!(
                            "{}  {}",
                            output::colored_entity_id(id),
                            output::colored_state(state)
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();

        if affected.is_empty() {
            out.print_data(&format!("✔ Called {}", service.bold()));
        } else {
            out.print_data(&format!("✔ Called {}", service.bold()));
            for line in &affected {
                out.print_data(&format!("  {line}"));
            }
        }
    }
    Ok(())
}

pub async fn list(
    out: &OutputConfig,
    client: &HaClient,
    domain: Option<&str>,
) -> Result<(), HaError> {
    let mut domains = api::services::list_services(client).await?;

    if let Some(d) = domain {
        domains.retain(|dom| dom.domain == d);
    }

    domains.sort_by(|a, b| a.domain.cmp(&b.domain));

    if out.is_json() {
        out.print_data(
            &serde_json::to_string_pretty(&serde_json::json!({"ok": true, "data": domains}))
                .expect("serialize"),
        );
    } else {
        let mut rows: Vec<Vec<String>> = domains
            .iter()
            .flat_map(|d| {
                d.services.iter().map(|(svc, info)| {
                    vec![
                        output::colored_entity_id(&format!("{}.{}", d.domain, svc)),
                        info.name.clone().unwrap_or_default(),
                        info.description.clone().unwrap_or_default(),
                    ]
                })
            })
            .collect();
        rows.sort_by(|a, b| a[0].cmp(&b[0]));

        // Only show NAME and DESCRIPTION columns if any row has non-empty values.
        let has_names = rows.iter().any(|r| !r[1].is_empty());
        let has_descriptions = rows.iter().any(|r| !r[2].is_empty());

        let (headers, display_rows): (&[&str], Vec<Vec<String>>) =
            match (has_names, has_descriptions) {
                (true, true) => (&["SERVICE", "NAME", "DESCRIPTION"], rows),
                (true, false) => (
                    &["SERVICE", "NAME"],
                    rows.into_iter()
                        .map(|mut r| {
                            r.pop();
                            r
                        })
                        .collect(),
                ),
                _ => (
                    &["SERVICE"],
                    rows.into_iter()
                        .map(|r| vec![r.into_iter().next().unwrap()])
                        .collect(),
                ),
            };
        out.print_data(&output::table(headers, &display_rows));
    }
    Ok(())
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
    async fn call_parses_domain_service_format() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/services/light/turn_on"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        let result = call(
            &json_out(),
            &client,
            "light.turn_on",
            Some("light.living_room"),
            None,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn call_returns_error_on_invalid_service_format() {
        let server = MockServer::start().await;
        let client = HaClient::new(server.uri(), "tok");
        let result = call(&json_out(), &client, "invalid_format", None, None).await;
        assert!(matches!(result, Err(crate::api::HaError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn list_returns_all_domains() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/services"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {"domain": "light", "services": {"turn_on": {"name": "Turn on", "description": "Turn on"}}},
                {"domain": "switch", "services": {"turn_off": {"name": "Turn off", "description": "Turn off"}}}
            ])))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        let result = list(&json_out(), &client, None).await;
        assert!(result.is_ok());
    }
}
