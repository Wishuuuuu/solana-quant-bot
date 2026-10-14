use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub rpc_url: String,
    pub database_url: String,
    pub redis_url: String,
    pub wallet_path: String,
    pub paper_trading: bool,
    pub risk: RiskConfig,
    pub trading: TradingConfig,
    pub notifications: NotificationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    pub take_profit_percentage: f64,
    pub initial_stop_loss_percentage: f64,
    pub trailing_stop_percentage: f64,
    pub max_open_positions: usize,
    pub max_position_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    pub default_slippage: u16,
    pub default_priority_fee: u64,
    pub min_hype_score: f64,
    pub max_spam_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub discord_webhook_url: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = env::var("CONFIG_PATH")
            .unwrap_or_else(|_| "config/config.toml".to_string());

        if Path::new(&config_path).exists() {
            Self::load_from_file(&config_path)
        } else {
            Self::load_from_env()
        }
    }

    fn load_from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path))?;

        config.validate()?;
        Ok(config)
    }

    fn load_from_env() -> Result<Self> {
        let config = Config {
            rpc_url: env::var("RPC_URL")
                .context("RPC_URL environment variable not set")?,
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://user:password@localhost/solana_quant".to_string()),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            wallet_path: env::var("WALLET_PATH")
                .unwrap_or_else(|_| "config/wallet.json".to_string()),
            paper_trading: env::var("PAPER_TRADING")
                .unwrap_or_else(|_| "false".to_string())
                .to_lowercase() == "true",
            risk: RiskConfig {
                take_profit_percentage: env::var("TAKE_PROFIT_PERCENTAGE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(100.0),
                initial_stop_loss_percentage: env::var("INITIAL_STOP_LOSS_PERCENTAGE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30.0),
                trailing_stop_percentage: env::var("TRAILING_STOP_PERCENTAGE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(15.0),
                max_open_positions: env::var("MAX_OPEN_POSITIONS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(5),
                max_position_size: env::var("MAX_POSITION_SIZE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1_000_000_000),
            },
            trading: TradingConfig {
                default_slippage: env::var("DEFAULT_SLIPPAGE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(100),
                default_priority_fee: env::var("DEFAULT_PRIORITY_FEE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1000),
                min_hype_score: env::var("MIN_HYPE_SCORE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.6),
                max_spam_score: env::var("MAX_SPAM_SCORE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.5),
            },
            notifications: NotificationConfig {
                telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN").ok(),
                telegram_chat_id: env::var("TELEGRAM_CHAT_ID").ok(),
                discord_webhook_url: env::var("DISCORD_WEBHOOK_URL").ok(),
            },
        };

        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        // Validate RPC URL
        if !self.rpc_url.starts_with("http://") && !self.rpc_url.starts_with("https://") {
            anyhow::bail!("RPC URL must start with http:// or https://");
        }

        // Validate risk parameters
        if self.risk.take_profit_percentage <= 0.0 {
            anyhow::bail!("Take profit percentage must be positive");
        }

        if self.risk.initial_stop_loss_percentage >= 100.0 {
            anyhow::bail!("Stop loss percentage must be less than 100");
        }

        if self.risk.trailing_stop_percentage >= self.risk.take_profit_percentage {
            anyhow::bail!("Trailing stop must be less than take profit");
        }

        if self.risk.max_open_positions == 0 {
            anyhow::bail!("Max open positions must be greater than 0");
        }

        // Validate trading parameters
        if self.trading.default_slippage > 1000 {
            anyhow::bail!("Slippage too high (max 1000 basis points)");
        }

        if self.trading.min_hype_score < 0.0 || self.trading.min_hype_score > 1.0 {
            anyhow::bail!("Min hype score must be between 0 and 1");
        }

        if self.trading.max_spam_score < 0.0 || self.trading.max_spam_score > 1.0 {
            anyhow::bail!("Max spam score must be between 0 and 1");
        }

        // Validate wallet path exists if not paper trading
        if !self.paper_trading {
            if !Path::new(&self.wallet_path).exists() {
                anyhow::bail!("Wallet file not found: {} (required for live trading)", self.wallet_path);
            }
        }

        Ok(())
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let toml_string = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        fs::write(path, toml_string)
            .with_context(|| format!("Failed to write config to: {}", path))?;

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rpc_url: "https://mainnet.helius-rpc.com/?api-key=YOUR_API_KEY".to_string(),
            database_url: "postgresql://user:password@localhost/solana_quant".to_string(),
            redis_url: "redis://127.0.0.1:6379".to_string(),
            wallet_path: "config/wallet.json".to_string(),
            paper_trading: true,
            risk: RiskConfig {
                take_profit_percentage: 100.0,
                initial_stop_loss_percentage: 30.0,
                trailing_stop_percentage: 15.0,
                max_open_positions: 5,
                max_position_size: 1_000_000_000,
            },
            trading: TradingConfig {
                default_slippage: 100,
                default_priority_fee: 1000,
                min_hype_score: 0.6,
                max_spam_score: 0.5,
            },
            notifications: NotificationConfig {
                telegram_bot_token: None,
                telegram_chat_id: None,
                discord_webhook_url: None,
            },
        }
    }
}
