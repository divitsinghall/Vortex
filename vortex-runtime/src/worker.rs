//! VortexWorker - The core runtime wrapper for executing JavaScript.
//!
//! This module provides the main abstraction for running JavaScript code
//! in a secure, sandboxed V8 isolate. It handles:
//! - V8 isolate initialization via deno_core
//! - Custom op registration for console capture and timing
//! - Event loop execution for async/await support
//! - Result collection with timing metrics
//! - Real-time log streaming via Redis Pub/Sub (optional)

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use anyhow::{anyhow, Result};
use deno_core::{extension, v8, JsRuntime, RuntimeOptions};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use crate::bootstrap::BOOTSTRAP_JS;
use crate::ops::{op_get_time_ms, op_log, LogEntry, LogStorage, RedisPublisher, RedisPublisherState};

/// Result of executing a JavaScript script in the Vortex runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// The return value of the script (last expression result), if any
    pub output: Option<Value>,
    /// All captured log entries from console.log, console.error, etc.
    pub logs: Vec<LogEntry>,
    /// Total execution time in milliseconds
    pub execution_time_ms: u64,
}

impl ExecutionResult {
    /// Create a new execution result
    pub fn new(output: Option<Value>, logs: Vec<LogEntry>, execution_time_ms: u64) -> Self {
        Self {
            output,
            logs,
            execution_time_ms,
        }
    }
}

// Define our extension that registers custom ops
// Now includes both LogStorage and RedisPublisherState
extension!(
    vortex_runtime,
    ops = [op_log, op_get_time_ms],
    options = {
        log_storage: LogStorage,
        redis_pub: RedisPublisherState,
    },
    state = |state, options| {
        state.put::<LogStorage>(options.log_storage);
        state.put::<RedisPublisherState>(options.redis_pub);
    }
);

/// VortexWorker - A secure JavaScript runtime built on deno_core.
///
/// # Architecture
///
/// The worker wraps a `JsRuntime` (which manages a V8 isolate) and provides:
/// - **Sandboxing**: No file system or network access by default
/// - **Log Capture**: Console output is intercepted and stored
/// - **Async Support**: Full async/await via tokio event loop integration
/// - **Metrics**: Execution timing for performance monitoring
/// - **Real-time Streaming**: Optional Redis Pub/Sub for live log streaming
///
/// # Example
///
/// ```rust,no_run
/// use vortex_runtime::VortexWorker;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let mut worker = VortexWorker::new()?;
///     let result = worker.run("console.log('Hello!'); 42").await?;
///     println!("Output: {:?}", result.output);
///     println!("Logs: {:?}", result.logs);
///     Ok(())
/// }
/// ```
pub struct VortexWorker {
    /// The underlying V8 runtime
    runtime: JsRuntime,
    /// Shared storage for capturing console.log output
    log_storage: LogStorage,
}

impl VortexWorker {
    /// Create a new VortexWorker with a fresh V8 isolate.
    ///
    /// This initializes the runtime, registers custom ops, and executes
    /// the bootstrap JavaScript to set up the environment.
    ///
    /// # Errors
    ///
    /// Returns an error if the bootstrap JavaScript fails to execute.
    pub fn new() -> Result<Self> {
        Self::new_with_redis(None, None)
    }

    /// Create a new VortexWorker with optional Redis Pub/Sub support.
    ///
    /// When a Redis client and function ID are provided, logs will be
    /// published in real-time to the Redis channel `logs:{function_id}`.
    ///
    /// # Arguments
    ///
    /// * `redis_client` - Optional Redis client for pub/sub
    /// * `function_id` - Optional function ID for the Redis channel name
    ///
    /// # Architecture: Non-blocking Redis Publishing
    ///
    /// To avoid blocking the V8 event loop, we use a "fire-and-forget" pattern:
    /// 1. op_log sends messages through an unbounded mpsc channel
    /// 2. A background tokio task receives messages and publishes to Redis
    /// 3. The op returns immediately without waiting for Redis confirmation
    ///
    /// This ensures JavaScript execution remains fast even if Redis is slow.
    pub fn new_with_redis(
        redis_client: Option<redis::Client>,
        function_id: Option<String>,
    ) -> Result<Self> {
        // Create shared log storage that ops can write to
        let log_storage: LogStorage = Rc::new(RefCell::new(Vec::new()));
        
        // Create Redis publisher state (initially None)
        let redis_pub_state: RedisPublisherState = Rc::new(RefCell::new(None));

        // If Redis client and function ID are provided, set up the publisher
        if let (Some(client), Some(func_id)) = (redis_client, function_id) {
            let (tx, mut rx) = mpsc::unbounded_channel::<String>();
            
            // Store the sender in the state
            redis_pub_state.borrow_mut().replace(RedisPublisher { sender: tx });
            
            // Spawn a background task to publish messages to Redis
            // This runs independently of the V8 event loop
            let channel = format!("logs:{}", func_id);
            tokio::spawn(async move {
                // Get async connection to Redis
                match client.get_multiplexed_async_connection().await {
                    Ok(mut conn) => {
                        // Process messages from the channel
                        while let Some(msg) = rx.recv().await {
                            // Publish to Redis, ignoring errors (fire-and-forget)
                            let publish_result: Result<(), redis::RedisError> = redis::cmd("PUBLISH")
                                .arg(&channel)
                                .arg(&msg)
                                .query_async(&mut conn)
                                .await;
                            
                            if let Err(e) = publish_result {
                                eprintln!("Redis publish error (non-fatal): {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to connect to Redis (logs won't stream): {}", e);
                        // Still drain the channel to avoid memory buildup
                        while rx.recv().await.is_some() {}
                    }
                }
            });
        }

        // Build the runtime with our extension
        // Note: We intentionally don't add deno_fs, deno_net, etc.
        // to maintain a secure sandbox
        let runtime = JsRuntime::new(RuntimeOptions {
            extensions: vec![vortex_runtime::init_ops(
                log_storage.clone(),
                redis_pub_state,
            )],
            ..Default::default()
        });

        let mut worker = Self {
            runtime,
            log_storage,
        };

        // Execute bootstrap code to set up the environment
        worker.bootstrap()?;

        Ok(worker)
    }

    /// Execute the bootstrap JavaScript to initialize the runtime environment.
    fn bootstrap(&mut self) -> Result<()> {
        self.runtime
            .execute_script("[vortex:bootstrap]", BOOTSTRAP_JS)
            .map_err(|e| anyhow!("Bootstrap failed: {}", e))?;
        Ok(())
    }

    /// Execute JavaScript code and return the result.
    ///
    /// This is the main entry point for running user code. It:
    /// 1. Clears any previous logs
    /// 2. Executes the provided JavaScript code
    /// 3. Runs the event loop to completion (for async code)
    /// 4. Collects and returns the result, logs, and timing
    ///
    /// # Arguments
    ///
    /// * `code` - The JavaScript code to execute
    ///
    /// # Returns
    ///
    /// An `ExecutionResult` containing the script's return value,
    /// captured logs, and execution time.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The JavaScript code has a syntax error
    /// - The script throws an uncaught exception
    /// - The event loop encounters an error
    pub async fn run(&mut self, code: &str) -> Result<ExecutionResult> {
        // Clear previous logs
        self.log_storage.borrow_mut().clear();

        let start = Instant::now();

        // Wrap user code to support:
        // 1. Top-level await syntax
        // 2. Multi-statement code blocks  
        //
        // Note: The async IIFE returns undefined unless code has explicit return.
        // For expression return values, use "return <expression>" in your code.
        let wrapped_code = format!(
            r#"
            (async () => {{
                {code}
            }})()
            "#
        );

        // Execute the script - this returns a Promise
        let promise = self
            .runtime
            .execute_script("[vortex:user_script]", wrapped_code)
            .map_err(|e| anyhow!("Script execution failed: {}", e))?;

        // Resolve the promise by running the event loop
        let resolved = self
            .runtime
            .resolve_value(promise)
            .await
            .map_err(|e| anyhow!("Event loop error: {}", e))?;

        // Try to get the result value
        let output = {
            let scope = &mut self.runtime.handle_scope();
            let local = v8::Local::new(scope, resolved);

            // Convert V8 value to serde_json
            if local.is_undefined() || local.is_null() {
                None
            } else {
                let json_str: Option<String> = v8::json::stringify(scope, local)
                    .map(|s: v8::Local<v8::String>| s.to_rust_string_lossy(scope));

                json_str.and_then(|s: String| serde_json::from_str(&s).ok())
            }
        };

        let execution_time_ms = start.elapsed().as_millis() as u64;

        // Collect logs
        let logs = self.log_storage.borrow().clone();

        Ok(ExecutionResult::new(output, logs, execution_time_ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_execution() {
        let mut worker = VortexWorker::new().unwrap();
        // Use explicit return since async IIFE requires it
        let result = worker.run("return 1 + 1").await.unwrap();
        assert_eq!(result.output, Some(serde_json::json!(2)));
    }

    #[tokio::test]
    async fn test_console_log_capture() {
        let mut worker = VortexWorker::new().unwrap();
        let result = worker.run("console.log('hello world')").await.unwrap();
        assert_eq!(result.logs.len(), 1);
        assert_eq!(result.logs[0].message, "hello world");
    }

    #[tokio::test]
    async fn test_multiple_logs() {
        let mut worker = VortexWorker::new().unwrap();
        let code = r#"
            console.log('first');
            console.log('second');
            console.log('third');
        "#;
        let result = worker.run(code).await.unwrap();
        assert_eq!(result.logs.len(), 3);
        assert_eq!(result.logs[0].message, "first");
        assert_eq!(result.logs[1].message, "second");
        assert_eq!(result.logs[2].message, "third");
    }

    #[tokio::test]
    async fn test_async_await() {
        let mut worker = VortexWorker::new().unwrap();
        let code = r#"
            const sleep = (ms) => new Promise(resolve => setTimeout(resolve, ms));
            console.log('start');
            await sleep(10);
            console.log('end');
            return 'done';
        "#;
        let result = worker.run(code).await.unwrap();
        assert_eq!(result.logs.len(), 2);
        assert_eq!(result.logs[0].message, "start");
        assert_eq!(result.logs[1].message, "end");
        assert_eq!(result.output, Some(serde_json::json!("done")));
    }
}
