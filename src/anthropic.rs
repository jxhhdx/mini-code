use crate::message_history::Message;
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{json, Value};

pub struct AnthropicClient {
    api_key: String,
    model: String,
    base_url: String,
    max_tokens: u32,
    client: Client,
}

impl AnthropicClient {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>, base_url: impl Into<String>, max_tokens: u32) -> Self {
        AnthropicClient {
            api_key: api_key.into(),
            model: model.into(),
            base_url: base_url.into(),
            max_tokens,
            client: Client::new(),
        }
    }

    pub async fn send_message(
        &self,
        messages: &[Message],
        tools: &[Value],
    ) -> Result<Vec<Message>> {
        let api_messages: Vec<Value> = messages.iter().map(|m| m.to_api_format()).collect();

        let mut body = json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "messages": api_messages,
        });

        if !tools.is_empty() {
            body["tools"] = json!(tools);
        }

        let url = format!("{}/v1/messages", self.base_url);
        let response = self.client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .with_context(|| format!("Failed to send request to Anthropic API"))?;

        let status = response.status();
        let response_text = response.text().await
            .with_context(|| "Failed to read response body")?;

        if !status.is_success() {
            anyhow::bail!("Anthropic API error ({}): {}", status, response_text);
        }

        let response_json: Value = serde_json::from_str(&response_text)
            .with_context(|| "Failed to parse API response as JSON")?;

        self.parse_response(&response_json)
    }

    fn parse_response(&self, response: &Value) -> Result<Vec<Message>> {
        let content = response["content"].as_array()
            .context("Missing 'content' array in API response")?;

        let mut messages = Vec::new();
        for block in content {
            let block_type = block["type"].as_str().context("Missing block type")?;
            match block_type {
                "text" => {
                    let text = block["text"].as_str().context("Missing text content")?;
                    messages.push(Message::assistant_text(text));
                }
                "tool_use" => {
                    let id = block["id"].as_str().context("Missing tool_use id")?;
                    let name = block["name"].as_str().context("Missing tool_use name")?;
                    let input = block["input"].clone();
                    messages.push(Message::assistant_tool_use(id, name, input));
                }
                _ => {}
            }
        }

        Ok(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use serde_json::json;

    #[tokio::test]
    async fn test_send_message_text_response() {
        let mut server = Server::new_async().await;
        let mock = server.mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({
                "id": "msg_123",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": "Hello!"}],
                "model": "claude-test",
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 10, "output_tokens": 5}
            }).to_string())
            .create();

        let client = AnthropicClient::new(
            "test-key",
            "claude-test",
            &server.url(),
            4096,
        );

        let messages = vec![Message::user("Hi")];
        let tools = vec![json!({"name": "test_tool", "description": "test", "input_schema": {"type": "object"}})];

        let result = client.send_message(&messages, &tools).await.unwrap();
        assert_eq!(result.len(), 1);
        match &result[0].content {
            crate::message_history::Content::Text { text } => {
                assert_eq!(text, "Hello!");
            }
            _ => panic!("Expected text response"),
        }

        mock.assert();
    }
}
