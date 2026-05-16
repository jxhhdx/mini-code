# ARCH-v2.0: mini-code 架构设计

## 架构目标

- **核心目标**：用 Rust 实现高性能 CLI 编码助手，体验对标 Claude Code
- **非功能需求**：启动 < 100ms，token 渲染延迟 < 10ms，内存 < 50MB，release binary < 10MB
- **约束**：单人开发，v2.0 周期约 1 周，MVP 覆盖 P0 的 6 个功能

---

## 0. Claude Code vs mini-code 技术栈对比

先看清楚 Claude Code 用什么、mini-code 对应用什么。

| 层级 | Claude Code | mini-code | 对比说明 |
|------|------------|-----------|---------|
| **语言** | TypeScript（严格模式） | Rust 2021 edition | Rust 启动更快、内存更低、无 GC 抖动 |
| **运行时** | Bun | tokio | Bun 和 tokio 都是各自生态的顶级异步运行时 |
| **TUI 框架** | React + Ink | ratatui + crossterm | Ink 是 React 渲染到终端，ratatui 是 Rust 的终端画布框架。都是声明式/组件化思想 |
| **Schema 校验** | Zod v4 | serde + schemars（可选） | TypeScript 需要 Zod 做运行时校验，Rust 编译期类型系统已覆盖大部分 |
| **HTTP 客户端** | Anthropic SDK | reqwest + 手写 SSE 解析 | Claude Code 用官方 SDK，mini-code 直接调 REST API 更轻量 |
| **CLI 解析** | Commander.js | clap（建议替换） | 目前 Cargo.toml 没加，后续应该加 |
| **持久化存储** | JSONL 文件（`history.jsonl`） | JSONL 文件 | **Claude Code 不用 SQLite**，核心存储都是 JSONL |
| **配置** | JSON（`settings.json`） | TOML（`config.toml`） | TOML 对手写更友好，Rust 生态标配 |
| **搜索** | ripgrep 库调用 | `grep` crate（regex） | 后续 P1 可集成 ripgrep 的 Rust binding |
| **凭证存储** | macOS Keychain | 文件存储（P2 加 keyring） | MVP 文件够用 |

**关键结论**：Claude Code 是一个用前端技术栈（React + Ink + TypeScript）做的 AI CLI 工具。mini-code 用 Rust 做，技术路线不同但架构模式完全可对标——TUI 用 ratatui 对标 Ink，存储用 JSONL 对标 Claude Code 的文件体系，Agent 循环和工具系统结构一致。

---

## 1. 技术选型

| 层级 | 选型 | 理由 |
|-----|------|------|
| 语言 | Rust 2021 edition | 启动速度、内存效率、类型安全 |
| 异步运行时 | tokio (full features) | Rust 异步标准，reqwest 底层依赖 |
| TUI 框架 | **ratatui + crossterm** | 对标 Claude Code 的 React+Ink。ratatui 是 Rust 生态最成熟的终端 UI 框架，支持分区渲染、组件化布局 |
| HTTP 客户端 | reqwest 0.12 + stream feature | 支持 SSE 流式读取 |
| 序列化 | serde + serde_json | TOML 配置 + JSONL 会话存储 |
| CLI 解析 | **clap 4** (derive 模式) | 对标 Commander.js，支持子命令、参数校验、自动 help |
| 配置目录 | directories 6 | 跨平台 XDG 路径 |
| 文件匹配 | glob 0.3 + regex 1 | grep/glob 工具实现 |
| 错误处理 | anyhow + thiserror | 应用层 anyhow，库层 thiserror |
| UUID | uuid 1 (v4) | 会话 ID |
| 时间 | chrono 0.4 | ISO 8601 时间戳 |

### 新增依赖（需加入 Cargo.toml）

```toml
clap = { version = "4", features = ["derive"] }
ratatui = "0.29"
crossterm = "0.28"
```

### 移除依赖

```toml
# rustyline = "15"   # 被 ratatui + crossterm 替代
# colored = "3"      # ratatui 内置样式系统
```

---

## 2. 模块架构

### 2.1 Crate 结构

```
mini-code (binary + library crate)
├── src/
│   ├── main.rs              # CLI 入口：clap 解析 + 启动 TUI
│   ├── lib.rs               # 库根：re-export 核心类型
│   ├── config.rs            # 配置加载/保存/向导
│   ├── tui.rs               # TUI 渲染引擎：状态栏、对话区、输入区、权限弹窗
│   ├── app.rs               # 应用状态管理：持有 Agent/Config/Session 引用，驱动 TUI 刷新
│   ├── session.rs           # 会话 CRUD + JSONL 持久化
│   ├── context.rs           # CLAUDE.md 发现与加载
│   ├── agent.rs             # Agent 主循环 (while true: think → act → observe)
│   ├── anthropic.rs         # Anthropic API 客户端 (SSE 流式)
│   ├── permissions.rs       # 权限决策引擎
│   └── tools/
│       ├── mod.rs           # Tool trait + ToolRegistry
│       ├── bash.rs          # Shell 命令执行
│       ├── read_file.rs     # 文件读取
│       ├── write_file.rs    # 文件写入
│       ├── edit_file.rs     # 精确字符串替换
│       ├── glob.rs          # 文件模式匹配
│       ├── grep.rs          # 文件内容搜索
│       └── list_dir.rs      # 目录遍历
```

### 2.2 模块职责与依赖

```
                         main.rs (clap CLI args)
                            │
                         app.rs (状态持有者，事件分发)
                       ┌───┼───┐
                       │        │
                       ▼        ▼
                    tui.rs    agent.rs
                (ratatui 渲染)  ┌───┼───┐
                       │        │       │
                       │        ▼       ▼
                       │  anthropic.rs  tools/mod.rs
                       │        │       ┌───┼───┐
                       │        │       │   ...  │
                       │        │       ▼       ▼
                       │        │   bash.rs  read_file.rs
                       │        │
                       ▼        ▼
                    session.rs ◄── config.rs
                 (JSONL 读写)      permissions.rs
                       │
                       ▼
              ~/.local/share/     ~/.config/
              mini-code/          mini-code/
```

**依赖原则**：
- `config` 无内部依赖（最底层）
- `session` 依赖 `config`（读取存储路径）
- `context` 依赖 `config`
- `permissions` 依赖 `config`（读取权限覆盖）
- `tools/*` 各自独立，仅依赖 `tools/mod.rs` 的 trait 定义
- `anthropic` 无内部依赖（纯 HTTP 客户端）
- `agent` 依赖 `anthropic` + `tools` + `permissions`
- `tui` 依赖 `app`（读取状态渲染）+ crossterm（终端控制）
- `app` 依赖 `agent` + `session` + `context` + `config` + `tui`（事件循环中枢）
- `main` 仅依赖 `app` + `config` + clap

### 2.3 模块不变量

| 模块 | 不变量 |
|------|--------|
| `session` | 写操作后 `updated_at` 必须更新；消息 `content` 必须符合 Anthropic API content block 格式；JSONL 每行一条完整记录 |
| `agent` | 每轮循环必须检查 `max_rounds` 上限；工具结果必须以 `tool_result` role 追加到消息列表 |
| `anthropic` | 请求必须设 `stream: true`；SSE 连接异常时只重试 1 次 |
| `permissions` | 用户配置覆盖优先于代码默认值；同会话内 "always" 授权仅当次有效 |
| `tools` | 所有工具必须声明副作用类型（只读/写入）；`execute` 不可 panic |
| `app` | 同一时刻只允许一个 Agent 循环运行；Ctrl+C 触发优雅退出保存 |
| `tui` | 渲染不得阻塞事件处理；输入区和对话区的焦点切换必须原子 |

---

## 3. TUI 布局设计

对标 Claude Code 的 React+Ink 渲染模式，ratatui 将终端分为三个固定区域：

```
┌──────────────────────────────────────────────────┐
│                                                  │
│   这是对话区（可滚动）                              │
│                                                  │
│   AI: 好的，让我看看当前代码...                     │
│                                                  │
│   ┌──────────────────────────────────────────┐   │
│   │ 🔧 bash                                   │   │
│   │    cargo build                            │   │
│   │    执行中...                               │   │
│   └──────────────────────────────────────────┘   │
│                                                  │
│   AI: 编译成功。                                  │
│                                                  │
├──────────────────────────────────────────────────┤
│  mini-code v2.0  │  session: feature-x  │  12 msgs │  ← 状态栏
├──────────────────────────────────────────────────┤
│  ⚡ bash(cargo build)                           │  ← 权限确认弹窗（按需出现）
│  执行? [Y/n/a(always)]                           │
├──────────────────────────────────────────────────┤
│  > 帮我重构 config.rs                             │  ← 输入区
└──────────────────────────────────────────────────┘
```

### TUI 事件流

```
crossterm 事件 (keyboard, resize)
       │
       ▼
  app.rs: handle_event()
       │
       ├─ 输入区获得焦点 → 累积文本
       ├─ Enter 按下 → 非 / 开头 → 触发 Agent::run()
       ├─ / 开头 → 路由到命令处理器
       ├─ 权限弹窗出现 → 焦点切换弹窗 (Y/n/a)
       └─ Ctrl+C → 触发优雅退出
       │
       ▼
  tui.rs: draw(frame)
       │
       ├─ 上 80% → 对话区 (Paragraph, 可滚动)
       ├─ 中 1行 → 状态栏 (Line, 固定)
       ├─ 中 1行 → 权限弹窗 (Paragraph, 条件渲染)
       └─ 下 3行 → 输入区 (Paragraph, 固定)
```

### 与 Claude Code 的 TUI 对应关系

| Claude Code (Ink) | mini-code (ratatui) | 说明 |
|-------------------|---------------------|------|
| `Static` / `Box` | `Paragraph` + `Block` | 纯文本渲染区域 |
| `TextInput` | 自建输入组件 | Ink 用 `<TextInput>`, ratatui 手动处理 key event |
| `useInput` hook | crossterm `Event::Key` | 键盘事件捕获 |
| `useStdout` resize | crossterm `Event::Resize` | 窗口大小变化 |
| `render()` 返回组件树 | `draw(frame)` 分区渲染 | 两者都是 immediate mode 渲染 |

---

## 4. 核心接口设计

### 4.1 工具 Trait（对标 Claude Code 的 `buildTool()` 工厂函数）

```rust
// tools/mod.rs

/// 工具副作用类型（对标 Claude Code 的只读/写入分类）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SideEffect {
    ReadOnly,   // 只读，可并发执行
    Write,      // 写入，需串行化
}

/// 工具权限等级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionLevel {
    Safe,      // 静默执行
    Confirm,   // 需 Y/n 确认
    Deny,      // 需手动输入 "allow"
}

/// 核心工具 trait
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;          // JSON Schema
    fn side_effect(&self) -> SideEffect;
    fn default_permission(&self) -> PermissionLevel;
    async fn execute(&self, input: Value) -> Result<String>;
}

/// 工具注册表
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
    permission_overrides: HashMap<String, PermissionLevel>,
}

impl ToolRegistry {
    pub fn register(&mut self, tool: Box<dyn Tool>) { ... }
    pub fn get(&self, name: &str) -> Option<&Box<dyn Tool>> { ... }
    pub fn resolve_permission(&self, name: &str) -> PermissionLevel { ... }
    /// 生成发送给 API 的 tools 数组
    pub fn tool_definitions(&self) -> Vec<ToolDefinition> { ... }
}
```

### 4.2 应用状态（对标 Claude Code 的 Store 模式）

```rust
// app.rs

/// 应用运行模式
pub enum AppMode {
    Input,              // 输入模式，等待用户键入
    Streaming,          // AI 正在输出，实时渲染 text_delta
    WaitingPermission,  // 等待用户确认工具执行
    Executing,          // 工具正在执行
}

/// 应用全局状态
pub struct App {
    pub mode: AppMode,
    pub input: String,                  // 当前输入缓冲区
    pub conversation: Vec<ConversationLine>,  // 对话区内容
    pub status_text: String,            // 状态栏文本
    pub permission_prompt: Option<PermissionPrompt>,  // 权限弹窗
    pub agent: Agent,
    pub session_manager: SessionManager,
    pub config: Arc<Config>,
    pub running: bool,
}

impl App {
    /// 主事件循环入口
    pub async fn run(&mut self, terminal: &mut Terminal<...>) -> Result<()>;

    /// 处理用户输入（非 / 开头）
    pub async fn handle_chat_input(&mut self, text: String) -> Result<()>;

    /// 处理斜杠命令
    pub fn handle_command(&mut self, cmd: &str) -> Result<()>;

    /// 处理权限确认
    pub fn handle_permission(&mut self, choice: char) -> Result<()>;
}
```

### 4.3 Agent 循环接口（对标 Claude Code 的 `query.ts`）

```rust
// agent.rs

pub struct Agent {
    client: AnthropicClient,
    registry: Arc<ToolRegistry>,
    config: Arc<Config>,
    max_rounds: usize,
}

pub enum AgentEvent {
    /// 流式文本增量，直接渲染
    TextDelta(String),
    /// 工具调用完成（累积了完整参数），等待权限+执行
    ToolUse { id: String, name: String, input: Value },
    /// 工具执行结果
    ToolResult { id: String, content: String, is_error: bool },
    /// 本轮 API 调用结束
    TurnFinished,
    /// 整个 agent 循环结束（模型不再调用工具）
    LoopFinished,
    /// 错误
    Error(String),
}

impl Agent {
    /// 创建  Agent，设定运行参数
    pub fn new(client: AnthropicClient, registry: Arc<ToolRegistry>, config: Arc<Config>) -> Self;

    /// 核心循环，对标 Claude Code query.ts 的 while(true) 循环。
    ///
    /// 1. 将 user message 追加到 messages
    /// 2. 循环:
    ///    a. 调用 Anthropic API (streaming)
    ///    b. 解析 SSE 事件 → 通过 event_tx 发送
    ///    c. 收集 tool_use blocks → 执行 → 追加 tool_result
    ///    d. 如果没有 tool_use → 退出循环
    /// 3. 返回最终的 messages 列表
    pub async fn run(
        &mut self,
        messages: &mut Vec<Message>,
        system_prompt: &str,
        event_tx: mpsc::UnboundedSender<AgentEvent>,
    ) -> Result<()>;
}
```

### 4.4 Anthropic API 客户端

```rust
// anthropic.rs

pub struct AnthropicClient {
    api_key: String,
    model: String,
    base_url: String,
    max_tokens: u32,
    client: reqwest::Client,
}

pub enum StreamEvent {
    TextDelta(String),
    ContentBlockStart { index: usize, block_type: String },
    ContentBlockDelta { index: usize, delta: Value },
    ContentBlockStop { index: usize },
    MessageStop,
    Error { code: String, message: String },
}

impl AnthropicClient {
    pub async fn stream_message(
        &self,
        messages: &[Message],
        system: &str,
        tools: &[ToolDefinition],
        event_tx: mpsc::UnboundedSender<StreamEvent>,
    ) -> Result<()>;
}
```

### 4.5 会话管理（JSONL 格式，对标 Claude Code）

```rust
// session.rs

pub struct Session {
    pub id: String,          // UUID v4
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<Message>,
}

pub struct SessionMeta {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
}

pub struct SessionManager {
    base_dir: PathBuf,      // ~/.local/share/mini-code/sessions/
    active_id: Option<String>,
    active_session: Option<Session>,
}

impl SessionManager {
    pub fn new(config: &Config) -> Result<Self>;
    pub fn create(&mut self, name: &str) -> Result<()>;
    pub fn switch(&mut self, id: &str) -> Result<()>;
    pub fn delete(&mut self, id: &str) -> Result<()>;
    pub fn list(&self) -> Result<Vec<SessionMeta>>;
    pub fn rename(&mut self, name: &str) -> Result<()>;
    pub fn clear_messages(&mut self) -> Result<()>;
    pub fn active(&self) -> Option<&Session>;
    pub fn add_message(&mut self, msg: Message);
    /// 追加写 JSONL：在文件末尾追加一行 JSON
    pub fn save(&self) -> Result<()>;
}
```

---

## 5. 并发模型

### 5.1 总体策略

**单线程异步 + Channel 解耦**。TUI 事件循环和 Agent 循环在同一线程，通过 channel 传递 Agent 事件驱动 TUI 刷新。

```
tokio runtime (single-threaded)

app.rs 事件循环 ──────────────────────────────────────────>

  crossterm Event::Key ──► 路由判断
       │
       ├─ 回车 → spawn Agent::run()
       │              │
       │              ▼
       │         anthropic SSE stream
       │              │
       │    mpsc::unbounded_channel<AgentEvent>
       │              │
       │              ▼
       │         event_tx.send(TextDelta("好的..."))
       │         event_tx.send(ToolUse { ... })
       │         event_tx.send(LoopFinished)
       │              │
       │              ▼
       │         app 收到 event → 更新状态 → tui.draw()
       │
       └─ 权限弹窗内按键 → handle_permission() → 继续 agent loop
```

### 5.2 关键并发点

| 场景 | 方案 | 理由 |
|------|------|------|
| Agent 事件 →  TUI | `tokio::mpsc::unbounded_channel` | token 速率级别的消息量，无界 channel 不会积压 |
| Agent 执行中 → 用户输入 | 阻塞输入直到 LoopFinished | 简化交互模型，对标 Claude Code 的行为 |
| Ctrl+C 中断 | `tokio::signal` + 原子标志位 | 设置退出标志后先 save 再退出 |
| Session 读写 | 单线程无竞争，不需要锁 | IO 发生在 agent 循环外（启动、退出时） |

---

## 6. 数据模型

### 6.1 会话存储格式（JSONL）

```
~/.local/share/mini-code/sessions/
├── index.jsonl          # 会话索引，一行一个会话
├── <uuid>.jsonl         # 每个会话一个 JSONL 文件，一行一条消息
```

**index.jsonl**（每行一条会话元信息）：
```jsonl
{"id":"uuid-1","name":"feature-x","created_at":"2026-05-16T12:00:00Z","updated_at":"2026-05-16T13:30:00Z","message_count":12}
{"id":"uuid-2","name":"bugfix-y","created_at":"2026-05-15T09:00:00Z","updated_at":"2026-05-15T10:00:00Z","message_count":3}
```

**{uuid}.jsonl**（每行一条完整消息）：
```jsonl
{"role":"user","content":[{"type":"text","text":"帮我重构 config.rs"}]}
{"role":"assistant","content":[{"type":"text","text":"好的，让我先看看当前代码。"},{"type":"tool_use","id":"toolu_001","name":"read_file","input":{"path":"src/config.rs"}}]}
{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_001","content":"// file content..."}]}
```

**为什么用 JSONL 而非 JSON**：

| 对比项 | JSON 单文件 | JSONL 追加写 |
|--------|------------|-------------|
| 写入 | 需全量序列化重写 | 只追加一行，O(1) |
| 读取 | 一次读到内存 | 逐行流式读取，可部分加载 |
| 损坏恢复 | 一个错全坏 | 只损一行，其余可读 |
| 大会话 | 万行消息时序列化耗时 | 追加无性能退化 |
| 对标 | — | **Claude Code 用的就是 JSONL** |

### 6.2 配置格式（TOML）

```toml
# ~/.config/mini-code/config.toml

[api]
api_key = "sk-ant-xxx"
model = "claude-sonnet-4-6"
base_url = "https://api.anthropic.com"
max_tokens = 8192

[permissions]
bash = "confirm"
write_file = "confirm"
read_file = "safe"
glob = "safe"
grep = "safe"
list_dir = "safe"
edit_file = "confirm"

[behavior]
auto_save = true
theme = "dark"
```

### 6.3 用户数据目录总览（对标 Claude Code 的 `~/.claude/`）

```
~/.config/mini-code/
  config.toml               # 用户配置
  instructions.md            # 全局系统指令（可选）

~/.local/share/mini-code/
  sessions/
    index.jsonl              # 会话索引
    <uuid>.jsonl             # 各会话消息
  history.jsonl              # 全局对话历史（P1）
  memory/                    # 项目记忆（P1）
```

---

## 7. 核心流程

### 7.1 启动流程

```
main()
  ├─ 1. clap 解析 CLI args
  │     └─ mini-code [--model <model>] [--session <id>]
  ├─ 2. Config::load_or_create()
  │     ├─ 不存在 → 交互式配置向导
  │     └─ 存在 → 加载 TOML
  ├─ 3. SessionManager::new() → 读取 index.jsonl
  ├─ 4. ContextLoader::load() → 搜索 CLAUDE.md
  ├─ 5. ToolRegistry::init() → 注册 7 个工具
  ├─ 6. AnthropicClient::new(config)
  ├─ 7. Agent::new(client, registry, config)
  ├─ 8. App::new(agent, session_manager, config)
  └─ 9. app.run(terminal)
        ├─ crossterm 初始化终端 (raw mode, alternate screen)
        ├─ tui::draw() 渲染初始界面
        └─ 事件循环
```

### 7.2 Agent 循环

```
Agent::run(messages, system_prompt, event_tx)
  │
  ├─ 1. 构建 API 请求体
  │     { model, max_tokens, system, messages, tools, stream: true }
  │
  ├─ 2. while true (对标 Claude Code query.ts):
  │     │
  │     ├─ 2a. 调用 API，读取 SSE 流
  │     │     ├─ text_delta → event_tx.send(TextDelta(...))
  │     │     ├─ tool_use → 累积参数到 content_block_stop
  │     │     └─ message_stop → 本轮结束
  │     │
  │     ├─ 2b. 构建 assistant message (text + tool_use blocks)
  │     │     追加到 messages
  │     │
  │     ├─ 2c. 如果没有 tool_use → 退出循环
  │     │
  │     ├─ 2d. 对每个 tool_use:
  │     │     ├─ event_tx.send(ToolUse { ... })
  │     │     ├─ 等待外部调用 execute_tool() (权限已通过)
  │     │     ├─ 执行 → 获取结果
  │     │     ├─ event_tx.send(ToolResult { ... })
  │     │     └─ 追加 tool_result 到 messages
  │     │
  │     └─ 2e. event_tx.send(TurnFinished) → 继续下一轮
  │
  ├─ 3. event_tx.send(LoopFinished)
  └─ 4. 返回 messages
```

### 7.3 权限确认交互（TUI 弹窗模式）

Claude Code 的权限确认是内联在对话流中的。mini-code 用 TUI 弹窗实现：

```
┌──────────────────────────────────────────────────┐
│  ...对话内容...                                    │
├──────────────────────────────────────────────────┤
│  ⚡ bash(cargo build)                            │
│  命令: cargo build                               │
│  超时: 120s                                      │
│  目录: /Users/gaoxiang/Workspace2026/mini-code    │
│  ──────────────────────────────────────────────── │
│  执行? [Y]是 [n]否 [a]本次会话始终允许             │
└──────────────────────────────────────────────────┘
```

---

## 8. 错误处理策略

### 8.1 分层

| 层级 | 方案 | 示例 |
|------|------|------|
| 库层 (lib) | `thiserror` 枚举 | `ConfigError::NotFound`, `SessionError::InvalidId` |
| 应用层 | `anyhow::Result` + `.context()` | 带上下文的错误传播 |
| TUI 渲染 | 对话区渲染错误提示 | `"⚠️ API 调用失败: connection timeout"` |

### 8.2 恢复策略

| 场景 | 策略 |
|------|------|
| API 网络超时 | 重试 1 次（2s 间隔），失败则在对话区显示错误 |
| API 4xx | 解析 error body 展示，不重试 |
| API 5xx | 重试 1 次，失败则提示 |
| 工具执行失败 | `is_error: true` 返回给模型，让模型自修复 |
| 会话文件损坏 | 跳过损坏行（JSONL 按行隔离），警告用户 |
| max_rounds 达限 | 对话区提示"已达最大对话轮数 (25)，请重新输入问题" |
| Ctrl+C | 保存当前会话 → 退出 raw mode → 恢复终端 |

### 8.3 panic

- `ratatui` panic hook：panic 前恢复终端，避免终端乱码
- 工具 `execute` 禁止 panic：所有错误路径返回 `Err`

---

## 9. 测试策略

### 9.1 分层

```
        ╱   集成测试   ╲        (完整 agent loop + mock API + mock TUI)
       ╱───────────────╲
      ╱   模块测试      ╲      (每个模块关键场景)
     ╱───────────────────╲
    ╱    单元测试          ╲   (trait 实现、解析、权限决策)
   ╱─────────────────────────╲
```

### 9.2 范围

| 模块 | 测试重点 | 工具 |
|------|---------|------|
| `config` | TOML 解析、默认值、向导交互 | 单元测试 |
| `session` | JSONL 读写、损坏行跳过、索引维护 | 单元测试 + tempfile |
| `anthropic` | SSE 解析、错误映射、重试逻辑 | 单元测试 + mockito |
| `agent` | 空消息、max_rounds、工具执行路径 | 集成测试 + mock API |
| `permissions` | 三级决策、配置覆盖、always 模式 | 单元测试 |
| `tools/*` | 各工具正常/异常、参数校验 | 单元测试 + tempfile |
| `tui` | 布局渲染、事件分发 | 集成测试（ratatui 有 `TestBackend`） |
| `app` | 命令路由、状态转换 | 单元测试 |

---

## 10. 构建与发布

```
dev:
  cargo run
  cargo test
  cargo clippy

release:
  cargo build --release
  strip target/release/mini-code   # macOS: 去掉 debug 符号
  # 目标: < 10MB

dist:
  - GitHub Releases: macOS arm64/x64, Linux x64 二进制
  - Homebrew tap
  - cargo install --git
```

---

## 11. 风险与应对

| 风险 | 概率 | 影响 | 应对 |
|------|------|------|------|
| Anthropic API 变更 | 低 | 高 | `anthropic-version` header 锁定版本；消息存储用 API 原生格式 |
| SSE 跨 chunk 解析 | 中 | 中 | 增量解析需处理 JSON partial；单元测试覆盖截断场景 |
| 大文件 OOM | 低 | 中 | `read_file` 限制行数，`limit` 参数硬上限 |
| 会话文件膨胀 | 中 | 低 | JSONL 追加写天然抗膨胀；P1 加 auto-compaction |
| Ctrl+C 丢数据 | 中 | 低 | panic hook + signal handler 双重保障 |
| ratatui 学习曲线 | 高 | 中 | 仅用 `Paragraph` + `Block` + `Line` 三个基础组件，不碰复杂布局 |

---

## 12. 架构权衡记录 (ADR)

### ADR-001: 消息存储采用 Anthropic API 原生格式

**决定**：session JSONL 中 `content` 字段直接存储 Anthropic API content block 数组。

**Claude Code 做法**：同样直接存 API 格式，不引入中间层。

**理由**：减少转换代码；API 新增 content block 类型时无需改动存储层；发送请求时零转换。

**代价**：切换非 Anthropic 模型时需在 `anthropic.rs` 层做格式转换。

---

### ADR-002: 使用 ratatui + crossterm 而非 rustyline

**决定**：采用 TUI 框架做分区渲染，对标 Claude Code 的 React+Ink。

**Claude Code 做法**：React + Ink - 对话区、输入区、状态栏分区域渲染，工具调用内联展示。

**理由**：
- 固定的状态栏（会话名、消息数）提升体验
- 工具调用可视化（进度提示、权限弹窗）
- 对话区和输入区视觉分离
- crossterm 提供 raw mode、键盘事件、色彩，ratatui 提供布局引擎

**代价**：比 rustyline 多约 200 行 TUI 框架代码。但用户体验提升显著。

---

### ADR-003: 使用 JSONL 而非 SQLite

**决定**：会话存储使用追加写 JSONL 文件。

**Claude Code 做法**：同样用文件存储，核心存储都是 JSONL（`history.jsonl`、session 文件），不使用 SQLite。

**理由**：
- JSONL 追加写是 O(1)，不会随着会话变长而退化
- 按行隔离损坏，一行坏了不影响其余数据
- 人类可读，`cat` / `tail` 就能调试
- 零依赖，编译时间不受影响
- Claude Code 证明了文件存储能支撑生产级使用

**代价**：复杂查询（按日期/关键词搜历史）做不到。P2 需要这类查询时，考虑建 SQLite 索引库（不替代主存储）。

---

### ADR-004: 单线程异步

**决定**：使用单线程 tokio runtime。

**理由**：API 延迟是瓶颈（秒级），CPU 不是。单线程避免 `Send + Sync` 传染。工具执行串行已满足 MVP 需求。P1 子代理用 `spawn_blocking` 隔离。

---

### ADR-005: 工具结果返回纯字符串

**决定**：`Tool::execute` → `Result<String>`。

**理由**：结果最终作为 `tool_result.content` 文本发给模型，String 够用。后续加 `execute_structured` 可扩展。

---

## 13. 与 PRD-v2.0 的映射

| PRD 功能 | 对应模块 | 关键接口 |
|----------|---------|---------|
| 1. 配置管理 | `config.rs` | `Config::load_or_create()` |
| 2. 流式对话 | `anthropic.rs` + `agent.rs` | `stream_message()`, `AgentEvent::TextDelta` |
| 3. 会话管理 | `session.rs` | `SessionManager::{create, switch, delete, list}` |
| 4. 工具系统 | `tools/mod.rs` + `permissions.rs` | `Tool trait`, `ToolRegistry`, `PermissionLevel` |
| 5. REPL/TUI 交互 | `tui.rs` + `app.rs` | ratatui 三区布局，crossterm 键盘事件 |
| 6. 系统提示词 | `context.rs` | `ContextLoader::load()`, CLAUDE.md 向上搜索 |

---

## 14. 与 Claude Code 架构的核心差异化

| 维度 | mini-code 比 Claude Code | 原因 |
|------|--------------------------|------|
| 启动速度 | 更快（native binary vs Bun JIT） | Rust 编译为原生代码 |
| 内存占用 | 更低（无 GC，无 v8/bun 堆） | Rust 的所有权模型 |
| 工具安全 | 同等（三级权限 + 配置覆盖） | 直接对标设计 |
| 上下文压缩 | MVP 不做（P1） | Claude Code 有三层压缩（46K 行 QueryEngine），单人开发优先级排后 |
| 插件系统 | P2 | MVP 先跑通核心循环 |
| 子代理 | P1 | P0 先做好单 Agent |
| 多模型 | P2 | 先做好 Anthropic API |

---

*文档版本: ARCH-v2.0 | 日期: 2026-05-16 | 对应 PRD: v2.0*
