use crate::api::{EntityState, HaClient, HaError};

pub async fn get_state(client: &HaClient, entity_id: &str) -> Result<EntityState, HaError> {
    let resp = client
        .get(&format!("/api/states/{entity_id}"))
        .send()
        .await?;
    match resp.status().as_u16() {
        200 => Ok(resp.json().await?),
        401 | 403 => Err(HaError::Auth(format!("Unauthorized accessing {entity_id}"))),
        404 => Err(HaError::NotFound(format!("Entity '{entity_id}' not found"))),
        status => Err(HaError::Api {
            status,
            message: resp.text().await.unwrap_or_default(),
        }),
    }
}

pub async fn list_states(client: &HaClient) -> Result<Vec<EntityState>, HaError> {
    let resp = client.get("/api/states").send().await?;
    match resp.status().as_u16() {
        200 => Ok(resp.json().await?),
        401 | 403 => Err(HaError::Auth("Unauthorized".into())),
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
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn mock_client(server: &MockServer) -> HaClient {
        HaClient::new(server.uri(), "test-token")
    }

    #[tokio::test]
    async fn get_state_returns_entity() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/states/light.living_room"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "entity_id": "light.living_room",
                "state": "on",
                "attributes": {"brightness": 128},
                "last_changed": "2026-01-01T00:00:00Z",
                "last_updated": "2026-01-01T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let state = get_state(&client, "light.living_room").await.unwrap();
        assert_eq!(state.entity_id, "light.living_room");
        assert_eq!(state.state, "on");
    }

    #[tokio::test]
    async fn get_state_returns_not_found_on_404() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/states/light.missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let result = get_state(&client, "light.missing").await;
        assert!(matches!(result, Err(crate::api::HaError::NotFound(_))));
    }

    #[tokio::test]
    async fn list_states_returns_all_entities() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/states"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {"entity_id": "light.x", "state": "on", "attributes": {}, "last_changed": "2026-01-01T00:00:00Z", "last_updated": "2026-01-01T00:00:00Z"},
                {"entity_id": "switch.y", "state": "off", "attributes": {}, "last_changed": "2026-01-01T00:00:00Z", "last_updated": "2026-01-01T00:00:00Z"}
            ])))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let states = list_states(&client).await.unwrap();
        assert_eq!(states.len(), 2);
    }

    #[tokio::test]
    async fn get_state_returns_auth_error_on_401() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/states/light.x"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = mock_client(&server).await;
        let result = get_state(&client, "light.x").await;
        assert!(matches!(result, Err(crate::api::HaError::Auth(_))));
    }
}
