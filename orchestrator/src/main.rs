use anyhow::Result;
use chrono::Utc;
use redis::AsyncCommands;
use shared::{SentimentAnalysis, TradeSignal, TradingOpportunity};
use std::collections::HashMap;
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,orchestrator=debug")
        .init();

    info!("Starting Orchestrator...");

    let redis_client = redis::Client::open("redis://127.0.0.1/")?;
    let con = redis_client.get_async_connection().await?;

    let orchestrator = Orchestrator::new(con);

    orchestrator.run().await
}

pub struct Orchestrator {
    redis: redis::aio::Connection,
    pending_signals: HashMap<String, TradeSignal>,
    sentiment_cache: HashMap<String, SentimentAnalysis>,
}

impl Orchestrator {
    pub fn new(redis: redis::aio::Connection) -> Self {
        Self {
            redis,
            pending_signals: HashMap::new(),
            sentiment_cache: HashMap::new(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut pubsub = self.redis.as_pubsub();
        pubsub.subscribe("smart_money_signals").await?;
        pubsub.subscribe("sentiment_results").await?;

        info!("Orchestrator listening on channels...");

        loop {
            match pubsub.next_message().await {
                Ok(msg) => {
                    if let Some(data) = msg.get_payload() {
                        match msg.channel.as_str() {
                            "smart_money_signals" => {
                                if let Err(e) = self.handle_smart_money_signal(data).await {
                                    error!("Error handling smart money signal: {}", e);
                                }
                            }
                            "sentiment_results" => {
                                if let Err(e) = self.handle_sentiment_result(data).await {
                                    error!("Error handling sentiment result: {}", e);
                                }
                            }
                            _ => {
                                warn!("Unknown channel: {}", msg.channel);
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

    async fn handle_smart_money_signal(&mut self, data: String) -> Result<()> {
        let signal: TradeSignal = serde_json::from_str(&data)?;
        let token_key = signal.token_mint.to_string();

        info!("Received smart money signal for token: {}", token_key);

        self.pending_signals.insert(token_key.clone(), signal);

        if let Some(sentiment) = self.sentiment_cache.get(&token_key) {
            self.evaluate_opportunity(&token_key, sentiment).await?;
        } else {
            // Request sentiment analysis if not cached
            let _: () = self.redis
                .publish("sentiment_requests", token_key.clone())
                .await?;
            info!("Requested sentiment analysis for: {}", token_key);
        }

        Ok(())
    }

    async fn handle_sentiment_result(&mut self, data: String) -> Result<()> {
        let sentiment: serde_json::Value = serde_json::from_str(&data)?;
        let token_mint = sentiment.get("token_mint").and_then(|v| v.as_str()).unwrap_or("");
        let token_key = token_mint.to_string();

        info!("Received sentiment result for token: {}", token_key);

        let sentiment_analysis = SentimentAnalysis {
            token_mint: token_mint.parse().unwrap_or_default(),
            hype_score: sentiment.get("hype_score").and_then(|v| v.as_f64()).unwrap_or(0.0),
            sentiment: match sentiment.get("sentiment").and_then(|v| v.as_str()) {
                Some("Bullish") => shared::Sentiment::Bullish,
                Some("Bearish") => shared::Sentiment::Bearish,
                _ => shared::Sentiment::Neutral,
            },
            volume_velocity: sentiment.get("volume_velocity").and_then(|v| v.as_f64()).unwrap_or(0.0),
            spam_score: sentiment.get("spam_score").and_then(|v| v.as_f64()).unwrap_or(0.0),
            timestamp: Utc::now(),
        };

        self.sentiment_cache.insert(token_key.clone(), sentiment_analysis.clone());

        if let Some(signal) = self.pending_signals.get(&token_key) {
            self.evaluate_opportunity(&token_key, &sentiment_analysis).await?;
        }

        Ok(())
    }

    async fn evaluate_opportunity(&mut self, token_key: &str, sentiment: &SentimentAnalysis) -> Result<()> {
        if let Some(signal) = self.pending_signals.get(token_key) {
            let combined_score = self.calculate_combined_score(signal, sentiment);

            info!("Combined score for {}: {:.2}", token_key, combined_score);

            if combined_score > 0.7 {
                let opportunity = TradingOpportunity {
                    token_mint: signal.token_mint,
                    smart_money_signals: vec![signal.clone()],
                    sentiment: sentiment.clone(),
                    combined_score,
                    timestamp: Utc::now(),
                };

                let opportunity_json = serde_json::to_string(&opportunity)?;
                let _: () = self.redis
                    .publish("trading_opportunities", opportunity_json)
                    .await?;

                info!("Published trading opportunity for {}", token_key);
                
                // Remove from pending to avoid duplicate opportunities
                self.pending_signals.remove(token_key);
            } else {
                info!("Score too low for {}: {:.2}", token_key, combined_score);
            }
        }

        Ok(())
    }

    fn calculate_combined_score(&self, signal: &TradeSignal, sentiment: &SentimentAnalysis) -> f64 {
        let smart_money_weight = 0.4;
        let sentiment_weight = 0.6;

        let smart_money_score = 0.8; // Simplified - would calculate based on wallet stats

        let combined = (smart_money_score * smart_money_weight) + (sentiment.hype_score * sentiment_weight);

        // Penalize high spam
        let spam_penalty = sentiment.spam_score * 0.3;
        
        (combined - spam_penalty).max(0.0).min(1.0)
    }
}
