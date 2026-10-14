use anyhow::Result;
use chrono::{Duration, Utc};
use dashmap::DashMap;
use redis::AsyncCommands;
use shared::{Position, RiskAction, RiskDecision, Sentiment};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,risk_manager=debug")
        .init();

    info!("Starting Risk Manager...");

    let rpc_url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| "https://mainnet.helius-rpc.com/?api-key=YOUR_API_KEY".to_string());
    
    let client = RpcClient::new(rpc_url);
    let redis_client = redis::Client::open("redis://127.0.0.1/")?;
    let mut con = redis_client.get_async_connection().await?;

    let positions = Arc::new(DashMap::new());
    let risk_manager = RiskManager::new(client, positions.clone());

    let mut check_interval = interval(TokioDuration::from_secs(5));

    loop {
        check_interval.tick().await;
        
        if let Err(e) = risk_manager.check_all_positions().await {
            error!("Error checking positions: {}", e);
        }
    }
}

pub struct RiskManager {
    client: RpcClient,
    positions: Arc<DashMap<Pubkey, Position>>,
    config: RiskConfig,
}

#[derive(Clone)]
struct RiskConfig {
    take_profit_percentage: f64,
    initial_stop_loss_percentage: f64,
    trailing_stop_percentage: f64,
    max_position_size: u64,
    max_open_positions: usize,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            take_profit_percentage: 100.0, // 100% profit
            initial_stop_loss_percentage: 30.0, // 30% loss
            trailing_stop_percentage: 15.0, // 15% from ATH
            max_position_size: 1_000_000_000, // 1 SOL
            max_open_positions: 5,
        }
    }
}

impl RiskManager {
    pub fn new(client: RpcClient, positions: Arc<DashMap<Pubkey, Position>>) -> Self {
        Self {
            client,
            positions,
            config: RiskConfig::default(),
        }
    }

    pub async fn check_all_positions(&self) -> Result<()> {
        let mut positions_to_close = Vec::new();
        
        for mut entry in self.positions.iter_mut() {
            let token_mint = *entry.key();
            let mut position = entry.value().clone();
            
            match self.evaluate_position(&mut position).await {
                Ok(decision) => {
                    match decision.action {
                        RiskAction::TakeProfit => {
                            info!("Take profit triggered for {}: {}%", token_mint, position.pnl_percentage);
                            positions_to_close.push((token_mint, decision));
                        }
                        RiskAction::StopLoss => {
                            warn!("Stop loss triggered for {}: {}%", token_mint, position.pnl_percentage);
                            positions_to_close.push((token_mint, decision));
                        }
                        RiskAction::TrailingStop => {
                            info!("Trailing stop updated for {}: new stop at {}", token_mint, decision.new_stop_loss.unwrap_or(0.0));
                            position.stop_loss_price = decision.new_stop_loss.unwrap_or(position.stop_loss_price);
                            *entry.value_mut() = position;
                        }
                        RiskAction::EnterFull | RiskAction::EnterHalf | RiskAction::Skip => {
                            // Not applicable for existing positions
                        }
                    }
                }
                Err(e) => {
                    error!("Error evaluating position {}: {}", token_mint, e);
                }
            }
        }

        for (token_mint, decision) in positions_to_close {
            self.execute_exit_decision(token_mint, decision).await?;
        }

        Ok(())
    }

    pub async fn evaluate_position(&self, position: &mut Position) -> Result<RiskDecision> {
        let current_price = self.get_current_price(&position.token_mint).await?;
        position.current_price = current_price;
        
        let pnl_percentage = ((current_price - position.entry_price) / position.entry_price) * 100.0;
        position.pnl_percentage = pnl_percentage;

        if current_price > position.highest_price {
            position.highest_price = current_price;
        }

        if pnl_percentage >= self.config.take_profit_percentage {
            return Ok(RiskDecision {
                action: RiskAction::TakeProfit,
                reason: format!("Take profit target reached: {:.2}%", pnl_percentage),
                new_stop_loss: None,
                take_profit_percentage: Some(self.config.take_profit_percentage),
            });
        }

        if pnl_percentage <= -self.config.initial_stop_loss_percentage {
            return Ok(RiskDecision {
                action: RiskAction::StopLoss,
                reason: format!("Stop loss triggered: {:.2}%", pnl_percentage),
                new_stop_loss: None,
                take_profit_percentage: None,
            });
        }

        if pnl_percentage > 0.0 {
            let trailing_stop_price = position.highest_price * (1.0 - self.config.trailing_stop_percentage / 100.0);
            
            if current_price <= trailing_stop_price {
                return Ok(RiskDecision {
                    action: RiskAction::StopLoss,
                    reason: format!("Trailing stop triggered: {:.2}% from ATH", self.config.trailing_stop_percentage),
                    new_stop_loss: None,
                    take_profit_percentage: None,
                });
            }

            if trailing_stop_price > position.stop_loss_price {
                return Ok(RiskDecision {
                    action: RiskAction::TrailingStop,
                    reason: format("Updating trailing stop to {:.2}", trailing_stop_price),
                    new_stop_loss: Some(trailing_stop_price),
                    take_profit_percentage: None,
                });
            }
        }

        Ok(RiskDecision {
            action: RiskAction::Skip,
            reason: "No action needed".to_string(),
            new_stop_loss: None,
            take_profit_percentage: None,
        })
    }

    pub async fn evaluate_entry(&self, opportunity: &shared::TradingOpportunity) -> Result<RiskDecision> {
        if self.positions.len() >= self.config.max_open_positions {
            return Ok(RiskDecision {
                action: RiskAction::Skip,
                reason: "Maximum open positions reached".to_string(),
                new_stop_loss: None,
                take_profit_percentage: None,
            });
        }

        if opportunity.sentiment.hype_score < 0.6 {
            return Ok(RiskDecision {
                action: RiskAction::Skip,
                reason: "Hype score too low".to_string(),
                new_stop_loss: None,
                take_profit_percentage: None,
            });
        }

        if opportunity.sentiment.spam_score > 0.5 {
            return Ok(RiskDecision {
                action: RiskAction::Skip,
                reason: "High spam score detected".to_string(),
                new_stop_loss: None,
                take_profit_percentage: None,
            });
        }

        if opportunity.combined_score > 0.8 {
            Ok(RiskDecision {
                action: RiskAction::EnterFull,
                reason: "High confidence opportunity".to_string(),
                new_stop_loss: None,
                take_profit_percentage: Some(self.config.take_profit_percentage),
            })
        } else {
            Ok(RiskDecision {
                action: RiskAction::EnterHalf,
                reason: "Moderate confidence opportunity".to_string(),
                new_stop_loss: None,
                take_profit_percentage: Some(self.config.take_profit_percentage),
            })
        }
    }

    async fn get_current_price(&self, token_mint: &Pubkey) -> Result<f64> {
        // Simplified - would query Jupiter API or DEX
        Ok(1.0)
    }

    async fn execute_exit_decision(&self, token_mint: Pubkey, decision: RiskDecision) -> Result<()> {
        let redis_client = redis::Client::open("redis://127.0.0.1/")?;
        let mut con = redis_client.get_async_connection().await?;

        let exit_signal = serde_json::json!({
            "token_mint": token_mint.to_string(),
            "action": decision.action,
            "reason": decision.reason,
            "timestamp": Utc::now()
        });

        let _: () = con.publish("exit_signals", exit_signal.to_string()).await?;
        
        self.positions.remove(&token_mint);
        
        info!("Published exit signal for {}: {:?}", token_mint, decision.action);
        
        Ok(())
    }

    pub fn add_position(&self, position: Position) {
        self.positions.insert(position.token_mint, position);
        info!("Added position for token: {}", position.token_mint);
    }
}
