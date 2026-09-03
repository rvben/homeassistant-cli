use crate::api::{HaClient, HaError};
use crate::config::{self, Config};
use crate::output::OutputConfig;

pub async fn login(profile: Option<String>) {
    super::init::init(profile).await;
}

pub async fn status(
    profile: Option<String>,
    offline: bool,
    out: &OutputConfig,
) -> Result<(), HaError> {
    let config = Config::load(profile)?;
    if !offline {
        HaClient::new(&config.url, &config.token).validate().await?;
    }
    out.print_result(
        &serde_json::json!({
            "profile": config.profile,
            "status": if offline { "configured" } else { "ok" },
            "configured": true,
            "verified": !offline,
            "credential_source": config.credential_source,
            "url": config.url,
        }),
        &format!(
            "Profile '{}' is {} at {} ({}).",
            config.profile,
            if offline {
                "configured; network not checked"
            } else {
                "authenticated"
            },
            config.url,
            config.credential_source
        ),
    );
    Ok(())
}

pub fn logout(profile: Option<&str>, out: &OutputConfig) -> Result<(), HaError> {
    let profile = config::selected_profile_name(profile)?;
    let removed = config::remove_profile_token(&config::config_path(), &profile)?;
    out.print_result(
        &serde_json::json!({
            "profile": profile,
            "logged_out": true,
            "credential_removed": removed,
            "environment_override": std::env::var_os("HA_TOKEN").is_some(),
        }),
        &format!("Logged out profile '{profile}'."),
    );
    Ok(())
}
