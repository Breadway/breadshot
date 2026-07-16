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
        let mut config: Self = toml::from_str(&content)
            .with_context(|| format!("parsing {}", path.display()))?;
        config.save_dir = expand_tilde(config.save_dir);
        Ok(config)
    }
}

pub fn default_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
        .join("breadshot")
        .join("config.toml")
}

/// Expand a leading `~` (or `~/...`) to the user's home directory, the way a
/// shell would. `PathBuf`'s `Deserialize` does no such expansion, so a
/// documented config value like `save_dir = "~/Pictures/Screenshots"` would
/// otherwise be taken literally and create a `./~/Pictures/Screenshots`
/// directory relative to the current working directory.
fn expand_tilde(path: PathBuf) -> PathBuf {
    let Some(s) = path.to_str() else {
        return path;
    };
    if s == "~" {
        return dirs::home_dir().unwrap_or(path);
    }
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_tilde_prefix() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(
            expand_tilde(PathBuf::from("~/Pictures/Screenshots")),
            home.join("Pictures/Screenshots")
        );
    }

    #[test]
    fn expand_tilde_bare() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(expand_tilde(PathBuf::from("~")), home);
    }

    #[test]
    fn expand_tilde_absolute_untouched() {
        let p = PathBuf::from("/var/tmp/shots");
        assert_eq!(expand_tilde(p.clone()), p);
    }

    #[test]
    fn expand_tilde_no_expansion_mid_path() {
        // Only a leading ~ is special, matching shell behavior.
        let p = PathBuf::from("/home/user/~weird");
        assert_eq!(expand_tilde(p.clone()), p);
    }
}
