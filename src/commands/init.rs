use std::future::Future;
use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;

use owo_colors::OwoColorize;

use crate::api::HaError;
use crate::config;
use crate::output;

const SEP: &str = "──────────────────────────────────────";

fn sym_q() -> String {
    "?".green().bold().to_string()
}

fn sym_ok() -> String {
    "✔".green().to_string()
}

fn sym_fail() -> String {
    "✖".red().to_string()
}

fn sym_dim(s: &str) -> String {
    s.dimmed().to_string()
}

fn prompt_optional<R: BufRead, W: Write>(
    r: &mut R,
    w: &mut W,
    label: &str,
    default: &str,
) -> String {
    let _ = write!(w, "{} {}  [{}]: ", sym_q(), label, sym_dim(default));
    let _ = w.flush();
    let mut input = String::new();
    r.read_line(&mut input).unwrap_or(0);
    let trimmed = input.trim().to_owned();
    if trimmed.is_empty() {
        default.to_owned()
    } else {
        trimmed
    }
}

fn prompt_required<R: BufRead, W: Write>(
    r: &mut R,
    w: &mut W,
    label: &str,
    hint: &str,
) -> Option<String> {
    loop {
        let _ = write!(
            w,
            "{} {}  {}: ",
            sym_q(),
            label,
            sym_dim(&format!("[{hint}]"))
        );
        let _ = w.flush();
        let mut input = String::new();
        match r.read_line(&mut input) {
            Ok(0) | Err(_) => return None,
            Ok(_) => {}
        }
        let trimmed = input.trim().to_owned();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
        let _ = writeln!(w, "  {} {} is required.", sym_fail(), label);
    }
}

fn prompt_credential_update<R: BufRead, W: Write>(
    r: &mut R,
    w: &mut W,
    label: &str,
    current: &str,
) -> Option<String> {
    let hint = format!("{} (Enter to keep)", output::mask_credential(current));
    let _ = write!(w, "{} {}  {}: ", sym_q(), label, sym_dim(&hint));
    let _ = w.flush();
    let mut input = String::new();
    match r.read_line(&mut input) {
        Ok(0) | Err(_) => return None,
        Ok(_) => {}
    }
    let trimmed = input.trim().to_owned();
    Some(if trimmed.is_empty() {
        current.to_owned()
    } else {
        trimmed
    })
}

fn prompt_confirm<R: BufRead, W: Write>(
    r: &mut R,
    w: &mut W,
    label: &str,
    default_yes: bool,
) -> bool {
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
    println!(
        "{}",
        serde_json::to_string_pretty(&schema).expect("serialize")
    );
}

/// Interactive init flow with injectable IO and async validator for testing.
///
/// `validate` receives (url, token) and returns `Some(display_name)` on success
/// or `None` on auth failure.
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
        } else if existing_profiles.len() == 1 {
            (existing_profiles[0].clone(), true)
        } else {
            let options = existing_profiles.join("/");
            let chosen = prompt_optional(
                reader,
                writer,
                &format!("Profile  [{}]", options),
                &existing_profiles[0],
            );
            let profile = chosen.trim().to_owned();
            if !existing_profiles.contains(&profile) {
                let _ = writeln!(writer, "\n  {} Unknown profile '{}'.", sym_fail(), profile);
                return Ok(());
            }
            (profile, true)
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
        let Some(url) = prompt_required(
            reader,
            writer,
            "Home Assistant URL",
            "http://homeassistant.local:8123",
        ) else {
            let _ = writeln!(writer, "\nAborted.");
            return Ok(());
        };
        let token_url = format!("{}/profile/security", url.trim_end_matches('/'));
        let _ = writeln!(
            writer,
            "  {} Create a token at: {}",
            sym_ok(),
            sym_dim(&token_url)
        );
        let Some(token) = prompt_required(
            reader,
            writer,
            "Long-Lived Access Token",
            "paste token here",
        ) else {
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
    let _ = writeln!(
        writer,
        "  {} Config saved to {}",
        sym_ok(),
        sym_dim(&config_path.display().to_string())
    );
    let _ = writeln!(writer, "  Run: {}", run_cmd.bold());
    let _ = writer.flush();

    Ok(())
}

/// Entry point from main — uses real stdin/stdout and live API validation.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::TempDir;

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

        run_init(
            &mut reader,
            &mut writer,
            &path,
            None,
            |_url, _token| async { Some("Home Assistant".to_string()) },
        )
        .await
        .unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("http://ha.local:8123"));
        assert!(saved.contains("mytoken"));
        let output = String::from_utf8_lossy(&writer);
        assert!(output.contains("http://ha.local:8123/profile/security"));
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

        run_init(
            &mut reader,
            &mut writer,
            &path,
            Some("prod"),
            |_, _| async { Some("HA".into()) },
        )
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

        // \n accepts "update" default at action prompt, then Enter to keep both fields
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

        run_init(
            &mut reader,
            &mut writer,
            &path,
            Some("staging"),
            |_, _| async { Some("HA".into()) },
        )
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
