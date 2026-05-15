use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Content {
    Text { text: String },
    ToolUse { id: String, name: String, input: Value },
    ToolResult { tool_use_id: String, content: String, is_error: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub role: Role,
    pub content: Content,
}

impl Message {
    pub fn user(text: impl Into<String>) -> Self {
        Message {
            role: Role::User,
            content: Content::Text { text: text.into() },
        }
    }

    pub fn assistant_text(text: impl Into<String>) -> Self {
        Message {
            role: Role::Assistant,
            content: Content::Text { text: text.into() },
        }
    }

    pub fn assistant_tool_use(id: impl Into<String>, name: impl Into<String>, input: Value) -> Self {
        Message {
            role: Role::Assistant,
            content: Content::ToolUse {
                id: id.into(),
                name: name.into(),
                input,
            },
        }
    }

    pub fn tool_result(tool_use_id: impl Into<String>, content: impl Into<String>, is_error: bool) -> Self {
        Message {
            role: Role::User,
            content: Content::ToolResult {
                tool_use_id: tool_use_id.into(),
                content: content.into(),
                is_error,
            },
        }
    }

    pub fn to_api_format(&self) -> Value {
        let role = match self.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };

        let content = match &self.content {
            Content::Text { text } => {
                json!([{"type": "text", "text": text}])
            }
            Content::ToolUse { id, name, input } => {
                json!([{
                    "type": "tool_use",
                    "id": id,
                    "name": name,
                    "input": input
                }])
            }
            Content::ToolResult { tool_use_id, content, is_error } => {
                let mut result = json!({
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": content,
                });
                if *is_error {
                    result["is_error"] = json!(true);
                }
                json!([result])
            }
        };

        json!({
            "role": role,
            "content": content
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_roundtrip() {
        let messages = vec![
            Message::user("Hello"),
            Message::assistant_text("Hi there"),
        ];
        let json = serde_json::to_string(&messages).unwrap();
        let decoded: Vec<Message> = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].role, Role::User);
    }

    #[test]
    fn test_tool_use_message() {
        let msg = Message::assistant_tool_use("tool_123", "read_file", json!({"path": "/tmp/test"}));
        match &msg.content {
            Content::ToolUse { id, name, input } => {
                assert_eq!(id, "tool_123");
                assert_eq!(name, "read_file");
                assert_eq!(input["path"], "/tmp/test");
            }
            _ => panic!("Expected ToolUse content"),
        }
    }
}
