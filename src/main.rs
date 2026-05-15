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
