//! Build script for Vortex Runtime
//!
//! This script runs at compile time and creates a V8 snapshot containing
//! the pre-initialized JavaScript environment (console polyfills, timers, etc.)
//!
//! The snapshot is embedded into the final binary, enabling sub-millisecond cold starts.

use std::env;
use std::path::PathBuf;

// Import the actual ops module to ensure Single Source of Truth
#[path = "src/ops.rs"]
mod ops;

fn main() {
    use deno_core::extension;

    // Define the extension with ops AND the bootstrap JavaScript
    // The esm parameter points to the actual JS file
    // NOTE: Extension name MUST match the one in worker.rs
    extension!(
        vortex_runtime,
        ops = [
            ops::op_log,
            ops::op_get_time_ms,
            ops::op_sleep,
        ],
        esm_entry_point = "ext:vortex_runtime/bootstrap.js",
        esm = [dir "src", "bootstrap.js"],
    );

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let snapshot_path = out_dir.join("VORTEX_SNAPSHOT.bin");

    // Create the snapshot using deno_core's snapshot module
    let snapshot = deno_core::snapshot::create_snapshot(
        deno_core::snapshot::CreateSnapshotOptions {
            cargo_manifest_dir: env!("CARGO_MANIFEST_DIR"),
            startup_snapshot: None,
            skip_op_registration: false,
            extensions: vec![vortex_runtime::init_ops_and_esm()],
            with_runtime_cb: None,
            extension_transpiler: None,
        },
        None,
    )
    .expect("Failed to create snapshot");

    std::fs::write(&snapshot_path, snapshot.output).expect("Failed to write snapshot");

    // Tell Cargo to rerun this script if these files change
    println!("cargo:rerun-if-changed=src/bootstrap.js");
    println!("cargo:rerun-if-changed=src/ops.rs");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:warning=V8 Snapshot written to {:?}", snapshot_path);
}
