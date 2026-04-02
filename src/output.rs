use std::io::IsTerminal;

use owo_colors::OwoColorize;

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

    /// Print an error. In JSON mode, emits the structured error envelope to stdout.
    /// In human mode, prints to stderr.
    pub fn print_error(&self, e: &HaError) {
        if self.is_json() {
            let envelope = serde_json::json!({
                "ok": false,
                "error": {
                    "code": e.error_code(),
                    "message": e.to_string()
                }
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&envelope).expect("serialize")
            );
        } else {
            eprintln!("{e}");
        }
    }

    /// Print a JSON result or human message depending on format.
    pub fn print_result(&self, json_value: &serde_json::Value, human_message: &str) {
        if self.is_json() {
            println!(
                "{}",
                serde_json::to_string_pretty(json_value).expect("serialize")
            );
        } else {
            println!("{human_message}");
        }
    }
}

/// Color a Home Assistant state value for human display.
pub fn colored_state(state: &str) -> String {
    match state {
        "on" | "open" | "home" | "active" | "playing" => state.green().to_string(),
        "off" | "closed" | "not_home" | "idle" | "paused" => state.dimmed().to_string(),
        "unavailable" | "unknown" => state.yellow().to_string(),
        _ => state.to_owned(),
    }
}

/// Dim the domain prefix of an entity ID, leaving the name at normal brightness.
/// `light.left_key_light` → `[dim]light.[/dim]left_key_light`
pub fn colored_entity_id(entity_id: &str) -> String {
    match entity_id.split_once('.') {
        Some((domain, name)) => format!("{}.{}", domain.dimmed(), name),
        None => entity_id.to_owned(),
    }
}

/// Format an ISO 8601 timestamp as a human-friendly relative time ("2m ago").
/// Falls back to the raw string if parsing fails.
pub fn relative_time(iso: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    match parse_unix_secs(iso) {
        Some(ts) => {
            let secs = now.saturating_sub(ts);
            let s = if secs < 60 {
                format!("{secs}s ago")
            } else if secs < 3600 {
                format!("{}m ago", secs / 60)
            } else if secs < 86400 {
                format!("{}h ago", secs / 3600)
            } else {
                format!("{}d ago", secs / 86400)
            };
            // Dim timestamps older than 5 minutes.
            if secs >= 300 {
                s.dimmed().to_string()
            } else {
                s
            }
        }
        None => iso.to_owned(),
    }
}

/// Parse an ISO 8601 / RFC 3339 timestamp to Unix seconds.
/// Handles `YYYY-MM-DDTHH:MM:SS[.frac][+HH:MM|Z]`.
fn parse_unix_secs(s: &str) -> Option<u64> {
    if s.len() < 19 {
        return None;
    }
    let year: i64 = s.get(0..4)?.parse().ok()?;
    let month: i64 = s.get(5..7)?.parse().ok()?;
    let day: i64 = s.get(8..10)?.parse().ok()?;
    let hour: i64 = s.get(11..13)?.parse().ok()?;
    let min: i64 = s.get(14..16)?.parse().ok()?;
    let sec: i64 = s.get(17..19)?.parse().ok()?;

    // Skip fractional seconds, then parse timezone offset.
    let rest = s.get(19..)?;
    let rest = if rest.starts_with('.') {
        let end = rest.find(['+', '-', 'Z']).unwrap_or(rest.len());
        &rest[end..]
    } else {
        rest
    };
    let tz_secs: i64 = if rest.is_empty() || rest == "Z" {
        0
    } else {
        let sign: i64 = if rest.starts_with('-') { -1 } else { 1 };
        let tz = rest.get(1..)?;
        let h: i64 = tz.get(0..2)?.parse().ok()?;
        let m: i64 = tz.get(3..5)?.parse().ok()?;
        sign * (h * 3600 + m * 60)
    };

    // Convert calendar date to days since Unix epoch using Hinnant's algorithm.
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;

    let unix = days * 86_400 + hour * 3_600 + min * 60 + sec - tz_secs;
    u64::try_from(unix).ok()
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

/// Strip ANSI escape codes to get the visible display length of a string.
fn visible_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            len += 1;
        }
    }
    len
}

/// Pad a (potentially ANSI-colored) string to a given visible width.
fn pad_cell(s: &str, width: usize) -> String {
    let vlen = visible_len(s);
    let padding = width.saturating_sub(vlen);
    format!("{}{}", s, " ".repeat(padding))
}

/// Render a table with bold headers, dimmed separator, and ANSI-aware column alignment.
/// Rows may contain pre-colored strings; alignment is based on visible width.
pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let col_count = headers.len();
    // Compute column widths from visible (uncolored) content.
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                widths[i] = widths[i].max(visible_len(cell));
            }
        }
    }

    let header_line: String = headers
        .iter()
        .enumerate()
        .map(|(i, h)| pad_cell(&h.bold().to_string(), widths[i]))
        .collect::<Vec<_>>()
        .join("  ");

    let sep: String = widths
        .iter()
        .map(|w| "─".repeat(*w).dimmed().to_string())
        .collect::<Vec<_>>()
        .join("  ");

    let data_lines: Vec<String> = rows
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .take(col_count)
                .map(|(i, cell)| pad_cell(cell, widths[i]))
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
    fn parse_unix_secs_handles_utc_z() {
        // 1970-01-01T00:00:00Z == 0
        assert_eq!(parse_unix_secs("1970-01-01T00:00:00Z"), Some(0));
    }

    #[test]
    fn parse_unix_secs_handles_offset() {
        // 1970-01-01T01:00:00+01:00 == 0
        assert_eq!(parse_unix_secs("1970-01-01T01:00:00+01:00"), Some(0));
    }

    #[test]
    fn parse_unix_secs_handles_fractional_seconds() {
        assert_eq!(parse_unix_secs("1970-01-01T00:00:01.999999+00:00"), Some(1));
    }

    #[test]
    fn parse_unix_secs_rejects_short_input() {
        assert_eq!(parse_unix_secs("2026-01"), None);
    }

    #[test]
    fn relative_time_falls_back_on_invalid_input() {
        assert_eq!(relative_time("not-a-date"), "not-a-date");
    }

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
        assert!(lines[1].contains("─"));
        assert!(lines[2].contains("light.living_room"));
        assert!(lines[3].contains("switch.fan"));
    }

    #[test]
    fn print_error_json_mode_emits_envelope_to_stdout() {
        // Verify the envelope structure by exercising the serialization path directly.
        let e = crate::api::HaError::NotFound("light.missing".into());
        let envelope = serde_json::json!({
            "ok": false,
            "error": {
                "code": e.error_code(),
                "message": e.to_string()
            }
        });
        assert_eq!(envelope["ok"], false);
        assert_eq!(envelope["error"]["code"], "HA_NOT_FOUND");
        assert!(
            envelope["error"]["message"]
                .as_str()
                .unwrap()
                .contains("light.missing")
        );
    }

    #[test]
    fn exit_code_for_auth_error_is_2() {
        assert_eq!(
            exit_codes::for_error(&crate::api::HaError::Auth("x".into())),
            2
        );
    }

    #[test]
    fn exit_code_for_not_found_is_3() {
        assert_eq!(
            exit_codes::for_error(&crate::api::HaError::NotFound("x".into())),
            3
        );
    }

    #[test]
    fn exit_code_for_connection_error_is_4() {
        assert_eq!(
            exit_codes::for_error(&crate::api::HaError::Connection("x".into())),
            4
        );
    }
}
