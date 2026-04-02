# homeassistant-cli

Agent-friendly CLI for Home Assistant with JSON output, structured exit codes, and schema introspection.

## Installation

```bash
# via cargo
cargo install homeassistant-cli

# via Homebrew
brew install --formula https://raw.githubusercontent.com/rvben/homeassistant-cli/main/Formula/homeassistant-cli.rb
```

## Configuration

Run `ha init` to set up credentials interactively, or create `~/.config/ha/config.toml`:

```toml
[default]
url = "http://homeassistant.local:8123"
token = "eyJ..."
```

Create a long-lived access token in Home Assistant: **Settings → Profile → Long-Lived Access Tokens**.

Environment variables override the config file:
- `HA_URL` — Home Assistant URL
- `HA_TOKEN` — Long-lived access token
- `HA_PROFILE` — Active profile name

## Usage

```
ha entity get <entity_id>
ha entity list [--domain <domain>] [--state <state>] [--limit <n>]
ha entity watch <entity_id>

ha service call <domain.service> [--entity <id>] [--data <json>]
ha service list [--domain <domain>]

ha event fire <event_type> [--data <json>]
ha event watch [<event_type>]

ha schema
ha completions <bash|zsh|fish|elvish|powershell>
ha init [--profile <name>]
ha config show
ha config set <key> <value>
```

## Shell Completions

```bash
# zsh
ha completions zsh > ~/.zsh/completions/_ha

# bash
ha completions bash > /etc/bash_completion.d/ha

# fish
ha completions fish > ~/.config/fish/completions/ha.fish
```

## Agent Use

Output is JSON when stdout is not a terminal, or with `--output json`:

```json
{ "ok": true, "data": { ... } }
{ "ok": false, "error": { "code": "HA_NOT_FOUND", "message": "..." } }
```

Exit codes: `0` success, `1` error, `2` auth/config, `3` not found, `4` connection.

Run `ha schema` for a full machine-readable description of all commands, flags, and output shapes.

## License

MIT
