//! Custom deno_core operations for the Vortex runtime.
//!
//! # Snapshot Compatibility
//!
//! These ops are designed to work in TWO contexts:
//! 1. **Snapshot generation** (build.rs) - State is NOT present
//! 2. **Runtime execution** (worker.rs) - State IS present
//!
//! The ops use `OpState::try_borrow()` patterns to gracefully handle missing state.
//!
//! # Redis Pub/Sub Integration
//!
//! For real-time log streaming, we also store an optional `RedisPublisher`
//! that uses an unbounded mpsc channel to send logs to a background task
//! that publishes to Redis. This "fire-and-forget" pattern ensures:
//! - op_log remains synchronous and non-blocking
//! - V8 event loop is not blocked by Redis I/O
//! - Logs are still captured locally even if Redis is unavailable

use std::cell::RefCell;
use std::rc::Rc;

use chrono::{DateTime, Utc};
use deno_core::{op2, OpState};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// A single log entry captured from JavaScript console methods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// UTC timestamp when the log was captured
    pub timestamp: DateTime<Utc>,
    /// The log message content
    pub message: String,
}

impl LogEntry {
    /// Create a new log entry with the current timestamp
    pub fn new(message: String) -> Self {
        Self {
            timestamp: Utc::now(),
            message,
        }
    }
}

/// Type alias for the log storage used in OpState
pub type LogStorage = Rc<RefCell<Vec<LogEntry>>>;

/// Redis publisher for real-time log streaming.
/// Uses an unbounded mpsc channel for fire-and-forget publishing.
pub struct RedisPublisher {
    /// Sender channel to the background Redis publishing task
    pub sender: mpsc::UnboundedSender<String>,
}

/// Type alias for optional Redis publisher state
pub type RedisPublisherState = Rc<RefCell<Option<RedisPublisher>>>;

/// Custom operation to capture console.log messages.
///
/// This op is called from JavaScript via `Deno.core.ops.op_log(message)`.
/// Instead of printing to stdout, it stores the message in our log buffer
/// so it can be returned as part of the ExecutionResult.
///
/// # Snapshot Resilience
///
/// This op is called during BOTH snapshot generation and runtime execution.
/// During snapshot generation, OpState won't have LogStorage or RedisPublisher.
/// We use `OpState::try_borrow()` to gracefully handle this case.
///
/// # Arguments
/// * `state` - The operation state (may or may not contain our storage)
/// * `message` - The log message from JavaScript
#[op2(fast)]
pub fn op_log(state: &OpState, #[string] message: String) {
    // Try to get LogStorage - may not exist during snapshot generation
    if let Some(log_storage) = state.try_borrow::<LogStorage>() {
        let entry = LogEntry::new(message.clone());
        
        // Store locally for the ExecutionResult
        log_storage.borrow_mut().push(entry.clone());
        
        // Try to get RedisPublisher - may not exist
        if let Some(redis_pub) = state.try_borrow::<RedisPublisherState>() {
            // Fire-and-forget publish to Redis if configured
            if let Some(publisher) = redis_pub.borrow().as_ref() {
                if let Ok(json) = serde_json::to_string(&entry) {
                    // Ignore send errors - Redis publishing is best-effort
                    let _ = publisher.sender.send(json);
                }
            }
        }
    }
    // If no state, silently ignore (we're in snapshot generation)
}

/// Get the current time in milliseconds since Unix epoch.
///
/// This op supports timing operations in JavaScript.
/// It's marked as `fast` since it's a simple, synchronous operation.
#[op2(fast)]
pub fn op_get_time_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// Async sleep operation backed by tokio.
///
/// This replaces the busy-wait setTimeout loop with proper async sleeping.
/// The tokio runtime will properly yield the thread during the sleep,
/// allowing thousands of concurrent tenants without burning CPU cycles.
///
/// # Arguments
/// * `delay_ms` - The number of milliseconds to sleep
#[op2(async)]
pub async fn op_sleep(#[bigint] delay_ms: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry::new("test message".to_string());
        assert_eq!(entry.message, "test message");
        // Timestamp should be recent (within last second)
        let now = Utc::now();
        let diff = now.signed_duration_since(entry.timestamp);
        assert!(diff.num_seconds() < 1);
    }

    #[test]
    fn test_get_time_ms_logic() {
        // Test the underlying time logic (can't call op-decorated function directly)
        let get_time = || {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as f64)
                .unwrap_or(0.0)
        };
        
        let time1 = get_time();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let time2 = get_time();
        assert!(time2 > time1);
        assert!(time2 - time1 >= 10.0);
    }

    #[tokio::test]
    async fn test_op_sleep_logic() {
        // Test the underlying sleep logic
        let start = std::time::Instant::now();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let elapsed = start.elapsed().as_millis();
        assert!(elapsed >= 50);
        assert!(elapsed < 100); // Should be reasonably close
    }
}
