use anyhow::Result;
use chrono::{Duration, Utc};
use redis::AsyncCommands;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,auto_balance=debug")
        .init();

    info!("Starting Auto-Balance Management System...");

    let rpc_url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| "https://mainnet.helius-rpc.com/?api-key=YOUR_API_KEY".to_string());
    
    let client = RpcClient::new(rpc_url);
    
    let wallet_address = std::env::var("WALLET_ADDRESS")
        .unwrap_or_else(|_| "YOUR_WALLET_ADDRESS".to_string());
    
    let wallet_pubkey = Pubkey::from_str(&wallet_address)?;
    
    let redis_client = redis::Client::open("redis://127.0.0.1/")?;
    let mut con = redis_client.get_async_connection().await?;

    let balance_manager = AutoBalanceManager::new(client, wallet_pubkey);

    balance_manager.run_balance_loop(con).await?;

    Ok(())
}

pub struct AutoBalanceManager {
    client: RpcClient,
    wallet: Pubkey,
    min_sol_balance: u64,
    target_sol_balance: u64,
    max_sol_balance: u64,
    check_interval: Duration,
}

impl AutoBalanceManager {
    pub fn new(client: RpcClient, wallet: Pubkey) -> Self {
        Self {
            client,
            wallet,
            min_sol_balance: 0_1_000_000_000, // 0.1 SOL minimum
            target_sol_balance: 0_5_000_000_000, // 0.5 SOL target
            max_sol_balance: 2_000_000_000, // 2 SOL maximum
            check_interval: Duration::minutes(10),
        }
    }

    pub async fn run_balance_loop(&self, mut con: redis::aio::Connection) -> Result<()> {
        info!("Starting auto-balance loop (interval: {:?})", self.check_interval);

        loop {
            match self.check_and_manage_balance(&mut con).await {
                Ok(action) => {
                    if action != BalanceAction::None {
                        info!("Balance action taken: {:?}", action);
                    }
                }
                Err(e) => {
                    warn!("Balance check failed: {}", e);
                }
            }

            tokio::time::sleep(self.check_interval.to_std().unwrap()).await;
        }
    }

    async fn check_and_manage_balance(&self, con: &mut redis::aio::Connection) -> Result<BalanceAction> {
        let balance = self.get_sol_balance().await?;
        let balance_sol = balance as f64 / 1_000_000_000.0;

        info!("Current SOL balance: {:.4}", balance_sol);

        let action = if balance < self.min_sol_balance {
            // Balance too low - alert and pause trading
            warn!("SOL balance too low: {:.4} SOL (min: {:.4} SOL)", 
                balance_sol, self.min_sol_balance as f64 / 1_000_000_000.0);
            
            self.send_alert(con, "LOW_BALANCE", &format!("SOL balance too low: {:.4} SOL", balance_sol)).await?;
            
            // Pause trading
            self.pause_trading(con).await?;
            
            BalanceAction::LowBalance
        } else if balance < self.target_sol_balance {
            // Balance below target - alert but continue
            info!("SOL balance below target: {:.4} SOL (target: {:.4} SOL)", 
                balance_sol, self.target_sol_balance as f64 / 1_000_000_000.0);
            
            self.send_alert(con, "BELOW_TARGET", &format!("SOL balance below target: {:.4} SOL", balance_sol)).await?;
            
            BalanceAction::BelowTarget
        } else if balance > self.max_sol_balance {
            // Balance too high - consider withdrawing excess
            info!("SOL balance above maximum: {:.4} SOL (max: {:.4} SOL)", 
                balance_sol, self.max_sol_balance as f64 / 1_000_000_000.0);
            
            self.send_alert(con, "HIGH_BALANCE", &format!("SOL balance above maximum: {:.4} SOL", balance_sol)).await?;
            
            BalanceAction::HighBalance
        } else {
            // Balance is healthy
            BalanceAction::None
        };

        // Update balance in Redis
        let _: () = con.set("wallet_balance", balance_sol).await?;
        let _: () = con.expire("wallet_balance", 3600).await?;

        Ok(action)
    }

    async fn get_sol_balance(&self) -> Result<u64> {
        let balance = self.client.get_balance(&self.wallet)?;
        Ok(balance)
    }

    async fn send_alert(&self, con: &mut redis::aio::Connection, alert_type: &str, message: &str) -> Result<()> {
        let alert = serde_json::json!({
            "type": alert_type,
            "message": message,
            "timestamp": Utc::now()
        });

        let _: () = con.publish("balance_alerts", alert.to_string()).await?;
        Ok(())
    }

    async fn pause_trading(&self, con: &mut redis::aio::Connection) -> Result<()> {
        let _: () = con.set("trading_paused", "true").await?;
        let _: () = con.expire("trading_paused", 3600).await?;
        
        info!("Trading paused due to low balance");
        Ok(())
    }

    async fn resume_trading(&self, con: &mut redis::aio::Connection) -> Result<()> {
        let _: () = con.del("trading_paused").await?;
        
        info!("Trading resumed");
        Ok(())
    }

    pub async fn calculate_optimal_position_size(&self, con: &mut redis::aio::Connection) -> Result<f64> {
        let balance = self.get_sol_balance().await?;
        let balance_sol = balance as f64 / 1_000_000_000.0;

        // Reserve 20% for gas fees
        let available_for_trading = balance_sol * 0.8;

        // Get number of open positions
        let open_positions: i32 = con.get("open_positions_count").await.unwrap_or(0);
        
        // Divide available balance among positions (max 5)
        let max_positions = 5;
        let position_count = open_positions.min(max_positions) as f64;
        
        let position_size = if position_count > 0.0 {
            available_for_trading / position_count
        } else {
            available_for_trading / 5.0 // Default to 1/5 of available
        };

        // Cap at 20% of total balance per position
        let max_position_size = balance_sol * 0.2;
        
        Ok(position_size.min(max_position_size))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BalanceAction {
    None,
    LowBalance,
    BelowTarget,
    HighBalance,
}
