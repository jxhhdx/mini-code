# mini-code 测试用例清单

## 1. config.rs — 配置模块

| # | 用例 | 输入 | 期望输出 |
|---|------|------|----------|
| 1.1 | 加载不存在的文件 | `Config::load("/nonexistent/config.toml")` | 返回 `Err` |
| 1.2 | 部分字段缺失用默认值 | TOML 只含 `api_key`，其他不写 | 缺失字段取默认值 |
| 1.3 | save 需自动创建父目录 | path 含不存在的目录 | 自动创建目录，写入成功 |
| 1.4 | 无效 TOML 格式 | 内容为 `{{{invalid toml` | 返回 `Err` |

## 2. message_history.rs — 消息类型

| # | 用例 | 输入 | 期望输出 |
|---|------|------|----------|
| 2.1 | ToolResult 序列化/反序列化 | `Message::tool_result("id", "result", false)` | JSON 包含 `tool_use_id`、`content`、`is_error: false` |
| 2.2 | `to_api_format` 文本消息 | `Message::user("hello")` | `{"role":"user","content":[{"type":"text","text":"hello"}]}` |
| 2.3 | `to_api_format` tool_use | `Message::assistant_tool_use(...)` | content 数组含 `"type":"tool_use"` |
| 2.4 | `to_api_format` tool_result 带 is_error | `Message::tool_result("id", "err", true)` | content 数组含 `"is_error": true` |

## 3. session.rs — 会话管理

| # | 用例 | 输入 | 期望输出 |
|---|------|------|----------|
| 3.1 | 空目录列出会话 | 新 SessionManager，无 session 文件 | 返回空列表 |
| 3.2 | load 不存在的 ID | `manager.load("nonexistent-id")` | 返回 `Err` |
| 3.3 | switch 不存在的 ID | `manager.switch("nonexistent")` | 返回 `Err` |
| 3.4 | delete 不存在的 ID | `manager.delete("nonexistent")` | 不报错，静默处理 |
| 3.5 | save 更新 `updated_at` | 创建后 sleep 1ms，再 save | `updated_at > created_at` |
| 3.6 | 目录中损坏的 JSON 被跳过 | 放一个 `{"bad": json` 文件 | `list()` 跳过坏文件，不崩溃 |
| 3.7 | update_current 不匹配当前 session | 传一个非当前 session | 不保存（返回 Ok，无效果） |
| 3.8 | 列出时按 updated_at 降序 | 创建 2 个 session，更新老的 | 更新的排前面 |
| 3.9 | 切换后 current 返回正确 session | switch 后调 `current()` | ID 匹配 |

## 4. tools/mod.rs — 工具注册表

| # | 用例 | 输入 | 期望输出 |
|---|------|------|----------|
| 4.1 | `ToolRegistry::get` 找到 | `registry.get("read_file")` | 返回 `Some(&dyn Tool)` |
| 4.2 | `ToolRegistry::get` 未找到 | `registry.get("nonexistent")` | 返回 `None` |
| 4.3 | `execute` 未找到工具 | `registry.execute("nonexistent", ...)` | 返回 `Err(ToolError::NotFound(...))` |
| 4.4 | `definitions()` 返回正确数量 | `default_registry()` | `len() == 5` |
| 4.5 | `ToolError` Display | `ToolError::Execution("msg".into())` | 打印 `"msg"` |

## 5. tools/read_file.rs — 读文件

| # | 用例 | 输入 | 期望输出 |
|---|------|------|----------|
| 5.1 | 缺少 path 参数 | `execute(json!({}), false)` | `Err(ToolError::Execution(...))` |
| 5.2 | 权限不足 | 尝试读无权限文件 | `Err` |

## 6. tools/write_file.rs — 写文件

| # | 用例 | 输入 | 期望输出 |
|---|------|------|----------|
| 6.1 | 缺少 path | `execute(json!({"content":"x"}), false)` | `Err` |
| 6.2 | 缺少 content | `execute(json!({"path":"/tmp/x"}), false)` | `Err` |
| 6.3 | 父目录不存在自动创建 | path 有多层不存在目录 | 自动创建，写入成功 |

## 7. tools/search_replace.rs — 搜索替换

| # | 用例 | 输入 | 期望输出 |
|---|------|------|----------|
| 7.1 | 文件不存在 | `path` 指向不存在的文件 | `Err` |
| 7.2 | 多次匹配全部替换 | old 出现 3 次 | 全部替换，返回 `3 occurrence(s)` |
| 7.3 | 缺少 old_string | `json!({"path":"...","new_string":"x"})` | `Err` |

## 8. tools/bash.rs — Shell 执行

| # | 用例 | 输入 | 期望输出 |
|---|------|------|----------|
| 8.1 | 缺少 command | `execute(json!({}), false)` | `Err` |
| 8.2 | 带 stderr 但成功 | `echo out && echo err >&2` | 输出含 `out`，也含 `stderr: err` |
| 8.3 | 非零退出码 | `exit 1` | `Err(ToolError::Execution(...))` |

## 9. tools/list_dir.rs — 列目录

| # | 用例 | 输入 | 期望输出 |
|---|------|------|----------|
| 9.1 | 缺少 path | `execute(json!({}), false)` | `Err` |
| 9.2 | 空目录 | 空 temp 目录 | 返回空字符串 |
| 9.3 | 混合文件和目录 | 目录中放文件和子目录 | `[FILE]` 和 `[DIR]` 前缀正确 |

## 10. anthropic.rs — API 客户端

| # | 用例 | 输入 | 期望输出 |
|---|------|------|----------|
| 10.1 | API 返回 4xx 错误 | mock 返回 401 | `Err` |
| 10.2 | API 返回 5xx 错误 | mock 返回 500 | `Err` |
| 10.3 | tool_use 响应解析 | API 返回 `type: "tool_use"` | 正确解析为 `Content::ToolUse` |
| 10.4 | 响应缺少 content 字段 | API 返回体无 `content` | `Err` |
| 10.5 | 未知 block type | API 返回 `type: "unknown_type"` | 跳过，不崩溃 |

---

> 现有测试：21 个 | 缺失测试：35 个 | 补全后总计：56 个
