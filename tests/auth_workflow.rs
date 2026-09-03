use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn config_path(config_home: &TempDir) -> std::path::PathBuf {
    config_home.path().join("ha/config.toml")
}

fn write_profiles(config_home: &TempDir) {
    let path = config_path(config_home);
    std::fs::create_dir_all(path.parent().expect("config parent")).expect("config directory");
    std::fs::write(
        path,
        r#"active_profile = "default"

[default]
url = "http://default-ha.local:8123"
token = "default-token"

[cabin]
url = "http://cabin-ha.local:8123"
token = "cabin-token"
"#,
    )
    .expect("config file");
}

fn ha(config_home: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ha"))
        .args(args)
        .env("XDG_CONFIG_HOME", config_home.path())
        .env_remove("HA_URL")
        .env_remove("HA_TOKEN")
        .env_remove("HA_PROFILE")
        .output()
        .expect("ha command")
}

fn stdout_json(output: Output) -> Value {
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("JSON stdout")
}

#[test]
fn canonical_profile_auth_and_config_workflow() {
    let config_home = TempDir::new().expect("temp config home");
    write_profiles(&config_home);

    let profiles = stdout_json(ha(&config_home, &["profile", "list", "--output", "json"]));
    assert_eq!(profiles["total"], 2);
    assert_eq!(profiles["items"][0]["name"], "default");
    assert_eq!(profiles["items"][0]["active"], true);

    let selected = stdout_json(ha(
        &config_home,
        &["profile", "use", "cabin", "--output", "json"],
    ));
    assert_eq!(selected["profile"], "cabin");
    assert_eq!(selected["active"], true);

    let status = stdout_json(ha(
        &config_home,
        &["auth", "status", "--offline", "--output", "json"],
    ));
    assert_eq!(status["profile"], "cabin");
    assert_eq!(status["verified"], false);
    assert_eq!(status["credential_source"], "config-file");

    let doctor = stdout_json(ha(
        &config_home,
        &["doctor", "--offline", "--output", "json"],
    ));
    assert_eq!(doctor["ok"], true);
    assert_eq!(doctor["offline"], true);
    assert_eq!(doctor["checks"].as_array().map(Vec::len), Some(4));

    let shown = stdout_json(ha(&config_home, &["config", "show", "--output", "json"]));
    assert_eq!(shown["data"]["active_profile"], "cabin");

    let path = stdout_json(ha(&config_home, &["config", "path", "--output", "json"]));
    assert_eq!(
        path["config_path"].as_str(),
        Some(config_path(&config_home).to_string_lossy().as_ref())
    );

    let logout = stdout_json(ha(&config_home, &["auth", "logout", "--output", "json"]));
    assert_eq!(logout["profile"], "cabin");
    assert_eq!(logout["credential_removed"], true);

    let config: toml::Value = toml::from_str(
        &std::fs::read_to_string(config_path(&config_home)).expect("updated config"),
    )
    .expect("valid TOML");
    assert_eq!(
        config["cabin"]["url"].as_str(),
        Some("http://cabin-ha.local:8123")
    );
    assert!(config["cabin"].get("token").is_none());
    assert_eq!(config["default"]["token"].as_str(), Some("default-token"));

    stdout_json(ha(
        &config_home,
        &["config", "set", "token", "refreshed", "--output", "json"],
    ));
    let refreshed: toml::Value = toml::from_str(
        &std::fs::read_to_string(config_path(&config_home)).expect("refreshed config"),
    )
    .expect("valid TOML");
    assert_eq!(
        refreshed["cabin"]["url"].as_str(),
        Some("http://cabin-ha.local:8123")
    );
    assert_eq!(refreshed["cabin"]["token"].as_str(), Some("refreshed"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(config_path(&config_home))
            .expect("config metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
}

#[test]
fn profile_remove_requires_yes_and_updates_active_profile() {
    let config_home = TempDir::new().expect("temp config home");
    write_profiles(&config_home);
    stdout_json(ha(
        &config_home,
        &["profile", "use", "cabin", "--output", "json"],
    ));

    let denied = ha(
        &config_home,
        &["profile", "remove", "cabin", "--output", "json"],
    );
    assert_eq!(denied.status.code(), Some(6));

    let removed = stdout_json(ha(
        &config_home,
        &["profile", "remove", "cabin", "--yes", "--output", "json"],
    ));
    assert_eq!(removed["profile"], "cabin");
    assert_eq!(removed["removed"], true);

    let shown = stdout_json(ha(&config_home, &["config", "show", "--output", "json"]));
    assert_eq!(shown["data"]["active_profile"], "default");
}

#[test]
fn auth_login_and_init_share_the_setup_flow() {
    let config_home = TempDir::new().expect("temp config home");
    let login = stdout_json(ha(
        &config_home,
        &["auth", "login", "--profile", "cabin", "--output", "json"],
    ));
    let init = stdout_json(ha(
        &config_home,
        &["init", "--profile", "cabin", "--output", "json"],
    ));
    assert_eq!(login["requiredFields"], init["requiredFields"]);
    assert_eq!(login["configPath"], init["configPath"]);
}

#[test]
fn schema_advertises_the_standard_auth_contract() {
    let config_home = TempDir::new().expect("temp config home");
    let schema = stdout_json(ha(&config_home, &["schema"]));
    let names = schema["commands"]
        .as_array()
        .expect("commands")
        .iter()
        .filter_map(|command| command["name"].as_str())
        .collect::<Vec<_>>();
    for required in [
        "auth login",
        "auth status",
        "auth logout",
        "profile list",
        "profile use",
        "profile remove",
        "config show",
        "config path",
        "doctor",
    ] {
        assert!(names.contains(&required), "schema missing {required}");
    }
}

#[test]
fn help_is_an_informational_exit_without_an_error_envelope() {
    let config_home = TempDir::new().expect("temp config home");
    for args in [
        &["--help"][..],
        &["auth", "--help"][..],
        &["profile", "--help"][..],
    ] {
        let output = ha(&config_home, args);
        assert!(output.status.success());
        assert!(
            output.stderr.is_empty(),
            "unexpected help error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!String::from_utf8_lossy(&output.stdout).contains("\"error\""));
    }
}
