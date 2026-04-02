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
    #[serde(default)]
    default: RawProfile,
    #[serde(flatten)]
    profiles: BTreeMap<String, RawProfile>,
}

/// Resolved credentials for the active profile.
#[derive(Debug, Clone)]
pub struct Config {
    pub url: String,
    pub token: String,
}

impl Config {
    pub fn load(profile_arg: Option<String>) -> Result<Self, HaError> {
        let file_profile = load_file_profile(profile_arg.as_deref())?;

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

        Ok(Self { url, token })
    }
}

fn load_file_profile(profile_arg: Option<&str>) -> Result<RawProfile, HaError> {
    let path = config_path();
    if !path.exists() {
        return Ok(RawProfile::default());
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| HaError::Other(format!("Failed to read config: {e}")))?;

    let raw: RawConfig = toml::from_str(&content)
        .map_err(|e| HaError::Other(format!("Invalid config file: {e}")))?;

    let profile_name = profile_arg
        .map(|s| s.to_owned())
        .or_else(|| std::env::var("HA_PROFILE").ok().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| "default".to_owned());

    if profile_name == "default" {
        return Ok(raw.default);
    }

    raw.profiles.get(&profile_name).cloned().ok_or_else(|| {
        HaError::InvalidInput(format!("Profile '{profile_name}' not found in config."))
    })
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
    pub env_url: Option<String>,
    pub env_token: Option<String>,
    pub env_profile: Option<String>,
}

pub fn config_summary() -> ConfigSummary {
    let config_file = config_path();
    let file_exists = config_file.exists();
    let mut profiles = Vec::new();

    if file_exists
        && let Ok(content) = std::fs::read_to_string(&config_file)
        && let Ok(raw) = toml::from_str::<RawConfig>(&content)
    {
        profiles.push(ProfileSummary {
            name: "default".into(),
            url: raw.default.url,
            token: raw.default.token,
        });
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

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| HaError::Other(e.to_string()))?;
    }

    let content = toml::to_string(&raw).map_err(|e| HaError::Other(e.to_string()))?;
    std::fs::write(path, content).map_err(|e| HaError::Other(e.to_string()))?;

    Ok(())
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
    let mut names = vec!["default".to_owned()];
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
}
