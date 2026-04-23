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
            "4": "connection error",
            "5": "partial failure (some items in a batch succeeded, some failed)"
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
                "description": "List all entities, optionally filtered by domain, state, or count",
                "flags": [
                    {"name": "--domain", "description": "Filter by domain (e.g. light, switch, sensor)"},
                    {"name": "--state", "description": "Filter by state value (e.g. on, off, unavailable)"},
                    {"name": "--limit", "description": "Maximum number of entities to return"}
                ],
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
                "name": "registry entity list",
                "description": "List registered entities from the Home Assistant entity registry (WebSocket API)",
                "flags": [
                    {"name": "--integration", "description": "Filter by integration/platform (e.g. hue, zha)"},
                    {"name": "--domain", "description": "Filter by domain (e.g. light, switch)"}
                ],
                "json_shape": {
                    "ok": true,
                    "data": [{
                        "entity_id": "string",
                        "platform": "string",
                        "name": "string | null",
                        "original_name": "string | null",
                        "disabled_by": "string | null",
                        "area_id": "string | null",
                        "device_id": "string | null"
                    }]
                }
            },
            {
                "name": "registry entity remove",
                "description": "Permanently remove entities from the entity registry. --dry-run never connects. In a TTY, requires --yes to bypass the confirmation prompt.",
                "args": [{"name": "entity_ids", "required": true, "description": "One or more entity IDs to remove"}],
                "flags": [
                    {"name": "--dry-run", "description": "Print what would be removed without connecting to Home Assistant"},
                    {"name": "--yes", "description": "Skip the interactive confirmation prompt"}
                ],
                "exit_codes": {"5": "one or more removals failed; see per-entity status in data[]"},
                "json_shape": {
                    "ok": "bool (true only when every removal succeeded)",
                    "data": [{
                        "entity_id": "string",
                        "status": "removed | not_found | error | dry_run",
                        "error": "string (only present when status is not_found or error)"
                    }]
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
            },
            {
                "name": "completions",
                "description": "Generate shell completions",
                "args": [{"name": "shell", "required": true, "values": ["bash", "zsh", "fish", "elvish", "powershell"]}]
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
        assert!(names.contains(&"registry entity list"));
        assert!(names.contains(&"registry entity remove"));
        assert!(names.contains(&"schema"));
        assert!(names.contains(&"init"));
        assert!(names.contains(&"config show"));
        assert!(names.contains(&"config set"));
    }

    #[test]
    fn schema_entity_get_has_json_shape() {
        let schema = build_schema();
        let commands = schema["commands"].as_array().unwrap();
        let entity_get = commands.iter().find(|c| c["name"] == "entity get").unwrap();
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
