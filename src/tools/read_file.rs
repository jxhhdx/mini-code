use super::{Tool, ToolError};
use serde_json::Value;

pub struct ReadFileTool;

impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file at the given path."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to read"
                }
            },
            "required": ["path"]
        })
    }

    fn execute(&self, input: Value, _confirm: bool) -> Result<String, ToolError> {
        let path = input["path"].as_str().ok_or_else(|| {
            ToolError::Execution("Missing 'path' parameter".to_string())
        })?;

        std::fs::read_to_string(path).map_err(|e| {
            ToolError::Execution(format!("Failed to read file '{}': {}", path, e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_file_success() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"hello world").unwrap();

        let tool = ReadFileTool;
        let result = tool.execute(json!({"path": file.path()}), false).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_read_file_not_found() {
        let tool = ReadFileTool;
        let result = tool.execute(json!({"path": "/nonexistent/path"}), false);
        assert!(result.is_err());
    }
}
