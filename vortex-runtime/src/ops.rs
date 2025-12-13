//! Custom deno_core operations for the Vortex runtime.
//!
//! Operations (Ops) are the bridge between JavaScript and Rust. They allow
//! JavaScript code running in the V8 isolate to call into Rust functions.
//!
//! # Architecture Decision: OpState for Log Storage
//!
//! We use `OpState` to store a `Rc<RefCell<Vec<LogEntry>>>` that accumulates
//! log messages during script execution. This approach:
//! - Allows synchronous op calls (no async overhead for logging)
//! - Keeps logs isolated per-execution
//! - Enables easy collection after script completion

use std::cell::RefCell;
use std::rc::Rc;

use chrono::{DateTime, Utc};
use deno_core::op2;
use serde::{Deserialize, Serialize};

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

/// Custom operation to capture console.log messages.
///
/// This op is called from JavaScript via `Deno.core.ops.op_log(message)`.
/// Instead of printing to stdout, it stores the message in our log buffer
/// so it can be returned as part of the ExecutionResult.
///
/// # Arguments
/// * `state` - The operation state containing our log storage
/// * `message` - The log message from JavaScript
#[op2(fast)]
pub fn op_log(#[state] log_storage: &LogStorage, #[string] message: String) {
    let entry = LogEntry::new(message);
    log_storage.borrow_mut().push(entry);
}

/// Get the current time in milliseconds since Unix epoch.
///
/// This op supports our setTimeout polyfill by providing accurate timing.
/// It's marked as `fast` since it's a simple, synchronous operation.
#[op2(fast)]
pub fn op_get_time_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
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
}

