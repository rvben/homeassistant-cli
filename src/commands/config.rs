use crate::config;
use crate::output::{mask_credential, OutputConfig};

pub fn show(out: &OutputConfig, profile_arg: Option<&str>) {
    let _ = profile_arg; // profile_arg reserved for future use; show always displays all profiles
    let summary = config::config_summary();

    if out.is_json() {
        let profiles_json: Vec<serde_json::Value> = summary
            .profiles
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "url": p.url,
                    "token": p.token.as_deref().map(mask_credential)
                })
            })
            .collect();

        out.print_data(
            &serde_json::to_string_pretty(&serde_json::json!({
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
            }))
            .expect("serialize"),
        );
    } else {
        println!("Config file: {}", summary.config_file.display());
        if !summary.file_exists {
            println!("  (not found — run `ha init` to create it)");
            return;
        }
        for p in &summary.profiles {
            println!("\n[{}]", p.name);
            println!(
                "  url   = {}",
                p.url.as_deref().unwrap_or("(not set)")
            );
            println!(
                "  token = {}",
                p.token
                    .as_deref()
                    .map(mask_credential)
                    .unwrap_or_else(|| "(not set)".into())
            );
        }
        if summary.env_url.is_some()
            || summary.env_token.is_some()
            || summary.env_profile.is_some()
        {
            println!("\nEnvironment overrides:");
            if let Some(v) = &summary.env_url {
                println!("  HA_URL={v}");
            }
            if let Some(v) = &summary.env_token {
                println!("  HA_TOKEN={}", mask_credential(v));
            }
            if let Some(v) = &summary.env_profile {
                println!("  HA_PROFILE={v}");
            }
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

    let (current_url, current_token) =
        config::read_profile_credentials(&path, profile).unwrap_or_default();

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
        out.print_data(
            &serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "data": {"key": key, "profile": profile}
            }))
            .expect("serialize"),
        );
    } else {
        println!("✔ Set {} for profile '{}'", key, profile);
    }
}

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
    fn show_does_not_panic_with_no_config() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        let _env = EnvVarGuard::set("XDG_CONFIG_HOME", &dir.path().to_string_lossy());
        // Just verify no panic
        show(&json_out(), None);
    }

    #[test]
    fn set_writes_url_to_config() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            "[default]\nurl = \"http://old:8123\"\ntoken = \"old-token\"\n",
        )
        .unwrap();
        let _env = EnvVarGuard::set("XDG_CONFIG_HOME", &dir.path().to_string_lossy());

        set(&json_out(), None, "url", "http://new:8123");

        let path = dir.path().join("ha").join("config.toml");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("http://new:8123"));
        assert!(content.contains("old-token"), "token must not be changed");
    }
}
