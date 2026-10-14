use anyhow::Result;
use dashmap::DashMap;
use redis::AsyncCommands;
use shared::{SmartMoneyWallet, TradeAction, TradeSignal};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcTransactionConfig, RpcTransactionLogsFilter, RpcTransactionLogsConfig};
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use std::str::FromStr;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,smart_money_radar=debug")
        .init();

    info!("Starting Smart Money Radar...");

    let rpc_url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| "https://mainnet.helius-rpc.com/?api-key=YOUR_API_KEY".to_string());
    
    let client = RpcClient::new(rpc_url);
    let redis_client = redis::Client::open("redis://127.0.0.1/")?;
    let mut con = redis_client.get_async_connection().await?;

    let radar = SmartMoneyRadar::new(client, Arc::new(DashMap::new())).await?;

    let mut interval = interval(Duration::from_millis(500));

    loop {
        interval.tick().await;
        
        match radar.scan_smart_money_transactions().await {
            Ok(signals) => {
                for signal in signals {
                    info!("Smart money signal detected: {:?}", signal);
                    
                    let signal_json = serde_json::to_string(&signal)?;
                    let _: () = con.publish("smart_money_signals", signal_json).await?;
                }
            }
            Err(e) => {
                error!("Error scanning transactions: {}", e);
            }
        }
    }
}

pub struct SmartMoneyRadar {
    client: RpcClient,
    tracked_wallets: Arc<DashMap<Pubkey, SmartMoneyWallet>>,
    pump_program_id: Pubkey,
}

impl SmartMoneyRadar {
    pub async fn new(client: RpcClient, tracked_wallets: Arc<DashMap<Pubkey, SmartMoneyWallet>>) -> Result<Self> {
        let pump_program_id = Pubkey::from_str("6EF8rrecthR5Dkjon8nkgcFkXL5Y2mM5SqFVvG3dZq")?;

        let mut radar = Self {
            client,
            tracked_wallets,
            pump_program_id,
        };

        radar.load_smart_money_wallets().await?;
        
        Ok(radar)
    }

    async fn load_smart_money_wallets(&mut self) -> Result<()> {
        info!("Loading smart money wallets...");
        
        let sample_wallets = vec![
            ("DjxX4M7iYFg5a7v8c9d0e1f2g3h4i5j6k7l8m9n0", 0.75, 150, 2.5),
            ("EjyY5N8jZh6b8c9d0e1f2g3h4i5j6k7l8m9n0o1", 0.68, 200, 1.8),
        ];

        for (address, win_rate, trades, profit_factor) in sample_wallets {
            if let Ok(pubkey) = Pubkey::from_str(address) {
                let wallet = SmartMoneyWallet {
                    address: pubkey,
                    win_rate,
                    total_trades: trades,
                    profit_factor,
                    last_active: chrono::Utc::now(),
                    tags: vec!["whale".to_string(), "early_mover".to_string()],
                };
                self.tracked_wallets.insert(pubkey, wallet);
                info!("Loaded smart money wallet: {}", address);
            }
        }

        Ok(())
    }

    pub async fn scan_smart_money_transactions(&self) -> Result<Vec<TradeSignal>> {
        let mut signals = Vec::new();
        
        for wallet_entry in self.tracked_wallets.iter() {
            let wallet = wallet_entry.value();
            
            match self.get_recent_transactions(&wallet.address).await {
                Ok(transactions) => {
                    for tx in transactions {
                        if let Some(signal) = self.analyze_transaction(&wallet.address, tx).await? {
                            signals.push(signal);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to get transactions for {}: {}", wallet.address, e);
                }
            }
        }

        Ok(signals)
    }

    async fn get_recent_transactions(&self, address: &Pubkey) -> Result<Vec<EncodedConfirmedTransactionWithStatusMeta>> {
        let signatures = self.client.get_signatures_for_address_with_config(
            address,
            solana_client::rpc_config::RpcSignaturesForAddressConfig {
                limit: Some(5),
                ..Default::default()
            },
        )?;

        let mut transactions = Vec::new();
        for sig_info in signatures {
            if let Ok(tx) = self.client.get_transaction_with_config(
                &sig_info.signature,
                RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::Json),
                    commitment: Some(solana_sdk::commitment_config::CommitmentConfig::confirmed()),
                    ..Default::default()
                },
            ) {
                transactions.push(tx);
            }
        }

        Ok(transactions)
    }

    async fn analyze_transaction(
        &self,
        wallet: &Pubkey,
        tx: EncodedConfirmedTransactionWithStatusMeta,
    ) -> Result<Option<TradeSignal>> {
        let transaction = tx.transaction.transaction.decode();
        
        if let Some(transaction) = transaction {
            for instruction in transaction.message.instructions {
                let program_id = transaction.message.account_keys[instruction.program_id_index as usize];
                
                if program_id == self.pump_program_id {
                    let action = self.determine_trade_action(&instruction.data)?;
                    
                    let token_mint = self.extract_token_mint(&transaction, &instruction)?;
                    
                    return Ok(Some(TradeSignal {
                        wallet: *wallet,
                        token_mint,
                        action,
                        amount: 1_000_000, // Simplified - would need actual amount parsing
                        timestamp: chrono::Utc::now(),
                        signature: tx.transaction.transaction.signatures[0].to_string(),
                    }));
                }
            }
        }

        Ok(None)
    }

    fn determine_trade_action(&self, instruction_data: &[u8]) -> Result<TradeAction> {
        if instruction_data.is_empty() {
            return Ok(TradeAction::Buy);
        }
        
        let instruction_type = instruction_data[0];
        match instruction_type {
            0 => Ok(TradeAction::Buy),
            1 => Ok(TradeAction::Sell),
            _ => Ok(TradeAction::Buy),
        }
    }

    fn extract_token_mint(&self, _transaction: &solana_sdk::transaction::Transaction, _instruction: &solana_sdk::instruction::Instruction) -> Result<Pubkey> {
        Ok(Pubkey::from_str("So11111111111111111111111111111111111111112")?)
    }
}
