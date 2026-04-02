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

    if let Some(entity_id) = entity && let Some(obj) = body.as_object_mut() {
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
        out.print_data(&format!("✔ Called {service}"));
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
        let rows: Vec<Vec<String>> = domains
            .iter()
            .flat_map(|d| {
                d.services.iter().map(|(svc, info)| {
                    vec![
                        format!("{}.{}", d.domain, svc),
                        info.name.clone().unwrap_or_default(),
                        info.description.clone().unwrap_or_default(),
                    ]
                })
            })
            .collect();
        out.print_data(&output::table(&["SERVICE", "NAME", "DESCRIPTION"], &rows));
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
