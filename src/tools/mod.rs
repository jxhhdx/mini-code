use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;

pub mod bash;
pub mod list_dir;
pub mod read_file;
pub mod search_replace;
pub mod write_file;

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("{0}")]
    Execution(String),
    #[error("User cancelled")]
    Cancelled,
    #[error("Tool not found: {0}")]
    NotFound(String),
}

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> Value;

    fn execute(&self, input: Value, confirm: bool) -> Result<String, ToolError>;

    fn definition(&self) -> Value {
        json!({
            "name": self.name(),
            "description": self.description(),
            "input_schema": self.schema()
        })
    }
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        ToolRegistry {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn definitions(&self) -> Vec<Value> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    pub fn execute(&self, name: &str, input: Value, confirm: bool) -> Result<String, ToolError> {
        let tool = self.tools.get(name).ok_or_else(|| ToolError::NotFound(name.to_string()))?;
        tool.execute(input, confirm)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn default_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(read_file::ReadFileTool));
    registry.register(Box::new(write_file::WriteFileTool));
    registry.register(Box::new(search_replace::SearchReplaceTool));
    registry.register(Box::new(bash::BashTool));
    registry.register(Box::new(list_dir::ListDirTool));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct DummyTool;

    impl Tool for DummyTool {
        fn name(&self) -> &str {
            "dummy"
        }

        fn description(&self) -> &str {
            "A dummy tool"
        }

        fn schema(&self) -> Value {
            json!({
                "type": "object",
                "properties": {}
            })
        }

        fn execute(&self, _input: Value, _confirm: bool) -> Result<String, ToolError> {
            Ok("done".to_string())
        }
    }

    #[test]
    fn test_tool_definition() {
        let dummy = DummyTool;
        let def = dummy.definition();
        assert_eq!(def["name"], "dummy");
        assert_eq!(def["description"], "A dummy tool");
    }

    #[test]
    fn test_tool_execution() {
        let dummy = DummyTool;
        let result = dummy.execute(json!({}), false).unwrap();
        assert_eq!(result, "done");
    }
}
