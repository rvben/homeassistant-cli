use crate::api::{HaClient, HaError};
use crate::config::Config;
use crate::output::OutputConfig;

pub async fn run(
    profile: Option<String>,
    offline: bool,
    out: &OutputConfig,
) -> Result<(), HaError> {
    let config = Config::load(profile)?;
    let authentication = if offline {
        "network check skipped".to_owned()
    } else {
        HaClient::new(&config.url, &config.token).validate().await?
    };
    let checks = serde_json::json!([
        {"name": "configuration", "ok": true, "detail": format!("profile '{}'", config.profile)},
        {"name": "credentials", "ok": true, "detail": config.credential_source},
        {"name": "url", "ok": true, "detail": config.url},
        {"name": "authentication", "ok": true, "detail": authentication},
    ]);
    if out.is_json() {
        out.print_data(
            &serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "offline": offline,
                "checks": checks,
            }))
            .expect("serialize doctor result"),
        );
    } else {
        out.print_data(if offline {
            "Home Assistant connection (offline)"
        } else {
            "Home Assistant connection"
        });
        for check in checks.as_array().expect("checks are an array") {
            out.print_data(&format!(
                "  ✓ {:<16} {}",
                check["name"].as_str().unwrap_or("check"),
                check["detail"].as_str().unwrap_or_default()
            ));
        }
    }
    Ok(())
}
