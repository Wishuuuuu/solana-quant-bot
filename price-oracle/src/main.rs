use anyhow::Result;
use dashmap::DashMap;
use jupiter_integration::JupiterClient;
use redis::AsyncCommands;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, sleep};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,price_oracle=debug")
        .init();

    info!("Starting Price Oracle Service...");

    let jupiter_client = JupiterClient::new();
    let redis_client = redis::Client::open("redis://127.0.0.1/")?;
    let mut con = redis_client.get_async_connection().await?;

    let price_cache = Arc::new(DashMap::new());
    let oracle = PriceOracle::new(jupiter_client, price_cache);

    let mut update_interval = interval(Duration::from_secs(5));

    // SOL token mint
    let sol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112")?;
    let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?;

    loop {
        update_interval.tick().await;

        // Update SOL price
        match oracle.get_price(&sol_mint, &usdc_mint).await {
            Ok(price) => {
                info!("SOL/USDC price: {}", price);
                
                let price_data = serde_json::json!({
                    "token": "SOL",
                    "price": price,
                    "timestamp": chrono::Utc::now()
                });
                
                let _: () = con.publish("price_updates", price_data.to_string()).await?;
                let _: () = con.set("price:SOL", price).await?;
                let _: () = con.expire("price:SOL", 60).await?;
            }
            Err(e) => {
                error!("Failed to get SOL price: {}", e);
            }
        }

        // Track price for any active positions
        if let Ok(keys) = con.keys("position:*").await {
            for key in keys {
                if let Ok(position_data) = con.get::<_, String>(&key).await {
                    if let Ok(position) = serde_json::from_str::<serde_json::Value>(&position_data) {
                        if let Some(token_mint_str) = position.get("token_mint").and_then(|v| v.as_str()) {
                            if let Ok(token_mint) = Pubkey::from_str(token_mint_str) {
                                match oracle.get_price(&token_mint, &sol_mint).await {
                                    Ok(price) => {
                                        let price_key = format!("price:{}", token_mint_str);
                                        let _: () = con.set(&price_key, price).await?;
                                        let _: () = con.expire(&price_key, 60).await?;
                                    }
                                    Err(e) => {
                                        warn!("Failed to get price for {}: {}", token_mint_str, e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub struct PriceOracle {
    jupiter_client: JupiterClient,
    price_cache: Arc<DashMap<String, f64>>,
}

impl PriceOracle {
    pub fn new(jupiter_client: JupiterClient, price_cache: Arc<DashMap<String, f64>>) -> Self {
        Self {
            jupiter_client,
            price_cache,
        }
    }

    pub async fn get_price(&self, input_mint: &Pubkey, output_mint: &Pubkey) -> Result<f64> {
        let cache_key = format!("{}:{}", input_mint, output_mint);
        
        // Check cache first
        if let Some(cached_price) = self.price_cache.get(&cache_key) {
            return Ok(*cached_price);
        }

        // Fetch from Jupiter
        let amount = 1_000_000; // 1 unit for price calculation
        let price = self.jupiter_client.get_best_price(input_mint, output_mint, amount).await?;

        // Cache the price
        self.price_cache.insert(cache_key, price);

        Ok(price)
    }

    pub async fn get_token_price_usdc(&self, token_mint: &Pubkey) -> Result<f64> {
        let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?;
        self.get_price(token_mint, &usdc_mint).await
    }

    pub async fn get_token_price_sol(&self, token_mint: &Pubkey) -> Result<f64> {
        let sol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112")?;
        self.get_price(token_mint, &sol_mint).await
    }

    pub async fn watch_price(&self, token_mint: &Pubkey, quote_mint: &Pubkey) -> Result<f64> {
        let mut last_price = self.get_price(token_mint, quote_mint).await?;
        
        loop {
            sleep(Duration::from_secs(1)).await;
            
            match self.get_price(token_mint, quote_mint).await {
                Ok(current_price) => {
                    let change_percent = ((current_price - last_price) / last_price) * 100.0;
                    
                    if change_percent.abs() > 5.0 {
                        info!("Price alert: {} changed by {:.2}%", token_mint, change_percent);
                    }
                    
                    last_price = current_price;
                }
                Err(e) => {
                    error!("Error watching price: {}", e);
                }
            }
        }
    }
}
