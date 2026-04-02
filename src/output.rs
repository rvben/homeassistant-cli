use clap::ValueEnum;

#[derive(Clone, Debug, ValueEnum)]
pub enum OutputFormat {
    Json,
    Text,
}

pub struct OutputConfig {
    pub format: OutputFormat,
    pub quiet: bool,
}

impl OutputConfig {
    pub fn new(format: Option<OutputFormat>, quiet: bool) -> Self {
        Self {
            format: format.unwrap_or(OutputFormat::Text),
            quiet,
        }
    }
}

pub mod exit_codes {
    use crate::api::HaError;

    pub const CONFIG_ERROR: i32 = 2;
    pub const API_ERROR: i32 = 3;
    pub const NOT_FOUND: i32 = 4;

    pub fn for_error(e: &HaError) -> i32 {
        match e {
            HaError::NotFound(_) => NOT_FOUND,
            HaError::Config(_) => CONFIG_ERROR,
            _ => API_ERROR,
        }
    }
}
