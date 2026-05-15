# mini-code Design Spec

## Overview

mini-code is a Rust-based AI coding assistant CLI inspired by Claude Code. It provides a REPL interface where users can have multi-turn conversations with Anthropic's Claude models, with the model able to invoke tools to read files, edit code, execute shell commands, and explore the filesystem.

## Goals

- Provide a lightweight, fast alternative to Claude Code for core coding workflows
- Support multi-session conversation management
- Enable safe, user-confirmed file and shell operations via Tool Use
- Be simple to build, understand, and extend

## Non-Goals

- Full parity with Claude Code (no MCP servers, no IDE integration, no image input)
- Support for multiple AI providers (Anthropic only, for now)
- Web UI or TUI interface (plain terminal REPL only)

## Architecture

Single crate with layered modules:

```
src/
├── main.rs            # CLI entry: parse args, load config, start REPL
├── config.rs          # Config struct + read/write ~/.mini-code/config.toml
├── session.rs         # Session management: create/switch/list/delete, persist to ~/.mini-code/sessions/
├── anthropic.rs       # API client: wrap Messages API, handle tool use loop
├── repl.rs            # Interaction loop: read user input, render assistant response, handle slash commands
├── tools/
│   ├── mod.rs         # Tool registry, dispatcher
│   ├── read_file.rs
│   ├── write_file.rs
│   ├── search_replace.rs
│   ├── bash.rs
│   └── list_dir.rs
└── message_history.rs # Message serialization/deserialization (for session persistence)
```

### Execution Flow

1. `main.rs` loads config, initializes `SessionManager`
2. Starts `Repl`, enters read-eval-print loop
3. User input -> appended to current session message history -> call `AnthropicClient`
4. API returns content or `tool_use` block -> if tool_use, `tools::execute()` runs it
5. Tool result sent back to API -> loop until model outputs final text response
6. Auto-save session after each conversation turn

## Tool Use Design

Anthropic Tool Use interaction pattern:
1. User message + available tools list -> API
2. API may return `tool_use` block (model wants to execute tool)
3. Program executes tool, returns result as `tool_result` block
4. API may return another `tool_use`, or final text response

### Supported Tools

| Tool Name | Function | Parameters |
|-----------|----------|------------|
| `read_file` | Read file content | `path: string` |
| `write_file` | Write complete file (overwrite) | `path: string, content: string` |
| `search_replace` | Incremental edit | `path: string, old_string: string, new_string: string` |
| `bash` | Execute shell command | `command: string, timeout?: number` |
| `list_dir` | List directory contents | `path: string` |

### Safety Rules

- `bash` commands require user confirmation (Y/n) before execution
- `write_file` overwriting an existing file requires user confirmation
- `search_replace` requires exact `old_string` match; if not found, return error to model for retry
- Each tool returns structured JSON result with `success: bool` and `output`/`error` fields

### Code Editing Flow

- Model prefers `search_replace` for modifying existing files
- `write_file` is used only for creating new files
- If `search_replace` fails (old_string not found), error is returned to model, which typically corrects and retries

## Session Management

Each session is an independent message history stored at `~/.mini-code/sessions/<id>.json`:

```json
{
  "id": "abc123",
  "name": "rust-sorting",
  "created_at": "2026-05-15T10:00:00Z",
  "updated_at": "2026-05-15T11:30:00Z",
  "messages": [
    {"role": "user", "content": "帮我写一个排序函数"},
    {"role": "assistant", "content": "..."}
  ]
}
```

### REPL Slash Commands

| Command | Function |
|---------|----------|
| `/new <name>` | Create new session and switch to it |
| `/sessions` | List all sessions |
| `/switch <id>` | Switch to specified session |
| `/rename <name>` | Rename current session |
| `/delete <id>` | Delete a session |
| `/clear` | Clear current session messages (keep session) |
| `/config` | View/edit config |
| `/help` | Show help |
| `/exit` | Exit |

## Configuration

Stored at `~/.mini-code/config.toml`:

```toml
[api]
api_key = "sk-ant-..."
model = "claude-sonnet-4-6"
base_url = "https://api.anthropic.com"
max_tokens = 4096

[behavior]
bash_confirm = true
write_confirm = true
auto_save = true
theme = "dark"
```

On first launch without a config file, interactively guide the user through creation.

## Error Handling

| Scenario | Handling |
|----------|----------|
| API call failure (network/timeout) | Retry 2 times, then return error to model |
| Tool execution failure (file not found, permission denied) | Return error as tool_result, let model decide next step |
| `search_replace` old_string mismatch | Return specific error ("matching content not found"), model typically retries |
| User cancels bash/write confirmation | Return "user cancelled operation" to model |
| Corrupted session file | Detect on startup, prompt user to recover or delete |

All errors are surfaced as text in the message history; the REPL never crashes.

## Testing Strategy

### Unit Tests
- `tools/`: Each tool tested independently using temp directories and mock commands
- `session.rs`: Session CRUD, serialization/deserialization
- `config.rs`: Config read/write, defaults
- `message_history.rs`: Message format conversion (internal format <-> Anthropic API format)

### Integration Tests
- `anthropic.rs`: Use `mockito` or `wiremock` to simulate Anthropic API, test complete tool use loop
  - Scenario: user request -> API returns tool_use -> execute tool -> return result -> API returns final response

### End-to-End Tests
- Optional for mini project due to maintenance overhead

### Out of Scope
- Real Anthropic API calls (costly, unstable)
- Color output testing
