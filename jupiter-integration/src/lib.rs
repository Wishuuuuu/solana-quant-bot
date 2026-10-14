use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tracing::{debug, info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: String,
    pub slippage_bps: u16,
    pub only_direct_routes: bool,
    pub as_legacy_transaction: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub input_mint: String,
    pub in_amount: String,
    pub output_mint: String,
    pub out_amount: String,
    pub other_amount_threshold: String,
    pub swap_mode: String,
    pub slippage_bps: u16,
    pub route_plan: Vec<RoutePlan>,
    pub context_slot: u64,
    pub time_taken: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoutePlan {
    pub swap_info: SwapInfo,
    pub percent: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapInfo {
    pub amm_key: String,
    pub label: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub fee_amount: String,
    pub fee_mint: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapRequest {
    pub quote_response: QuoteResponse,
    pub user_public_key: String,
    pub wrap_and_unwrap_sol: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_compute_unit_limit: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapResponse {
    pub swap_transaction: String,
    pub last_valid_block_height: u64,
}

pub struct JupiterClient {
    client: Client,
    base_url: String,
}

impl JupiterClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://quote-api.jup.ag/v6".to_string(),
        }
    }

    pub fn with_base_url(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub async fn get_quote(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        amount: u64,
        slippage_bps: u16,
    ) -> Result<QuoteResponse> {
        let url = format!("{}/quote", self.base_url);
        
        let params = [
            ("inputMint", input_mint.to_string()),
            ("outputMint", output_mint.to_string()),
            ("amount", amount.to_string()),
            ("slippageBps", slippage_bps.to_string()),
            ("onlyDirectRoutes", "false"),
            ("asLegacyTransaction", "false"),
        ];

        debug!("Requesting Jupiter quote: {} -> {} (amount: {})", input_mint, output_mint, amount);

        let response = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Jupiter API error: {}", error_text));
        }

        let quote: QuoteResponse = response.json().await?;
        
        info!("Quote received: {} in -> {} out", quote.in_amount, quote.out_amount);
        
        Ok(quote)
    }

    pub async fn get_swap_transaction(
        &self,
        quote: &QuoteResponse,
        user_public_key: &Pubkey,
    ) -> Result<SwapResponse> {
        let url = format!("{}/swap", self.base_url);
        
        let swap_request = SwapRequest {
            quote_response: quote.clone(),
            user_public_key: user_public_key.to_string(),
            wrap_and_unwrap_sol: true,
            dynamic_compute_unit_limit: Some(true),
        };

        debug!("Requesting swap transaction for user: {}", user_public_key);

        let response = self
            .client
            .post(&url)
            .json(&swap_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Jupiter swap error: {}", error_text));
        }

        let swap_response: SwapResponse = response.json().await?;
        
        info!("Swap transaction received, valid until block: {}", swap_response.last_valid_block_height);
        
        Ok(swap_response)
    }

    pub async fn execute_swap(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        amount: u64,
        slippage_bps: u16,
        user_public_key: &Pubkey,
    ) -> Result<String> {
        let quote = self.get_quote(input_mint, output_mint, amount, slippage_bps).await?;
        
        let swap_response = self.get_swap_transaction(&quote, user_public_key).await?;
        
        Ok(swap_response.swap_transaction)
    }

    pub async fn get_best_price(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        amount: u64,
    ) -> Result<f64> {
        let quote = self.get_quote(input_mint, output_mint, amount, 100).await?;
        
        let in_amount: u64 = quote.in_amount.parse()?;
        let out_amount: u64 = quote.out_amount.parse()?;
        
        let price = out_amount as f64 / in_amount as f64;
        
        Ok(price)
    }
}

impl Default for JupiterClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_quote() {
        let client = JupiterClient::new();
        
        let sol = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let usdc = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        
        let result = client.get_quote(&sol, &usdc, 1_000_000, 100).await;
        
        match result {
            Ok(quote) => {
                println!("Quote: {:?}", quote);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}
