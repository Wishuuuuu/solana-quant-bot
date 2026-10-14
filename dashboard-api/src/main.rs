use anyhow::Result;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use chrono::{Duration, Utc};
use database::Database;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,dashboard_api=debug")
        .init();

    info!("Starting Dashboard API...");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://user:password@localhost/solana_quant".to_string());
    
    let db = Database::new(&database_url).await?;

    let app = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/positions", get(get_positions))
        .route("/api/performance", get(get_performance))
        .route("/api/stats", get(get_stats))
        .route("/api/recent-trades", get(get_recent_trades))
        .layer(CorsLayer::permissive())
        .with_state(db);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("Dashboard API listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: Utc::now(),
    })
}

async fn get_positions(State(db): State<Database>) -> Result<Json<Vec<PositionResponse>>, StatusCode> {
    match db.get_active_positions().await {
        Ok(positions) => {
            let response: Vec<PositionResponse> = positions
                .into_iter()
                .map(|p| PositionResponse {
                    token_mint: p.token_mint.to_string(),
                    entry_price: p.entry_price,
                    current_price: p.current_price,
                    size: p.size,
                    pnl_percentage: p.pnl_percentage,
                    highest_price: p.highest_price,
                    stop_loss_price: p.stop_loss_price,
                    take_profit_price: p.take_profit_price,
                    entry_time: p.entry_time,
                })
                .collect();
            Ok(Json(response))
        }
        Err(e) => {
            warn!("Error fetching positions: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_performance(
    State(db): State<Database>,
    Query(params): Query<PerformanceQuery>,
) -> Result<Json<PerformanceResponse>, StatusCode> {
    let days = params.days.unwrap_or(7);

    match db.get_performance_metrics(days).await {
        Ok(metrics) => {
            let total_trades: i32 = metrics.iter().map(|m| m.total_trades).sum();
            let winning_trades: i32 = metrics.iter().map(|m| m.winning_trades).sum();
            let losing_trades: i32 = metrics.iter().map(|m| m.losing_trades).sum();
            let total_pnl: f64 = metrics.iter().map(|m| m.total_pnl).sum();
            let max_drawdown: f64 = metrics.iter().map(|m| m.max_drawdown).fold(0.0_f64, f64::max);
            
            let win_rate = if total_trades > 0 {
                winning_trades as f64 / total_trades as f64 * 100.0
            } else {
                0.0
            };

            let response = PerformanceResponse {
                total_trades,
                winning_trades,
                losing_trades,
                win_rate,
                total_pnl,
                max_drawdown,
                sharpe_ratio: metrics.first().and_then(|m| m.sharpe_ratio),
                avg_holding_time_hours: metrics.first().and_then(|m| m.avg_holding_time_hours),
                daily_metrics: metrics,
            };

            Ok(Json(response))
        }
        Err(e) => {
            warn!("Error fetching performance: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_stats(State(db): State<Database>) -> Result<Json<StatsResponse>, StatusCode> {
    // Simplified stats - in production would query database
    Ok(Json(StatsResponse {
        active_positions: 3,
        total_trades_today: 15,
        win_rate_today: 73.3,
        pnl_today: 2.5,
        uptime_hours: 24.5,
        last_signal: Utc::now() - Duration::minutes(5),
    }))
}

async fn get_recent_trades() -> Json<Vec<RecentTradeResponse>> {
    // Mock data - in production would query database
    let trades = vec![
        RecentTradeResponse {
            token_mint: "TokenA".to_string(),
            action: "BUY".to_string(),
            amount: 0.5,
            price: 1.25,
            timestamp: Utc::now() - Duration::minutes(10),
            pnl: None,
        },
        RecentTradeResponse {
            token_mint: "TokenB".to_string(),
            action: "SELL".to_string(),
            amount: 0.3,
            price: 2.0,
            timestamp: Utc::now() - Duration::hours(1),
            pnl: Some(0.5),
        },
    ];

    Json(trades)
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    timestamp: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct PositionResponse {
    token_mint: String,
    entry_price: f64,
    current_price: f64,
    size: u64,
    pnl_percentage: f64,
    highest_price: f64,
    stop_loss_price: f64,
    take_profit_price: f64,
    entry_time: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct PerformanceQuery {
    days: Option<i32>,
}

#[derive(Debug, Serialize)]
struct PerformanceResponse {
    total_trades: i32,
    winning_trades: i32,
    losing_trades: i32,
    win_rate: f64,
    total_pnl: f64,
    max_drawdown: f64,
    sharpe_ratio: Option<f64>,
    avg_holding_time_hours: Option<f64>,
    daily_metrics: Vec<database::PerformanceMetric>,
}

#[derive(Debug, Serialize)]
struct StatsResponse {
    active_positions: i32,
    total_trades_today: i32,
    win_rate_today: f64,
    pnl_today: f64,
    uptime_hours: f64,
    last_signal: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct RecentTradeResponse {
    token_mint: String,
    action: String,
    amount: f64,
    price: f64,
    timestamp: chrono::DateTime<Utc>,
    pnl: Option<f64>,
}
