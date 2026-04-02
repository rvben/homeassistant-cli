use clap::{Parser, Subcommand};

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
