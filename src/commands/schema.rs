pub fn build_schema() -> serde_json::Value {
    serde_json::json!({
        "clispec": "0.2",
        "name": "ha",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Home Assistant CLI - agent-friendly with structured output and schema introspection",
        "global_args": [
            {
                "name": "--profile",
                "type": "string",
                "required": false,
                "description": "Config profile to use",
                "env": "HA_PROFILE"
            },
            {
                "name": "--output",
                "type": "string",
                "required": false,
                "enum": ["auto", "text", "json"],
                "default": "auto",
                "description": "Output format. auto selects JSON when piped, text in a terminal. Explicit value always wins.",
                "env": "HA_OUTPUT"
            },
            {
                "name": "-o",
                "type": "string",
                "required": false,
                "description": "Alias for --output"
            },
            {
                "name": "--quiet",
                "type": "boolean",
                "required": false,
                "default": false,
                "description": "Suppress non-data output"
            }
        ],
        "commands": [
            {
                "name": "entity get",
                "description": "Get the current state of an entity",
                "mutating": false,
                "args": [
                    {
                        "name": "entity_id",
                        "type": "string",
                        "required": true,
                        "description": "Entity ID (e.g. light.living_room)"
                    }
                ],
                "output_fields": [
                    {"name": "entity_id", "type": "string"},
                    {"name": "state", "type": "string"},
                    {"name": "attributes", "type": "object"},
                    {"name": "last_changed", "type": "string", "description": "ISO 8601 timestamp"},
                    {"name": "last_updated", "type": "string", "description": "ISO 8601 timestamp"}
                ]
            },
            {
                "name": "entity list",
                "description": "List all entities, optionally filtered by domain, state, or count",
                "mutating": false,
                "args": [
                    {
                        "name": "--domain",
                        "type": "string",
                        "required": false,
                        "description": "Filter by domain (e.g. light, switch, sensor)"
                    },
                    {
                        "name": "--state",
                        "type": "string",
                        "required": false,
                        "description": "Filter by state value (e.g. on, off, unavailable)"
                    },
                    {
                        "name": "--limit",
                        "type": "integer",
                        "required": false,
                        "default": 100,
                        "description": "Maximum number of entities to return"
                    },
                    {
                        "name": "--offset",
                        "type": "integer",
                        "required": false,
                        "default": 0,
                        "description": "Number of results to skip (pagination)"
                    },
                    {
                        "name": "--fields",
                        "type": "string",
                        "required": false,
                        "description": "Comma-separated list of fields to include (e.g. entity_id,state)"
                    }
                ],
                "output_fields": [
                    {"name": "items", "type": "array", "description": "Array of entity state objects"},
                    {"name": "total", "type": "integer", "description": "Total number of matching entities before pagination"},
                    {"name": "limit", "type": "integer"},
                    {"name": "offset", "type": "integer"}
                ]
            },
            {
                "name": "entity watch",
                "description": "Stream state changes for an entity (SSE, runs until Ctrl+C)",
                "mutating": false,
                "args": [
                    {
                        "name": "entity_id",
                        "type": "string",
                        "required": true,
                        "description": "Entity ID to watch"
                    }
                ],
                "output_fields": [
                    {"name": "entity_id", "type": "string"},
                    {"name": "new_state", "type": "object | null"},
                    {"name": "old_state", "type": "object | null"}
                ]
            },
            {
                "name": "service call",
                "description": "Call a Home Assistant service. Requires --yes or JSON mode when stdin is not a TTY.",
                "mutating": true,
                "args": [
                    {
                        "name": "service",
                        "type": "string",
                        "required": true,
                        "description": "Service in domain.service format (e.g. light.turn_on)"
                    },
                    {
                        "name": "--entity",
                        "type": "string",
                        "required": false,
                        "description": "Target entity ID"
                    },
                    {
                        "name": "--data",
                        "type": "string",
                        "required": false,
                        "description": "Additional service data as JSON string"
                    },
                    {
                        "name": "--yes",
                        "type": "boolean",
                        "required": false,
                        "default": false,
                        "description": "Skip the confirmation prompt (required when stdin is not a TTY)"
                    }
                ],
                "output_fields": [
                    {"name": "ok", "type": "boolean"},
                    {"name": "data", "type": "array", "description": "Array of affected entity states"}
                ]
            },
            {
                "name": "service list",
                "description": "List available services",
                "mutating": false,
                "args": [
                    {
                        "name": "--domain",
                        "type": "string",
                        "required": false,
                        "description": "Filter by domain"
                    },
                    {
                        "name": "--limit",
                        "type": "integer",
                        "required": false,
                        "default": 100,
                        "description": "Maximum number of domains to return"
                    },
                    {
                        "name": "--offset",
                        "type": "integer",
                        "required": false,
                        "default": 0,
                        "description": "Number of domains to skip (pagination)"
                    },
                    {
                        "name": "--fields",
                        "type": "string",
                        "required": false,
                        "description": "Comma-separated list of fields to include"
                    }
                ],
                "output_fields": [
                    {"name": "items", "type": "array", "description": "Array of service domain objects"},
                    {"name": "total", "type": "integer"},
                    {"name": "limit", "type": "integer"},
                    {"name": "offset", "type": "integer"}
                ]
            },
            {
                "name": "event fire",
                "description": "Fire a Home Assistant event. Requires --yes or JSON mode when stdin is not a TTY.",
                "mutating": true,
                "args": [
                    {
                        "name": "event_type",
                        "type": "string",
                        "required": true,
                        "description": "Event type to fire"
                    },
                    {
                        "name": "--data",
                        "type": "string",
                        "required": false,
                        "description": "Event data as JSON string"
                    },
                    {
                        "name": "--yes",
                        "type": "boolean",
                        "required": false,
                        "default": false,
                        "description": "Skip the confirmation prompt (required when stdin is not a TTY)"
                    }
                ],
                "output_fields": [
                    {"name": "ok", "type": "boolean"},
                    {"name": "data", "type": "object", "description": "Response from Home Assistant"}
                ]
            },
            {
                "name": "event watch",
                "description": "Stream Home Assistant events (SSE, runs until Ctrl+C)",
                "mutating": false,
                "args": [
                    {
                        "name": "event_type",
                        "type": "string",
                        "required": false,
                        "description": "Filter by event type"
                    }
                ],
                "output_fields": [
                    {"name": "event_type", "type": "string"},
                    {"name": "data", "type": "object"},
                    {"name": "time_fired", "type": "string", "description": "ISO 8601 timestamp"}
                ]
            },
            {
                "name": "registry entity list",
                "description": "List registered entities from the Home Assistant entity registry (WebSocket API)",
                "mutating": false,
                "args": [
                    {
                        "name": "--integration",
                        "type": "string",
                        "required": false,
                        "description": "Filter by integration/platform (e.g. hue, zha)"
                    },
                    {
                        "name": "--domain",
                        "type": "string",
                        "required": false,
                        "description": "Filter by domain (e.g. light, switch)"
                    }
                ],
                "output_fields": [
                    {"name": "entity_id", "type": "string"},
                    {"name": "platform", "type": "string"},
                    {"name": "name", "type": "string | null"},
                    {"name": "original_name", "type": "string | null"},
                    {"name": "disabled_by", "type": "string | null"},
                    {"name": "area_id", "type": "string | null"},
                    {"name": "device_id", "type": "string | null"}
                ]
            },
            {
                "name": "registry entity remove",
                "description": "Permanently remove entities from the entity registry. Use --dry-run to preview. Requires --yes when stdin is not a TTY.",
                "mutating": true,
                "args": [
                    {
                        "name": "entity_ids",
                        "type": "string[]",
                        "required": true,
                        "description": "One or more entity IDs to remove"
                    },
                    {
                        "name": "--dry-run",
                        "type": "boolean",
                        "required": false,
                        "default": false,
                        "description": "Print what would be removed without connecting to Home Assistant"
                    },
                    {
                        "name": "--yes",
                        "type": "boolean",
                        "required": false,
                        "default": false,
                        "description": "Skip the confirmation prompt (required when stdin is not a TTY)"
                    }
                ],
                "output_fields": [
                    {"name": "ok", "type": "boolean"},
                    {"name": "data", "type": "array", "description": "Per-entity removal status"},
                    {"name": "entity_id", "type": "string"},
                    {"name": "status", "type": "string", "description": "removed | not_found | error | dry_run"},
                    {"name": "error", "type": "string | null"}
                ]
            },
            {
                "name": "init",
                "description": "Set up credentials interactively. When stdout is not a TTY, prints JSON setup instructions.",
                "mutating": true,
                "args": [
                    {
                        "name": "--profile",
                        "type": "string",
                        "required": false,
                        "description": "Profile to create or update"
                    }
                ],
                "output_fields": [
                    {"name": "configPath", "type": "string", "description": "Absolute path to the config file that will be written"},
                    {"name": "pathResolution", "type": "string", "description": "Description of how the config path is resolved"},
                    {"name": "recommendedPermissions", "type": "string", "description": "Recommended file permissions for the config file"},
                    {"name": "tokenInstructions", "type": "object", "description": "Step-by-step instructions for creating a long-lived access token"},
                    {"name": "requiredFields", "type": "array", "description": "Config keys required in the profile: url, token"},
                    {"name": "example", "type": "object", "description": "Example config file path and format"}
                ]
            },
            {
                "name": "config show",
                "description": "Show current configuration and active profile",
                "mutating": false,
                "args": [],
                "output_fields": [
                    {"name": "config_file", "type": "string"},
                    {"name": "file_exists", "type": "boolean"},
                    {"name": "profiles", "type": "array"},
                    {"name": "env", "type": "object"}
                ]
            },
            {
                "name": "config set",
                "description": "Set a config value in the active profile",
                "mutating": true,
                "args": [
                    {
                        "name": "key",
                        "type": "string",
                        "required": true,
                        "enum": ["url", "token"],
                        "description": "Config key to set"
                    },
                    {
                        "name": "value",
                        "type": "string",
                        "required": true,
                        "description": "Value to set"
                    }
                ],
                "output_fields": [
                    {"name": "ok", "type": "boolean"},
                    {"name": "key", "type": "string"},
                    {"name": "profile", "type": "string"}
                ]
            },
            {
                "name": "schema",
                "description": "Print this machine-readable schema. Use for agent introspection.",
                "mutating": false,
                "args": [],
                "output_fields": []
            },
            {
                "name": "completions",
                "description": "Generate shell completions",
                "mutating": false,
                "args": [
                    {
                        "name": "shell",
                        "type": "string",
                        "required": true,
                        "enum": ["bash", "zsh", "fish", "elvish", "powershell"],
                        "description": "Shell to generate completions for"
                    }
                ],
                "output_fields": []
            }
        ],
        "errors": [
            {
                "kind": "auth",
                "exit_code": 2,
                "retryable": false,
                "description": "Authentication failed. Token is missing, expired, or invalid."
            },
            {
                "kind": "not_found",
                "exit_code": 3,
                "retryable": false,
                "description": "The requested entity, service, or resource does not exist."
            },
            {
                "kind": "connection",
                "exit_code": 4,
                "retryable": true,
                "description": "Could not reach Home Assistant. Check URL and network connectivity."
            },
            {
                "kind": "partial_failure",
                "exit_code": 5,
                "retryable": false,
                "description": "Batch operation: some items succeeded and some failed. See per-item status in data[]."
            },
            {
                "kind": "confirmation_required",
                "exit_code": 6,
                "retryable": false,
                "description": "Destructive command requires --yes when stdin is not a TTY."
            },
            {
                "kind": "conflict",
                "exit_code": 7,
                "retryable": false,
                "description": "Resource exists with a different configuration than requested."
            },
            {
                "kind": "invalid_input",
                "exit_code": 1,
                "retryable": false,
                "description": "Invalid argument, flag value, or JSON input."
            },
            {
                "kind": "api_error",
                "exit_code": 1,
                "retryable": false,
                "description": "Home Assistant returned a non-2xx response."
            },
            {
                "kind": "error",
                "exit_code": 1,
                "retryable": false,
                "description": "General error not covered by a more specific kind."
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

    /// The clispec v0.2 JSON Schema, vendored for offline validation.
    const CLISPEC_SCHEMA_V0_2: &str = include_str!("../../tests/fixtures/clispec-v0.2.json");

    fn validate_against_v0_2(instance: &serde_json::Value) -> Result<(), String> {
        let schema: serde_json::Value = serde_json::from_str(CLISPEC_SCHEMA_V0_2)
            .expect("vendored clispec schema must be valid JSON");
        let validator = jsonschema::draft202012::new(&schema)
            .map_err(|e| format!("vendored schema is not a valid Draft 2020-12 schema: {e}"))?;
        match validator.iter_errors(instance).next() {
            None => Ok(()),
            Some(err) => Err(format!("{}: {}", err.instance_path, err)),
        }
    }

    #[test]
    fn schema_is_valid_json() {
        let schema = build_schema();
        assert!(schema.is_object());
    }

    #[test]
    fn schema_validates_against_clispec_v0_2() {
        let schema = build_schema();
        validate_against_v0_2(&schema)
            .expect("ha schema must validate against clispec v0.2 JSON Schema");
    }

    #[test]
    fn schema_has_clispec_version() {
        let schema = build_schema();
        assert_eq!(schema["clispec"], "0.2");
    }

    #[test]
    fn schema_has_global_args_array() {
        let schema = build_schema();
        let global_args = schema["global_args"].as_array().unwrap();
        let names: Vec<&str> = global_args
            .iter()
            .map(|a| a["name"].as_str().unwrap())
            .collect();
        assert!(
            names.contains(&"--output"),
            "global_args must include --output"
        );
        assert!(
            names.contains(&"--profile"),
            "global_args must include --profile"
        );
        assert!(
            names.contains(&"--quiet"),
            "global_args must include --quiet"
        );
    }

    #[test]
    fn schema_global_args_have_required_type_field() {
        let schema = build_schema();
        let global_args = schema["global_args"].as_array().unwrap();
        for arg in global_args {
            assert!(
                arg.get("type").is_some(),
                "global arg '{}' is missing required 'type' field",
                arg["name"]
            );
        }
    }

    #[test]
    fn schema_output_global_arg_has_auto_default() {
        let schema = build_schema();
        let global_args = schema["global_args"].as_array().unwrap();
        let output_arg = global_args
            .iter()
            .find(|a| a["name"] == "--output")
            .expect("--output must be in global_args");
        assert_eq!(
            output_arg["default"], "auto",
            "--output default must be 'auto' (three-valued flag)"
        );
        let values = output_arg["enum"].as_array().unwrap();
        let value_strings: Vec<&str> = values.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(value_strings.contains(&"auto"));
        assert!(value_strings.contains(&"text"));
        assert!(value_strings.contains(&"json"));
    }

    #[test]
    fn schema_commands_array_has_all_expected_commands() {
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
    fn schema_all_commands_have_mutating_field() {
        let schema = build_schema();
        let commands = schema["commands"].as_array().unwrap();
        for cmd in commands {
            assert!(
                cmd.get("mutating").is_some_and(|m| m.is_boolean()),
                "command '{}' is missing required 'mutating' boolean field",
                cmd["name"]
            );
        }
    }

    #[test]
    fn schema_all_command_args_have_type_field() {
        let schema = build_schema();
        let commands = schema["commands"].as_array().unwrap();
        for cmd in commands {
            if let Some(args) = cmd.get("args").and_then(|a| a.as_array()) {
                for arg in args {
                    assert!(
                        arg.get("type").is_some(),
                        "arg '{}' in command '{}' is missing required 'type' field",
                        arg["name"],
                        cmd["name"]
                    );
                }
            }
        }
    }

    #[test]
    fn schema_errors_array_has_required_kinds() {
        let schema = build_schema();
        let errors = schema["errors"].as_array().unwrap();
        let kinds: Vec<&str> = errors.iter().map(|e| e["kind"].as_str().unwrap()).collect();
        assert!(kinds.contains(&"auth"), "errors must include 'auth' kind");
        assert!(
            kinds.contains(&"not_found"),
            "errors must include 'not_found' kind"
        );
        assert!(
            kinds.contains(&"connection"),
            "errors must include 'connection' kind"
        );
        assert!(
            kinds.contains(&"confirmation_required"),
            "errors must include 'confirmation_required' kind"
        );
        assert!(
            kinds.contains(&"conflict"),
            "errors must include 'conflict' kind"
        );
    }

    #[test]
    fn schema_all_error_kinds_have_exit_code() {
        let schema = build_schema();
        let errors = schema["errors"].as_array().unwrap();
        for error in errors {
            assert!(
                error.get("exit_code").is_some_and(|c| c.is_u64()),
                "error kind '{}' is missing required 'exit_code' field",
                error["kind"]
            );
        }
    }

    #[test]
    fn schema_list_commands_have_pagination_args() {
        let schema = build_schema();
        let commands = schema["commands"].as_array().unwrap();
        let list_commands = ["entity list", "service list"];
        for list_name in list_commands {
            let cmd = commands
                .iter()
                .find(|c| c["name"] == list_name)
                .unwrap_or_else(|| panic!("command '{}' must exist in schema", list_name));
            let args = cmd["args"].as_array().unwrap();
            let arg_names: Vec<&str> = args.iter().map(|a| a["name"].as_str().unwrap()).collect();
            assert!(
                arg_names.contains(&"--limit"),
                "command '{}' must declare --limit",
                list_name
            );
            assert!(
                arg_names.contains(&"--offset"),
                "command '{}' must declare --offset",
                list_name
            );
            assert!(
                arg_names.contains(&"--fields"),
                "command '{}' must declare --fields",
                list_name
            );
        }
    }

    #[test]
    fn schema_mutating_commands_declare_yes_flag() {
        let schema = build_schema();
        let commands = schema["commands"].as_array().unwrap();
        // service call, event fire, and registry entity remove are mutating and confirm.
        let confirming_commands = ["service call", "event fire", "registry entity remove"];
        for cmd_name in confirming_commands {
            let cmd = commands
                .iter()
                .find(|c| c["name"] == cmd_name)
                .unwrap_or_else(|| panic!("command '{}' must exist in schema", cmd_name));
            let args = cmd["args"].as_array().unwrap();
            let arg_names: Vec<&str> = args.iter().map(|a| a["name"].as_str().unwrap()).collect();
            assert!(
                arg_names.contains(&"--yes"),
                "mutating command '{}' must declare --yes flag",
                cmd_name
            );
        }
    }

    #[test]
    fn schema_has_output_fields_on_data_commands() {
        let schema = build_schema();
        let commands = schema["commands"].as_array().unwrap();
        let data_commands = ["entity get", "entity list", "config show"];
        for cmd_name in data_commands {
            let cmd = commands
                .iter()
                .find(|c| c["name"] == cmd_name)
                .unwrap_or_else(|| panic!("command '{}' must exist in schema", cmd_name));
            assert!(
                cmd.get("output_fields")
                    .and_then(|f| f.as_array())
                    .is_some_and(|a| !a.is_empty()),
                "command '{}' must declare non-empty output_fields",
                cmd_name
            );
        }
    }
}
