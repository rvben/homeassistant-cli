use clap::ValueEnum;

#[derive(Clone, Debug, ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
    Plain,
}

pub struct OutputConfig {
    pub format: OutputFormat,
    pub quiet: bool,
}

impl OutputConfig {
    pub fn new(format: Option<OutputFormat>, quiet: bool) -> Self {
        Self {
            format: format.unwrap_or(OutputFormat::Plain),
            quiet,
        }
    }
}

pub mod exit_codes {
    use crate::api::HaError;

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
