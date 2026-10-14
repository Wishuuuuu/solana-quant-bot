use anyhow::Result;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, warn};

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub exponential_backoff: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            exponential_backoff: true,
        }
    }
}

pub async fn retry_with_backoff<F, T, E>(
    operation: F,
    config: &RetryConfig,
    operation_name: &str,
) -> Result<T>
where
    F: Fn() -> Result<T, E>,
    E: std::error::Error + Send + Sync + 'static,
{
    let mut attempt = 0;
    let mut delay = config.initial_delay;

    loop {
        attempt += 1;

        match operation() {
            Ok(result) => {
                if attempt > 1 {
                    warn!("{} succeeded after {} attempts", operation_name, attempt);
                }
                return Ok(result);
            }
            Err(e) => {
                if attempt >= config.max_attempts {
                    error!("{} failed after {} attempts: {}", operation_name, attempt, e);
                    return Err(anyhow::anyhow!("Operation failed after {} attempts: {}", attempt, e));
                }

                warn!("{} attempt {} failed: {}, retrying in {:?}", 
                    operation_name, attempt, e, delay);

                sleep(delay).await;

                if config.exponential_backoff {
                    delay = std::cmp::min(delay * 2, config.max_delay);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    failure_count: Arc<AtomicU32>,
    failure_threshold: u32,
    timeout: Duration,
    last_failure_time: Arc<std::sync::Mutex<Option<std::time::Instant>>>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, timeout: Duration) -> Self {
        Self {
            failure_count: Arc::new(AtomicU32::new(0)),
            failure_threshold,
            timeout,
            last_failure_time: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub async fn call<F, T, E>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        // Check if circuit is open
        {
            let last_failure = self.last_failure_time.lock().unwrap();
            if let Some(failure_time) = *last_failure {
                if failure_time.elapsed() < self.timeout {
                    return Err(anyhow::anyhow!("Circuit breaker is open"));
                }
            }
        }

        // Execute operation
        match operation() {
            Ok(result) => {
                self.failure_count.store(0, Ordering::SeqCst);
                *self.last_failure_time.lock().unwrap() = None;
                Ok(result)
            }
            Err(e) => {
                let count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
                *self.last_failure_time.lock().unwrap() = Some(std::time::Instant::now());

                if count >= self.failure_threshold {
                    error!("Circuit breaker opened after {} failures", count);
                }

                Err(anyhow::anyhow!("Operation failed: {}", e))
            }
        }
    }

    pub fn reset(&self) {
        self.failure_count.store(0, Ordering::SeqCst);
        *self.last_failure_time.lock().unwrap() = None;
    }

    pub fn is_open(&self) -> bool {
        let last_failure = self.last_failure_time.lock().unwrap();
        if let Some(failure_time) = *last_failure {
            if failure_time.elapsed() < self.timeout {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone)]
pub struct ErrorRecovery {
    circuit_breakers: Arc<std::sync::Mutex<std::collections::HashMap<String, CircuitBreaker>>>,
}

impl ErrorRecovery {
    pub fn new() -> Self {
        Self {
            circuit_breakers: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn get_circuit_breaker(&self, name: &str, threshold: u32, timeout: Duration) -> CircuitBreaker {
        let mut breakers = self.circuit_breakers.lock().unwrap();
        
        breakers
            .entry(name.to_string())
            .or_insert_with(|| CircuitBreaker::new(threshold, timeout))
            .clone()
    }

    pub async fn execute_with_recovery<F, T, E>(
        &self,
        component_name: &str,
        operation: F,
    ) -> Result<T>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        let retry_config = RetryConfig::default();
        let circuit_breaker = self.get_circuit_breaker(component_name, 5, Duration::from_secs(60));

        let result = circuit_breaker.call(|| {
            retry_with_backoff(&operation, &retry_config, component_name)
        }).await;

        if result.is_err() {
            error!("Error recovery failed for component: {}", component_name);
        }

        result
    }

    pub fn reset_all(&self) {
        let mut breakers = self.circuit_breakers.lock().unwrap();
        for (_, breaker) in breakers.iter() {
            breaker.reset();
        }
    }
}

impl Default for ErrorRecovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_with_backoff() {
        let mut attempts = 0;
        
        let result = retry_with_backoff(
            || {
                attempts += 1;
                if attempts < 3 {
                    Err(anyhow::anyhow!("Temporary failure"))
                } else {
                    Ok("success")
                }
            },
            &RetryConfig::default(),
            "test_operation"
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempts, 3);
    }

    #[test]
    fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new(3, Duration::from_secs(10));
        
        // First few failures
        for _ in 0..3 {
            let result = breaker.call(|| Err::<(), _>(anyhow::anyhow!("failed")));
            assert!(result.is_err());
        }

        // Circuit should be open
        assert!(breaker.is_open());

        // Reset after timeout
        std::thread::sleep(Duration::from_secs(11));
        assert!(!breaker.is_open());

        // Should work after reset
        let result = breaker.call(|| Ok::<(), _>(()));
        assert!(result.is_ok());
    }
}
