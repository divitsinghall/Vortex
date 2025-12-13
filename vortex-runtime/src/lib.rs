//! Vortex Runtime - A secure V8 JavaScript runtime for the Vortex FaaS platform
//!
//! This crate provides a sandboxed JavaScript execution environment built on top of
//! `deno_core` and the V8 engine. It captures console output, supports async/await,
//! and provides execution timing metrics.

mod bootstrap;
mod ops;
mod worker;

pub use ops::LogEntry;
pub use worker::{ExecutionResult, VortexWorker};
