use anyhow::Result;
use chrono::{Duration, NaiveDateTime, Utc};
use database::Database;
use serde::{Deserialize, Serialize};
use shared::{Position, RiskAction, Sentiment, SentimentAnalysis, TradeAction, TradeSignal};
use std::collections::HashMap;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,backtesting=debug")
        .init();

    info!("Starting Backtesting Engine...");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://user:password@localhost/solana_quant".to_string());
    
    let db = Database::new(&database_url).await?;

    let backtester = Backtester::new(db);

    // Define strategy parameters
    let strategy = StrategyParameters {
        take_profit_percentage: 100.0,
        initial_stop_loss_percentage: 30.0,
        trailing_stop_percentage: 15.0,
        min_hype_score: 0.6,
        max_spam_score: 0.5,
        min_smart_money_signals: 1,
        max_position_size: 1_000_000_000, // 1 SOL
        initial_capital: 10.0, // 10 SOL
    };

    // Run backtest
    let result = backtester.run_backtest(
        "Base Strategy",
        strategy,
        Utc::now() - Duration::days(30),
        Utc::now(),
    ).await?;

    info!("Backtest Results:");
    info!("Total Return: {:.2}%", result.total_return);
    info!("Win Rate: {:.2}%", result.win_rate);
    info!("Max Drawdown: {:.2}%", result.max_drawdown);
    info!("Sharpe Ratio: {:.4}", result.sharpe_ratio.unwrap_or(0.0));
    info!("Total Trades: {}", result.total_trades);

    // Save results to database
    // backtester.save_backtest_result(result).await?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyParameters {
    pub take_profit_percentage: f64,
    pub initial_stop_loss_percentage: f64,
    pub trailing_stop_percentage: f64,
    pub min_hype_score: f64,
    pub max_spam_score: f64,
    pub min_smart_money_signals: i32,
    pub max_position_size: u64,
    pub initial_capital: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub strategy_name: String,
    pub start_date: chrono::DateTime<Utc>,
    pub end_date: chrono::DateTime<Utc>,
    pub initial_capital: f64,
    pub final_capital: f64,
    pub total_return: f64,
    pub total_trades: i32,
    pub winning_trades: i32,
    pub losing_trades: i32,
    pub win_rate: f64,
    pub max_drawdown: f64,
    pub sharpe_ratio: Option<f64>,
    pub sortino_ratio: Option<f64>,
    pub avg_holding_time_hours: f64,
    pub equity_curve: Vec<(chrono::DateTime<Utc>, f64)>,
    pub trades: Vec<BacktestTrade>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestTrade {
    pub token_mint: String,
    pub entry_time: chrono::DateTime<Utc>,
    pub exit_time: chrono::DateTime<Utc>,
    pub entry_price: f64,
    pub exit_price: f64,
    pub pnl: f64,
    pub pnl_percentage: f64,
    pub holding_time_hours: f64,
    pub exit_reason: String,
}

pub struct Backtester {
    db: Database,
}

impl Backtester {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn run_backtest(
        &self,
        strategy_name: &str,
        params: StrategyParameters,
        start_date: chrono::DateTime<Utc>,
        end_date: chrono::DateTime<Utc>,
    ) -> Result<BacktestResult> {
        info!("Running backtest for strategy: {}", strategy_name);

        let mut capital = params.initial_capital;
        let mut equity_curve = Vec::new();
        let mut trades = Vec::new();
        let mut positions: HashMap<String, Position> = HashMap::new();

        // Simulate price data (in production, would load from database)
        let price_data = self.generate_mock_price_data(start_date, end_date).await?;

        // Simulate signals (in production, would load from database)
        let signals = self.generate_mock_signals(start_date, end_date).await?;

        let mut current_time = start_date;
        let mut winning_trades = 0;
        let mut losing_trades = 0;
        let mut max_capital = capital;
        let mut max_drawdown = 0.0;

        while current_time < end_date {
            // Process signals
            for signal in &signals {
                if signal.timestamp > current_time && signal.timestamp < current_time + Duration::hours(1) {
                    // Check if we should enter
                    if self.should_enter(&signal, &params) {
                        if positions.len() < 5 { // Max 5 positions
                            let position = self.create_position(signal, &params, capital)?;
                            positions.insert(signal.token_mint.to_string(), position.clone());
                            capital -= position.size as f64 / 1_000_000_000.0; // Deduct from capital
                        }
                    }
                }
            }

            // Update positions
            let mut positions_to_close = Vec::new();
            for (token_mint, mut position) in positions.iter_mut() {
                if let Some(current_price) = price_data.get(&current_time) {
                    position.current_price = *current_price;
                    position.pnl_percentage = ((current_price - position.entry_price) / position.entry_price) * 100.0;

                    if current_price > position.highest_price {
                        position.highest_price = *current_price;
                    }

                    // Check exit conditions
                    if let Some(exit_reason) = self.check_exit_conditions(&position, &params) {
                        positions_to_close.push((token_mint.clone(), position.clone(), exit_reason));
                    }
                }
            }

            // Close positions
            for (token_mint, position, exit_reason) in positions_to_close {
                let pnl = position.pnl_percentage / 100.0 * (position.size as f64 / 1_000_000_000.0);
                capital += position.size as f64 / 1_000_000_000.0 + pnl;

                if pnl > 0.0 {
                    winning_trades += 1;
                } else {
                    losing_trades += 1;
                }

                trades.push(BacktestTrade {
                    token_mint: token_mint.clone(),
                    entry_time: position.entry_time,
                    exit_time: current_time,
                    entry_price: position.entry_price,
                    exit_price: position.current_price,
                    pnl,
                    pnl_percentage: position.pnl_percentage,
                    holding_time_hours: (current_time - position.entry_time).num_hours() as f64,
                    exit_reason,
                });

                positions.remove(&token_mint);
            }

            // Track equity
            equity_curve.push((current_time, capital));

            // Calculate drawdown
            if capital > max_capital {
                max_capital = capital;
            }
            let drawdown = (max_capital - capital) / max_capital * 100.0;
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }

            current_time = current_time + Duration::hours(1);
        }

        // Close remaining positions at end
        for (token_mint, position) in positions {
            let pnl = position.pnl_percentage / 100.0 * (position.size as f64 / 1_000_000_000.0);
            capital += position.size as f64 / 1_000_000_000.0 + pnl;

            if pnl > 0.0 {
                winning_trades += 1;
            } else {
                losing_trades += 1;
            }

            trades.push(BacktestTrade {
                token_mint,
                entry_time: position.entry_time,
                exit_time: end_date,
                entry_price: position.entry_price,
                exit_price: position.current_price,
                pnl,
                pnl_percentage: position.pnl_percentage,
                holding_time_hours: (end_date - position.entry_time).num_hours() as f64,
                exit_reason: "End of backtest".to_string(),
            });
        }

        let total_trades = winning_trades + losing_trades;
        let win_rate = if total_trades > 0 {
            winning_trades as f64 / total_trades as f64 * 100.0
        } else {
            0.0
        };

        let total_return = ((capital - params.initial_capital) / params.initial_capital) * 100.0;

        let sharpe_ratio = self.calculate_sharpe_ratio(&equity_curve);

        let avg_holding_time = if !trades.is_empty() {
            trades.iter().map(|t| t.holding_time_hours).sum::<f64>() / trades.len() as f64
        } else {
            0.0
        };

        Ok(BacktestResult {
            strategy_name: strategy_name.to_string(),
            start_date,
            end_date,
            initial_capital: params.initial_capital,
            final_capital: capital,
            total_return,
            total_trades,
            winning_trades,
            losing_trades,
            win_rate,
            max_drawdown,
            sharpe_ratio,
            sortino_ratio: None,
            avg_holding_time_hours: avg_holding_time,
            equity_curve,
            trades,
        })
    }

    fn should_enter(&self, signal: &TradeSignal, params: &StrategyParameters) -> bool {
        // In production, would check sentiment analysis
        // For now, assume all signals are valid
        true
    }

    fn create_position(&self, signal: &TradeSignal, params: &StrategyParameters, capital: f64) -> Result<Position> {
        let position_size = (params.max_position_size as f64 * 0.2) as u64; // 20% of max per trade
        let entry_price = 1.0; // Mock price

        Ok(Position {
            token_mint: signal.token_mint,
            entry_price,
            current_price: entry_price,
            size: position_size,
            pnl_percentage: 0.0,
            highest_price: entry_price,
            stop_loss_price: entry_price * (1.0 - params.initial_stop_loss_percentage / 100.0),
            take_profit_price: entry_price * (1.0 + params.take_profit_percentage / 100.0),
            entry_time: signal.timestamp,
        })
    }

    fn check_exit_conditions(&self, position: &Position, params: &StrategyParameters) -> Option<String> {
        if position.pnl_percentage >= params.take_profit_percentage {
            return Some("Take Profit".to_string());
        }

        if position.pnl_percentage <= -params.initial_stop_loss_percentage {
            return Some("Stop Loss".to_string());
        }

        if position.pnl_percentage > 0.0 {
            let trailing_stop = position.highest_price * (1.0 - params.trailing_stop_percentage / 100.0);
            if position.current_price <= trailing_stop {
                return Some("Trailing Stop".to_string());
            }
        }

        None
    }

    fn calculate_sharpe_ratio(&self, equity_curve: &[(chrono::DateTime<Utc>, f64)]) -> Option<f64> {
        if equity_curve.len() < 2 {
            return None;
        }

        let returns: Vec<f64> = equity_curve
            .windows(2)
            .map(|w| (w[1].1 - w[0].1) / w[0].1)
            .collect();

        if returns.is_empty() {
            return None;
        }

        let avg_return = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance = returns.iter().map(|r| (r - avg_return).powi(2)).sum::<f64>() / returns.len() as f64;
        let std_dev = variance.sqrt();

        if std_dev == 0.0 {
            return None;
        }

        // Assume risk-free rate of 0% for simplicity
        Some(avg_return / std_dev * (365.0_f64).sqrt()) // Annualized
    }

    async fn generate_mock_price_data(
        &self,
        start: chrono::DateTime<Utc>,
        end: chrono::DateTime<Utc>,
    ) -> Result<HashMap<chrono::DateTime<Utc>, f64>> {
        let mut data = HashMap::new();
        let mut current = start;
        let mut price = 1.0;

        while current < end {
            // Random walk
            price *= 1.0 + (rand::random::<f64>() - 0.5) * 0.1;
            price = price.max(0.01); // Minimum price
            data.insert(current, price);
            current = current + Duration::hours(1);
        }

        Ok(data)
    }

    async fn generate_mock_signals(
        &self,
        start: chrono::DateTime<Utc>,
        end: chrono::DateTime<Utc>,
    ) -> Result<Vec<TradeSignal>> {
        let mut signals = Vec::new();
        let mut current = start;

        while current < end {
            // Generate random signals
            if rand::random::<f64>() < 0.1 { // 10% chance per hour
                signals.push(TradeSignal {
                    wallet: solana_sdk::pubkey::Pubkey::new_unique(),
                    token_mint: solana_sdk::pubkey::Pubkey::new_unique(),
                    action: TradeAction::Buy,
                    amount: 1_000_000,
                    timestamp: current,
                    signature: format!("sig_{}", current.timestamp()),
                });
            }
            current = current + Duration::hours(1);
        }

        Ok(signals)
    }
}
