use super::{Tool, ToolError};
use serde_json::Value;

pub struct SearchReplaceTool;

impl Tool for SearchReplaceTool {
    fn name(&self) -> &str {
        "search_replace"
    }

    fn description(&self) -> &str {
        "Replace occurrences of old_string with new_string in a file."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "The exact string to search for"
                },
                "new_string": {
                    "type": "string",
                    "description": "The string to replace with"
                }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    fn execute(&self, input: Value, _confirm: bool) -> Result<String, ToolError> {
        let path = input["path"].as_str().ok_or_else(|| {
            ToolError::Execution("Missing 'path' parameter".to_string())
        })?;
        let old_string = input["old_string"].as_str().ok_or_else(|| {
            ToolError::Execution("Missing 'old_string' parameter".to_string())
        })?;
        let new_string = input["new_string"].as_str().ok_or_else(|| {
            ToolError::Execution("Missing 'new_string' parameter".to_string())
        })?;

        let content = std::fs::read_to_string(path).map_err(|e| {
            ToolError::Execution(format!("Failed to read file '{}': {}", path, e))
        })?;

        if !content.contains(old_string) {
            return Err(ToolError::Execution(format!(
                "Could not find the specified text in '{}'. The old_string must match exactly.",
                path
            )));
        }

        let new_content = content.replace(old_string, new_string);
        let count = content.matches(old_string).count();

        std::fs::write(path, new_content).map_err(|e| {
            ToolError::Execution(format!("Failed to write file '{}': {}", path, e))
        })?;

        Ok(format!(
            "Successfully replaced {} occurrence(s) in '{}'",
            count, path
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_search_replace_success() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"foo bar baz").unwrap();

        let tool = SearchReplaceTool;
        let result = tool.execute(
            json!({
                "path": file.path(),
                "old_string": "bar",
                "new_string": "qux"
            }),
            false,
        ).unwrap();

        assert!(result.contains("Successfully"));
        assert_eq!(std::fs::read_to_string(file.path()).unwrap(), "foo qux baz");
    }

    #[test]
    fn test_search_replace_not_found() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"foo bar baz").unwrap();

        let tool = SearchReplaceTool;
        let result = tool.execute(
            json!({
                "path": file.path(),
                "old_string": "notfound",
                "new_string": "qux"
            }),
            false,
        );

        assert!(result.is_err());
    }
}
