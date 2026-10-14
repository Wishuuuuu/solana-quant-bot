use anyhow::Result;
use chrono::{Duration, Utc};
use redis::AsyncCommands;
use std::collections::HashMap;
use std::time::Duration as StdDuration;
use tokio::time::{interval, sleep};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,health_checker=debug")
        .init();

    info!("Starting Health Checker Service...");

    let redis_client = redis::Client::open("redis://127.0.0.1/")?;
    let mut con = redis_client.get_async_connection().await?;

    let health_checker = HealthChecker::new();

    let mut check_interval = interval(StdDuration::from_secs(30));

    loop {
        check_interval.tick().await;

        let health_status = health_checker.check_all_components().await;

        // Store health status in Redis
        let health_json = serde_json::to_string(&health_status)?;
        let _: () = con.set("health_status", health_json).await?;
        let _: () = con.expire("health_status", 60).await?;

        // Check for unhealthy components
        for (component, status) in &health_status {
            if !status.healthy {
                warn!("Component {} is unhealthy: {}", component, status.message);
                
                // Log to system events
                let event = serde_json::json!({
                    "component": component,
                    "event_type": "health_check_failed",
                    "severity": "WARNING",
                    "message": &status.message,
                    "timestamp": Utc::now()
                });
                
                let _: () = con.publish("system_alerts", event.to_string()).await?;

                // Auto-restart logic (simplified)
                if status.last_healthy < Utc::now() - Duration::minutes(5) {
                    error!("Component {} has been unhealthy for 5+ minutes, restart recommended", component);
                    
                    let restart_event = serde_json::json!({
                        "component": component,
                        "event_type": "restart_required",
                        "severity": "CRITICAL",
                        "message": format!("Component {} requires restart", component),
                        "timestamp": Utc::now()
                    });
                    
                    let _: () = con.publish("restart_requests", restart_event.to_string()).await?;
                }
            }
        }

        info!("Health check completed: {}/{} components healthy", 
            health_status.values().filter(|s| s.healthy).count(),
            health_status.len()
        );
    }
}

#[derive(Debug, Clone)]
pub struct ComponentHealth {
    pub healthy: bool,
    pub message: String,
    pub last_healthy: chrono::DateTime<Utc>,
    pub response_time_ms: Option<u64>,
}

pub struct HealthChecker {
    components: Vec<String>,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            components: vec![
                "smart-money-radar".to_string(),
                "ai-sentiment-engine".to_string(),
                "execution-engine".to_string(),
                "risk-manager".to_string(),
                "orchestrator".to_string(),
                "price-oracle".to_string(),
                "dashboard-api".to_string(),
            ],
        }
    }

    pub async fn check_all_components(&self) -> HashMap<String, ComponentHealth> {
        let mut health_status = HashMap::new();

        for component in &self.components {
            let status = self.check_component(component).await;
            health_status.insert(component.clone(), status);
        }

        health_status
    }

    async fn check_component(&self, component: &str) -> ComponentHealth {
        let start = std::time::Instant::now();

        match component.as_str() {
            "dashboard-api" => self.check_dashboard_api().await,
            "redis" => self.check_redis().await,
            "database" => self.check_database().await,
            _ => self.check_component_heartbeat(component).await,
        }
    }

    async fn check_dashboard_api(&self) -> ComponentHealth {
        let client = reqwest::Client::new();
        let start = std::time::Instant::now();

        match client.get("http://localhost:3000/api/health").send().await {
            Ok(response) => {
                let response_time = start.elapsed().as_millis() as u64;
                
                if response.status().is_success() {
                    ComponentHealth {
                        healthy: true,
                        message: "Dashboard API is responding".to_string(),
                        last_healthy: Utc::now(),
                        response_time_ms: Some(response_time),
                    }
                } else {
                    ComponentHealth {
                        healthy: false,
                        message: format!("Dashboard API returned status: {}", response.status()),
                        last_healthy: Utc::now() - Duration::minutes(10),
                        response_time_ms: Some(response_time),
                    }
                }
            }
            Err(e) => {
                ComponentHealth {
                    healthy: false,
                    message: format!("Dashboard API error: {}", e),
                    last_healthy: Utc::now() - Duration::minutes(10),
                    response_time_ms: None,
                }
            }
        }
    }

    async fn check_redis(&self) -> ComponentHealth {
        let client = redis::Client::open("redis://127.0.0.1/");
        
        match client {
            Ok(redis_client) => {
                match redis_client.get_connection() {
                    Ok(mut con) => {
                        let start = std::time::Instant::now();
                        match redis::cmd("PING").query::<String>(&mut con) {
                            Ok(_) => {
                                let response_time = start.elapsed().as_millis() as u64;
                                ComponentHealth {
                                    healthy: true,
                                    message: "Redis is responding".to_string(),
                                    last_healthy: Utc::now(),
                                    response_time_ms: Some(response_time),
                                }
                            }
                            Err(e) => {
                                ComponentHealth {
                                    healthy: false,
                                    message: format!("Redis PING failed: {}", e),
                                    last_healthy: Utc::now() - Duration::minutes(10),
                                    response_time_ms: None,
                                }
                            }
                        }
                    }
                    Err(e) => {
                        ComponentHealth {
                            healthy: false,
                            message: format!("Redis connection failed: {}", e),
                            last_healthy: Utc::now() - Duration::minutes(10),
                            response_time_ms: None,
                        }
                    }
                }
            }
            Err(e) => {
                ComponentHealth {
                    healthy: false,
                    message: format!("Redis client error: {}", e),
                    last_healthy: Utc::now() - Duration::minutes(10),
                    response_time_ms: None,
                }
            }
        }
    }

    async fn check_database(&self) -> ComponentHealth {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://user:password@localhost/solana_quant".to_string());
        
        let start = std::time::Instant::now();
        
        match sqlx::query("SELECT 1").execute(&database_url).await {
            Ok(_) => {
                let response_time = start.elapsed().as_millis() as u64;
                ComponentHealth {
                    healthy: true,
                    message: "Database is responding".to_string(),
                    last_healthy: Utc::now(),
                    response_time_ms: Some(response_time),
                }
            }
            Err(e) => {
                ComponentHealth {
                    healthy: false,
                    message: format!("Database error: {}", e),
                    last_healthy: Utc::now() - Duration::minutes(10),
                    response_time_ms: None,
                }
            }
        }
    }

    async fn check_component_heartbeat(&self, component: &str) -> ComponentHealth {
        let redis_client = redis::Client::open("redis://127.0.0.1/");
        
        if let Ok(redis_client) = redis_client {
            if let Ok(mut con) = redis_client.get_connection() {
                let heartbeat_key = format!("heartbeat:{}", component);
                
                match redis::cmd("GET").arg(&heartbeat_key).query::<String>(&mut con) {
                    Ok(heartbeat) => {
                        if let Ok(timestamp) = heartbeat.parse::<i64>() {
                            let last_heartbeat = chrono::DateTime::from_timestamp(timestamp, 0);
                            
                            if let Some(last_heartbeat) = last_heartbeat {
                                let duration = Utc::now() - last_heartbeat;
                                
                                if duration.num_seconds() < 60 {
                                    ComponentHealth {
                                        healthy: true,
                                        message: format!("Last heartbeat: {}s ago", duration.num_seconds()),
                                        last_healthy: last_heartbeat,
                                        response_time_ms: None,
                                    }
                                } else {
                                    ComponentHealth {
                                        healthy: false,
                                        message: format!("Stale heartbeat: {}s ago", duration.num_seconds()),
                                        last_healthy: last_heartbeat,
                                        response_time_ms: None,
                                    }
                                }
                            } else {
                                ComponentHealth {
                                    healthy: false,
                                    message: "Invalid heartbeat timestamp".to_string(),
                                    last_healthy: Utc::now() - Duration::minutes(10),
                                    response_time_ms: None,
                                }
                            }
                        } else {
                            ComponentHealth {
                                healthy: false,
                                message: "Failed to parse heartbeat".to_string(),
                                last_healthy: Utc::now() - Duration::minutes(10),
                                response_time_ms: None,
                            }
                        }
                    }
                    Err(e) => {
                        ComponentHealth {
                            healthy: false,
                            message: format!("No heartbeat found: {}", e),
                            last_healthy: Utc::now() - Duration::minutes(10),
                            response_time_ms: None,
                        }
                    }
                }
            } else {
                ComponentHealth {
                    healthy: false,
                    message: "Redis connection failed".to_string(),
                    last_healthy: Utc::now() - Duration::minutes(10),
                    response_time_ms: None,
                }
            }
        } else {
            ComponentHealth {
                healthy: false,
                message: "Redis client error".to_string(),
                last_healthy: Utc::now() - Duration::minutes(10),
                response_time_ms: None,
            }
        }
    }

    pub async fn send_heartbeat(&self, component: &str) -> Result<()> {
        let redis_client = redis::Client::open("redis://127.0.0.1/")?;
        let mut con = redis_client.get_async_connection().await?;
        
        let heartbeat_key = format!("heartbeat:{}", component);
        let timestamp = Utc::now().timestamp();
        
        let _: () = con.set(&heartbeat_key, timestamp).await?;
        let _: () = con.expire(&heartbeat_key, 120).await?;
        
        Ok(())
    }
}
