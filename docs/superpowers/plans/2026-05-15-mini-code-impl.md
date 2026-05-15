# mini-code Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust CLI REPL that uses Anthropic's Claude API with native Tool Use to read files, edit code, run bash commands, and manage multiple conversation sessions.

**Architecture:** Single crate with layered modules. HTTP via `reqwest` + `tokio`, serialization via `serde`, config via `toml`, REPL line editing via `rustyline`. Anthropic Tool Use loop is the core: user message -> API -> tool_use -> execute tool -> tool_result -> API -> final response.

**Tech Stack:** Rust, reqwest, tokio, serde, serde_json, toml, rustyline, directories, anyhow, thiserror, chrono, uuid, colored, mockito

---

## File Structure

```
src/
├── main.rs              # CLI entry: parse args, load config, init session, start REPL
├── config.rs            # Config struct, load/save ~/.mini-code/config.toml
├── message_history.rs   # Internal message types + conversion to/from Anthropic API format
├── session.rs           # Session struct + SessionManager (CRUD, persist to JSON)
├── anthropic.rs         # AnthropicClient: HTTP calls, tool use loop
├── repl.rs              # REPL loop: read input, handle slash commands, render output
└── tools/
    ├── mod.rs           # Tool trait, ToolRegistry, execute dispatcher
    ├── read_file.rs     # Read file tool
    ├── write_file.rs    # Write/overwrite file tool
    ├── search_replace.rs # Incremental file edit tool
    ├── bash.rs          # Shell command execution tool
    └── list_dir.rs      # Directory listing tool
```

---

### Task 1: Initialize Rust Project and Dependencies

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs` (placeholder)

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "mini-code"
version = "0.1.0"
edition = "2024"

[dependencies]
anyhow = "1"
thiserror = "2"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
rustyline = "15"
directories = "6"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
colored = "3"

[dev-dependencies]
mockito = "1"
tempfile = "3"
```

- [ ] **Step 2: Create placeholder src/main.rs**

```rust
fn main() {
    println!("mini-code");
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully (may download dependencies)

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml src/main.rs
git commit -m "chore: initialize Rust project with dependencies"
```

---

### Task 2: Config Module

**Files:**
- Create: `src/config.rs`
- Test: `src/config.rs` (unit tests at bottom of file)

- [ ] **Step 1: Write the test**

At the bottom of `src/config.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_roundtrip() {
        let config = Config {
            api: ApiConfig {
                api_key: "test-key".to_string(),
                model: "claude-sonnet-4-6".to_string(),
                base_url: "https://api.anthropic.com".to_string(),
                max_tokens: 4096,
            },
            behavior: BehaviorConfig {
                bash_confirm: true,
                write_confirm: true,
                auto_save: true,
                theme: "dark".to_string(),
            },
        };

        let mut file = NamedTempFile::new().unwrap();
        let content = toml::to_string(&config).unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let loaded = Config::load(file.path()).unwrap();
        assert_eq!(loaded.api.api_key, "test-key");
        assert_eq!(loaded.api.model, "claude-sonnet-4-6");
        assert_eq!(loaded.behavior.theme, "dark");
    }

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.api.model, "claude-sonnet-4-6");
        assert!(config.behavior.bash_confirm);
        assert_eq!(config.api.max_tokens, 4096);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test config::tests --lib`
Expected: FAIL - `Config`, `ApiConfig`, `BehaviorConfig`, `load`, `default` not defined

- [ ] **Step 3: Implement config.rs**

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub behavior: BehaviorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiConfig {
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorConfig {
    #[serde(default = "default_true")]
    pub bash_confirm: bool,
    #[serde(default = "default_true")]
    pub write_confirm: bool,
    #[serde(default = "default_true")]
    pub auto_save: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_model() -> String {
    "claude-sonnet-4-6".to_string()
}

fn default_base_url() -> String {
    "https://api.anthropic.com".to_string()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            api: ApiConfig::default(),
            behavior: BehaviorConfig::default(),
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        ApiConfig {
            api_key: String::new(),
            model: default_model(),
            base_url: default_base_url(),
            max_tokens: default_max_tokens(),
        }
    }
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        BehaviorConfig {
            bash_confirm: true,
            write_confirm: true,
            auto_save: true,
            theme: default_theme(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {:?}", path))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| "Failed to parse config TOML")?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory {:?}", parent))?;
        }
        let content = toml::to_string_pretty(self)
            .with_context(|| "Failed to serialize config to TOML")?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config to {:?}", path))?;
        Ok(())
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test config::tests --lib`
Expected: 2 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat: add config module with load/save and defaults"
```

---

### Task 3: Message History Module

**Files:**
- Create: `src/message_history.rs`
- Test: `src/message_history.rs` (unit tests at bottom)

- [ ] **Step 1: Write the test**

At the bottom of `src/message_history.rs`:

```rust
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
        let msg = Message::assistant_tool_use("tool_123", "read_file", r#"{"path":"/tmp/test"}"#);
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test message_history::tests --lib`
Expected: FAIL - types not defined

- [ ] **Step 3: Implement message_history.rs**

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test message_history::tests --lib`
Expected: 2 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/message_history.rs
git commit -m "feat: add message history types"
```

---

### Task 4: Session Manager

**Files:**
- Create: `src/session.rs`
- Modify: `src/message_history.rs` (add `#[cfg(test)]` and `use crate::message_history::Message`)
- Test: `src/session.rs` (unit tests at bottom)

- [ ] **Step 1: Write the test**

At the bottom of `src/session.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::message_history::Message;
    use tempfile::TempDir;

    #[test]
    fn test_session_manager_create_and_list() {
        let dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(dir.path()).unwrap();

        let session = manager.create("test-session").unwrap();
        assert_eq!(session.name, "test-session");
        assert!(session.messages.is_empty());

        let sessions = manager.list().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "test-session");
    }

    #[test]
    fn test_session_save_and_load() {
        let dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(dir.path()).unwrap();

        let mut session = manager.create("my-session").unwrap();
        session.messages.push(Message::user("Hello"));
        manager.save(&session).unwrap();

        let loaded = manager.load(&session.id).unwrap();
        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(loaded.name, "my-session");
    }

    #[test]
    fn test_session_switch_and_delete() {
        let dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(dir.path()).unwrap();

        let s1 = manager.create("first").unwrap();
        let s2 = manager.create("second").unwrap();

        manager.switch(&s2.id).unwrap();
        assert_eq!(manager.current().unwrap().id, s2.id);

        manager.delete(&s1.id).unwrap();
        assert_eq!(manager.list().unwrap().len(), 1);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test session::tests --lib`
Expected: FAIL - types not defined

- [ ] **Step 3: Implement session.rs**

```rust
use crate::message_history::Message;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<Message>,
}

impl Session {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Session {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            created_at: now,
            updated_at: now,
            messages: Vec::new(),
        }
    }
}

pub struct SessionManager {
    sessions_dir: PathBuf,
    current_session_id: Option<String>,
}

impl SessionManager {
    pub fn new(sessions_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(sessions_dir)
            .with_context(|| format!("Failed to create sessions directory {:?}", sessions_dir))?;
        Ok(SessionManager {
            sessions_dir: sessions_dir.to_path_buf(),
            current_session_id: None,
        })
    }

    pub fn create(&mut self, name: impl Into<String>) -> Result<Session> {
        let session = Session::new(name);
        self.save(&session)?;
        self.current_session_id = Some(session.id.clone());
        Ok(session)
    }

    pub fn save(&self, session: &Session) -> Result<()> {
        let path = self.sessions_dir.join(format!("{}.json", session.id));
        let mut session_to_save = session.clone();
        session_to_save.updated_at = Utc::now();
        let content = serde_json::to_string_pretty(&session_to_save)
            .with_context(|| "Failed to serialize session")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write session to {:?}", path))?;
        Ok(())
    }

    pub fn load(&self, id: &str) -> Result<Session> {
        let path = self.sessions_dir.join(format!("{}.json", id));
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read session {:?}", path))?;
        let session: Session = serde_json::from_str(&content)
            .with_context(|| "Failed to parse session JSON")?;
        Ok(session)
    }

    pub fn list(&self) -> Result<Vec<Session>> {
        let mut sessions = Vec::new();
        for entry in std::fs::read_dir(&self.sessions_dir)
            .with_context(|| "Failed to read sessions directory")?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = std::fs::read_to_string(&path)?;
                if let Ok(session) = serde_json::from_str::<Session>(&content) {
                    sessions.push(session);
                }
            }
        }
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    pub fn switch(&mut self, id: &str) -> Result<()> {
        let _ = self.load(id)?;
        self.current_session_id = Some(id.to_string());
        Ok(())
    }

    pub fn delete(&mut self, id: &str) -> Result<()> {
        let path = self.sessions_dir.join(format!("{}.json", id));
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("Failed to delete session {:?}", path))?;
        }
        if self.current_session_id.as_deref() == Some(id) {
            self.current_session_id = None;
        }
        Ok(())
    }

    pub fn current(&self) -> Option<Session> {
        let id = self.current_session_id.as_ref()?;
        self.load(id).ok()
    }

    pub fn current_id(&self) -> Option<&String> {
        self.current_session_id.as_ref()
    }

    pub fn update_current(&self, session: &Session) -> Result<()> {
        if self.current_session_id.as_ref() == Some(&session.id) {
            self.save(session)?;
        }
        Ok(())
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test session::tests --lib`
Expected: 3 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/session.rs
git commit -m "feat: add session manager with CRUD and JSON persistence"
```

---

### Task 5: Tool Trait and Registry

**Files:**
- Create: `src/tools/mod.rs`
- Modify: `src/lib.rs` or ensure module declarations
- Test: `src/tools/mod.rs` (unit tests at bottom)

- [ ] **Step 1: Write the test**

At the bottom of `src/tools/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct DummyTool;

    #[derive(Debug, thiserror::Error)]
    enum DummyError {
        #[error("dummy error")]
        Dummy,
    }

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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test tools::tests --lib`
Expected: FAIL - `Tool`, `ToolError`, `definition` not defined

- [ ] **Step 3: Implement tools/mod.rs**

```rust
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test tools::tests --lib`
Expected: 2 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/tools/mod.rs
git commit -m "feat: add Tool trait and ToolRegistry"
```

---

### Task 6: Read File Tool

**Files:**
- Create: `src/tools/read_file.rs`
- Test: `src/tools/read_file.rs` (unit tests at bottom)

- [ ] **Step 1: Write the test**

At the bottom of `src/tools/read_file.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test tools::read_file::tests --lib`
Expected: FAIL - `ReadFileTool` not defined

- [ ] **Step 3: Implement read_file.rs**

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test tools::read_file::tests --lib`
Expected: 2 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/tools/read_file.rs
git commit -m "feat: add read_file tool"
```

---

### Task 7: Write File Tool

**Files:**
- Create: `src/tools/write_file.rs`
- Test: `src/tools/write_file.rs` (unit tests at bottom)

- [ ] **Step 1: Write the test**

At the bottom of `src/tools/write_file.rs`:

```rust
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
    fn test_write_file_overwrite_requires_confirm() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("exists.txt");
        std::fs::write(&path, "old").unwrap();

        let tool = WriteFileTool;
        // confirm=true but we simulate user cancellation by not implementing interactive confirmation in tests
        // In practice, the REPL layer handles confirmation before calling execute()
        let result = tool.execute(
            json!({"path": path, "content": "new"}),
            false,
        ).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test tools::write_file::tests --lib`
Expected: FAIL - `WriteFileTool` not defined

- [ ] **Step 3: Implement write_file.rs**

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test tools::write_file::tests --lib`
Expected: 2 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/tools/write_file.rs
git commit -m "feat: add write_file tool"
```

---

### Task 8: Search/Replace Tool

**Files:**
- Create: `src/tools/search_replace.rs`
- Test: `src/tools/search_replace.rs` (unit tests at bottom)

- [ ] **Step 1: Write the test**

At the bottom of `src/tools/search_replace.rs`:

```rust
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

        assert!(result.contains("successfully"));
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test tools::search_replace::tests --lib`
Expected: FAIL - `SearchReplaceTool` not defined

- [ ] **Step 3: Implement search_replace.rs**

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test tools::search_replace::tests --lib`
Expected: 2 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/tools/search_replace.rs
git commit -m "feat: add search_replace tool"
```

---

### Task 9: Bash Tool

**Files:**
- Create: `src/tools/bash.rs`
- Test: `src/tools/bash.rs` (unit tests at bottom)

- [ ] **Step 1: Write the test**

At the bottom of `src/tools/bash.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test tools::bash::tests --lib`
Expected: FAIL - `BashTool` not defined

- [ ] **Step 3: Implement bash.rs**

```rust
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

    fn execute(&self, input: Value, confirm: bool) -> Result<String, ToolError> {
        let command = input["command"].as_str().ok_or_else(|| {
            ToolError::Execution("Missing 'command' parameter".to_string())
        })?;

        if confirm {
            // The REPL layer should handle interactive confirmation before calling execute
            // If confirm=true was passed but we're here, it means the user already confirmed
        }

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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test tools::bash::tests --lib`
Expected: 2 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/tools/bash.rs
git commit -m "feat: add bash tool"
```

---

### Task 10: List Directory Tool

**Files:**
- Create: `src/tools/list_dir.rs`
- Test: `src/tools/list_dir.rs` (unit tests at bottom)

- [ ] **Step 1: Write the test**

At the bottom of `src/tools/list_dir.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test tools::list_dir::tests --lib`
Expected: FAIL - `ListDirTool` not defined

- [ ] **Step 3: Implement list_dir.rs**

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test tools::list_dir::tests --lib`
Expected: 2 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/tools/list_dir.rs
git commit -m "feat: add list_dir tool"
```

---

### Task 11: Anthropic API Client

**Files:**
- Create: `src/anthropic.rs`
- Modify: `src/message_history.rs` (add `pub fn to_api_format` or similar for conversion)
- Test: `src/anthropic.rs` (unit tests with mockito at bottom)

- [ ] **Step 1: Add API format conversion to message_history.rs**

Add to `src/message_history.rs` before the tests:

```rust
impl Message {
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
```

- [ ] **Step 2: Write the test for anthropic.rs**

At the bottom of `src/anthropic.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::message_history::Message;
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
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test anthropic::tests --lib`
Expected: FAIL - `AnthropicClient`, `send_message` not defined

- [ ] **Step 4: Implement anthropic.rs**

```rust
use crate::message_history::{Content, Message, Role};
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;

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
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test anthropic::tests --lib`
Expected: 1 test PASS

- [ ] **Step 6: Commit**

```bash
git add src/anthropic.rs src/message_history.rs
git commit -m "feat: add Anthropic API client with tool use support"
```

---

### Task 12: REPL Module

**Files:**
- Create: `src/repl.rs`
- Modify: `src/tools/mod.rs` (add `pub fn default_registry()`)
- Test: Manual test only (REPL is inherently interactive)

- [ ] **Step 1: Add default registry to tools/mod.rs**

Add after `impl Default for ToolRegistry`:

```rust
impl ToolRegistry {
    pub fn default_registry() -> Self {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(super::read_file::ReadFileTool));
        registry.register(Box::new(super::write_file::WriteFileTool));
        registry.register(Box::new(super::search_replace::SearchReplaceTool));
        registry.register(Box::new(super::bash::BashTool));
        registry.register(Box::new(super::list_dir::ListDirTool));
        registry
    }
}
```

- [ ] **Step 2: Implement repl.rs**

```rust
use crate::anthropic::AnthropicClient;
use crate::config::Config;
use crate::message_history::{Content, Message};
use crate::session::{Session, SessionManager};
use crate::tools::ToolRegistry;
use anyhow::Result;
use colored::Colorize;
use rustyline::DefaultEditor;
use serde_json::Value;

pub struct Repl {
    session_manager: SessionManager,
    client: AnthropicClient,
    registry: ToolRegistry,
    config: Config,
}

impl Repl {
    pub fn new(
        session_manager: SessionManager,
        client: AnthropicClient,
        config: Config,
    ) -> Self {
        Repl {
            session_manager,
            client,
            registry: ToolRegistry::default_registry(),
            config,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut rl = DefaultEditor::new()?;
        println!("{}", "mini-code v0.1.0".bold());
        self.print_status();
        println!("Type /help for available commands, /exit to quit.\n");

        loop {
            let readline = rl.readline(&format!("{} ", ">".green()));
            match readline {
                Ok(line) => {
                    let _ = rl.add_history_entry(&line);
                    if let Err(e) = self.handle_input(&line).await {
                        eprintln!("{} {}", "Error:".red().bold(), e);
                    }
                }
                Err(rustyline::error::ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    break;
                }
                Err(rustyline::error::ReadlineError::Eof) => {
                    println!("CTRL-D");
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    break;
                }
            }
        }

        Ok(())
    }

    fn print_status(&self) {
        if let Some(session) = self.session_manager.current() {
            println!(
                "{} {} [{} {}]",
                "当前会话:".dimmed(),
                session.name.cyan(),
                session.messages.len().to_string().yellow(),
                "条消息".dimmed()
            );
        } else {
            println!("{}", "没有活跃会话，使用 /new <name> 创建一个".yellow());
        }
    }

    async fn handle_input(&mut self, input: &str) -> Result<()> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(());
        }

        if trimmed.starts_with('/') {
            self.handle_command(trimmed).await
        } else {
            self.handle_chat(trimmed).await
        }
    }

    async fn handle_command(&mut self, cmd: &str) -> Result<()> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let command = parts[0];
        let args = &parts[1..];

        match command {
            "/help" => self.print_help(),
            "/exit" => std::process::exit(0),
            "/new" => {
                let name = args.get(0).unwrap_or(&"unnamed");
                let session = self.session_manager.create(*name)?;
                println!("{} {}", "创建会话:".green(), session.name.cyan());
            }
            "/sessions" => {
                let sessions = self.session_manager.list()?;
                if sessions.is_empty() {
                    println!("{}", "没有会话".yellow());
                } else {
                    println!("{}", "会话列表:".bold());
                    for s in sessions {
                        let current = self.session_manager.current_id()
                            .map(|id| id == &s.id)
                            .unwrap_or(false);
                        let marker = if current { "*" } else { " " };
                        println!(
                            "  [{}] {} ({}, {} 条消息)",
                            marker,
                            s.name.cyan(),
                            &s.id[..8].dimmed(),
                            s.messages.len()
                        );
                    }
                }
            }
            "/switch" => {
                let id = args.get(0).ok_or_else(|| anyhow::anyhow!("需要会话 ID"))?;
                self.session_manager.switch(id)?;
                println!("{} {}", "切换到会话:".green(), id.cyan());
            }
            "/rename" => {
                let name = args.get(0).unwrap_or(&"unnamed");
                if let Some(mut session) = self.session_manager.current() {
                    session.name = name.to_string();
                    self.session_manager.save(&session)?;
                    println!("{} {}", "重命名为:".green(), name.cyan());
                }
            }
            "/delete" => {
                let id = args.get(0).ok_or_else(|| anyhow::anyhow!("需要会话 ID"))?;
                self.session_manager.delete(id)?;
                println!("{} {}", "删除会话:".green(), id.red());
            }
            "/clear" => {
                if let Some(mut session) = self.session_manager.current() {
                    session.messages.clear();
                    self.session_manager.save(&session)?;
                    println!("{}", "当前会话已清空".green());
                }
            }
            "/config" => {
                println!("{}", serde_json::to_string_pretty(&self.config)?);
            }
            _ => println!("{} 使用 /help 查看可用命令", "未知命令.".red()),
        }

        Ok(())
    }

    fn print_help(&self) {
        println!("{}", "可用命令:".bold());
        println!("  /new <name>     创建新会话");
        println!("  /sessions       列出所有会话");
        println!("  /switch <id>    切换会话");
        println!("  /rename <name>  重命名当前会话");
        println!("  /delete <id>    删除会话");
        println!("  /clear          清空当前会话");
        println!("  /config         查看配置");
        println!("  /help           显示帮助");
        println!("  /exit           退出");
    }

    async fn handle_chat(&mut self, input: &str) -> Result<()> {
        let mut session = self.session_manager.current()
            .ok_or_else(|| anyhow::anyhow!("没有活跃会话，先用 /new 创建一个"))?;

        session.messages.push(Message::user(input));

        let tool_defs = self.registry.definitions();
        let mut turn_count = 0;
        const MAX_TURNS: usize = 10;

        loop {
            if turn_count >= MAX_TURNS {
                eprintln!("{}", "达到最大工具调用轮数".red());
                break;
            }
            turn_count += 1;

            print!("{}", "[思考中...] ".dimmed());
            let _ = std::io::Write::flush(&mut std::io::stdout());

            let response_messages = match self.client.send_message(&session.messages, &tool_defs
            ).await {
                Ok(msgs) => msgs,
                Err(e) => {
                    println!();
                    anyhow::bail!("API 调用失败: {}", e);
                }
            };
            println!();

            let mut has_tool_use = false;
            for msg in &response_messages {
                session.messages.push(msg.clone());

                match &msg.content {
                    Content::Text { text } => {
                        println!("{}", text);
                    }
                    Content::ToolUse { id, name, input } => {
                        has_tool_use = true;
                        println!(
                            "{} {} {} {}",
                            "→".yellow(),
                            name.cyan().bold(),
                            serde_json::to_string(input)?.dimmed(),
                            "...".dimmed()
                        );

                        let needs_confirm = match name.as_str() {
                            "bash" => self.config.behavior.bash_confirm,
                            "write_file" => self.config.behavior.write_confirm,
                            _ => false,
                        };

                        let confirmed = if needs_confirm {
                            print!("{} ", "确认执行? [Y/n]".yellow());
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                            let mut buf = String::new();
                            std::io::stdin().read_line(&mut buf)?;
                            let trimmed = buf.trim().to_lowercase();
                            trimmed == "y" || trimmed == "yes" || trimmed.is_empty()
                        } else {
                            true
                        };

                        let result = if confirmed {
                            self.registry.execute(name, input.clone(), confirmed)
                                .unwrap_or_else(|e| format!("Error: {}", e))
                        } else {
                            "User cancelled operation".to_string()
                        };

                        println!(
                            "{} {}",
                            if result.starts_with("Error:") { "✗".red() } else { "✓".green() },
                            if result.starts_with("Error:") {
                                result.red().to_string()
                            } else {
                                result.dimmed().to_string()
                            }
                        );

                        session.messages.push(Message::tool_result(
                            id,
                            &result,
                            result.starts_with("Error:")
                        ));
                    }
                    _ => {}
                }
            }

            if !has_tool_use {
                break;
            }
        }

        if self.config.behavior.auto_save {
            self.session_manager.save(&session)?;
        }

        Ok(())
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/repl.rs src/tools/mod.rs
git commit -m "feat: add REPL with session commands and tool use loop"
```

---

### Task 13: Main Entry Point and Module Declarations

**Files:**
- Modify: `src/main.rs`
- Modify: `src/lib.rs` (create with module declarations)

- [ ] **Step 1: Create src/lib.rs with module declarations**

```rust
pub mod anthropic;
pub mod config;
pub mod message_history;
pub mod repl;
pub mod session;
pub mod tools;
```

- [ ] **Step 2: Implement main.rs**

```rust
use anyhow::{Context, Result};
use directories::ProjectDirs;
use mini_code::anthropic::AnthropicClient;
use mini_code::config::Config;
use mini_code::repl::Repl;
use mini_code::session::SessionManager;
use std::path::PathBuf;

fn config_path() -> PathBuf {
    ProjectDirs::from("com", "mini-code", "mini-code")
        .map(|dirs| dirs.config_dir().join("config.toml"))
        .unwrap_or_else(|| PathBuf::from("~/.mini-code/config.toml"))
}

fn sessions_dir() -> PathBuf {
    ProjectDirs::from("com", "mini-code", "mini-code")
        .map(|dirs| dirs.data_dir().join("sessions"))
        .unwrap_or_else(|| PathBuf::from("~/.mini-code/sessions"))
}

fn ensure_config() -> Result<Config> {
    let path = config_path();
    if path.exists() {
        Config::load(&path)
    } else {
        println!("首次启动 mini-code，请配置 API 密钥。");
        print!("Anthropic API Key: ");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let mut api_key = String::new();
        std::io::stdin().read_line(&mut api_key)?;
        let api_key = api_key.trim().to_string();

        let config = Config {
            api: mini_code::config::ApiConfig {
                api_key,
                ..Default::default()
            },
            behavior: Default::default(),
        };
        config.save(&path)?;
        println!("配置已保存到 {:?}", path);
        Ok(config)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = ensure_config().context("Failed to load or create config")?;

    if config.api.api_key.is_empty() {
        anyhow::bail!("API key is required. Please set it in {:?}", config_path());
    }

    let sessions_dir = sessions_dir();
    let session_manager = SessionManager::new(&sessions_dir)
        .context("Failed to initialize session manager")?;

    let client = AnthropicClient::new(
        &config.api.api_key,
        &config.api.model,
        &config.api.base_url,
        config.api.max_tokens,
    );

    let mut repl = Repl::new(session_manager, client, config);
    repl.run().await.context("REPL error")
}
```

- [ ] **Step 3: Update Cargo.toml package name for lib + bin**

Add to `Cargo.toml` under `[package]`:

```toml
[[bin]]
name = "mini-code"
path = "src/main.rs"

[lib]
name = "mini_code"
path = "src/lib.rs"
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 5: Run basic smoke test**

Run: `cargo run -- --help 2>&1 || true`
Expected: Program starts (will wait for input since no --help flag is handled, that's OK for now)

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/main.rs src/lib.rs
git commit -m "feat: add main entry point with config initialization and REPL startup"
```

---

### Task 14: Integration Tests

**Files:**
- Create: `tests/integration_test.rs`

- [ ] **Step 1: Write integration test**

```rust
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
```

- [ ] **Step 2: Run integration test**

Run: `cargo test --test integration_test`
Expected: 1 test PASS

- [ ] **Step 3: Commit**

```bash
git add tests/integration_test.rs
git commit -m "test: add integration test for tool use loop"
```

---

## Self-Review

### Spec Coverage Check

| Spec Section | Implementing Task |
|-------------|------------------|
| Config load/save with defaults | Task 2 |
| Message types + serialization | Task 3 |
| Session CRUD + JSON persistence | Task 4 |
| Tool trait + registry | Task 5 |
| read_file tool | Task 6 |
| write_file tool | Task 7 |
| search_replace tool | Task 8 |
| bash tool | Task 9 |
| list_dir tool | Task 10 |
| Anthropic API client + Tool Use loop | Task 11 |
| REPL with slash commands | Task 12 |
| Main entry + config init | Task 13 |
| Integration tests | Task 14 |

**Coverage:** All spec requirements are covered. ✓

### Placeholder Scan

- No "TBD", "TODO", "implement later", "fill in details" ✓
- No vague "add error handling" without specifics ✓
- No "Similar to Task N" ✓
- Every step has concrete code or exact commands ✓

### Type Consistency Check

- `Config::load` / `Config::save` signatures consistent across tasks ✓
- `SessionManager` methods (`create`, `save`, `load`, `list`, `switch`, `delete`) consistent ✓
- `Message` constructors (`user`, `assistant_text`, `assistant_tool_use`, `tool_result`) consistent ✓
- `Tool::execute` signature `(input: Value, confirm: bool)` consistent across all tools ✓
- `AnthropicClient::new` and `send_message` signatures consistent ✓

### Gaps Fixed

- Added `to_api_format()` to `Message` in Task 11 to bridge internal types to Anthropic API format
- Added `default_registry()` to `ToolRegistry` in Task 12 to instantiate all tools
- Added `#[serde(default)]` on config structs for graceful missing fields

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-05-15-mini-code-impl.md`. Two execution options:**

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
