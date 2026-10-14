use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, postgres::PgPoolOptions};
use shared::{SmartMoneyWallet, TradeSignal, SentimentAnalysis, Position};

pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    pub async fn insert_smart_money_wallet(&self, wallet: &SmartMoneyWallet) -> Result<i32> {
        let row = sqlx::query!(
            r#"
            INSERT INTO smart_money_wallets (address, win_rate, total_trades, profit_factor, last_active, tags, is_active)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (address) DO UPDATE SET
                win_rate = EXCLUDED.win_rate,
                total_trades = EXCLUDED.total_trades,
                profit_factor = EXCLUDED.profit_factor,
                last_active = EXCLUDED.last_active,
                updated_at = NOW()
            RETURNING id
            "#,
            wallet.address.to_string(),
            wallet.win_rate,
            wallet.total_trades as i32,
            wallet.profit_factor,
            wallet.last_active,
            &wallet.tags,
            true
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }

    pub async fn insert_trade_signal(&self, signal: &TradeSignal) -> Result<i32> {
        let action = match signal.action {
            shared::TradeAction::Buy => "BUY",
            shared::TradeAction::Sell => "SELL",
        };

        let row = sqlx::query!(
            r#"
            INSERT INTO trade_signals (wallet_address, token_mint, action, amount, signature, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (signature) DO NOTHING
            RETURNING id
            "#,
            signal.wallet.to_string(),
            signal.token_mint.to_string(),
            action,
            signal.amount as i64,
            signal.signature,
            signal.timestamp
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }

    pub async fn insert_sentiment_analysis(&self, sentiment: &SentimentAnalysis) -> Result<i32> {
        let sentiment_label = match sentiment.sentiment {
            shared::Sentiment::Bullish => "Bullish",
            shared::Sentiment::Bearish => "Bearish",
            shared::Sentiment::Neutral => "Neutral",
        };

        let row = sqlx::query!(
            r#"
            INSERT INTO sentiment_analysis (token_mint, hype_score, sentiment, volume_velocity, spam_score, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id
            "#,
            sentiment.token_mint.to_string(),
            sentiment.hype_score,
            sentiment_label,
            sentiment.volume_velocity,
            sentiment.spam_score,
            sentiment.timestamp
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }

    pub async fn insert_position(&self, position: &Position) -> Result<i32> {
        let row = sqlx::query!(
            r#"
            INSERT INTO positions (
                token_mint, entry_price, current_price, size, pnl_percentage,
                highest_price, stop_loss_price, take_profit_price, entry_time, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id
            "#,
            position.token_mint.to_string(),
            position.entry_price,
            position.current_price,
            position.size as i64,
            position.pnl_percentage,
            position.highest_price,
            position.stop_loss_price,
            position.take_profit_price,
            position.entry_time,
            "OPEN"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }

    pub async fn update_position(&self, position_id: i32, position: &Position) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE positions
            SET current_price = $2, pnl_percentage = $3, highest_price = $4,
                stop_loss_price = $5, updated_at = NOW()
            WHERE id = $1
            "#,
            position_id,
            position.current_price,
            position.pnl_percentage,
            position.highest_price,
            position.stop_loss_price
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn close_position(&self, position_id: i32, exit_reason: &str) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE positions
            SET status = 'CLOSED', exit_time = NOW(), exit_reason = $2, updated_at = NOW()
            WHERE id = $1
            "#,
            position_id,
            exit_reason
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_active_positions(&self) -> Result<Vec<Position>> {
        let rows = sqlx::query!(
            r#"
            SELECT token_mint, entry_price, current_price, size, pnl_percentage,
                   highest_price, stop_loss_price, take_profit_price, entry_time
            FROM positions
            WHERE status = 'OPEN'
            ORDER BY entry_time DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let positions = rows.into_iter().map(|row| Position {
            token_mint: row.token_mint.parse().unwrap_or_default(),
            entry_price: row.entry_price,
            current_price: row.current_price.unwrap_or(row.entry_price),
            size: row.size as u64,
            pnl_percentage: row.pnl_percentage.unwrap_or(0.0),
            highest_price: row.highest_price.unwrap_or(row.entry_price),
            stop_loss_price: row.stop_loss_price.unwrap_or(0.0),
            take_profit_price: row.take_profit_price.unwrap_or(0.0),
            entry_time: row.entry_time,
        }).collect();

        Ok(positions)
    }

    pub async fn log_system_event(&self, component: &str, event_type: &str, severity: &str, message: &str, metadata: Option<serde_json::Value>) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO system_events (component, event_type, severity, message, metadata)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            component,
            event_type,
            severity,
            message,
            metadata
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_performance_metrics(&self, days: i32) -> Result<Vec<PerformanceMetric>> {
        let rows = sqlx::query!(
            r#"
            SELECT date, total_trades, winning_trades, losing_trades, win_rate,
                   total_pnl, max_drawdown, sharpe_ratio, avg_holding_time_hours
            FROM performance_metrics
            WHERE date >= NOW() - INTERVAL '1 day' * $1
            ORDER BY date DESC
            "#,
            days
        )
        .fetch_all(&self.pool)
        .await?;

        let metrics = rows.into_iter().map(|row| PerformanceMetric {
            date: row.date,
            total_trades: row.total_trades,
            winning_trades: row.winning_trades,
            losing_trades: row.losing_trades,
            win_rate: row.win_rate,
            total_pnl: row.total_pnl,
            max_drawdown: row.max_drawdown,
            sharpe_ratio: row.sharpe_ratio,
            avg_holding_time_hours: row.avg_holding_time_hours,
        }).collect();

        Ok(metrics)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceMetric {
    pub date: chrono::NaiveDate,
    pub total_trades: i32,
    pub winning_trades: i32,
    pub losing_trades: i32,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub max_drawdown: f64,
    pub sharpe_ratio: Option<f64>,
    pub avg_holding_time_hours: Option<f64>,
}
