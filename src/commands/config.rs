use crate::api::HaError;
use crate::config;
use crate::output::{OutputConfig, mask_credential};

pub fn show(out: &OutputConfig, profile_arg: Option<&str>) {
    let summary = config::config_summary(profile_arg);

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
                    "active_profile": summary.active_profile,
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
        println!("Active profile: {}", summary.active_profile);
        if !summary.file_exists {
            println!("  (not found — run `ha init` to create it)");
            return;
        }
        for p in &summary.profiles {
            println!("\n[{}]", p.name);
            println!("  url   = {}", p.url.as_deref().unwrap_or("(not set)"));
            println!(
                "  token = {}",
                p.token
                    .as_deref()
                    .map(mask_credential)
                    .unwrap_or_else(|| "(not set)".into())
            );
        }
        if summary.env_url.is_some() || summary.env_token.is_some() || summary.env_profile.is_some()
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

pub fn path(out: &OutputConfig) {
    let path = config::config_path();
    out.print_result(
        &serde_json::json!({"config_path": path}),
        &path.display().to_string(),
    );
}

pub fn profile_list(out: &OutputConfig, profile_arg: Option<&str>) {
    let summary = config::config_summary(profile_arg);
    let items = summary
        .profiles
        .iter()
        .map(|profile| {
            serde_json::json!({
                "name": profile.name,
                "active": profile.name == summary.active_profile,
                "url": profile.url,
                "configured": profile.url.as_ref().is_some_and(|value| !value.is_empty())
                    && profile.token.as_ref().is_some_and(|value| !value.is_empty()),
            })
        })
        .collect::<Vec<_>>();
    if out.is_json() {
        out.print_data(
            &serde_json::to_string_pretty(
                &serde_json::json!({"items": items, "total": items.len()}),
            )
            .expect("serialize profiles"),
        );
    } else if items.is_empty() {
        out.print_data("No profiles configured. Run `ha init`.");
    } else {
        for item in items {
            out.print_data(&format!(
                "{} {:<20} {}",
                if item["active"].as_bool().unwrap_or(false) {
                    "*"
                } else {
                    " "
                },
                item["name"].as_str().unwrap_or_default(),
                item["url"].as_str().unwrap_or("(not set)")
            ));
        }
    }
}

pub fn profile_use(out: &OutputConfig, name: &str) -> Result<(), HaError> {
    config::use_profile(&config::config_path(), name)?;
    out.print_result(
        &serde_json::json!({"profile": name, "active": true}),
        &format!("Active profile set to '{name}'."),
    );
    Ok(())
}

pub fn profile_remove(out: &OutputConfig, name: &str, yes: bool) -> Result<(), HaError> {
    if !yes {
        return Err(HaError::ConfirmationRequired(
            "profile removal requires --yes".to_owned(),
        ));
    }
    config::remove_profile(&config::config_path(), name)?;
    out.print_result(
        &serde_json::json!({"profile": name, "removed": true}),
        &format!("Profile '{name}' removed."),
    );
    Ok(())
}

pub fn set(out: &OutputConfig, profile_arg: Option<&str>, key: &str, value: &str) {
    if key != "url" && key != "token" {
        eprintln!("Unknown config key '{key}'. Valid keys: url, token");
        std::process::exit(crate::output::exit_codes::GENERAL_ERROR);
    }

    let path = config::config_path();
    let profile = match config::selected_profile_name(profile_arg) {
        Ok(profile) => profile,
        Err(error) => {
            out.print_error(&error);
            std::process::exit(crate::output::exit_codes::for_error(&error));
        }
    };

    if let Err(e) = config::write_profile_value(&path, &profile, key, value) {
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
