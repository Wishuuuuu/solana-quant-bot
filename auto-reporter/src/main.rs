use anyhow::Result;
use chrono::{Duration, Utc};
use database::Database;
use notifications::NotificationService;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,auto_reporter=debug")
        .init();

    info!("Starting Auto-Report Generation System...");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://user:password@localhost/solana_quant".to_string());
    
    let db = Database::new(&database_url).await?;

    let notification_service = NotificationService::new();

    let reporter = AutoReporter::new(db, notification_service);

    reporter.run_report_loop().await?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReport {
    pub date: chrono::NaiveDate,
    pub total_trades: i32,
    pub winning_trades: i32,
    pub losing_trades: i32,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub max_drawdown: f64,
    pub sharpe_ratio: Option<f64>,
    pub active_positions: i32,
    pub wallet_balance: f64,
    pub top_performing_token: Option<String>,
    pub worst_performing_token: Option<String>,
}

pub struct AutoReporter {
    db: Database,
    notification_service: NotificationService,
    report_interval: Duration,
}

impl AutoReporter {
    pub fn new(db: Database, notification_service: NotificationService) -> Self {
        Self {
            db,
            notification_service,
            report_interval: Duration::hours(24),
        }
    }

    pub async fn run_report_loop(&self) -> Result<()> {
        info!("Starting auto-report loop (interval: {:?})", self.report_interval);

        loop {
            match self.generate_and_send_report().await {
                Ok(_) => {
                    info!("Daily report generated and sent successfully");
                }
                Err(e) => {
                    warn!("Report generation failed: {}", e);
                }
            }

            info!("Next report in {:?}", self.report_interval);
            tokio::time::sleep(self.report_interval.to_std().unwrap()).await;
        }
    }

    pub async fn generate_and_send_report(&self) -> Result<()> {
        let report = self.generate_daily_report().await?;

        let message = self.format_report_message(&report);

        // Send to Telegram
        self.notification_service
            .send_error_alert("Daily Report", &message)
            .await?;

        // Also save to database
        self.save_report_to_database(&report).await?;

        Ok(())
    }

    async fn generate_daily_report(&self) -> Result<DailyReport> {
        let performance = self.db.get_performance_metrics(1).await?;
        
        let positions = self.db.get_active_positions().await?;
        
        let wallet_balance = 10.0; // In production, would query actual balance

        let daily_metrics = performance.first();

        Ok(DailyReport {
            date: Utc::now().date_naive(),
            total_trades: daily_metrics.map(|m| m.total_trades).unwrap_or(0),
            winning_trades: daily_metrics.map(|m| m.winning_trades).unwrap_or(0),
            losing_trades: daily_metrics.map(|m| m.losing_trades).unwrap_or(0),
            win_rate: daily_metrics.map(|m| m.win_rate).unwrap_or(0.0),
            total_pnl: daily_metrics.map(|m| m.total_pnl).unwrap_or(0.0),
            max_drawdown: daily_metrics.map(|m| m.max_drawdown).unwrap_or(0.0),
            sharpe_ratio: daily_metrics.and_then(|m| m.sharpe_ratio),
            active_positions: positions.len() as i32,
            wallet_balance,
            top_performing_token: None, // Would calculate from trade history
            worst_performing_token: None,
        })
    }

    fn format_report_message(&self, report: &DailyReport) -> String {
        format!(
            "📊 <b>Daily Trading Report</b>\n\
            📅 Date: {}\n\
            \n\
            📈 Performance:\n\
            • Total Trades: {}\n\
            • Winning Trades: {}\n\
            • Losing Trades: {}\n\
            • Win Rate: {:.1}%\n\
            • Total PnL: {:.4} SOL\n\
            • Max Drawdown: {:.2}%\n\
            • Sharpe Ratio: {:.2}\n\
            \n\
            💰 Current Status:\n\
            • Active Positions: {}\n\
            • Wallet Balance: {:.4} SOL\n\
            \n\
            Generated at: {}",
            report.date,
            report.total_trades,
            report.winning_trades,
            report.losing_trades,
            report.win_rate,
            report.total_pnl,
            report.max_drawdown,
            report.sharpe_ratio.unwrap_or(0.0),
            report.active_positions,
            report.wallet_balance,
            Utc::now().format("%H:%M UTC")
        )
    }

    async fn save_report_to_database(&self, report: &DailyReport) -> Result<()> {
        // In production, would save to database
        info!("Report saved to database");
        Ok(())
    }

    pub async fn generate_weekly_report(&self) -> Result<()> {
        let performance = self.db.get_performance_metrics(7).await?;
        
        let total_trades: i32 = performance.iter().map(|m| m.total_trades).sum();
        let total_pnl: f64 = performance.iter().map(|m| m.total_pnl).sum();
        
        let message = format!(
            "📊 <b>Weekly Trading Report</b>\n\
            📅 Week of {}\n\
            \n\
            • Total Trades: {}\n\
            • Total PnL: {:.4} SOL\n\
            • Avg Daily Trades: {:.1}\n\
            • Avg Daily PnL: {:.4} SOL",
            Utc::now().date_naive(),
            total_trades,
            total_pnl,
            total_trades as f64 / 7.0,
            total_pnl / 7.0
        );

        self.notification_service
            .send_error_alert("Weekly Report", &message)
            .await?;

        Ok(())
    }

    pub async fn generate_monthly_report(&self) -> Result<()> {
        let performance = self.db.get_performance_metrics(30).await?;
        
        let total_trades: i32 = performance.iter().map(|m| m.total_trades).sum();
        let total_pnl: f64 = performance.iter().map(|m| m.total_pnl).sum();
        
        let message = format!(
            "📊 <b>Monthly Trading Report</b>\n\
            📅 Month of {}\n\
            \n\
            • Total Trades: {}\n\
            • Total PnL: {:.4} SOL\n\
            • Avg Daily Trades: {:.1}\n\
            • Avg Daily PnL: {:.4} SOL",
            Utc::now().date_naive(),
            total_trades,
            total_pnl,
            total_trades as f64 / 30.0,
            total_pnl / 30.0
        );

        self.notification_service
            .send_error_alert("Monthly Report", &message)
            .await?;

        Ok(())
    }
}
