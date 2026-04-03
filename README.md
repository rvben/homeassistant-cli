[![CI](https://github.com/rvben/homeassistant-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/rvben/homeassistant-cli/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/homeassistant-cli.svg)](https://crates.io/crates/homeassistant-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![codecov](https://codecov.io/gh/rvben/homeassistant-cli/graph/badge.svg)](https://codecov.io/gh/rvben/homeassistant-cli)

# homeassistant-cli

A command-line interface for [Home Assistant](https://www.home-assistant.io/) -- manage entities, call services, and stream events from your terminal.

## Install

```bash
# From crates.io
cargo install homeassistant-cli

# From GitHub Releases (Linux, macOS)
curl -fsSL https://github.com/rvben/homeassistant-cli/releases/latest/download/ha-$(uname -m)-unknown-linux-gnu.tar.gz | tar xz
```

## Quick Start

```bash
# Interactive setup (creates config with your HA token)
ha init

# List all lights
ha entity list --domain light

# Turn on a light
ha service call light.turn_on --entity light.living_room

# Get the current state of a sensor
ha entity get sensor.temperature

# Stream state changes for an entity
ha entity watch sensor.temperature
```

## Configuration

### Config file

`~/.config/ha/config.toml`

```toml
[default]
url = "http://homeassistant.local:8123"
token = "eyJ..."

[cabin]
url = "http://cabin-ha.local:8123"
token = "eyJ..."
```

Create a long-lived access token in Home Assistant: **Settings > Profile > Long-Lived Access Tokens**.

### Environment variables

| Variable | Description |
|---|---|
| `HA_URL` | Home Assistant URL |
| `HA_TOKEN` | Long-lived access token |
| `HA_PROFILE` | Active profile name (default: `default`) |

### Precedence

CLI flags > environment variables > config file

## Commands

### Entities

| Command | Description |
|---------|-------------|
| `ha entity get <entity_id>` | Get the current state of an entity |
| `ha entity list [--domain <d>] [--state <s>] [--limit <n>]` | List entities with optional filters |
| `ha entity watch <entity_id>` | Stream real-time state changes (SSE) |

### Services

| Command | Description |
|---------|-------------|
| `ha service call <domain.service> [--entity <id>] [--data <json>]` | Call a service |
| `ha service list [--domain <d>]` | List available services |

### Events

| Command | Description |
|---------|-------------|
| `ha event fire <event_type> [--data <json>]` | Fire an event |
| `ha event watch [<event_type>]` | Stream events in real time |

### Configuration & Setup

| Command | Description |
|---------|-------------|
| `ha init [--profile <name>]` | Set up credentials interactively |
| `ha config show` | Show current configuration |
| `ha config set <key> <value>` | Set a config value |
| `ha schema` | Print machine-readable schema of all commands |
| `ha completions <shell>` | Generate shell completions (bash, zsh, fish, elvish, powershell) |

## Shell Completions

```bash
# zsh
ha completions zsh > ~/.zsh/completions/_ha

# bash
ha completions bash > /etc/bash_completion.d/ha

# fish
ha completions fish > ~/.config/fish/completions/ha.fish
```

## Agent Integration

### JSON output

JSON output is automatic when stdout is not a TTY, or forced with `--output json`. Data goes to stdout, messages to stderr.

```bash
ha entity list --domain light --output json | jq '.[].entity_id'
```

### Schema introspection

```bash
ha schema | jq '.commands | keys'
```

The `schema` command outputs a JSON description of all commands, arguments, and output shapes -- enabling AI agents to discover operations without parsing help text.

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | General error |
| `2` | Auth or config error |
| `3` | Entity/resource not found |
| `4` | Connection error |

## License

MIT
