use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartMoneyWallet {
    pub address: Pubkey,
    pub win_rate: f64,
    pub total_trades: u64,
    pub profit_factor: f64,
    pub last_active: DateTime<Utc>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSignal {
    pub wallet: Pubkey,
    pub token_mint: Pubkey,
    pub action: TradeAction,
    pub amount: u64,
    pub timestamp: DateTime<Utc>,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeAction {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentAnalysis {
    pub token_mint: Pubkey,
    pub hype_score: f64,
    pub sentiment: Sentiment,
    pub volume_velocity: f64,
    pub spam_score: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Sentiment {
    Bullish,
    Bearish,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingOpportunity {
    pub token_mint: Pubkey,
    pub smart_money_signals: Vec<TradeSignal>,
    pub sentiment: SentimentAnalysis,
    pub combined_score: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub token_mint: Pubkey,
    pub entry_price: f64,
    pub current_price: f64,
    pub size: u64,
    pub pnl_percentage: f64,
    pub highest_price: f64,
    pub stop_loss_price: f64,
    pub take_profit_price: f64,
    pub entry_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExecution {
    pub opportunity: TradingOpportunity,
    pub amount: u64,
    pub priority_fee: u64,
    pub slippage: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskDecision {
    pub action: RiskAction,
    pub reason: String,
    pub new_stop_loss: Option<f64>,
    pub take_profit_percentage: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskAction {
    EnterFull,
    EnterHalf,
    Skip,
    TakeProfit,
    StopLoss,
    TrailingStop,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_money_wallet_creation() {
        let wallet = SmartMoneyWallet {
            address: Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
            win_rate: 0.75,
            total_trades: 100,
            profit_factor: 2.5,
            last_active: chrono::Utc::now(),
            tags: vec!["whale".to_string()],
        };

        assert_eq!(wallet.win_rate, 0.75);
        assert_eq!(wallet.total_trades, 100);
        assert!(wallet.tags.contains(&"whale".to_string()));
    }

    #[test]
    fn test_trade_signal_serialization() {
        let signal = TradeSignal {
            wallet: Pubkey::new_unique(),
            token_mint: Pubkey::new_unique(),
            action: TradeAction::Buy,
            amount: 1_000_000,
            timestamp: chrono::Utc::now(),
            signature: "test_signature".to_string(),
        };

        let serialized = serde_json::to_string(&signal).unwrap();
        let deserialized: TradeSignal = serde_json::from_str(&serialized).unwrap();

        assert_eq!(signal.amount, deserialized.amount);
        assert_eq!(signal.action, deserialized.action);
    }

    #[test]
    fn test_sentiment_analysis_creation() {
        let sentiment = SentimentAnalysis {
            token_mint: Pubkey::new_unique(),
            hype_score: 0.85,
            sentiment: Sentiment::Bullish,
            volume_velocity: 0.9,
            spam_score: 0.1,
            timestamp: chrono::Utc::now(),
        };

        assert_eq!(sentiment.hype_score, 0.85);
        assert!(matches!(sentiment.sentiment, Sentiment::Bullish));
        assert!(sentiment.hype_score > sentiment.spam_score);
    }

    #[test]
    fn test_position_pnl_calculation() {
        let mut position = Position {
            token_mint: Pubkey::new_unique(),
            entry_price: 1.0,
            current_price: 1.5,
            size: 1_000_000,
            pnl_percentage: 0.0,
            highest_price: 1.0,
            stop_loss_price: 0.7,
            take_profit_price: 2.0,
            entry_time: chrono::Utc::now(),
        };

        position.pnl_percentage = ((position.current_price - position.entry_price) / position.entry_price) * 100.0;

        assert_eq!(position.pnl_percentage, 50.0);
    }
}
