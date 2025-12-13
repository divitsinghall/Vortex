//! Vortex Runtime CLI
//!
//! This binary executes JavaScript files through the VortexWorker and outputs
//! the execution result as JSON to stdout. It is designed to be invoked by
//! the Vortex API (Go) for function execution.
//!
//! Usage:
//!   vortex-runtime <path-to-js-file>
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

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return Err(anyhow!(
            "Usage: {} <path-to-js-file>\n\n\
             Executes JavaScript from a file and outputs JSON result to stdout.",
            args.get(0).unwrap_or(&"vortex-runtime".to_string())
        ));
    }

    let file_path = &args[1];

    // Read JavaScript code from file
    let code = fs::read_to_string(file_path)
        .map_err(|e| anyhow!("Failed to read file '{}': {}", file_path, e))?;

    // Create worker and execute
    let mut worker = VortexWorker::new()
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
