use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub behavior: BehaviorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiConfig {
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorConfig {
    #[serde(default = "default_true")]
    pub bash_confirm: bool,
    #[serde(default = "default_true")]
    pub write_confirm: bool,
    #[serde(default = "default_true")]
    pub auto_save: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_model() -> String {
    "claude-sonnet-4-6".to_string()
}

fn default_base_url() -> String {
    "https://api.anthropic.com".to_string()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            api: ApiConfig::default(),
            behavior: BehaviorConfig::default(),
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        ApiConfig {
            api_key: String::new(),
            model: default_model(),
            base_url: default_base_url(),
            max_tokens: default_max_tokens(),
        }
    }
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        BehaviorConfig {
            bash_confirm: true,
            write_confirm: true,
            auto_save: true,
            theme: default_theme(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {:?}", path))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| "Failed to parse config TOML")?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory {:?}", parent))?;
        }
        let content = toml::to_string_pretty(self)
            .with_context(|| "Failed to serialize config to TOML")?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config to {:?}", path))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_roundtrip() {
        let config = Config {
            api: ApiConfig {
                api_key: "test-key".to_string(),
                model: "claude-sonnet-4-6".to_string(),
                base_url: "https://api.anthropic.com".to_string(),
                max_tokens: 4096,
            },
            behavior: BehaviorConfig {
                bash_confirm: true,
                write_confirm: true,
                auto_save: true,
                theme: "dark".to_string(),
            },
        };

        let mut file = NamedTempFile::new().unwrap();
        let content = toml::to_string(&config).unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let loaded = Config::load(file.path()).unwrap();
        assert_eq!(loaded.api.api_key, "test-key");
        assert_eq!(loaded.api.model, "claude-sonnet-4-6");
        assert_eq!(loaded.behavior.theme, "dark");
    }

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.api.model, "claude-sonnet-4-6");
        assert!(config.behavior.bash_confirm);
        assert_eq!(config.api.max_tokens, 4096);
    }
}
