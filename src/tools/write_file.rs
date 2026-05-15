use super::{Tool, ToolError};
use serde_json::Value;
use std::path::Path;

pub struct WriteFileTool;

impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file at the given path. Overwrites if the file exists."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    fn execute(&self, input: Value, _confirm: bool) -> Result<String, ToolError> {
        let path = input["path"].as_str().ok_or_else(|| {
            ToolError::Execution("Missing 'path' parameter".to_string())
        })?;
        let content = input["content"].as_str().ok_or_else(|| {
            ToolError::Execution("Missing 'content' parameter".to_string())
        })?;

        let path = Path::new(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ToolError::Execution(format!("Failed to create directory: {}", e))
            })?;
        }

        std::fs::write(path, content).map_err(|e| {
            ToolError::Execution(format!("Failed to write file '{}': {}", path.display(), e))
        })?;

        Ok(format!("File '{}' written successfully ({} bytes)", path.display(), content.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn test_write_file_new() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");

        let tool = WriteFileTool;
        let result = tool.execute(
            json!({"path": path, "content": "hello"}),
            false,
        ).unwrap();
        assert!(result.contains("successfully"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn test_write_file_overwrite() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("exists.txt");
        std::fs::write(&path, "old").unwrap();

        let tool = WriteFileTool;
        let result = tool.execute(
            json!({"path": path, "content": "new"}),
            false,
        ).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    }
}
