# mini-code 设计文档

## 概述

mini-code 是一个受 Claude Code 启发的、基于 Rust 的 AI 编码助手 CLI。它提供一个 REPL 交互界面，用户可以与 Anthropic 的 Claude 模型进行多轮对话，模型可以调用工具来读取文件、编辑代码、执行 shell 命令和浏览文件系统。

## 目标

- 为核心编码工作流提供一个轻量、快速的 Claude Code 替代品
- 支持多会话对话管理
- 通过 Tool Use 实现安全的、经用户确认的文件和 shell 操作
- 易于构建、理解和扩展

## 非目标

- 与 Claude Code 完全对等（不支持 MCP 服务器、IDE 集成、图片输入）
- 支持多个 AI 提供商（目前仅 Anthropic）
- Web UI 或 TUI 界面（仅终端 REPL）

## 架构

单 crate，分层模块：

```
src/
├── main.rs            # CLI 入口：解析参数、加载配置、启动 REPL
├── config.rs          # 配置结构体 + 读写 ~/.mini-code/config.toml
├── session.rs         # 会话管理：创建/切换/列出/删除，持久化到 ~/.mini-code/sessions/
├── anthropic.rs       # API 客户端：封装 Messages API，处理 tool use 循环
├── repl.rs            # 交互循环：读取用户输入，渲染助手回复，处理 slash 命令
├── tools/
│   ├── mod.rs         # 工具注册表、调度器
│   ├── read_file.rs
│   ├── write_file.rs
│   ├── search_replace.rs
│   ├── bash.rs
│   └── list_dir.rs
└── message_history.rs # 消息序列化/反序列化（用于会话持久化）
```

### 执行流程

1. `main.rs` 加载配置，初始化 `SessionManager`
2. 启动 `Repl`，进入读取-评估-打印循环
3. 用户输入 → 追加到当前会话消息历史 → 调用 `AnthropicClient`
4. API 返回内容或 `tool_use` 块 → 若是 tool_use，`tools::execute()` 执行它
5. 工具结果回传给 API → 循环直到模型输出最终文本回复
6. 每轮对话后自动保存会话

## Tool Use 设计

Anthropic Tool Use 交互模式：
1. 用户消息 + 可用工具列表 → API
2. API 可能返回 `tool_use` 块（模型想要执行工具）
3. 程序执行工具，将结果作为 `tool_result` 块回传
4. API 可能再次返回 `tool_use`，或输出最终文本回复

### 支持的工具

| 工具名 | 功能 | 参数 |
|--------|------|------|
| `read_file` | 读取文件内容 | `path: string` |
| `write_file` | 写入完整文件（覆盖） | `path: string, content: string` |
| `search_replace` | 增量编辑 | `path: string, old_string: string, new_string: string` |
| `bash` | 执行 shell 命令 | `command: string, timeout?: number` |
| `list_dir` | 列出目录内容 | `path: string` |

### 安全规则

- `bash` 命令执行前需要用户确认（Y/n）
- `write_file` 覆盖已有文件前需要用户确认
- `search_replace` 要求 `old_string` 完全匹配；未找到时返回错误给模型重试
- 每个工具返回结构化 JSON 结果，包含 `success: bool` 和 `output`/`error` 字段

### 代码编辑流程

- 模型优先使用 `search_replace` 修改现有文件
- 仅创建新文件时使用 `write_file`
- 如果 `search_replace` 失败（old_string 未找到），错误返回给模型，模型通常会修正后重试

## 会话管理

每个会话是一个独立的消息历史，存储在 `~/.mini-code/sessions/<id>.json`：

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

### REPL Slash 命令

| 命令 | 功能 |
|------|------|
| `/new <name>` | 创建新会话并切换 |
| `/sessions` | 列出所有会话 |
| `/switch <id>` | 切换到指定会话 |
| `/rename <name>` | 重命名当前会话 |
| `/delete <id>` | 删除会话 |
| `/clear` | 清空当前会话消息（保留会话）|
| `/config` | 查看/编辑配置 |
| `/help` | 显示帮助 |
| `/exit` | 退出 |

## 配置

存储在 `~/.mini-code/config.toml`：

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

首次启动如果没有配置文件，交互式引导用户创建。

## 错误处理

| 场景 | 处理方式 |
|------|----------|
| API 调用失败（网络/超时） | 重试 2 次，仍失败则返回错误给模型 |
| 工具执行失败（文件不存在、权限不足） | 将错误作为 tool_result 回传，让模型决定下一步 |
| `search_replace` old_string 不匹配 | 返回具体错误（"未找到匹配内容"），模型通常重试 |
| 用户取消 bash/写入确认 | 回传 "用户取消了操作" 给模型 |
| 会话文件损坏 | 启动时检测，提示用户恢复或删除 |

所有错误最终都表现为消息历史中的文本，REPL 永不崩溃。

## 测试策略

### 单元测试
- `tools/`：每个工具独立测试，使用临时目录和 mock 命令
- `session.rs`：测试会话 CRUD、序列化/反序列化
- `config.rs`：测试配置读写、默认值
- `message_history.rs`：测试消息格式转换（内部格式 ↔ Anthropic API 格式）

### 集成测试
- `anthropic.rs`：使用 `mockito` 或 `wiremock` 模拟 Anthropic API，测试完整 tool use 循环
  - 场景：用户请求 → API 返回 tool_use → 执行工具 → 回传结果 → API 返回最终回复

### 端到端测试
- 对于 mini 项目可选，维护成本较高

### 不测试的内容
- 真实的 Anthropic API 调用（成本高、不稳定）
- 颜色输出测试
