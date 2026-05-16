use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionLevel {
    Safe,
    Confirm,
    Deny,
}

impl std::fmt::Display for PermissionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionLevel::Safe => write!(f, "safe"),
            PermissionLevel::Confirm => write!(f, "confirm"),
            PermissionLevel::Deny => write!(f, "deny"),
        }
    }
}

pub struct PermissionChecker {
    config: crate::config::PermissionsConfig,
}

impl PermissionChecker {
    pub fn new(config: crate::config::PermissionsConfig) -> Self {
        PermissionChecker { config }
    }

    pub fn check(&self, tool: &str) -> PermissionLevel {
        match tool {
            "bash" => self.config.bash,
            "write_file" => self.config.write_file,
            "read_file" => self.config.read_file,
            "edit_file" => self.config.edit_file,
            "glob" => self.config.glob,
            "grep" => self.config.grep,
            "list_dir" => self.config.list_dir,
            _ => PermissionLevel::Confirm,
        }
    }
}
