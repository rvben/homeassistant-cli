# Home Assistant CLI — Design Spec

**Date:** 2026-04-02  
**Status:** Approved

## Goal

A Rust CLI (`ha`) for Home Assistant that is great for humans and excellent for agents. The differentiating feature is **agent-first design as a first-class citizen**: structured output, stable schemas, semantic exit codes, and a `ha schema` command that lets agents self-orient without hallucinating flags. Humans get beautiful, colored output. Neither audience is compromised for the other.

This is the third CLI in the `shelly`/`unifi` family, following the same conventions.

---

## Command Structure

Noun-verb subcommands mapping directly to HA's REST structure:

```
ha entity get <entity_id>
ha entity list [--domain <domain>]
ha entity watch <entity_id>

ha service call <domain.service> [--entity <id>] [--data <json>]
ha service list [--domain <domain>]

ha event fire <event_type> [--data <json>]
ha event watch [<event_type>]

ha schema
ha init [--profile <name>]
ha config show [--profile <name>]
ha config set <key> <value> [--profile <name>]
```

Global flags: `--profile <name>`, `--output json|yaml|table|plain`.

Scope is intentionally v1-only: entity state, service calls, events. Automations, history, and config management are out of scope and will be added in future versions.

---

## Configuration & Authentication

**Config file:** `~/.config/ha/config.toml`

```toml
[default]
url = "http://homeassistant.local:8123"
token = "eyJ..."

[profiles.prod]
url = "https://ha.example.com"
token = "eyJ..."
```

**Environment variables** override config file per-field:
- `HA_URL` — overrides `url`
- `HA_TOKEN` — overrides `token`
- `HA_PROFILE` — overrides active profile

Profile selection via `--profile <name>` flag or `HA_PROFILE` env var. Env vars take precedence over config file values.

Connection is validated lazily on first command, not at startup.

---

## `ha init` Command

Follows the pattern established in zoom-cli and jira-cli:

- **Entry point** detects TTY: if stdout is not a terminal, print JSON schema (config path, token creation instructions, required fields, example config) and exit.
- **Interactive flow** via injectable `run_init<R, W, Fut>` for full testability.
- **Context-aware prompting:**
  - First setup (no config): silently default to "default" profile, prompt URL + token.
  - Config exists, no `--profile`: show existing profiles, ask update/add.
  - `--profile` given: update if exists, create new if not.
- **Credential masking** on updates: show masked current value, "Enter to keep".
- **Inline validation**: call HA's `/api/` endpoint after entry. Show `✔ Connected as <name>` or `✖ Could not validate` with save-anyway prompt.
- **Graceful EOF**: abort cleanly at any prompt.
- **Outro**: print config path + `ha entity list` as the suggested next command. Include `--profile` flag in the example for non-default profiles.
- **JSON mode output** includes steps to create a long-lived token (HA UI: Settings → Profile → Long-Lived Access Tokens).

---

## Output System

**Auto-detection:** TTY → `table`/`plain` with colors. Pipe → `json`. Always overridable with `--output`.

**Human output** — colored, compact:
```
● light.living_room      on    brightness: 128
● sensor.temperature     17.4°C
● switch.garden_lights   off
```

**JSON envelope** — stable across versions:
```json
{ "ok": true, "data": { ... } }
{ "ok": false, "error": { "code": "HA_ENTITY_NOT_FOUND", "message": "..." } }
```

**Rules:**
- Errors always go to stderr, never mixed with stdout.
- Exit codes are semantic: `0` success, `1` general error, `2` auth failure, `3` not found, `4` connection error.
- `ha schema` returns a JSON document describing every command, its flags, argument types, and the exact shape of its `data` field.

---

## Architecture

```
src/
  main.rs          # CLI definition (clap), dispatch, global flags
  config.rs        # Config read/write, profile resolution, env var override
  output.rs        # OutputConfig, print_data/print_error, TTY detection, mask_credential
  lib.rs           # Re-exports for integration tests
  api/
    mod.rs         # HaClient, error types with codes
    entities.rs    # get, list, watch (SSE)
    services.rs    # call, list
    events.rs      # fire, watch (SSE)
  commands/
    mod.rs
    init.rs        # run_init<R,W,Fut> + interactive + JSON schema mode
    entity.rs      # get, list, watch
    service.rs     # call, list
    event.rs       # fire, watch
    schema.rs      # ha schema — static JSON description of all commands
    config.rs      # ha config show/set
  test_support.rs  # shared helpers (fake client, temp config, etc.)
```

**Key crates:** `clap`, `reqwest`, `tokio`, `serde_json`, `toml`, `owo-colors`, `tabled`, `eventsource-client`.

---

## Testing

- `run_init` and all interactive flows tested via injectable `Cursor` + `TempDir`.
- API layer tested via a fake `HaClient` from `test_support.rs`.
- Schema output tested to ensure stability (no accidental shape changes).
- Use `cargo nextest` for all test runs.
