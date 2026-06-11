use clap::{CommandFactory, Parser, Subcommand};

use homeassistant_cli::output::{OutputConfig, OutputFormat, exit_codes};
use homeassistant_cli::{api, commands};

#[derive(Parser)]
#[command(
    name = "ha",
    version,
    about = "CLI for Home Assistant. Run `ha schema` for machine-readable introspection.",
    arg_required_else_help = true
)]
struct Cli {
    /// Config profile to use [env: HA_PROFILE]
    #[arg(long, env = "HA_PROFILE", global = true)]
    profile: Option<String>,

    /// Output format: auto (default), text, or json. Explicit value always wins over TTY detection. [env: HA_OUTPUT]
    #[arg(long, short = 'o', value_enum, env = "HA_OUTPUT", global = true)]
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

    /// Manage the Home Assistant entity/device/area registry (WebSocket API)
    #[command(subcommand, arg_required_else_help = true)]
    Registry(RegistryCommand),

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

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand)]
enum EntityCommand {
    /// Get the current state of an entity
    Get { entity_id: String },
    /// List all entities, optionally filtered by domain, state, or count
    List {
        #[arg(long)]
        domain: Option<String>,
        /// Filter by state value (e.g. on, off, unavailable)
        #[arg(long)]
        state: Option<String>,
        /// Maximum number of results to return (default: 100)
        #[arg(long, default_value = "100")]
        limit: usize,
        /// Number of results to skip before returning (pagination offset)
        #[arg(long, default_value = "0")]
        offset: usize,
        /// Comma-separated list of fields to include in output (e.g. entity_id,state)
        #[arg(long)]
        fields: Option<String>,
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
        /// Skip the interactive confirmation prompt (required when stdin is not a TTY)
        #[arg(long)]
        yes: bool,
    },
    /// List available services
    List {
        #[arg(long)]
        domain: Option<String>,
        /// Maximum number of results to return (default: 100)
        #[arg(long, default_value = "100")]
        limit: usize,
        /// Number of results to skip before returning (pagination offset)
        #[arg(long, default_value = "0")]
        offset: usize,
        /// Comma-separated list of fields to include in output (e.g. domain,services)
        #[arg(long)]
        fields: Option<String>,
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
        /// Skip the interactive confirmation prompt (required when stdin is not a TTY)
        #[arg(long)]
        yes: bool,
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

#[derive(Subcommand)]
enum RegistryCommand {
    /// Entity registry operations
    #[command(subcommand, arg_required_else_help = true)]
    Entity(RegistryEntityCommand),
}

#[derive(Subcommand)]
enum RegistryEntityCommand {
    /// List registered entities
    List {
        /// Filter by integration/platform (e.g. hue, zha)
        #[arg(long)]
        integration: Option<String>,
        /// Filter by domain (e.g. light, switch)
        #[arg(long)]
        domain: Option<String>,
    },
    /// Remove entities from the registry. Requires --yes in interactive mode.
    Remove {
        /// Entity IDs to remove (one or more)
        #[arg(required = true)]
        entity_ids: Vec<String>,
        /// Print what would be removed without connecting to Home Assistant
        #[arg(long)]
        dry_run: bool,
        /// Skip the interactive confirmation prompt
        #[arg(long)]
        yes: bool,
    },
}

#[tokio::main]
async fn main() {
    // Use try_parse so we can intercept clap errors and emit a structured
    // error envelope as the last line of stderr (spec requirement).
    let cli = Cli::try_parse().unwrap_or_else(|e| {
        // Print clap's human-readable message first, then emit the structured
        // envelope as the very last stderr line (spec: last line of stderr).
        let _ = e.print();
        let message = e
            .to_string()
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("invalid input")
            .trim()
            .to_owned();
        let envelope = serde_json::json!({
            "error": {
                "kind": "invalid_input",
                "message": message,
            }
        });
        eprintln!("{}", serde_json::to_string(&envelope).expect("serialize"));
        std::process::exit(e.exit_code());
    });
    let out = OutputConfig::new(cli.output, cli.quiet);

    match cli.command {
        Command::Init { profile } => {
            commands::init::init(profile).await;
        }
        Command::Schema => {
            commands::schema::print_schema();
        }
        Command::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "ha", &mut std::io::stdout());
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
                    out.print_error(&e);
                    std::process::exit(exit_codes::for_error(&e));
                }
            };
            let client = api::HaClient::new(&cfg.url, &cfg.token);

            let result = match command {
                Command::Entity(cmd) => match cmd {
                    EntityCommand::Get { entity_id } => {
                        commands::entity::get(&out, &client, &entity_id).await
                    }
                    EntityCommand::List {
                        domain,
                        state,
                        limit,
                        offset,
                        fields,
                    } => {
                        commands::entity::list(
                            &out,
                            &client,
                            domain.as_deref(),
                            state.as_deref(),
                            limit,
                            offset,
                            fields.as_deref(),
                        )
                        .await
                    }
                    EntityCommand::Watch { entity_id } => {
                        commands::entity::watch(&out, &client, &entity_id).await
                    }
                },
                Command::Service(cmd) => match cmd {
                    ServiceCommand::Call {
                        service,
                        entity,
                        data,
                        yes,
                    } => {
                        commands::service::call(
                            &out,
                            &client,
                            &service,
                            entity.as_deref(),
                            data.as_deref(),
                            yes,
                        )
                        .await
                    }
                    ServiceCommand::List {
                        domain,
                        limit,
                        offset,
                        fields,
                    } => {
                        commands::service::list(
                            &out,
                            &client,
                            domain.as_deref(),
                            limit,
                            offset,
                            fields.as_deref(),
                        )
                        .await
                    }
                },
                Command::Event(cmd) => match cmd {
                    EventCommand::Fire {
                        event_type,
                        data,
                        yes,
                    } => {
                        commands::event::fire(&out, &client, &event_type, data.as_deref(), yes)
                            .await
                    }
                    EventCommand::Watch { event_type } => {
                        commands::event::watch(&out, &client, event_type.as_deref()).await
                    }
                },
                Command::Registry(cmd) => match cmd {
                    RegistryCommand::Entity(sub) => match sub {
                        RegistryEntityCommand::List {
                            integration,
                            domain,
                        } => {
                            commands::registry::entity_list(
                                &out,
                                &cfg.url,
                                &cfg.token,
                                integration.as_deref(),
                                domain.as_deref(),
                            )
                            .await
                        }
                        RegistryEntityCommand::Remove {
                            entity_ids,
                            dry_run,
                            yes,
                        } => {
                            commands::registry::entity_remove(
                                &out,
                                &cfg.url,
                                &cfg.token,
                                &entity_ids,
                                dry_run,
                                yes,
                            )
                            .await
                        }
                    },
                },
                Command::Init { .. }
                | Command::Schema
                | Command::Config(_)
                | Command::Completions { .. } => unreachable!(),
            };

            if let Err(e) = result {
                let code = exit_codes::for_error(&e);
                out.print_error(&e);
                std::process::exit(code);
            }
        }
    }
}
