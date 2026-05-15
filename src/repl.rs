use crate::anthropic::AnthropicClient;
use crate::config::Config;
use crate::message_history::{Content, Message};
use crate::session::SessionManager;
use crate::tools::{self, ToolRegistry};
use anyhow::Result;
use colored::Colorize;
use rustyline::DefaultEditor;

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
            registry: tools::default_registry(),
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

            let response_messages = match self.client.send_message(
                &session.messages, &tool_defs
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
