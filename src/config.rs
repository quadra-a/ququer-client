use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_server")]
    pub server: String,
    #[serde(default = "default_output")]
    pub output: String,
}

fn default_server() -> String {
    "https://ququer.ai".to_string()
}

fn default_output() -> String {
    "json".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: default_server(),
            output: default_output(),
        }
    }
}

pub fn set_config_dir(path: PathBuf) {
    CONFIG_DIR.set(path).ok();
}

pub fn ququer_dir() -> Result<PathBuf> {
    let dir = match CONFIG_DIR.get() {
        Some(p) => p.clone(),
        None => dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?
            .join(".ququer"),
    };
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn load_config() -> Result<Config> {
    let path = ququer_dir()?.join("config.toml");
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        Ok(toml::from_str(&content)?)
    } else {
        Ok(Config::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let config = Config::default();
        assert_eq!(config.server, "https://ququer.ai");
        assert_eq!(config.output, "json");
    }

    #[test]
    fn parse_partial_toml() {
        let toml_str = r#"server = "https://example.com""#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server, "https://example.com");
        assert_eq!(config.output, "json"); // default
    }

    #[test]
    fn parse_full_toml() {
        let toml_str = r#"
server = "https://ququer.io"
output = "text"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server, "https://ququer.io");
        assert_eq!(config.output, "text");
    }

    #[test]
    fn parse_empty_toml_uses_defaults() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.server, "https://ququer.ai");
        assert_eq!(config.output, "json");
    }
}
