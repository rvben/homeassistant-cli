pub fn build_schema() -> serde_json::Value {
    serde_json::json!({
        "name": "ha",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Home Assistant CLI — agent-friendly with structured output and schema introspection",
        "global_flags": [
            {"name": "--profile", "env": "HA_PROFILE", "description": "Config profile to use"},
            {"name": "--output", "values": ["json", "table", "plain"], "env": "HA_OUTPUT", "description": "Output format (auto: json when piped, table in TTY)"},
            {"name": "--quiet", "description": "Suppress non-data output"}
        ],
        "error_envelope": {"ok": false, "error": {"code": "string", "message": "string"}},
        "exit_codes": {
            "0": "success",
            "1": "general error",
            "2": "auth/config error",
            "3": "not found",
            "4": "connection error"
        },
        "commands": [
            {
                "name": "entity get",
                "description": "Get the current state of an entity",
                "args": [{"name": "entity_id", "required": true, "description": "Entity ID (e.g. light.living_room)"}],
                "json_shape": {
                    "ok": true,
                    "data": {
                        "entity_id": "string",
                        "state": "string",
                        "attributes": "object",
                        "last_changed": "ISO 8601 timestamp",
                        "last_updated": "ISO 8601 timestamp"
                    }
                }
            },
            {
                "name": "entity list",
                "description": "List all entities, optionally filtered by domain",
                "flags": [{"name": "--domain", "description": "Filter by domain (e.g. light, switch, sensor)"}],
                "json_shape": {
                    "ok": true,
                    "data": [{"entity_id": "string", "state": "string", "attributes": "object", "last_changed": "string", "last_updated": "string"}]
                }
            },
            {
                "name": "entity watch",
                "description": "Stream state changes for an entity (SSE, runs until Ctrl+C)",
                "args": [{"name": "entity_id", "required": true}],
                "json_shape": {
                    "ok": true,
                    "data": {"entity_id": "string", "new_state": "EntityState | null", "old_state": "EntityState | null"}
                }
            },
            {
                "name": "service call",
                "description": "Call a Home Assistant service",
                "args": [{"name": "service", "required": true, "description": "Service in domain.service format (e.g. light.turn_on)"}],
                "flags": [
                    {"name": "--entity", "description": "Target entity ID"},
                    {"name": "--data", "description": "Additional service data as JSON string"}
                ],
                "json_shape": {"ok": true, "data": "array of affected states"}
            },
            {
                "name": "service list",
                "description": "List available services",
                "flags": [{"name": "--domain", "description": "Filter by domain"}],
                "json_shape": {
                    "ok": true,
                    "data": [{"domain": "string", "services": {"service_name": {"name": "string", "description": "string"}}}]
                }
            },
            {
                "name": "event fire",
                "description": "Fire a Home Assistant event",
                "args": [{"name": "event_type", "required": true}],
                "flags": [{"name": "--data", "description": "Event data as JSON string"}],
                "json_shape": {"ok": true, "data": {"message": "string"}}
            },
            {
                "name": "event watch",
                "description": "Stream Home Assistant events (SSE, runs until Ctrl+C)",
                "args": [{"name": "event_type", "required": false, "description": "Filter by event type"}],
                "json_shape": {
                    "ok": true,
                    "data": {"event_type": "string", "data": "object", "time_fired": "ISO 8601 timestamp"}
                }
            },
            {
                "name": "init",
                "description": "Set up credentials interactively. When stdout is not a TTY, prints JSON setup instructions.",
                "flags": [{"name": "--profile", "description": "Profile to create or update"}]
            },
            {
                "name": "config show",
                "description": "Show current configuration and active profile"
            },
            {
                "name": "config set",
                "description": "Set a config value in the active profile",
                "args": [
                    {"name": "key", "required": true, "description": "Config key: url or token"},
                    {"name": "value", "required": true}
                ]
            },
            {
                "name": "schema",
                "description": "Print this machine-readable schema. Use for agent introspection.",
                "json_shape": "this document"
            }
        ]
    })
}

pub fn print_schema() {
    println!(
        "{}",
        serde_json::to_string_pretty(&build_schema()).expect("serialize")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_valid_json() {
        let schema = build_schema();
        assert!(schema.is_object());
    }

    #[test]
    fn schema_has_expected_commands() {
        let schema = build_schema();
        let commands = schema["commands"].as_array().unwrap();
        let names: Vec<&str> = commands
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"entity get"));
        assert!(names.contains(&"entity list"));
        assert!(names.contains(&"entity watch"));
        assert!(names.contains(&"service call"));
        assert!(names.contains(&"service list"));
        assert!(names.contains(&"event fire"));
        assert!(names.contains(&"event watch"));
        assert!(names.contains(&"schema"));
        assert!(names.contains(&"init"));
        assert!(names.contains(&"config show"));
        assert!(names.contains(&"config set"));
    }

    #[test]
    fn schema_entity_get_has_json_shape() {
        let schema = build_schema();
        let commands = schema["commands"].as_array().unwrap();
        let entity_get = commands
            .iter()
            .find(|c| c["name"] == "entity get")
            .unwrap();
        assert!(entity_get["json_shape"]["data"]["entity_id"].is_string());
        assert!(entity_get["json_shape"]["data"]["state"].is_string());
    }

    #[test]
    fn schema_includes_global_flags() {
        let schema = build_schema();
        let globals = schema["global_flags"].as_array().unwrap();
        let flag_names: Vec<&str> = globals
            .iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        assert!(flag_names.contains(&"--output"));
        assert!(flag_names.contains(&"--profile"));
        assert!(flag_names.contains(&"--quiet"));
    }
}
