use super::{Tool, ToolError};
use serde_json::Value;

pub struct ListDirTool;

impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn description(&self) -> &str {
        "List the contents of a directory."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the directory to list"
                }
            },
            "required": ["path"]
        })
    }

    fn execute(&self, input: Value, _confirm: bool) -> Result<String, ToolError> {
        let path = input["path"].as_str().ok_or_else(|| {
            ToolError::Execution("Missing 'path' parameter".to_string())
        })?;

        let entries = std::fs::read_dir(path).map_err(|e| {
            ToolError::Execution(format!("Failed to read directory '{}': {}", path, e))
        })?;

        let mut lines = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| {
                ToolError::Execution(format!("Failed to read directory entry: {}", e))
            })?;
            let name = entry.file_name().to_string_lossy().to_string();
            let file_type = entry.file_type().map_err(|e| {
                ToolError::Execution(format!("Failed to get file type: {}", e))
            })?;
            let prefix = if file_type.is_dir() {
                "[DIR]"
            } else {
                "[FILE]"
            };
            lines.push(format!("{} {}", prefix, name));
        }

        lines.sort();
        Ok(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn test_list_dir() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "").unwrap();
        std::fs::write(dir.path().join("b.txt"), "").unwrap();

        let tool = ListDirTool;
        let result = tool.execute(
            json!({"path": dir.path()}),
            false,
        ).unwrap();

        assert!(result.contains("a.txt"));
        assert!(result.contains("b.txt"));
    }

    #[test]
    fn test_list_dir_not_found() {
        let tool = ListDirTool;
        let result = tool.execute(
            json!({"path": "/nonexistent/dir"}),
            false,
        );
        assert!(result.is_err());
    }
}
