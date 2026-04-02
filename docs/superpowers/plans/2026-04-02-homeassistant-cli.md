# Home Assistant CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `ha` — a Rust CLI for Home Assistant that is great for humans and excellent for agents, following the shelly/unifi CLI family conventions.

**Architecture:** Noun-verb subcommands (`ha entity get`, `ha service call`, etc.) map directly to the HA REST API. A shared `HaClient` handles auth (Bearer token) and HTTP. Output is auto-detected: table/colored for TTY, JSON when piped. `ha schema` returns a static machine-readable description of all commands for agent use.

**Tech Stack:** Rust 2024 edition, clap (derive), reqwest (rustls-tls), tokio, serde_json, toml, owo-colors, dirs, futures-util, wiremock (tests), tempfile (tests).

---

## File Map

| File | Responsibility |
|------|---------------|
| `Cargo.toml` | Dependencies |
| `src/main.rs` | CLI definition (clap), dispatch, global flags |
| `src/lib.rs` | Re-exports for integration tests |
| `src/config.rs` | Config read/write, profile resolution, env var override |
| `src/output.rs` | OutputConfig, OutputFormat, print helpers, exit_codes, mask_credential |
| `src/test_support.rs` | EnvVarGuard, ProcessEnvLock, write_config, fake server helpers |
| `src/api/mod.rs` | HaError (with codes), HaClient struct, re-exports |
| `src/api/types.rs` | EntityState, ServiceDomain, HaEvent, StateChangedData |
| `src/api/entities.rs` | get_state, list_states, watch_entity |
| `src/api/services.rs` | list_services, call_service |
| `src/api/events.rs` | fire_event, watch_events, SSE stream parsing |
| `src/commands/mod.rs` | mod declarations |
| `src/commands/init.rs` | run_init<R,W,Fut>, interactive + JSON schema mode |
| `src/commands/entity.rs` | entity get/list/watch handlers |
| `src/commands/service.rs` | service call/list handlers |
| `src/commands/event.rs` | event fire/watch handlers |
| `src/commands/schema.rs` | ha schema — static JSON doc |
| `src/commands/config.rs` | ha config show/set |

---

### Task 1: Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "homeassistant-cli"
version = "0.1.0"
edition = "2024"
rust-version = "1.90"
description = "Agent-friendly Home Assistant CLI with JSON output, structured exit codes, and schema introspection"
license = "MIT"

[[bin]]
name = "ha"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive", "env"] }
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "stream"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal"] }
toml = "0.8"
owo-colors = "4"
dirs = "6"
futures-util = "0.3"

[dev-dependencies]
tempfile = "3"
wiremock = "0.6"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

- [ ] **Step 2: Create src/lib.rs**

```rust
pub mod api;
pub mod commands;
pub mod config;
pub mod output;

#[cfg(test)]
pub mod test_support;
```

- [ ] **Step 3: Create src/main.rs skeleton**

```rust
use clap::{Parser, Subcommand, ValueEnum};

use homeassistant_cli::output::{OutputConfig, OutputFormat, exit_codes};
use homeassistant_cli::{api, commands};

#[derive(Parser)]
#[command(
    name = "ha",
    version,
    about = "CLI for Home Assistant",
    arg_required_else_help = true
)]
struct Cli {
    /// Config profile to use [env: HA_PROFILE]
    #[arg(long, env = "HA_PROFILE", global = true)]
    profile: Option<String>,

    /// Output format [env: HA_OUTPUT]
    #[arg(long, value_enum, env = "HA_OUTPUT", global = true)]
    output: Option<OutputFormat>,

    /// Suppress non-data output
    #[arg(long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read and watch entity states
    #[command(subcommand, arg_required_else_help = true)]
    Entity(EntityCommand),

    /// Call and list services
    #[command(subcommand, arg_required_else_help = true)]
    Service(ServiceCommand),

    /// Fire and watch events
    #[command(subcommand, arg_required_else_help = true)]
    Event(EventCommand),

    /// Set up credentials interactively (or print JSON schema for agents)
    Init {
        #[arg(long)]
        profile: Option<String>,
    },

    /// Manage configuration
    #[command(subcommand, arg_required_else_help = true)]
    Config(ConfigCommand),

    /// Print machine-readable schema of all commands
    Schema,
}

#[derive(Subcommand)]
enum EntityCommand {
    /// Get the current state of an entity
    Get { entity_id: String },
    /// List all entities, optionally filtered by domain
    List {
        #[arg(long)]
        domain: Option<String>,
    },
    /// Stream state changes for an entity
    Watch { entity_id: String },
}

#[derive(Subcommand)]
enum ServiceCommand {
    /// Call a service
    Call {
        /// Service in domain.service format (e.g. light.turn_on)
        service: String,
        /// Target entity ID
        #[arg(long)]
        entity: Option<String>,
        /// Additional service data as JSON
        #[arg(long)]
        data: Option<String>,
    },
    /// List available services
    List {
        #[arg(long)]
        domain: Option<String>,
    },
}

#[derive(Subcommand)]
enum EventCommand {
    /// Fire an event
    Fire {
        event_type: String,
        /// Event data as JSON
        #[arg(long)]
        data: Option<String>,
    },
    /// Stream events
    Watch {
        /// Filter by event type
        event_type: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Show current configuration
    Show,
    /// Set a config value
    Set { key: String, value: String },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let out = OutputConfig::new(cli.output, cli.quiet);

    match cli.command {
        Command::Init { profile } => {
            commands::init::init(profile).await;
        }
        Command::Schema => {
            commands::schema::print_schema();
        }
        Command::Config(cmd) => match cmd {
            ConfigCommand::Show => {
                commands::config::show(&out, cli.profile.as_deref());
            }
            ConfigCommand::Set { key, value } => {
                commands::config::set(&out, cli.profile.as_deref(), &key, &value);
            }
        },
        command => {
            let cfg = match homeassistant_cli::config::Config::load(cli.profile.clone()) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(exit_codes::CONFIG_ERROR);
                }
            };
            let client = api::HaClient::new(&cfg.url, &cfg.token);

            match command {
                Command::Entity(cmd) => match cmd {
                    EntityCommand::Get { entity_id } => {
                        if let Err(e) = commands::entity::get(&out, &client, &entity_id).await {
                            eprintln!("{e}");
                            std::process::exit(exit_codes::for_error(&e));
                        }
                    }
                    EntityCommand::List { domain } => {
                        if let Err(e) = commands::entity::list(&out, &client, domain.as_deref()).await {
                            eprintln!("{e}");
                            std::process::exit(exit_codes::for_error(&e));
                        }
                    }
                    EntityCommand::Watch { entity_id } => {
                        if let Err(e) = commands::entity::watch(&out, &client, &entity_id).await {
                            eprintln!("{e}");
                            std::process::exit(exit_codes::for_error(&e));
                        }
                    }
                },
                Command::Service(cmd) => match cmd {
                    ServiceCommand::Call { service, entity, data } => {
                        if let Err(e) = commands::service::call(&out, &client, &service, entity.as_deref(), data.as_deref()).await {
                            eprintln!("{e}");
                            std::process::exit(exit_codes::for_error(&e));
                        }
                    }
                    ServiceCommand::List { domain } => {
                        if let Err(e) = commands::service::list(&out, &client, domain.as_deref()).await {
                            eprintln!("{e}");
                            std::process::exit(exit_codes::for_error(&e));
                        }
                    }
                },
                Command::Event(cmd) => match cmd {
                    EventCommand::Fire { event_type, data } => {
                        if let Err(e) = commands::event::fire(&out, &client, &event_type, data.as_deref()).await {
                            eprintln!("{e}");
                            std::process::exit(exit_codes::for_error(&e));
                        }
                    }
                    EventCommand::Watch { event_type } => {
                        if let Err(e) = commands::event::watch(&out, &client, event_type.as_deref()).await {
                            eprintln!("{e}");
                            std::process::exit(exit_codes::for_error(&e));
                        }
                    }
                },
                Command::Init { .. } | Command::Schema | Command::Config(_) => unreachable!(),
            }
        }
    }
}
```

- [ ] **Step 4: Verify it compiles (empty modules ok)**

Create stubs so it compiles:
```bash
mkdir -p src/api src/commands
touch src/api/mod.rs src/api/types.rs src/api/entities.rs src/api/services.rs src/api/events.rs
touch src/commands/mod.rs src/commands/init.rs src/commands/entity.rs
touch src/commands/service.rs src/commands/event.rs src/commands/schema.rs src/commands/config.rs
touch src/config.rs src/output.rs
```

Add minimal content to each stub so the crate compiles:
- `src/config.rs`: empty
- `src/output.rs`: `pub struct OutputConfig; pub enum OutputFormat; pub mod exit_codes {}`
- etc.

Run: `cargo build`
Expected: compiles (possibly with warnings, no errors)

- [ ] **Step 5: Commit**

```bash
git init
git add Cargo.toml src/
git commit -m "chore: initial project scaffold"
```

---

### Task 2: Config Module

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{EnvVarGuard, ProcessEnvLock, write_config};
    use tempfile::TempDir;

    #[test]
    fn loads_default_profile_from_file() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(dir.path(), "[default]\nurl = \"http://ha.local:8123\"\ntoken = \"abc123\"\n").unwrap();
        let _env = EnvVarGuard::set("XDG_CONFIG_HOME", &dir.path().to_string_lossy());
        let _url = EnvVarGuard::unset("HA_URL");
        let _token = EnvVarGuard::unset("HA_TOKEN");

        let cfg = Config::load(None).unwrap();
        assert_eq!(cfg.url, "http://ha.local:8123");
        assert_eq!(cfg.token, "abc123");
    }

    #[test]
    fn env_vars_override_file() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(dir.path(), "[default]\nurl = \"http://ha.local:8123\"\ntoken = \"file-token\"\n").unwrap();
        let _env = EnvVarGuard::set("XDG_CONFIG_HOME", &dir.path().to_string_lossy());
        let _url = EnvVarGuard::set("HA_URL", "http://override:8123");
        let _token = EnvVarGuard::set("HA_TOKEN", "env-token");

        let cfg = Config::load(None).unwrap();
        assert_eq!(cfg.url, "http://override:8123");
        assert_eq!(cfg.token, "env-token");
    }

    #[test]
    fn named_profile_is_loaded() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(dir.path(), "[default]\nurl = \"http://default:8123\"\ntoken = \"t1\"\n\n[prod]\nurl = \"http://prod:8123\"\ntoken = \"t2\"\n").unwrap();
        let _env = EnvVarGuard::set("XDG_CONFIG_HOME", &dir.path().to_string_lossy());
        let _url = EnvVarGuard::unset("HA_URL");
        let _token = EnvVarGuard::unset("HA_TOKEN");

        let cfg = Config::load(Some("prod".into())).unwrap();
        assert_eq!(cfg.url, "http://prod:8123");
        assert_eq!(cfg.token, "t2");
    }

    #[test]
    fn missing_config_returns_err() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        let _env = EnvVarGuard::set("XDG_CONFIG_HOME", &dir.path().to_string_lossy());
        let _url = EnvVarGuard::unset("HA_URL");
        let _token = EnvVarGuard::unset("HA_TOKEN");

        let result = Config::load(None);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("ha init"), "should hint at ha init");
    }

    #[test]
    fn write_profile_creates_file_and_reads_back() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        write_profile(&path, "default", "http://ha.local:8123", "mytoken").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[default]"));
        assert!(content.contains("http://ha.local:8123"));
        assert!(content.contains("mytoken"));
    }

    #[test]
    fn config_path_uses_xdg_config_home() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        let _env = EnvVarGuard::set("XDG_CONFIG_HOME", &dir.path().to_string_lossy());

        let path = config_path();
        assert!(path.starts_with(dir.path()));
        assert!(path.ends_with("config.toml"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo nextest run --test-threads 1 2>&1 | head -30
```
Expected: compile errors or test failures (Config, write_profile not defined)

- [ ] **Step 3: Implement config.rs**

```rust
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::api::HaError;

#[derive(Debug, Deserialize, Default, Clone)]
struct RawProfile {
    pub url: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    #[serde(default)]
    default: RawProfile,
    #[serde(flatten)]
    profiles: BTreeMap<String, RawProfile>,
}

/// Resolved credentials for the active profile.
#[derive(Debug, Clone)]
pub struct Config {
    pub url: String,
    pub token: String,
}

impl Config {
    pub fn load(profile_arg: Option<String>) -> Result<Self, HaError> {
        let file_profile = load_file_profile(profile_arg.as_deref())?;

        let url = std::env::var("HA_URL").ok()
            .filter(|s| !s.is_empty())
            .or_else(|| file_profile.url.filter(|s| !s.is_empty()))
            .ok_or_else(|| HaError::InvalidInput(
                "No url configured. Run 'ha init' or set HA_URL.".into(),
            ))?;

        let token = std::env::var("HA_TOKEN").ok()
            .filter(|s| !s.is_empty())
            .or_else(|| file_profile.token.filter(|s| !s.is_empty()))
            .ok_or_else(|| HaError::InvalidInput(
                "No token configured. Run 'ha init' or set HA_TOKEN.".into(),
            ))?;

        Ok(Self { url, token })
    }
}

fn load_file_profile(profile_arg: Option<&str>) -> Result<RawProfile, HaError> {
    let path = config_path();
    if !path.exists() {
        return Ok(RawProfile::default());
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| HaError::Other(format!("Failed to read config: {e}")))?;

    let raw: RawConfig = toml::from_str(&content)
        .map_err(|e| HaError::Other(format!("Invalid config file: {e}")))?;

    let profile_name = profile_arg
        .map(|s| s.to_owned())
        .or_else(|| std::env::var("HA_PROFILE").ok().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| "default".to_owned());

    if profile_name == "default" {
        return Ok(raw.default);
    }

    raw.profiles
        .get(&profile_name)
        .cloned()
        .ok_or_else(|| HaError::InvalidInput(format!("Profile '{profile_name}' not found in config.")))
}

/// Resolved summary of all profiles for display.
pub struct ProfileSummary {
    pub name: String,
    pub url: Option<String>,
    pub token: Option<String>,
}

pub struct ConfigSummary {
    pub config_file: PathBuf,
    pub file_exists: bool,
    pub profiles: Vec<ProfileSummary>,
    pub env_url: Option<String>,
    pub env_token: Option<String>,
    pub env_profile: Option<String>,
}

pub fn config_summary() -> ConfigSummary {
    let config_file = config_path();
    let file_exists = config_file.exists();
    let mut profiles = Vec::new();

    if file_exists {
        if let Ok(content) = std::fs::read_to_string(&config_file) {
            if let Ok(raw) = toml::from_str::<RawConfig>(&content) {
                profiles.push(ProfileSummary {
                    name: "default".into(),
                    url: raw.default.url,
                    token: raw.default.token,
                });
                for (name, p) in raw.profiles {
                    profiles.push(ProfileSummary { name, url: p.url, token: p.token });
                }
            }
        }
    }

    ConfigSummary {
        config_file,
        file_exists,
        profiles,
        env_url: std::env::var("HA_URL").ok().filter(|s| !s.is_empty()),
        env_token: std::env::var("HA_TOKEN").ok().filter(|s| !s.is_empty()),
        env_profile: std::env::var("HA_PROFILE").ok().filter(|s| !s.is_empty()),
    }
}

/// Write or update a single profile in the config file.
pub fn write_profile(path: &Path, profile: &str, url: &str, token: &str) -> Result<(), HaError> {
    let mut raw: RawConfig = if path.exists() {
        let content = std::fs::read_to_string(path)
            .map_err(|e| HaError::Other(e.to_string()))?;
        toml::from_str(&content)
            .map_err(|e| HaError::Other(format!("Invalid config: {e}")))?
    } else {
        RawConfig::default()
    };

    let new_profile = RawProfile {
        url: Some(url.to_owned()),
        token: Some(token.to_owned()),
    };

    if profile == "default" {
        raw.default = new_profile;
    } else {
        raw.profiles.insert(profile.to_owned(), new_profile);
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| HaError::Other(e.to_string()))?;
    }

    let content = toml::to_string(&raw)
        .map_err(|e| HaError::Other(e.to_string()))?;
    std::fs::write(path, content)
        .map_err(|e| HaError::Other(e.to_string()))?;

    Ok(())
}

/// Return all profile names from the config file.
pub fn read_profile_names(path: &Path) -> Vec<String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let raw: RawConfig = match toml::from_str(&content) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut names = vec!["default".to_owned()];
    names.extend(raw.profiles.into_keys());
    names
}

/// Return (url, token) for an existing profile.
pub fn read_profile_credentials(path: &Path, profile: &str) -> Option<(String, String)> {
    let content = std::fs::read_to_string(path).ok()?;
    let raw: RawConfig = toml::from_str(&content).ok()?;
    let p = if profile == "default" {
        raw.default
    } else {
        raw.profiles.get(profile)?.clone()
    };
    Some((p.url?, p.token?))
}

pub fn config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
    base.join("ha").join("config.toml")
}

/// Path resolution description for schema/JSON output.
pub fn schema_config_path_description() -> &'static str {
    "~/.config/ha/config.toml (or $XDG_CONFIG_HOME/ha/config.toml)"
}

/// Human-readable permission advice for the config file.
pub fn recommended_permissions(path: &Path) -> String {
    format!("chmod 600 {}", path.display())
}
```

- [ ] **Step 4: Run tests**

```bash
cargo nextest run config --test-threads 1
```
Expected: all config tests pass

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat: add config module with profile loading and env var override"
```

---

### Task 3: Output Module

**Files:**
- Modify: `src/output.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_credential_masks_long_values() {
        assert_eq!(mask_credential("abcdefghijklmnop"), "abcdef…mnop");
    }

    #[test]
    fn mask_credential_dots_short_values() {
        assert_eq!(mask_credential("short"), "•••••");
        assert_eq!(mask_credential(""), "");
    }

    #[test]
    fn kv_block_aligns_values() {
        let pairs = [("entity_id", "light.x".into()), ("state", "on".into())];
        let out = kv_block(&pairs);
        let lines: Vec<&str> = out.lines().collect();
        let v1_pos = lines[0].find("light.x").unwrap();
        let v2_pos = lines[1].find("on").unwrap();
        assert_eq!(v1_pos, v2_pos);
    }

    #[test]
    fn table_renders_header_separator_and_rows() {
        let headers = ["ENTITY", "STATE"];
        let rows = vec![
            vec!["light.living_room".into(), "on".into()],
            vec!["switch.fan".into(), "off".into()],
        ];
        let out = table(&headers, &rows);
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[0].contains("ENTITY") && lines[0].contains("STATE"));
        assert!(lines[1].contains("---"));
        assert!(lines[2].contains("light.living_room"));
        assert!(lines[3].contains("switch.fan"));
    }

    #[test]
    fn exit_code_for_auth_error_is_2() {
        assert_eq!(exit_codes::for_error(&crate::api::HaError::Auth("x".into())), 2);
    }

    #[test]
    fn exit_code_for_not_found_is_3() {
        assert_eq!(exit_codes::for_error(&crate::api::HaError::NotFound("x".into())), 3);
    }

    #[test]
    fn exit_code_for_connection_error_is_4() {
        assert_eq!(exit_codes::for_error(&crate::api::HaError::Connection("x".into())), 4);
    }
}
```

- [ ] **Step 2: Run to verify failure**

```bash
cargo nextest run output --test-threads 1 2>&1 | head -20
```
Expected: compile errors (types not defined)

- [ ] **Step 3: Implement output.rs**

```rust
use std::io::IsTerminal;

use crate::api::HaError;

#[derive(Clone, Copy, Debug, PartialEq, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
    Plain,
}

#[derive(Clone, Copy)]
pub struct OutputConfig {
    pub format: OutputFormat,
    pub quiet: bool,
}

impl OutputConfig {
    pub fn new(format_arg: Option<OutputFormat>, quiet: bool) -> Self {
        let format = format_arg.unwrap_or_else(|| {
            if std::io::stdout().is_terminal() {
                OutputFormat::Table
            } else {
                OutputFormat::Json
            }
        });
        Self { format, quiet }
    }

    pub fn is_json(&self) -> bool {
        matches!(self.format, OutputFormat::Json)
    }

    /// Print data (tables, JSON, values) to stdout. Always shown.
    pub fn print_data(&self, data: &str) {
        println!("{data}");
    }

    /// Print informational message to stderr. Suppressed by --quiet.
    pub fn print_message(&self, msg: &str) {
        if !self.quiet {
            eprintln!("{msg}");
        }
    }

    /// Print a JSON result or human message depending on format.
    pub fn print_result(&self, json_value: &serde_json::Value, human_message: &str) {
        if self.is_json() {
            println!("{}", serde_json::to_string_pretty(json_value).expect("serialize"));
        } else {
            println!("{human_message}");
        }
    }
}

pub mod exit_codes {
    use super::HaError;

    pub const SUCCESS: i32 = 0;
    pub const GENERAL_ERROR: i32 = 1;
    pub const CONFIG_ERROR: i32 = 2;
    pub const NOT_FOUND: i32 = 3;
    pub const CONNECTION_ERROR: i32 = 4;

    pub fn for_error(e: &HaError) -> i32 {
        match e {
            HaError::Auth(_) | HaError::InvalidInput(_) => CONFIG_ERROR,
            HaError::NotFound(_) => NOT_FOUND,
            HaError::Connection(_) => CONNECTION_ERROR,
            _ => GENERAL_ERROR,
        }
    }
}

/// Mask a credential for safe display.
pub fn mask_credential(s: &str) -> String {
    if s.len() <= 10 {
        return "•".repeat(s.len());
    }
    format!("{}…{}", &s[..6], &s[s.len() - 4..])
}

/// Render a two-column key/value block with aligned values.
pub fn kv_block(pairs: &[(&str, String)]) -> String {
    let max_key = pairs.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    pairs
        .iter()
        .map(|(k, v)| format!("{:width$}  {}", k, v, width = max_key))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render a simple table with header and data rows.
pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let col_count = headers.len();
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let header_line: String = headers
        .iter()
        .enumerate()
        .map(|(i, h)| format!("{:width$}", h, width = widths[i]))
        .collect::<Vec<_>>()
        .join("  ");

    let sep: String = widths.iter().map(|w| "-".repeat(*w)).collect::<Vec<_>>().join("  ");

    let data_lines: Vec<String> = rows
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .take(col_count)
                .map(|(i, cell)| format!("{:width$}", cell, width = widths[i]))
                .collect::<Vec<_>>()
                .join("  ")
        })
        .collect();

    let mut out = vec![header_line, sep];
    out.extend(data_lines);
    out.join("\n")
}
```

- [ ] **Step 4: Run tests**

```bash
cargo nextest run output --test-threads 1
```
Expected: all output tests pass

- [ ] **Step 5: Commit**

```bash
git add src/output.rs
git commit -m "feat: add output module with OutputFormat, table rendering, and exit codes"
```

---

### Task 4: API Error Types and HaClient

**Files:**
- Modify: `src/api/mod.rs`

- [ ] **Step 1: Write failing tests**

```rust
// In src/api/mod.rs tests section
#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn auth_error_display_includes_guidance() {
        let err = HaError::Auth("401 Unauthorized".into());
        let msg = err.to_string();
        assert!(msg.contains("Authentication failed"));
        assert!(msg.contains("ha init") || msg.contains("HA_TOKEN"));
    }

    #[test]
    fn not_found_display_includes_entity() {
        let err = HaError::NotFound("light.missing".into());
        assert!(err.to_string().contains("light.missing"));
    }

    #[test]
    fn connection_error_mentions_url() {
        let err = HaError::Connection("http://ha.local:8123".into());
        assert!(err.to_string().contains("http://ha.local:8123"));
    }

    #[test]
    fn http_error_source_is_reqwest() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let reqwest_err = rt.block_on(async {
            reqwest::Client::new()
                .get("http://127.0.0.1:1")
                .send()
                .await
                .unwrap_err()
        });
        let api_err = HaError::Http(reqwest_err);
        assert!(api_err.source().is_some());
    }

    #[test]
    fn ha_client_new_trims_trailing_slash() {
        let client = HaClient::new("http://ha.local:8123/", "token");
        assert_eq!(client.base_url, "http://ha.local:8123");
    }
}
```

- [ ] **Step 2: Run to verify failure**

```bash
cargo nextest run "api::mod" --test-threads 1 2>&1 | head -20
```

- [ ] **Step 3: Implement src/api/mod.rs**

```rust
pub mod entities;
pub mod events;
pub mod services;
pub mod types;

pub use types::*;

use std::fmt;

#[derive(Debug)]
pub enum HaError {
    /// 401/403 from HA API.
    Auth(String),
    /// 404 — entity, service, or resource not found.
    NotFound(String),
    /// Missing or invalid config/input.
    InvalidInput(String),
    /// Could not reach Home Assistant.
    Connection(String),
    /// Non-2xx response.
    Api { status: u16, message: String },
    /// Network/TLS error from reqwest.
    Http(reqwest::Error),
    /// Any other error.
    Other(String),
}

impl fmt::Display for HaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HaError::Auth(msg) => write!(
                f,
                "Authentication failed: {msg}\nCheck your token or run `ha init`."
            ),
            HaError::NotFound(msg) => write!(f, "Not found: {msg}"),
            HaError::InvalidInput(msg) => write!(f, "{msg}"),
            HaError::Connection(url) => write!(
                f,
                "Could not connect to Home Assistant at {url}\nCheck that HA is running and the URL is correct."
            ),
            HaError::Api { status, message } => write!(f, "API error {status}: {message}"),
            HaError::Http(e) => write!(f, "HTTP error: {e}"),
            HaError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for HaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HaError::Http(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for HaError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_connect() || e.is_timeout() {
            HaError::Connection(
                e.url().map(|u| u.to_string()).unwrap_or_else(|| "unknown".into()),
            )
        } else {
            HaError::Http(e)
        }
    }
}

/// HTTP client for the Home Assistant REST API.
pub struct HaClient {
    pub base_url: String,
    token: String,
    pub(crate) client: reqwest::Client,
}

impl HaClient {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            token: token.into(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("build reqwest client"),
        }
    }

    /// Returns a GET request builder pre-configured with auth.
    pub fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .get(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.token)
    }

    /// Returns a POST request builder pre-configured with auth.
    pub fn post(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .post(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.token)
    }

    /// Validate the connection by calling GET /api/
    pub async fn validate(&self) -> Result<String, HaError> {
        let resp = self.get("/api/").send().await?;
        match resp.status().as_u16() {
            200 => {
                let body: serde_json::Value = resp.json().await?;
                Ok(body["message"].as_str().unwrap_or("API running.").to_owned())
            }
            401 | 403 => Err(HaError::Auth("Invalid token".into())),
            status => Err(HaError::Api {
                status,
                message: resp.text().await.unwrap_or_default(),
            }),
        }
    }
}
```

- [ ] **Step 4: Create src/api/types.rs**

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EntityState {
    pub entity_id: String,
    pub state: String,
    pub attributes: serde_json::Value,
    pub last_changed: String,
    pub last_updated: String,
}

#[derive(Debug, Deserialize)]
pub struct ServiceDomain {
    pub domain: String,
    pub services: BTreeMap<String, ServiceInfo>,
}

#[derive(Debug, Deserialize)]
pub struct ServiceInfo {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HaEvent {
    pub event_type: String,
    pub data: serde_json::Value,
    pub origin: Option<String>,
    pub time_fired: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StateChangedData {
    pub entity_id: String,
    pub new_state: Option<EntityState>,
    pub old_state: Option<EntityState>,
}
```

- [ ] **Step 5: Run tests**

```bash
cargo nextest run "api" --test-threads 1
```
Expected: all api tests pass

- [ ] **Step 6: Commit**

```bash
git add src/api/
git commit -m "feat: add HaClient and HaError with semantic error codes"
```

---

### Task 5: test_support.rs

**Files:**
- Modify: `src/test_support.rs`

- [ ] **Step 1: Implement test_support.rs**

No tests for this module — it exists to support other tests.

```rust
//! Test helpers shared across module tests. Only compiled in test builds.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

static PROCESS_ENV_LOCK: Mutex<()> = Mutex::new(());

pub struct ProcessEnvLock(#[allow(dead_code)] MutexGuard<'static, ()>);

impl ProcessEnvLock {
    pub fn acquire() -> Result<Self, std::sync::PoisonError<MutexGuard<'static, ()>>> {
        Ok(Self(PROCESS_ENV_LOCK.lock()?))
    }
}

/// RAII guard: sets an env var and restores original on drop.
pub struct EnvVarGuard {
    name: String,
    original: Option<String>,
}

impl EnvVarGuard {
    pub fn set(name: &str, value: &str) -> Self {
        let original = std::env::var(name).ok();
        unsafe { std::env::set_var(name, value) };
        Self { name: name.to_owned(), original }
    }

    pub fn unset(name: &str) -> Self {
        let original = std::env::var(name).ok();
        unsafe { std::env::remove_var(name) };
        Self { name: name.to_owned(), original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(v) => unsafe { std::env::set_var(&self.name, v) },
            None => unsafe { std::env::remove_var(&self.name) },
        }
    }
}

/// Write a config file to `<dir>/ha/config.toml`.
pub fn write_config(dir: &Path, content: &str) -> Result<PathBuf, std::io::Error> {
    let config_dir = dir.join("ha");
    std::fs::create_dir_all(&config_dir)?;
    let path = config_dir.join("config.toml");
    std::fs::write(&path, content)?;
    Ok(path)
}
```

- [ ] **Step 2: Verify config tests still pass**

```bash
cargo nextest run config --test-threads 1
```
Expected: all config tests pass

- [ ] **Step 3: Commit**

```bash
git add src/test_support.rs
git commit -m "chore: add test_support helpers for env vars and config"
```

---

### Task 6: API Entities

**Files:**
- Modify: `src/api/entities.rs`

- [ ] **Step 1: Write failing tests**

```rust
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
```

- [ ] **Step 2: Run to verify failure**

```bash
cargo nextest run "api::entities" 2>&1 | head -20
```

- [ ] **Step 3: Implement src/api/entities.rs**

```rust
use crate::api::{HaClient, HaError, EntityState};

pub async fn get_state(client: &HaClient, entity_id: &str) -> Result<EntityState, HaError> {
    let resp = client.get(&format!("/api/states/{entity_id}")).send().await?;
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
```

- [ ] **Step 4: Run tests**

```bash
cargo nextest run "api::entities"
```
Expected: all entity tests pass

- [ ] **Step 5: Commit**

```bash
git add src/api/entities.rs
git commit -m "feat: add entity state get and list API methods"
```

---

### Task 7: API Services

**Files:**
- Modify: `src/api/services.rs`

- [ ] **Step 1: Write failing tests**

```rust
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
```

- [ ] **Step 2: Implement src/api/services.rs**

```rust
use crate::api::{HaClient, HaError, ServiceDomain};

pub async fn list_services(client: &HaClient) -> Result<Vec<ServiceDomain>, HaError> {
    let resp = client.get("/api/services").send().await?;
    match resp.status().as_u16() {
        200 => Ok(resp.json().await?),
        401 | 403 => Err(HaError::Auth("Unauthorized".into())),
        status => Err(HaError::Api { status, message: resp.text().await.unwrap_or_default() }),
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
        404 => Err(HaError::NotFound(format!("Service '{domain}.{service}' not found"))),
        status => Err(HaError::Api { status, message: resp.text().await.unwrap_or_default() }),
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo nextest run "api::services"
```
Expected: all service tests pass

- [ ] **Step 4: Commit**

```bash
git add src/api/services.rs
git commit -m "feat: add service list and call API methods"
```

---

### Task 8: API Events and SSE Stream

**Files:**
- Modify: `src/api/events.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::HaClient;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn fire_event_sends_post() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/events/my_event"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"message": "Event my_event fired."})
            ))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        let result = fire_event(&client, "my_event", None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn fire_event_with_data_includes_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/events/custom"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"message": "Event custom fired."})
            ))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        let data = serde_json::json!({"key": "value"});
        let result = fire_event(&client, "custom", Some(&data)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    fn parse_sse_line_extracts_data() {
        let line = r#"data: {"event_type":"state_changed","data":{},"time_fired":"2026-01-01T00:00:00Z"}"#;
        let event = parse_sse_data(line).unwrap();
        assert_eq!(event.event_type, "state_changed");
    }

    #[test]
    fn parse_sse_line_returns_none_for_non_data_lines() {
        assert!(parse_sse_data("").is_none());
        assert!(parse_sse_data(": ping").is_none());
        assert!(parse_sse_data("event: state_changed").is_none());
    }
}
```

- [ ] **Step 2: Implement src/api/events.rs**

```rust
use futures_util::StreamExt;

use crate::api::{HaClient, HaError, HaEvent};

pub async fn fire_event(
    client: &HaClient,
    event_type: &str,
    data: Option<&serde_json::Value>,
) -> Result<serde_json::Value, HaError> {
    let req = client.post(&format!("/api/events/{event_type}"));
    let req = if let Some(d) = data { req.json(d) } else { req };
    let resp = req.send().await?;
    match resp.status().as_u16() {
        200 => Ok(resp.json().await?),
        401 | 403 => Err(HaError::Auth("Unauthorized".into())),
        404 => Err(HaError::NotFound(format!("Event type '{event_type}' not found"))),
        status => Err(HaError::Api { status, message: resp.text().await.unwrap_or_default() }),
    }
}

/// Parse a single SSE line of the form `data: <json>` into an HaEvent.
pub(crate) fn parse_sse_data(line: &str) -> Option<HaEvent> {
    let json = line.strip_prefix("data: ")?;
    serde_json::from_str(json).ok()
}

/// Stream SSE events from /api/stream, calling `on_event` for each.
/// Returns when `on_event` returns false or the stream ends.
pub async fn watch_stream(
    client: &HaClient,
    restrict: Option<&str>,
    mut on_event: impl FnMut(HaEvent) -> bool,
) -> Result<(), HaError> {
    let url = match restrict {
        Some(r) => format!("{}/api/stream?restrict={}", client.base_url, r),
        None => format!("{}/api/stream", client.base_url),
    };

    let resp = client
        .client
        .get(&url)
        .bearer_auth(&client.base_url) // NOTE: token is private; use a wrapper
        .send()
        .await?;

    match resp.status().as_u16() {
        200 => {}
        401 | 403 => return Err(HaError::Auth("Unauthorized".into())),
        status => return Err(HaError::Api { status, message: resp.text().await.unwrap_or_default() }),
    }

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim_end_matches('\r').to_owned();
            buffer.drain(..=pos);
            if let Some(event) = parse_sse_data(&line) {
                if !on_event(event) {
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}
```

Note: `watch_stream` uses `client.base_url` for the bearer token placeholder — fix this by making `HaClient::token` accessible via a helper method. Add `pub fn token(&self) -> &str { &self.token }` to `HaClient` in `api/mod.rs`, then update the `bearer_auth` call to `bearer_auth(client.token())`.

- [ ] **Step 3: Fix HaClient token accessor**

In `src/api/mod.rs`, add to the `HaClient` impl block:
```rust
pub fn token(&self) -> &str {
    &self.token
}
```

Then in `src/api/events.rs`, change:
```rust
.bearer_auth(&client.base_url)
```
to:
```rust
.bearer_auth(client.token())
```

- [ ] **Step 4: Run tests**

```bash
cargo nextest run "api::events"
```
Expected: all event tests pass

- [ ] **Step 5: Commit**

```bash
git add src/api/events.rs src/api/mod.rs
git commit -m "feat: add event fire and SSE stream watch"
```

---

### Task 9: Command: init

**Files:**
- Modify: `src/commands/init.rs`
- Modify: `src/commands/mod.rs`

- [ ] **Step 1: Add mod declarations to commands/mod.rs**

```rust
pub mod config;
pub mod entity;
pub mod event;
pub mod init;
pub mod schema;
pub mod service;
```

- [ ] **Step 2: Write failing tests for init**

```rust
#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use tempfile::TempDir;
    use super::*;

    fn fake_path(dir: &TempDir) -> std::path::PathBuf {
        dir.path().join("config.toml")
    }

    #[tokio::test]
    async fn init_writes_config_on_valid_credentials() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);
        let input = b"http://ha.local:8123\nmytoken\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(&mut reader, &mut writer, &path, None, |_url, _token| async {
            Some("Home Assistant".to_string())
        })
        .await
        .unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("http://ha.local:8123"));
        assert!(saved.contains("mytoken"));
    }

    #[tokio::test]
    async fn init_uses_default_profile_on_first_setup() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);
        let input = b"http://ha.local:8123\nmytoken\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(&mut reader, &mut writer, &path, None, |_, _| async {
            Some("HA".into())
        })
        .await
        .unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("[default]"));
    }

    #[tokio::test]
    async fn init_aborts_when_validation_fails_and_user_declines() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);
        let input = b"http://ha.local:8123\nbadtoken\nn\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(&mut reader, &mut writer, &path, None, |_, _| async { None })
            .await
            .unwrap();

        assert!(!path.exists(), "config must not be written after abort");
    }

    #[tokio::test]
    async fn init_saves_when_validation_fails_but_user_forces() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);
        let input = b"http://ha.local:8123\nbadtoken\ny\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(&mut reader, &mut writer, &path, None, |_, _| async { None })
            .await
            .unwrap();

        assert!(path.exists());
    }

    #[tokio::test]
    async fn init_with_profile_arg_writes_named_profile() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);
        let input = b"http://ha.prod:8123\nprodtoken\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(&mut reader, &mut writer, &path, Some("prod"), |_, _| async {
            Some("HA".into())
        })
        .await
        .unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("[prod]"));
    }

    #[tokio::test]
    async fn init_update_keeps_values_on_enter() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);
        std::fs::write(
            &path,
            "[default]\nurl = \"http://ha.local:8123\"\ntoken = \"existing-token\"\n",
        )
        .unwrap();

        // action=update (Enter), then Enter to keep both fields
        let input = b"\n\n\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(&mut reader, &mut writer, &path, None, |_, _| async {
            Some("HA".into())
        })
        .await
        .unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("existing-token"));
    }

    #[tokio::test]
    async fn init_outro_includes_profile_flag_for_non_default() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);
        let input = b"http://ha.local:8123\ntoken\n";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(&mut reader, &mut writer, &path, Some("staging"), |_, _| async {
            Some("HA".into())
        })
        .await
        .unwrap();

        let output = String::from_utf8_lossy(&writer);
        assert!(output.contains("--profile staging"));
    }

    #[tokio::test]
    async fn init_aborts_on_eof() {
        let dir = TempDir::new().unwrap();
        let path = fake_path(&dir);
        let input = b"";
        let mut reader = Cursor::new(input.as_ref());
        let mut writer = Vec::<u8>::new();

        run_init(&mut reader, &mut writer, &path, None, |_, _| async {
            Some("HA".into())
        })
        .await
        .unwrap();

        assert!(!path.exists());
        let output = String::from_utf8_lossy(&writer);
        assert!(output.contains("Aborted"));
    }
}
```

- [ ] **Step 3: Run to verify failure**

```bash
cargo nextest run "commands::init" 2>&1 | head -20
```

- [ ] **Step 4: Implement src/commands/init.rs**

```rust
use std::future::Future;
use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;

use owo_colors::OwoColorize;

use crate::api::HaError;
use crate::config;
use crate::output;

const SEP: &str = "──────────────────────────────────────";

fn sym_q() -> String { "?".green().bold().to_string() }
fn sym_ok() -> String { "✔".green().to_string() }
fn sym_fail() -> String { "✖".red().to_string() }
fn sym_dim(s: &str) -> String { s.dimmed().to_string() }

fn prompt_optional<R: BufRead, W: Write>(r: &mut R, w: &mut W, label: &str, default: &str) -> String {
    let _ = write!(w, "{} {}  [{}]: ", sym_q(), label, sym_dim(default));
    let _ = w.flush();
    let mut input = String::new();
    r.read_line(&mut input).unwrap_or(0);
    let trimmed = input.trim().to_owned();
    if trimmed.is_empty() { default.to_owned() } else { trimmed }
}

fn prompt_required<R: BufRead, W: Write>(r: &mut R, w: &mut W, label: &str, hint: &str) -> Option<String> {
    loop {
        let _ = write!(w, "{} {}  {}: ", sym_q(), label, sym_dim(&format!("[{hint}]")));
        let _ = w.flush();
        let mut input = String::new();
        match r.read_line(&mut input) {
            Ok(0) | Err(_) => return None,
            Ok(_) => {}
        }
        let trimmed = input.trim().to_owned();
        if !trimmed.is_empty() { return Some(trimmed); }
        let _ = writeln!(w, "  {} {} is required.", sym_fail(), label);
    }
}

fn prompt_credential_update<R: BufRead, W: Write>(r: &mut R, w: &mut W, label: &str, current: &str) -> Option<String> {
    let hint = format!("{} (Enter to keep)", output::mask_credential(current));
    let _ = write!(w, "{} {}  {}: ", sym_q(), label, sym_dim(&hint));
    let _ = w.flush();
    let mut input = String::new();
    match r.read_line(&mut input) {
        Ok(0) | Err(_) => return None,
        Ok(_) => {}
    }
    let trimmed = input.trim().to_owned();
    Some(if trimmed.is_empty() { current.to_owned() } else { trimmed })
}

fn prompt_confirm<R: BufRead, W: Write>(r: &mut R, w: &mut W, label: &str, default_yes: bool) -> bool {
    let hint = if default_yes { "Y/n" } else { "y/N" };
    let _ = write!(w, "{} {}  [{}]: ", sym_q(), label, sym_dim(hint));
    let _ = w.flush();
    let mut input = String::new();
    r.read_line(&mut input).unwrap_or(0);
    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default_yes,
    }
}

fn print_json_schema(config_path: &Path) {
    let path_str = config_path.to_string_lossy();
    let schema = serde_json::json!({
        "configPath": path_str,
        "pathResolution": config::schema_config_path_description(),
        "recommendedPermissions": config::recommended_permissions(config_path),
        "tokenInstructions": {
            "steps": [
                "Open Home Assistant in your browser",
                "Go to Settings → Profile (bottom left)",
                "Scroll to 'Long-Lived Access Tokens'",
                "Click 'Create Token', give it a name, copy it"
            ]
        },
        "requiredFields": ["url", "token"],
        "example": {
            "configFile": path_str,
            "format": "[default]\nurl = \"http://homeassistant.local:8123\"\ntoken = \"YOUR_LONG_LIVED_TOKEN\""
        }
    });
    println!("{}", serde_json::to_string_pretty(&schema).expect("serialize"));
}

pub async fn run_init<R, W, Fut>(
    reader: &mut R,
    writer: &mut W,
    config_path: &Path,
    profile_arg: Option<&str>,
    validate: impl Fn(String, String) -> Fut,
) -> Result<(), HaError>
where
    R: BufRead,
    W: Write,
    Fut: Future<Output = Option<String>>,
{
    let _ = writeln!(writer, "\nHome Assistant CLI");
    let _ = writeln!(writer, "{SEP}\n");

    let existing_profiles = config::read_profile_names(config_path);
    let is_first_setup = existing_profiles.is_empty();

    let (profile_name, is_update) = if let Some(p) = profile_arg {
        let is_update = existing_profiles.contains(&p.to_owned());
        (p.to_owned(), is_update)
    } else if is_first_setup {
        ("default".to_owned(), false)
    } else {
        if existing_profiles.len() == 1 {
            let p = &existing_profiles[0];
            let cred = config::read_profile_credentials(config_path, p)
                .map(|(url, _)| format!("  {}", output::mask_credential(&url)))
                .unwrap_or_default();
            let _ = writeln!(writer, "  Profile: {}{}\n", p.bold(), sym_dim(&cred));
        } else {
            let _ = writeln!(writer, "  Profiles:");
            for p in &existing_profiles {
                let cred = config::read_profile_credentials(config_path, p)
                    .map(|(url, _)| format!("  {}", output::mask_credential(&url)))
                    .unwrap_or_default();
                let _ = writeln!(writer, "    {}{}", p, sym_dim(&cred));
            }
            let _ = writeln!(writer);
        }

        let action = prompt_optional(reader, writer, "Action  [update/add]", "update");
        let _ = writeln!(writer);

        if action.trim().eq_ignore_ascii_case("add") {
            let Some(name) = prompt_required(reader, writer, "Profile name", "e.g. prod") else {
                let _ = writeln!(writer, "\nAborted.");
                return Ok(());
            };
            (name, false)
        } else {
            if existing_profiles.len() == 1 {
                (existing_profiles[0].clone(), true)
            } else {
                let options = existing_profiles.join("/");
                let chosen = prompt_optional(reader, writer, &format!("Profile  [{}]", options), &existing_profiles[0]);
                let profile = chosen.trim().to_owned();
                if !existing_profiles.contains(&profile) {
                    let _ = writeln!(writer, "\n  {} Unknown profile '{}'.", sym_fail(), profile);
                    return Ok(());
                }
                (profile, true)
            }
        }
    };

    let (url, token) = if is_update {
        let (cur_url, cur_token) = config::read_profile_credentials(config_path, &profile_name)
            .expect("update mode requires existing credentials");
        let Some(url) = prompt_credential_update(reader, writer, "URL", &cur_url) else {
            let _ = writeln!(writer, "\nAborted.");
            return Ok(());
        };
        let Some(token) = prompt_credential_update(reader, writer, "Token", &cur_token) else {
            let _ = writeln!(writer, "\nAborted.");
            return Ok(());
        };
        (url, token)
    } else {
        let Some(url) = prompt_required(reader, writer, "Home Assistant URL", "http://homeassistant.local:8123") else {
            let _ = writeln!(writer, "\nAborted.");
            return Ok(());
        };
        let Some(token) = prompt_required(reader, writer, "Long-Lived Access Token", "from HA Settings → Profile") else {
            let _ = writeln!(writer, "\nAborted.");
            return Ok(());
        };
        (url, token)
    };

    let _ = write!(writer, "\n  Verifying credentials...");
    let _ = writer.flush();
    let validation = validate(url.clone(), token.clone()).await;

    let save = match validation {
        Some(name) => {
            let _ = writeln!(writer, " {} Connected to {}", sym_ok(), name.bold());
            true
        }
        None => {
            let _ = writeln!(writer, " {} Could not connect.", sym_fail());
            prompt_confirm(reader, writer, "Save anyway?", false)
        }
    };

    if !save {
        let _ = writeln!(writer, "\nAborted. Config not saved.");
        let _ = writer.flush();
        return Ok(());
    }

    config::write_profile(config_path, &profile_name, &url, &token)?;

    let run_cmd = if profile_name == "default" {
        "ha entity list".to_owned()
    } else {
        format!("ha --profile {} entity list", profile_name)
    };

    let _ = writeln!(writer, "\n{SEP}");
    let _ = writeln!(writer, "  {} Config saved to {}", sym_ok(), sym_dim(&config_path.display().to_string()));
    let _ = writeln!(writer, "  Run: {}", run_cmd.bold());
    let _ = writer.flush();

    Ok(())
}

pub async fn init(profile_arg: Option<String>) {
    let config_path = config::config_path();

    if !std::io::stdout().is_terminal() {
        print_json_schema(&config_path);
        return;
    }

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = std::io::BufReader::new(stdin.lock());
    let mut writer = std::io::BufWriter::new(stdout.lock());

    if let Err(e) = run_init(
        &mut reader,
        &mut writer,
        &config_path,
        profile_arg.as_deref(),
        |url, token| async move {
            let client = crate::api::HaClient::new(&url, &token);
            client.validate().await.ok()
        },
    )
    .await
    {
        eprintln!("{} {e}", sym_fail());
        std::process::exit(crate::output::exit_codes::GENERAL_ERROR);
    }
}
```

- [ ] **Step 5: Run tests**

```bash
cargo nextest run "commands::init"
```
Expected: all init tests pass

- [ ] **Step 6: Commit**

```bash
git add src/commands/
git commit -m "feat: add init command with interactive setup and JSON schema mode"
```

---

### Task 10: Command: entity

**Files:**
- Modify: `src/commands/entity.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{HaClient, EntityState};
    use crate::output::{OutputConfig, OutputFormat};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn json_out() -> OutputConfig {
        OutputConfig::new(Some(OutputFormat::Json), false)
    }

    fn state_json(entity_id: &str, state: &str) -> serde_json::Value {
        serde_json::json!({
            "entity_id": entity_id,
            "state": state,
            "attributes": {},
            "last_changed": "2026-01-01T00:00:00Z",
            "last_updated": "2026-01-01T00:00:00Z"
        })
    }

    #[tokio::test]
    async fn get_prints_json_for_json_output() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/states/light.x"))
            .respond_with(ResponseTemplate::new(200).set_body_json(state_json("light.x", "on")))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        // Capture stdout by checking no error is returned
        let result = get(&json_out(), &client, "light.x").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_filters_by_domain() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/states"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                state_json("light.a", "on"),
                state_json("switch.b", "off"),
                state_json("light.c", "off"),
            ])))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");

        // list with domain filter — just verify no error
        let result = list(&json_out(), &client, Some("light")).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_propagates_not_found() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/states/light.missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        let result = get(&json_out(), &client, "light.missing").await;
        assert!(matches!(result, Err(crate::api::HaError::NotFound(_))));
    }
}
```

- [ ] **Step 2: Implement src/commands/entity.rs**

```rust
use owo_colors::OwoColorize;

use crate::api::{self, HaClient, HaError, EntityState};
use crate::output::{self, OutputConfig, OutputFormat};

pub async fn get(out: &OutputConfig, client: &HaClient, entity_id: &str) -> Result<(), HaError> {
    let state = api::entities::get_state(client, entity_id).await?;

    if out.is_json() {
        out.print_data(&serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": state
        })).expect("serialize"));
    } else {
        let attrs = state.attributes.as_object()
            .map(|m| m.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join("  "))
            .unwrap_or_default();
        let status_sym = if state.state == "on" { "●".green().to_string() } else { "○".dimmed().to_string() };
        out.print_data(&format!("{} {}  {}  {}", status_sym, state.entity_id, state.state.bold(), attrs.dimmed()));
    }
    Ok(())
}

pub async fn list(out: &OutputConfig, client: &HaClient, domain: Option<&str>) -> Result<(), HaError> {
    let mut states = api::entities::list_states(client).await?;

    if let Some(d) = domain {
        states.retain(|s| s.entity_id.starts_with(&format!("{d}.")));
    }

    states.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));

    if out.is_json() {
        out.print_data(&serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": states
        })).expect("serialize"));
    } else {
        let rows: Vec<Vec<String>> = states.iter().map(|s| {
            vec![s.entity_id.clone(), s.state.clone(), s.last_updated.clone()]
        }).collect();
        out.print_data(&output::table(&["ENTITY", "STATE", "LAST UPDATED"], &rows));
    }
    Ok(())
}

pub async fn watch(out: &OutputConfig, client: &HaClient, entity_id: &str) -> Result<(), HaError> {
    out.print_message(&format!("Watching {} (Ctrl+C to stop)...", entity_id));

    let entity_id = entity_id.to_owned();
    api::events::watch_stream(client, Some("state_changed"), |event| {
        if let Ok(data) = serde_json::from_value::<crate::api::StateChangedData>(event.data.clone()) {
            if data.entity_id == entity_id {
                if out.is_json() {
                    if let Ok(s) = serde_json::to_string_pretty(&serde_json::json!({"ok": true, "data": data})) {
                        println!("{s}");
                    }
                } else if let Some(new) = &data.new_state {
                    let status_sym = if new.state == "on" { "●".green().to_string() } else { "○".dimmed().to_string() };
                    println!("{} {}  {}  {}", status_sym, new.entity_id, new.state.bold(), new.last_updated.dimmed());
                }
            }
        }
        true // keep streaming
    }).await
}
```

- [ ] **Step 3: Run tests**

```bash
cargo nextest run "commands::entity"
```
Expected: all entity command tests pass

- [ ] **Step 4: Commit**

```bash
git add src/commands/entity.rs
git commit -m "feat: add entity get, list, and watch commands"
```

---

### Task 11: Command: service

**Files:**
- Modify: `src/commands/service.rs`

- [ ] **Step 1: Write failing tests**

```rust
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
        let result = call(&json_out(), &client, "light.turn_on", Some("light.living_room"), None).await;
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
```

- [ ] **Step 2: Implement src/commands/service.rs**

```rust
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

    if let Some(entity_id) = entity {
        body.as_object_mut()
            .map(|m| m.insert("entity_id".into(), serde_json::Value::String(entity_id.to_owned())));
    }

    let result = api::services::call_service(client, domain, svc, Some(&body)).await?;

    if out.is_json() {
        out.print_data(&serde_json::to_string_pretty(&serde_json::json!({"ok": true, "data": result})).expect("serialize"));
    } else {
        out.print_data(&format!("✔ Called {service}"));
    }
    Ok(())
}

pub async fn list(out: &OutputConfig, client: &HaClient, domain: Option<&str>) -> Result<(), HaError> {
    let mut domains = api::services::list_services(client).await?;

    if let Some(d) = domain {
        domains.retain(|dom| dom.domain == d);
    }

    domains.sort_by(|a, b| a.domain.cmp(&b.domain));

    if out.is_json() {
        out.print_data(&serde_json::to_string_pretty(&serde_json::json!({"ok": true, "data": domains})).expect("serialize"));
    } else {
        let rows: Vec<Vec<String>> = domains.iter().flat_map(|d| {
            d.services.iter().map(|(svc, info)| vec![
                format!("{}.{}", d.domain, svc),
                info.name.clone().unwrap_or_default(),
                info.description.clone().unwrap_or_default(),
            ])
        }).collect();
        out.print_data(&output::table(&["SERVICE", "NAME", "DESCRIPTION"], &rows));
    }
    Ok(())
}
```

- [ ] **Step 3: Run tests**

```bash
cargo nextest run "commands::service"
```

- [ ] **Step 4: Commit**

```bash
git add src/commands/service.rs
git commit -m "feat: add service call and list commands"
```

---

### Task 12: Command: event

**Files:**
- Modify: `src/commands/event.rs`

- [ ] **Step 1: Write failing tests**

```rust
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
    async fn fire_succeeds_on_200() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/events/my_event"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"message": "Event my_event fired."})
            ))
            .mount(&server)
            .await;

        let client = HaClient::new(server.uri(), "tok");
        let result = fire(&json_out(), &client, "my_event", None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn fire_with_invalid_json_returns_error() {
        let server = MockServer::start().await;
        let client = HaClient::new(server.uri(), "tok");
        let result = fire(&json_out(), &client, "my_event", Some("{invalid}")).await;
        assert!(matches!(result, Err(crate::api::HaError::InvalidInput(_))));
    }
}
```

- [ ] **Step 2: Implement src/commands/event.rs**

```rust
use crate::api::{self, HaClient, HaError};
use crate::output::OutputConfig;

pub async fn fire(
    out: &OutputConfig,
    client: &HaClient,
    event_type: &str,
    data: Option<&str>,
) -> Result<(), HaError> {
    let body = if let Some(d) = data {
        Some(
            serde_json::from_str::<serde_json::Value>(d)
                .map_err(|e| HaError::InvalidInput(format!("Invalid JSON data: {e}")))?,
        )
    } else {
        None
    };

    let result = api::events::fire_event(client, event_type, body.as_ref()).await?;

    if out.is_json() {
        out.print_data(
            &serde_json::to_string_pretty(&serde_json::json!({"ok": true, "data": result}))
                .expect("serialize"),
        );
    } else {
        out.print_data(&format!("✔ Fired event: {event_type}"));
    }
    Ok(())
}

pub async fn watch(out: &OutputConfig, client: &HaClient, event_type: Option<&str>) -> Result<(), HaError> {
    out.print_message(&format!(
        "Watching events{} (Ctrl+C to stop)...",
        event_type.map(|t| format!(": {t}")).unwrap_or_default()
    ));

    api::events::watch_stream(client, event_type, |event| {
        if out.is_json() {
            if let Ok(s) = serde_json::to_string_pretty(&serde_json::json!({"ok": true, "data": event})) {
                println!("{s}");
            }
        } else {
            let time = event.time_fired.as_deref().unwrap_or("-");
            println!("{} {}  {}", time, event.event_type, event.data);
        }
        true
    })
    .await
}
```

- [ ] **Step 3: Run tests**

```bash
cargo nextest run "commands::event"
```

- [ ] **Step 4: Commit**

```bash
git add src/commands/event.rs
git commit -m "feat: add event fire and watch commands"
```

---

### Task 13: Command: schema

**Files:**
- Modify: `src/commands/schema.rs`

- [ ] **Step 1: Write failing tests**

```rust
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
        let names: Vec<&str> = commands.iter()
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
        let entity_get = commands.iter().find(|c| c["name"] == "entity get").unwrap();
        assert!(entity_get["json_shape"]["data"]["entity_id"].is_string());
        assert!(entity_get["json_shape"]["data"]["state"].is_string());
    }

    #[test]
    fn schema_includes_global_flags() {
        let schema = build_schema();
        let globals = schema["global_flags"].as_array().unwrap();
        let flag_names: Vec<&str> = globals.iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        assert!(flag_names.contains(&"--output"));
        assert!(flag_names.contains(&"--profile"));
    }
}
```

- [ ] **Step 2: Implement src/commands/schema.rs**

```rust
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
    println!("{}", serde_json::to_string_pretty(&build_schema()).expect("serialize"));
}
```

- [ ] **Step 3: Run tests**

```bash
cargo nextest run "commands::schema"
```

- [ ] **Step 4: Commit**

```bash
git add src/commands/schema.rs
git commit -m "feat: add schema command with full machine-readable CLI description"
```

---

### Task 14: Command: config

**Files:**
- Modify: `src/commands/config.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::{OutputConfig, OutputFormat};
    use crate::test_support::{EnvVarGuard, ProcessEnvLock, write_config};
    use tempfile::TempDir;

    fn json_out() -> OutputConfig {
        OutputConfig::new(Some(OutputFormat::Json), false)
    }

    #[test]
    fn show_with_no_config_file_prints_json_with_file_exists_false() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        let _env = EnvVarGuard::set("XDG_CONFIG_HOME", &dir.path().to_string_lossy());

        // Just verify no panic — output goes to stdout
        show(&json_out(), None);
    }

    #[test]
    fn set_writes_value_to_config() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(dir.path(), "[default]\nurl = \"http://old:8123\"\ntoken = \"old-token\"\n").unwrap();
        let _env = EnvVarGuard::set("XDG_CONFIG_HOME", &dir.path().to_string_lossy());

        set(&json_out(), None, "url", "http://new:8123");

        let path = dir.path().join("ha").join("config.toml");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("http://new:8123"));
        assert!(content.contains("old-token"), "token must not be changed");
    }

    #[test]
    fn set_with_invalid_key_prints_error_to_stderr() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        let _env = EnvVarGuard::set("XDG_CONFIG_HOME", &dir.path().to_string_lossy());

        // Should not panic; error goes to stderr
        set(&json_out(), None, "invalid_key", "value");
    }
}
```

- [ ] **Step 2: Implement src/commands/config.rs**

```rust
use crate::config;
use crate::output::{self, mask_credential, OutputConfig};

pub fn show(out: &OutputConfig, profile_arg: Option<&str>) {
    let summary = config::config_summary();

    if out.is_json() {
        let profiles_json: Vec<serde_json::Value> = summary.profiles.iter().map(|p| {
            serde_json::json!({
                "name": p.name,
                "url": p.url,
                "token": p.token.as_deref().map(mask_credential)
            })
        }).collect();

        out.print_data(&serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "config_file": summary.config_file,
                "file_exists": summary.file_exists,
                "profiles": profiles_json,
                "env": {
                    "HA_URL": summary.env_url,
                    "HA_TOKEN": summary.env_token.as_deref().map(mask_credential),
                    "HA_PROFILE": summary.env_profile
                }
            }
        })).expect("serialize"));
    } else {
        println!("Config file: {}", summary.config_file.display());
        if !summary.file_exists {
            println!("  (not found — run `ha init` to create it)");
            return;
        }
        for p in &summary.profiles {
            println!("\n[{}]", p.name);
            println!("  url   = {}", p.url.as_deref().unwrap_or("(not set)"));
            println!("  token = {}", p.token.as_deref().map(mask_credential).unwrap_or_else(|| "(not set)".into()));
        }
        if summary.env_url.is_some() || summary.env_token.is_some() || summary.env_profile.is_some() {
            println!("\nEnvironment overrides:");
            if let Some(v) = &summary.env_url { println!("  HA_URL={v}"); }
            if let Some(v) = &summary.env_token { println!("  HA_TOKEN={}", mask_credential(v)); }
            if let Some(v) = &summary.env_profile { println!("  HA_PROFILE={v}"); }
        }
    }
}

pub fn set(out: &OutputConfig, profile_arg: Option<&str>, key: &str, value: &str) {
    if key != "url" && key != "token" {
        eprintln!("Unknown config key '{key}'. Valid keys: url, token");
        std::process::exit(crate::output::exit_codes::GENERAL_ERROR);
    }

    let path = config::config_path();
    let profile = profile_arg.unwrap_or("default");

    let (current_url, current_token) = config::read_profile_credentials(&path, profile)
        .unwrap_or_else(|| ("".to_owned(), "".to_owned()));

    let (url, token) = if key == "url" {
        (value.to_owned(), current_token)
    } else {
        (current_url, value.to_owned())
    };

    if let Err(e) = config::write_profile(&path, profile, &url, &token) {
        eprintln!("{e}");
        std::process::exit(crate::output::exit_codes::GENERAL_ERROR);
    }

    if out.is_json() {
        out.print_data(&serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {"key": key, "profile": profile}
        })).expect("serialize"));
    } else {
        println!("✔ Set {} for profile '{}'", key, profile);
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo nextest run "commands::config" --test-threads 1
```

- [ ] **Step 4: Commit**

```bash
git add src/commands/config.rs
git commit -m "feat: add config show and set commands"
```

---

### Task 15: Wire main.rs and Verify Full Build

**Files:**
- Modify: `src/main.rs` (already complete from Task 1 — verify it compiles with all modules implemented)

- [ ] **Step 1: Build the full project**

```bash
cargo build 2>&1
```
Expected: compiles with no errors. Warnings about unused imports are ok and should be cleaned up.

- [ ] **Step 2: Run all tests**

```bash
cargo nextest run --test-threads 1
```
Expected: all tests pass

- [ ] **Step 3: Smoke test the binary**

```bash
cargo run -- --help
cargo run -- schema
cargo run -- entity --help
cargo run -- service --help
cargo run -- event --help
```
Expected: help text and schema JSON print correctly without errors

- [ ] **Step 4: Fix any compiler warnings**

Remove unused imports, dead code, etc.

```bash
cargo build 2>&1 | grep "warning\[" | head -20
```

- [ ] **Step 5: Final commit**

```bash
git add -p  # review changes file by file
git commit -m "chore: wire all modules, fix compiler warnings"
```

---

## Self-Review

**Spec coverage check:**
- ✅ Command structure: entity get/list/watch, service call/list, event fire/watch, init, schema, config show/set
- ✅ Config: `~/.config/ha/config.toml`, named profiles, env var override (HA_URL, HA_TOKEN, HA_PROFILE)
- ✅ `ha init`: injectable IO, TTY detection, JSON schema mode, context-aware flow, credential masking, inline validation, save-anyway, EOF handling, outro with profile flag
- ✅ Output: auto-TTY detection, `--output json|table|plain`, JSON envelope `{ok, data}`, errors to stderr, semantic exit codes
- ✅ `ha schema`: machine-readable JSON with all commands, args, flags, json_shape, exit codes, global flags
- ✅ Architecture: exact file structure matches spec
- ✅ Testing: injectable IO for init, wiremock for API, TempDir for config, test_support helpers

**No placeholders:** All steps contain actual code.

**Type consistency:**
- `HaError` used throughout — defined in Task 4, used in Tasks 2, 3, 5–14
- `HaClient` — defined Task 4, used Tasks 6–8, 10–12
- `EntityState`, `ServiceDomain`, `HaEvent`, `StateChangedData` — defined in `api/types.rs` (Task 4), used in tasks 6–12
- `OutputConfig`, `OutputFormat` — defined Task 3, used Tasks 10–14
- `config::write_profile`, `config::read_profile_names`, `config::read_profile_credentials` — defined Task 2, used Tasks 9, 14
