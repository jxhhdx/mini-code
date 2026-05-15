use mini_code::anthropic::AnthropicClient;
use mini_code::message_history::Message;
use serde_json::json;

#[tokio::test]
async fn test_tool_use_loop() {
    let mut server = mockito::Server::new_async().await;

    // First response: tool_use
    let mock1 = server.mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json!({
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "tool_use",
                "id": "tu_1",
                "name": "read_file",
                "input": {"path": "/tmp/test.txt"}
            }],
            "model": "claude-test",
            "stop_reason": null
        }).to_string())
        .create();

    // Second response: final text
    let mock2 = server.mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json!({
            "id": "msg_2",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "Done!"}],
            "model": "claude-test",
            "stop_reason": "end_turn"
        }).to_string())
        .create();

    let client = AnthropicClient::new("test-key", "claude-test", &server.url(), 4096);

    let messages = vec![Message::user("Read the file")];
    let tools = vec![json!({
        "name": "read_file",
        "description": "Read file",
        "input_schema": {"type": "object", "properties": {"path": {"type": "string"}}}
    })];

    let response1 = client.send_message(&messages, &tools).await.unwrap();
    assert_eq!(response1.len(), 1);

    // Simulate tool execution and send result back
    let mut messages = messages.clone();
    messages.extend(response1);
    messages.push(Message::tool_result("tu_1", "file content", false));

    let response2 = client.send_message(&messages, &tools).await.unwrap();
    assert_eq!(response2.len(), 1);
    match &response2[0].content {
        mini_code::message_history::Content::Text { text } => {
            assert_eq!(text, "Done!");
        }
        _ => panic!("Expected text response"),
    }

    mock1.assert();
    mock2.assert();
}
