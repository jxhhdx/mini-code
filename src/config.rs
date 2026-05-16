use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::permissions::PermissionLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub permissions: PermissionsConfig,
    #[serde(default)]
    pub behavior: BehaviorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionsConfig {
    #[serde(default = "default_permission")]
    pub bash: PermissionLevel,
    #[serde(default = "default_permission")]
    pub write_file: PermissionLevel,
    #[serde(default = "default_permission")]
    pub read_file: PermissionLevel,
    #[serde(default = "default_permission")]
    pub edit_file: PermissionLevel,
    #[serde(default = "default_permission")]
    pub glob: PermissionLevel,
    #[serde(default = "default_permission")]
    pub grep: PermissionLevel,
    #[serde(default = "default_permission")]
    pub list_dir: PermissionLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    #[serde(default = "default_true")]
    pub auto_save: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_model() -> String {
    "claude-sonnet-4-6".into()
}

fn default_base_url() -> String {
    "https://api.anthropic.com".into()
}

fn default_max_tokens() -> u32 {
    8192
}

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "dark".into()
}

fn default_permission() -> PermissionLevel {
    PermissionLevel::Confirm
}

impl Default for Config {
    fn default() -> Self {
        Config {
            api: ApiConfig::default(),
            permissions: PermissionsConfig::default(),
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

impl Default for PermissionsConfig {
    fn default() -> Self {
        PermissionsConfig {
            bash: PermissionLevel::Confirm,
            write_file: PermissionLevel::Confirm,
            read_file: PermissionLevel::Safe,
            edit_file: PermissionLevel::Confirm,
            glob: PermissionLevel::Safe,
            grep: PermissionLevel::Safe,
            list_dir: PermissionLevel::Safe,
        }
    }
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        BehaviorConfig {
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
                .with_context(|| format!("Failed to create config dir {:?}", parent))?;
        }
        let content = toml::to_string_pretty(self)
            .with_context(|| "Failed to serialize config")?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config to {:?}", path))?;
        Ok(())
    }

    pub fn permission_for(&self, tool: &str) -> PermissionLevel {
        match tool {
            "bash" => self.permissions.bash,
            "write_file" => self.permissions.write_file,
            "read_file" => self.permissions.read_file,
            "edit_file" => self.permissions.edit_file,
            "glob" => self.permissions.glob,
            "grep" => self.permissions.grep,
            "list_dir" => self.permissions.list_dir,
            _ => PermissionLevel::Confirm,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_config_roundtrip() {
        let config = Config::default();
        let mut file = NamedTempFile::new().unwrap();
        let content = toml::to_string(&config).unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let loaded = Config::load(file.path()).unwrap();
        assert_eq!(loaded.api.model, "claude-sonnet-4-6");
        assert_eq!(loaded.api.max_tokens, 8192);
        assert!(loaded.behavior.auto_save);
    }

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.api.base_url, "https://api.anthropic.com");
    }

    #[test]
    fn test_permission_lookup() {
        let config = Config::default();
        assert_eq!(config.permission_for("read_file"), PermissionLevel::Safe);
        assert_eq!(config.permission_for("bash"), PermissionLevel::Confirm);
        assert_eq!(config.permission_for("unknown_tool"), PermissionLevel::Confirm);
    }
}
