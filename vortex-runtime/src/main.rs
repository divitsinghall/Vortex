//! Vortex Runtime CLI
//!
//! This binary executes JavaScript files through the VortexWorker and outputs
//! the execution result as JSON to stdout. It is designed to be invoked by
//! the Vortex API (Go) for function execution.
//!
//! Usage:
//!   vortex-runtime <path-to-js-file> [--redis-url <url>] [--function-id <id>]
//!
//! Options:
//!   --redis-url <url>    Redis URL for real-time log streaming (e.g., redis://localhost:6379)
//!   --function-id <id>   Function ID for Redis channel name (logs:<function_id>)
//!
//! Output (JSON to stdout):
//!   {
//!     "output": <any>,
//!     "logs": [{"timestamp": "...", "message": "..."}],
//!     "execution_time_ms": <number>
//!   }
//!
//! Errors are written to stderr and exit code 1 is returned.

use std::env;
use std::fs;
use std::process;

use anyhow::{anyhow, Result};
use serde::Serialize;
use vortex_runtime::{LogEntry, VortexWorker};

/// CLI output structure matching what the Go API expects.
#[derive(Serialize)]
struct CliOutput {
    output: Option<serde_json::Value>,
    logs: Vec<LogEntryOutput>,
    execution_time_ms: u64,
}

/// Log entry for CLI output (simpler format without chrono serialization issues).
#[derive(Serialize)]
struct LogEntryOutput {
    timestamp: String,
    message: String,
}

impl From<LogEntry> for LogEntryOutput {
    fn from(entry: LogEntry) -> Self {
        Self {
            timestamp: entry.timestamp.to_rfc3339(),
            message: entry.message,
        }
    }
}

/// Parsed CLI arguments
struct CliArgs {
    file_path: String,
    redis_url: Option<String>,
    function_id: Option<String>,
}

/// Parse command line arguments
fn parse_args() -> Result<CliArgs> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return Err(anyhow!(
            "Usage: {} <path-to-js-file> [--redis-url <url>] [--function-id <id>]\n\n\
             Executes JavaScript from a file and outputs JSON result to stdout.\n\n\
             Options:\n  \
               --redis-url <url>    Redis URL for real-time log streaming\n  \
               --function-id <id>   Function ID for Redis channel name",
            args.first().map(|s| s.as_str()).unwrap_or("vortex-runtime")
        ));
    }

    let file_path = args[1].clone();
    let mut redis_url: Option<String> = None;
    let mut function_id: Option<String> = None;

    // Parse optional arguments
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--redis-url" => {
                if i + 1 < args.len() {
                    redis_url = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return Err(anyhow!("--redis-url requires a value"));
                }
            }
            "--function-id" => {
                if i + 1 < args.len() {
                    function_id = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return Err(anyhow!("--function-id requires a value"));
                }
            }
            _ => {
                return Err(anyhow!("Unknown argument: {}", args[i]));
            }
        }
    }

    Ok(CliArgs {
        file_path,
        redis_url,
        function_id,
    })
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    // Parse command line arguments
    let cli_args = parse_args()?;

    // Read JavaScript code from file
    let code = fs::read_to_string(&cli_args.file_path)
        .map_err(|e| anyhow!("Failed to read file '{}': {}", cli_args.file_path, e))?;

    // Create Redis client if URL is provided
    let redis_client = if let Some(ref url) = cli_args.redis_url {
        Some(redis::Client::open(url.as_str())
            .map_err(|e| anyhow!("Failed to create Redis client: {}", e))?)
    } else {
        None
    };

    // Create worker with optional Redis support
    let mut worker = VortexWorker::new_with_redis(redis_client, cli_args.function_id)
        .map_err(|e| anyhow!("Failed to initialize runtime: {}", e))?;

    let result = worker
        .run(&code)
        .await
        .map_err(|e| anyhow!("Execution failed: {}", e))?;

    // Convert to CLI output format
    let output = CliOutput {
        output: result.output,
        logs: result.logs.into_iter().map(LogEntryOutput::from).collect(),
        execution_time_ms: result.execution_time_ms,
    };

    // Output JSON to stdout
    let json = serde_json::to_string(&output)
        .map_err(|e| anyhow!("Failed to serialize output: {}", e))?;

    println!("{}", json);

    Ok(())
}

