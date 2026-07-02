use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub save_dir: PathBuf,
    pub silent: bool,
    pub freeze: bool,
    pub notif_timeout: u32,
    pub date_format: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_dir: dirs::picture_dir()
                .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
                .join("Screenshots"),
            silent: false,
            freeze: false,
            notif_timeout: 5000,
            date_format: "%Y-%m-%d-%H%M%S".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        Self::load_from(&default_path())
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            tracing::debug!("no config at {}, using defaults", path.display());
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("parsing {}", path.display()))
    }
}

pub fn default_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
        .join("breadshot")
        .join("config.toml")
}
