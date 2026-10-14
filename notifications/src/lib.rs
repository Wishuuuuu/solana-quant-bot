use anyhow::Result;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    Telegram,
    Discord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    TradeExecuted,
    TakeProfit,
    StopLoss,
    NewOpportunity,
    SystemError,
    PositionOpened,
    PositionClosed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub alert_type: AlertType,
    pub message: String,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: chrono::DateTime<Utc>,
}

pub struct NotificationService {
    telegram_bot_token: Option<String>,
    telegram_chat_id: Option<String>,
    discord_webhook_url: Option<String>,
    client: Client,
}

impl NotificationService {
    pub fn new() -> Self {
        Self {
            telegram_bot_token: std::env::var("TELEGRAM_BOT_TOKEN").ok(),
            telegram_chat_id: std::env::var("TELEGRAM_CHAT_ID").ok(),
            discord_webhook_url: std::env::var("DISCORD_WEBHOOK_URL").ok(),
            client: Client::new(),
        }
    }

    pub fn with_credentials(
        telegram_bot_token: Option<String>,
        telegram_chat_id: Option<String>,
        discord_webhook_url: Option<String>,
    ) -> Self {
        Self {
            telegram_bot_token,
            telegram_chat_id,
            discord_webhook_url,
            client: Client::new(),
        }
    }

    pub async fn send(&self, notification: Notification) -> Result<()> {
        info!("Sending notification: {:?}", notification.alert_type);

        // Send to Telegram if configured
        if let (Some(token), Some(chat_id)) = (&self.telegram_bot_token, &self.telegram_chat_id) {
            if let Err(e) = self.send_telegram(token, chat_id, &notification).await {
                error!("Failed to send Telegram notification: {}", e);
            }
        }

        // Send to Discord if configured
        if let Some(webhook_url) = &self.discord_webhook_url {
            if let Err(e) = self.send_discord(webhook_url, &notification).await {
                error!("Failed to send Discord notification: {}", e);
            }
        }

        Ok(())
    }

    async fn send_telegram(&self, bot_token: &str, chat_id: &str, notification: &Notification) -> Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
        
        let emoji = match notification.alert_type {
            AlertType::TradeExecuted => "🚀",
            AlertType::TakeProfit => "💰",
            AlertType::StopLoss => "🛑",
            AlertType::NewOpportunity => "🎯",
            AlertType::SystemError => "⚠️",
            AlertType::PositionOpened => "📈",
            AlertType::PositionClosed => "📊",
        };

        let message = format!(
            "{} {}\n\n{}\n\nTime: {}",
            emoji,
            format!("{:?}", notification.alert_type),
            notification.message,
            notification.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );

        let payload = serde_json::json!({
            "chat_id": chat_id,
            "text": message,
            "parse_mode": "HTML"
        });

        let response = self.client.post(&url).json(&payload).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Telegram API error: {}", error_text));
        }

        info!("Telegram notification sent successfully");
        Ok(())
    }

    async fn send_discord(&self, webhook_url: &str, notification: &Notification) -> Result<()> {
        let color = match notification.alert_type {
            AlertType::TradeExecuted => 0x00ff00, // Green
            AlertType::TakeProfit => 0x00ff00,
            AlertType::StopLoss => 0xff0000, // Red
            AlertType::NewOpportunity => 0xffff00, // Yellow
            AlertType::SystemError => 0xff0000,
            AlertType::PositionOpened => 0x00ffff, // Cyan
            AlertType::PositionClosed => 0xff00ff, // Magenta
        };

        let payload = serde_json::json!({
            "embeds": [{
                "title": format!("{:?}", notification.alert_type),
                "description": notification.message,
                "color": color,
                "timestamp": notification.timestamp.to_rfc3339(),
            }]
        });

        let response = self.client.post(webhook_url).json(&payload).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Discord API error: {}", error_text));
        }

        info!("Discord notification sent successfully");
        Ok(())
    }

    pub async fn send_trade_alert(
        &self,
        token_mint: &str,
        action: &str,
        amount: f64,
        price: f64,
    ) -> Result<()> {
        let notification = Notification {
            alert_type: AlertType::TradeExecuted,
            message: format!(
                "<b>Trade Executed</b>\n\nToken: {}\nAction: {}\nAmount: {:.4} SOL\nPrice: ${:.4}",
                token_mint, action, amount, price
            ),
            metadata: Some(serde_json::json!({
                "token_mint": token_mint,
                "action": action,
                "amount": amount,
                "price": price
            })),
            timestamp: Utc::now(),
        };

        self.send(notification).await
    }

    pub async fn send_profit_alert(
        &self,
        token_mint: &str,
        pnl_percentage: f64,
        pnl_amount: f64,
    ) -> Result<()> {
        let notification = Notification {
            alert_type: AlertType::TakeProfit,
            message: format!(
                "<b>Take Profit Triggered</b>\n\nToken: {}\nPnL: {:.2}%\nProfit: {:.4} SOL",
                token_mint, pnl_percentage, pnl_amount
            ),
            metadata: Some(serde_json::json!({
                "token_mint": token_mint,
                "pnl_percentage": pnl_percentage,
                "pnl_amount": pnl_amount
            })),
            timestamp: Utc::now(),
        };

        self.send(notification).await
    }

    pub async fn send_loss_alert(
        &self,
        token_mint: &str,
        loss_percentage: f64,
        loss_amount: f64,
    ) -> Result<()> {
        let notification = Notification {
            alert_type: AlertType::StopLoss,
            message: format!(
                "<b>Stop Loss Triggered</b>\n\nToken: {}\nLoss: {:.2}%\nLoss: {:.4} SOL",
                token_mint, loss_percentage, loss_amount
            ),
            metadata: Some(serde_json::json!({
                "token_mint": token_mint,
                "loss_percentage": loss_percentage,
                "loss_amount": loss_amount
            })),
            timestamp: Utc::now(),
        };

        self.send(notification).await
    }

    pub async fn send_opportunity_alert(
        &self,
        token_mint: &str,
        score: f64,
        hype_score: f64,
    ) -> Result<()> {
        let notification = Notification {
            alert_type: AlertType::NewOpportunity,
            message: format!(
                "<b>New Trading Opportunity</b>\n\nToken: {}\nCombined Score: {:.2}\nHype Score: {:.2}",
                token_mint, score, hype_score
            ),
            metadata: Some(serde_json::json!({
                "token_mint": token_mint,
                "score": score,
                "hype_score": hype_score
            })),
            timestamp: Utc::now(),
        };

        self.send(notification).await
    }

    pub async fn send_error_alert(&self, component: &str, error_message: &str) -> Result<()> {
        let notification = Notification {
            alert_type: AlertType::SystemError,
            message: format!(
                "<b>System Error</b>\n\nComponent: {}\nError: {}",
                component, error_message
            ),
            metadata: Some(serde_json::json!({
                "component": component,
                "error": error_message
            })),
            timestamp: Utc::now(),
        };

        self.send(notification).await
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::new()
    }
}
