//! Test helpers shared across module tests. Only compiled in test builds.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

static PROCESS_ENV_LOCK: Mutex<()> = Mutex::new(());

pub struct ProcessEnvLock(#[allow(dead_code)] MutexGuard<'static, ()>);

impl ProcessEnvLock {
    pub fn acquire() -> Result<Self, std::sync::PoisonError<MutexGuard<'static, ()>>> {
        Ok(Self(PROCESS_ENV_LOCK.lock()?))
    }
}

/// RAII guard: sets an env var and restores original on drop.
pub struct EnvVarGuard {
    name: String,
    original: Option<String>,
}

impl EnvVarGuard {
    pub fn set(name: &str, value: &str) -> Self {
        let original = std::env::var(name).ok();
        // SAFETY: EnvVarGuard callers must hold ProcessEnvLock, which serializes
        // all env mutations across the test process, preventing concurrent access.
        unsafe { std::env::set_var(name, value) };
        Self { name: name.to_owned(), original }
    }

    pub fn unset(name: &str) -> Self {
        let original = std::env::var(name).ok();
        // SAFETY: EnvVarGuard callers must hold ProcessEnvLock, which serializes
        // all env mutations across the test process, preventing concurrent access.
        unsafe { std::env::remove_var(name) };
        Self { name: name.to_owned(), original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(v) => {
                // SAFETY: EnvVarGuard callers must hold ProcessEnvLock, which serializes
                // all env mutations across the test process, preventing concurrent access.
                unsafe { std::env::set_var(&self.name, v) }
            }
            None => {
                // SAFETY: EnvVarGuard callers must hold ProcessEnvLock, which serializes
                // all env mutations across the test process, preventing concurrent access.
                unsafe { std::env::remove_var(&self.name) }
            }
        }
    }
}

/// Write a config file to `<dir>/ha/config.toml`, creating parent dirs.
pub fn write_config(dir: &Path, content: &str) -> Result<PathBuf, std::io::Error> {
    let config_dir = dir.join("ha");
    std::fs::create_dir_all(&config_dir)?;
    let path = config_dir.join("config.toml");
    std::fs::write(&path, content)?;
    Ok(path)
}
