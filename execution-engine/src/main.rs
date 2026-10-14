use anyhow::Result;
use anchor_client::solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use anchor_client::{Client, Cluster};
use redis::AsyncCommands;
use shared::{RiskAction, Sentiment, TradingOpportunity};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,execution_engine=debug")
        .init();

    let paper_trading = std::env::var("PAPER_TRADING")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase() == "true";

    info!("Starting Execution Engine...");
    info!("Paper Trading Mode: {}", paper_trading);

    let wallet_keypair = load_wallet_keypair()?;
    info!("Loaded wallet: {}", wallet_keypair.pubkey());

    let client = Client::new_with_options(
        Cluster::Custom(
            "https://mainnet.helius-rpc.com/?api-key=YOUR_API_KEY".to_string(),
            "wss://mainnet.helius-rpc.com/?api-key=YOUR_API_KEY".to_string(),
        ),
        wallet_keypair.clone(),
        CommitmentConfig::confirmed(),
    );

    let redis_client = redis::Client::open("redis://127.0.0.1/")?;
    let mut con = redis_client.get_async_connection().await?;

    let paper_portfolio = Arc::new(RwLock::new(PaperPortfolio::new(10.0))); // 10 SOL paper capital

    let executor = ExecutionEngine::new(client, wallet_keypair, paper_trading, paper_portfolio);

    let mut pubsub = con.as_pubsub();
    pubsub.subscribe("trading_opportunities").await?;

    info!("Listening for trading opportunities...");

    loop {
        match pubsub.next_message().await {
            Ok(msg) => {
                if let Some(data) = msg.get_payload() {
                    match serde_json::from_str::<TradingOpportunity>(&data) {
                        Ok(opportunity) => {
                            info!("Received trading opportunity: {}", opportunity.token_mint);
                            
                            if opportunity.combined_score > 0.7 {
                                match executor.execute_trade(&opportunity).await {
                                    Ok(signature) => {
                                        info!("Trade executed successfully: {}", signature);
                                        if paper_trading {
                                            let portfolio = executor.paper_portfolio.read().await;
                                            info!("Paper Portfolio Balance: {} SOL", portfolio.balance);
                                        }
                                    }
                                    Err(e) => {
                                        error!("Trade execution failed: {}", e);
                                    }
                                }
                            } else {
                                info!("Skipping trade - score too low: {}", opportunity.combined_score);
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse opportunity: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Redis error: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

#[derive(Debug, Clone)]
struct PaperPosition {
    token_mint: Pubkey,
    entry_price: f64,
    size: f64,
    entry_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
struct PaperPortfolio {
    balance: f64,
    positions: Vec<PaperPosition>,
}

impl PaperPortfolio {
    fn new(initial_balance: f64) -> Self {
        Self {
            balance: initial_balance,
            positions: Vec::new(),
        }
    }

    fn add_position(&mut self, position: PaperPosition) {
        self.positions.push(position);
    }

    fn close_position(&mut self, token_mint: &Pubkey, exit_price: f64) -> Option<f64> {
        if let Some(pos) = self.positions.iter().position(|p| p.token_mint == *token_mint) {
            let position = self.positions.remove(pos);
            let pnl = (exit_price - position.entry_price) / position.entry_price * position.size;
            self.balance += position.size + pnl;
            return Some(pnl);
        }
        None
    }
}

pub struct ExecutionEngine {
    client: Client<Keypair>,
    wallet: Keypair,
    pump_program_id: Pubkey,
    paper_trading: bool,
    paper_portfolio: Arc<RwLock<PaperPortfolio>>,
}

impl ExecutionEngine {
    pub fn new(
        client: Client<Keypair>,
        wallet: Keypair,
        paper_trading: bool,
        paper_portfolio: Arc<RwLock<PaperPortfolio>>,
    ) -> Self {
        let pump_program_id = Pubkey::from_str("6EF8rrecthR5Dkjon8nkgcFkXL5Y2mM5SqFVvG3dZq").unwrap();
        
        Self {
            client,
            wallet,
            pump_program_id,
            paper_trading,
            paper_portfolio,
        }
    }

    pub async fn execute_trade(&self, opportunity: &TradingOpportunity) -> Result<String> {
        info!("Executing trade for token: {}", opportunity.token_mint);
        
        if self.paper_trading {
            return self.execute_paper_trade(opportunity).await;
        }
        
        let amount = 0_1_000_000; // 0.1 SOL in lamports
        let slippage = 100; // 1% slippage (100 basis points)
        
        let priority_fee = self.calculate_dynamic_priority_fee().await?;
        
        let mut instructions = vec![
            ComputeBudgetInstruction::set_compute_unit_limit(200_000),
            ComputeBudgetInstruction::set_compute_unit_price(priority_fee),
        ];

        let buy_instruction = self.create_buy_instruction(
            &opportunity.token_mint,
            amount,
            slippage,
        )?;

        instructions.push(buy_instruction);

        let recent_blockhash = self.client
            .rpc()
            .get_latest_blockhash()
            .map_err(|e| anyhow::anyhow!("Failed to get blockhash: {}", e))?;

        let mut transaction = Transaction::new_with_payer(&instructions, Some(&self.wallet.pubkey()));
        transaction.sign(&[&self.wallet], recent_blockhash);

        let signature = timeout(Duration::from_secs(10), async {
            self.client
                .rpc()
                .send_transaction_with_config(
                    &transaction,
                    solana_sdk::commitment_config::RpcSendTransactionConfig {
                        skip_preflight: true,
                        ..Default::default()
                    },
                )
        })
        .await
        .map_err(|_| anyhow::anyhow!("Transaction timeout"))?
        .map_err(|e| anyhow::anyhow!("Failed to send transaction: {}", e))?;

        info!("Transaction sent: {}", signature);
        
        Ok(signature.to_string())
    }

    async fn execute_paper_trade(&self, opportunity: &TradingOpportunity) -> Result<String> {
        info!("Paper trading: Simulating buy for {}", opportunity.token_mint);
        
        let trade_size = 0.1; // 0.1 SOL
        let entry_price = 1.0; // Mock price
        
        let mut portfolio = self.paper_portfolio.write().await;
        
        if portfolio.balance < trade_size {
            warn!("Insufficient paper balance: {} SOL", portfolio.balance);
            return Ok("insufficient_balance".to_string());
        }
        
        portfolio.balance -= trade_size;
        
        let position = PaperPosition {
            token_mint: opportunity.token_mint,
            entry_price,
            size: trade_size,
            entry_time: chrono::Utc::now(),
        };
        
        portfolio.add_position(position);
        
        let signature = format!("paper_{}", chrono::Utc::now().timestamp());
        info!("Paper trade executed: {}", signature);
        
        Ok(signature)
    }

    async fn calculate_dynamic_priority_fee(&self) -> Result<u64> {
        let base_fee = 1_000; // 0.000001 SOL per compute unit
        
        let network_priority = self.get_network_priority_fee().await.unwrap_or(1_000);
        
        let dynamic_fee = base_fee.saturating_add(network_priority);
        
        Ok(dynamic_fee)
    }

    async fn get_network_priority_fee(&self) -> Result<u64> {
        Ok(500) // Simplified - would query actual network fees
    }

    fn create_buy_instruction(
        &self,
        token_mint: &Pubkey,
        amount: u64,
        slippage: u16,
    ) -> Result<Instruction> {
        let program_id = self.pump_program_id;
        
        let accounts = vec![
            solana_sdk::instruction::AccountMeta::new(self.wallet.pubkey(), true),
            solana_sdk::instruction::AccountMeta::new(*token_mint, false),
            solana_sdk::instruction::AccountMeta::new_readonly(
                Pubkey::from_str("So11111111111111111111111111111111111111112")?,
                false,
            ),
        ];

        let data = vec![0]; // Instruction 0 = Buy

        Ok(Instruction {
            program_id,
            accounts,
            data,
        })
    }

    pub async fn execute_sell(
        &self,
        token_mint: &Pubkey,
        amount: u64,
        slippage: u16,
    ) -> Result<String> {
        info!("Executing sell for token: {}", token_mint);
        
        let priority_fee = self.calculate_dynamic_priority_fee().await?;
        
        let mut instructions = vec![
            ComputeBudgetInstruction::set_compute_unit_limit(200_000),
            ComputeBudgetInstruction::set_compute_unit_price(priority_fee),
        ];

        let sell_instruction = self.create_sell_instruction(token_mint, amount, slippage)?;
        instructions.push(sell_instruction);

        let recent_blockhash = self.client
            .rpc()
            .get_latest_blockhash()
            .map_err(|e| anyhow::anyhow!("Failed to get blockhash: {}", e))?;

        let mut transaction = Transaction::new_with_payer(&instructions, Some(&self.wallet.pubkey()));
        transaction.sign(&[&self.wallet], recent_blockhash);

        let signature = self.client
            .rpc()
            .send_transaction_with_config(
                &transaction,
                solana_sdk::commitment_config::RpcSendTransactionConfig {
                    skip_preflight: true,
                    ..Default::default()
                },
            )
            .map_err(|e| anyhow::anyhow!("Failed to send transaction: {}", e))?;

        Ok(signature.to_string())
    }

    fn create_sell_instruction(
        &self,
        token_mint: &Pubkey,
        amount: u64,
        slippage: u16,
    ) -> Result<Instruction> {
        let program_id = self.pump_program_id;
        
        let accounts = vec![
            solana_sdk::instruction::AccountMeta::new(self.wallet.pubkey(), true),
            solana_sdk::instruction::AccountMeta::new(*token_mint, false),
            solana_sdk::instruction::AccountMeta::new_readonly(
                Pubkey::from_str("So11111111111111111111111111111111111111112")?,
                false,
            ),
        ];

        let data = vec![1]; // Instruction 1 = Sell

        Ok(Instruction {
            program_id,
            accounts,
            data,
        })
    }
}

fn load_wallet_keypair() -> Result<Keypair> {
    let wallet_path = std::env::var("WALLET_PATH")
        .unwrap_or_else(|_| "config/wallet.json".to_string());
    
    if std::path::Path::new(&wallet_path).exists() {
        let keypair_bytes = std::fs::read(&wallet_path)?;
        let keypair: Keypair = serde_json::from_slice(&keypair_bytes)?;
        Ok(keypair)
    } else {
        warn!("Wallet file not found at {}, generating new wallet", wallet_path);
        
        std::fs::create_dir_all("config")?;
        let keypair = Keypair::new();
        let keypair_bytes = serde_json::to_vec(&keypair)?;
        std::fs::write(&wallet_path, keypair_bytes)?;
        
        info!("Generated new wallet: {}", keypair.pubkey());
        info!("SAVE THIS PRIVATE KEY SECURELY!");
        
        Ok(keypair)
    }
}
