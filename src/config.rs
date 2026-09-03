use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::api::HaError;

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
struct RawProfile {
    pub url: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct RawConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active_profile: Option<String>,
    #[serde(default)]
    default: RawProfile,
    #[serde(flatten)]
    profiles: BTreeMap<String, RawProfile>,
}

/// Resolved credentials for the active profile.
#[derive(Debug, Clone)]
pub struct Config {
    pub profile: String,
    pub url: String,
    pub token: String,
    pub credential_source: &'static str,
}

impl Config {
    pub fn load(profile_arg: Option<String>) -> Result<Self, HaError> {
        let (profile, file_profile) = load_file_profile(profile_arg.as_deref())?;
        let uses_environment = ["HA_URL", "HA_TOKEN"].iter().any(|name| {
            std::env::var(name)
                .ok()
                .is_some_and(|value| !value.is_empty())
        });

        let url = std::env::var("HA_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| file_profile.url.filter(|s| !s.is_empty()))
            .ok_or_else(|| {
                HaError::InvalidInput("No url configured. Run 'ha init' or set HA_URL.".into())
            })?;

        let token = std::env::var("HA_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| file_profile.token.filter(|s| !s.is_empty()))
            .ok_or_else(|| {
                HaError::InvalidInput("No token configured. Run 'ha init' or set HA_TOKEN.".into())
            })?;

        Ok(Self {
            profile,
            url,
            token,
            credential_source: if uses_environment {
                "environment"
            } else {
                "config-file"
            },
        })
    }
}

fn load_file_profile(profile_arg: Option<&str>) -> Result<(String, RawProfile), HaError> {
    let path = config_path();
    if !path.exists() {
        return Ok((
            profile_arg
                .map(str::to_owned)
                .or_else(|| std::env::var("HA_PROFILE").ok().filter(|s| !s.is_empty()))
                .unwrap_or_else(|| "default".to_owned()),
            RawProfile::default(),
        ));
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| HaError::Other(format!("Failed to read config: {e}")))?;

    let raw: RawConfig = toml::from_str(&content)
        .map_err(|e| HaError::Other(format!("Invalid config file: {e}")))?;

    let profile_name = profile_arg
        .map(|s| s.to_owned())
        .or_else(|| std::env::var("HA_PROFILE").ok().filter(|s| !s.is_empty()))
        .or_else(|| raw.active_profile.clone())
        .unwrap_or_else(|| "default".to_owned());

    if profile_name == "default" {
        return Ok((profile_name, raw.default));
    }

    let profile = raw.profiles.get(&profile_name).cloned().ok_or_else(|| {
        HaError::InvalidInput(format!("Profile '{profile_name}' not found in config."))
    })?;
    Ok((profile_name, profile))
}

pub struct ProfileSummary {
    pub name: String,
    pub url: Option<String>,
    pub token: Option<String>,
}

pub struct ConfigSummary {
    pub config_file: PathBuf,
    pub file_exists: bool,
    pub profiles: Vec<ProfileSummary>,
    pub active_profile: String,
    pub env_url: Option<String>,
    pub env_token: Option<String>,
    pub env_profile: Option<String>,
}

pub fn config_summary(profile_arg: Option<&str>) -> ConfigSummary {
    let config_file = config_path();
    let file_exists = config_file.exists();
    let mut profiles = Vec::new();

    let mut stored_active = None;
    if file_exists
        && let Ok(content) = std::fs::read_to_string(&config_file)
        && let Ok(raw) = toml::from_str::<RawConfig>(&content)
    {
        stored_active = raw.active_profile.clone();
        if raw.default.url.is_some() || raw.default.token.is_some() {
            profiles.push(ProfileSummary {
                name: "default".into(),
                url: raw.default.url,
                token: raw.default.token,
            });
        }
        for (name, p) in raw.profiles {
            profiles.push(ProfileSummary {
                name,
                url: p.url,
                token: p.token,
            });
        }
    }

    ConfigSummary {
        config_file,
        file_exists,
        profiles,
        active_profile: profile_arg
            .map(str::to_owned)
            .or_else(|| std::env::var("HA_PROFILE").ok().filter(|s| !s.is_empty()))
            .or(stored_active)
            .unwrap_or_else(|| "default".to_owned()),
        env_url: std::env::var("HA_URL").ok().filter(|s| !s.is_empty()),
        env_token: std::env::var("HA_TOKEN").ok().filter(|s| !s.is_empty()),
        env_profile: std::env::var("HA_PROFILE").ok().filter(|s| !s.is_empty()),
    }
}

/// Write or update a single profile in the config file.
pub fn write_profile(path: &Path, profile: &str, url: &str, token: &str) -> Result<(), HaError> {
    let mut raw: RawConfig = if path.exists() {
        let content = std::fs::read_to_string(path).map_err(|e| HaError::Other(e.to_string()))?;
        toml::from_str(&content).map_err(|e| HaError::Other(format!("Invalid config: {e}")))?
    } else {
        RawConfig::default()
    };

    let new_profile = RawProfile {
        url: Some(url.to_owned()),
        token: Some(token.to_owned()),
    };

    if profile == "default" {
        raw.default = new_profile;
    } else {
        raw.profiles.insert(profile.to_owned(), new_profile);
    }
    raw.active_profile = Some(profile.to_owned());

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| HaError::Other(e.to_string()))?;
    }

    write_raw_config(path, &raw)
}

pub fn write_profile_value(
    path: &Path,
    profile: &str,
    key: &str,
    value: &str,
) -> Result<(), HaError> {
    let mut raw: RawConfig = if path.exists() {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content)
            .map_err(|error| HaError::Other(format!("Invalid config file: {error}")))?
    } else {
        RawConfig::default()
    };
    let target = if profile == "default" {
        &mut raw.default
    } else {
        raw.profiles.entry(profile.to_owned()).or_default()
    };
    match key {
        "url" => target.url = Some(value.to_owned()),
        "token" => target.token = Some(value.to_owned()),
        _ => {
            return Err(HaError::InvalidInput(format!(
                "Unknown config key '{key}'. Valid keys: url, token"
            )));
        }
    }
    raw.active_profile = Some(profile.to_owned());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    write_raw_config(path, &raw)
}

fn write_raw_config(path: &Path, raw: &RawConfig) -> Result<(), HaError> {
    let content = toml::to_string(raw).map_err(|e| HaError::Other(e.to_string()))?;
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(content.as_bytes())?;
        let mut permissions = file.metadata()?.permissions();
        permissions.set_mode(0o600);
        file.set_permissions(permissions)?;
    }
    #[cfg(not(unix))]
    std::fs::write(path, content)?;
    Ok(())
}

pub fn selected_profile_name(profile_arg: Option<&str>) -> Result<String, HaError> {
    let path = config_path();
    let stored = if path.exists() {
        let content = std::fs::read_to_string(path)?;
        toml::from_str::<RawConfig>(&content)
            .map_err(|error| HaError::Other(format!("Invalid config file: {error}")))?
            .active_profile
    } else {
        None
    };
    Ok(profile_arg
        .map(str::to_owned)
        .or_else(|| std::env::var("HA_PROFILE").ok().filter(|s| !s.is_empty()))
        .or(stored)
        .unwrap_or_else(|| "default".to_owned()))
}

pub fn use_profile(path: &Path, profile: &str) -> Result<(), HaError> {
    let content = std::fs::read_to_string(path)?;
    let mut raw: RawConfig = toml::from_str(&content)
        .map_err(|error| HaError::Other(format!("Invalid config file: {error}")))?;
    let exists = if profile == "default" {
        raw.default.url.is_some() || raw.default.token.is_some()
    } else {
        raw.profiles.contains_key(profile)
    };
    if !exists {
        return Err(HaError::NotFound(format!("Profile '{profile}' not found.")));
    }
    raw.active_profile = Some(profile.to_owned());
    write_raw_config(path, &raw)
}

pub fn remove_profile(path: &Path, profile: &str) -> Result<(), HaError> {
    let content = std::fs::read_to_string(path)?;
    let mut raw: RawConfig = toml::from_str(&content)
        .map_err(|error| HaError::Other(format!("Invalid config file: {error}")))?;
    let removed = if profile == "default" {
        let exists = raw.default.url.is_some() || raw.default.token.is_some();
        raw.default = RawProfile::default();
        exists
    } else {
        raw.profiles.remove(profile).is_some()
    };
    if !removed {
        return Err(HaError::NotFound(format!("Profile '{profile}' not found.")));
    }
    if raw.active_profile.as_deref().unwrap_or("default") == profile {
        raw.active_profile = if raw.default.url.is_some() || raw.default.token.is_some() {
            Some("default".to_owned())
        } else {
            raw.profiles.keys().next().cloned()
        };
    }
    write_raw_config(path, &raw)
}

pub fn remove_profile_token(path: &Path, profile: &str) -> Result<bool, HaError> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let mut raw: RawConfig = toml::from_str(&content)
        .map_err(|error| HaError::Other(format!("Invalid config file: {error}")))?;
    let target = if profile == "default" {
        &mut raw.default
    } else if let Some(profile) = raw.profiles.get_mut(profile) {
        profile
    } else {
        return Ok(false);
    };
    let removed = target.token.take().is_some();
    if removed {
        write_raw_config(path, &raw)?;
    }
    Ok(removed)
}

/// Return all profile names from the config file (default first).
pub fn read_profile_names(path: &Path) -> Vec<String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let raw: RawConfig = match toml::from_str(&content) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut names = Vec::new();
    if raw.default.url.is_some() || raw.default.token.is_some() {
        names.push("default".to_owned());
    }
    names.extend(raw.profiles.into_keys());
    names
}

/// Return (url, token) for an existing profile, or None if not present.
pub fn read_profile_credentials(path: &Path, profile: &str) -> Option<(String, String)> {
    let content = std::fs::read_to_string(path).ok()?;
    let raw: RawConfig = toml::from_str(&content).ok()?;
    let p = if profile == "default" {
        raw.default
    } else {
        raw.profiles.get(profile)?.clone()
    };
    Some((p.url?, p.token?))
}

pub fn config_path() -> PathBuf {
    // Prefer XDG_CONFIG_HOME when set (cross-platform and testable on macOS).
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(dirs::config_dir)
        .unwrap_or_else(|| PathBuf::from("~/.config"));
    base.join("ha").join("config.toml")
}

pub fn schema_config_path_description() -> &'static str {
    "~/.config/ha/config.toml (or $XDG_CONFIG_HOME/ha/config.toml)"
}

pub fn recommended_permissions(path: &Path) -> String {
    format!("chmod 600 {}", path.display())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{EnvVarGuard, ProcessEnvLock, write_config};
    use tempfile::TempDir;

    #[test]
    fn loads_default_profile_from_file() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            "[default]\nurl = \"http://ha.local:8123\"\ntoken = \"abc123\"\n",
        )
        .unwrap();
        let _env = EnvVarGuard::set("XDG_CONFIG_HOME", &dir.path().to_string_lossy());
        let _url = EnvVarGuard::unset("HA_URL");
        let _token = EnvVarGuard::unset("HA_TOKEN");

        let cfg = Config::load(None).unwrap();
        assert_eq!(cfg.url, "http://ha.local:8123");
        assert_eq!(cfg.token, "abc123");
    }

    #[test]
    fn env_vars_override_file() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(
            dir.path(),
            "[default]\nurl = \"http://ha.local:8123\"\ntoken = \"file-token\"\n",
        )
        .unwrap();
        let _env = EnvVarGuard::set("XDG_CONFIG_HOME", &dir.path().to_string_lossy());
        let _url = EnvVarGuard::set("HA_URL", "http://override:8123");
        let _token = EnvVarGuard::set("HA_TOKEN", "env-token");

        let cfg = Config::load(None).unwrap();
        assert_eq!(cfg.url, "http://override:8123");
        assert_eq!(cfg.token, "env-token");
    }

    #[test]
    fn named_profile_is_loaded() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        write_config(dir.path(), "[default]\nurl = \"http://default:8123\"\ntoken = \"t1\"\n\n[prod]\nurl = \"http://prod:8123\"\ntoken = \"t2\"\n").unwrap();
        let _env = EnvVarGuard::set("XDG_CONFIG_HOME", &dir.path().to_string_lossy());
        let _url = EnvVarGuard::unset("HA_URL");
        let _token = EnvVarGuard::unset("HA_TOKEN");

        let cfg = Config::load(Some("prod".into())).unwrap();
        assert_eq!(cfg.url, "http://prod:8123");
        assert_eq!(cfg.token, "t2");
    }

    #[test]
    fn missing_config_returns_err() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        let _env = EnvVarGuard::set("XDG_CONFIG_HOME", &dir.path().to_string_lossy());
        let _url = EnvVarGuard::unset("HA_URL");
        let _token = EnvVarGuard::unset("HA_TOKEN");

        let result = Config::load(None);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("ha init"), "should hint at ha init");
    }

    #[test]
    fn write_profile_creates_file_and_reads_back() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        write_profile(&path, "default", "http://ha.local:8123", "mytoken").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[default]"));
        assert!(content.contains("http://ha.local:8123"));
        assert!(content.contains("mytoken"));
    }

    #[test]
    fn config_path_uses_xdg_config_home() {
        let _lock = ProcessEnvLock::acquire().unwrap();
        let dir = TempDir::new().unwrap();
        let _env = EnvVarGuard::set("XDG_CONFIG_HOME", &dir.path().to_string_lossy());

        let path = config_path();
        assert!(path.starts_with(dir.path()));
        assert!(path.ends_with("config.toml"));
    }

    #[test]
    fn removing_legacy_default_selects_the_next_profile() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            "[default]\nurl = \"http://default\"\ntoken = \"one\"\n\n[cabin]\nurl = \"http://cabin\"\ntoken = \"two\"\n",
        )
        .unwrap();

        remove_profile(&path, "default").unwrap();

        let saved: RawConfig = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved.active_profile.as_deref(), Some("cabin"));
        assert!(saved.default.url.is_none());
    }
}
