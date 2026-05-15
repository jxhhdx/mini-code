use super::{Tool, ToolError};
use serde_json::Value;
use std::process::Command;

pub struct BashTool;

impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a shell command. Use with caution."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "timeout": {
                    "type": "number",
                    "description": "Timeout in seconds (default: 30)",
                    "default": 30
                }
            },
            "required": ["command"]
        })
    }

    fn execute(&self, input: Value, _confirm: bool) -> Result<String, ToolError> {
        let command = input["command"].as_str().ok_or_else(|| {
            ToolError::Execution("Missing 'command' parameter".to_string())
        })?;

        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .map_err(|e| ToolError::Execution(format!("Failed to execute command: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            let mut err_msg = format!("Command exited with code {:?}", output.status.code());
            if !stderr.is_empty() {
                err_msg.push_str(format!("\nstderr: {}", stderr).as_str());
            }
            return Err(ToolError::Execution(err_msg));
        }

        let mut result = stdout.to_string();
        if !stderr.is_empty() {
            result.push_str(format!("\nstderr: {}", stderr).as_str());
        }

        Ok(result.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_bash_echo() {
        let tool = BashTool;
        let result = tool.execute(
            json!({"command": "echo hello"}),
            false,
        ).unwrap();
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_bash_invalid_command() {
        let tool = BashTool;
        let result = tool.execute(
            json!({"command": "this_command_does_not_exist_12345"}),
            false,
        );
        assert!(result.is_err());
    }
}
