use crate::api::{HaClient, HaError, ServiceDomain};

pub async fn list_services(client: &HaClient) -> Result<Vec<ServiceDomain>, HaError> {
    let resp = client.get("/api/services").send().await?;
    match resp.status().as_u16() {
        200 => Ok(resp.json().await?),
        401 | 403 => Err(HaError::Auth("Unauthorized".into())),
        status => Err(HaError::Api {
            status,
            message: resp.text().await.unwrap_or_default(),
        }),
    }
}

pub async fn call_service(
    client: &HaClient,
    domain: &str,
    service: &str,
    data: Option<&serde_json::Value>,
) -> Result<serde_json::Value, HaError> {
    let req = client.post(&format!("/api/services/{domain}/{service}"));
    let req = if let Some(d) = data { req.json(d) } else { req };
    let resp = req.send().await?;
    match resp.status().as_u16() {
        200 => Ok(resp.json().await?),
        401 | 403 => Err(HaError::Auth("Unauthorized".into())),
        404 => Err(HaError::NotFound(format!(
            "Service '{domain}.{service}' not found"
        ))),
        status => Err(HaError::Api {
            status,
            message: resp.text().await.unwrap_or_default(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::HaClient;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_services_returns_domains() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/services"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "domain": "light",
                    "services": {
                        "turn_on": {"name": "Turn on", "description": "Turn on a light"},
                        "turn_off": {"name": "Turn off", "description": "Turn off a light"}
                    }
                }
            ])))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        let domains = list_services(&client).await.unwrap();
        assert_eq!(domains.len(), 1);
        assert_eq!(domains[0].domain, "light");
        assert!(domains[0].services.contains_key("turn_on"));
    }

    #[tokio::test]
    async fn call_service_sends_post_with_data() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/services/light/turn_on"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        let result = call_service(
            &client,
            "light",
            "turn_on",
            Some(&serde_json::json!({"entity_id": "light.living_room"})),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn call_service_returns_not_found_on_404() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/services/fake/service"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        let result = call_service(&client, "fake", "service", None).await;
        assert!(matches!(result, Err(crate::api::HaError::NotFound(_))));
    }
}
