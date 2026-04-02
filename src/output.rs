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
/// Keeps first 6 and last 4 chars for long values; fully obscures short values.
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

    let sep: String = widths
        .iter()
        .map(|w| "-".repeat(*w))
        .collect::<Vec<_>>()
        .join("  ");

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
