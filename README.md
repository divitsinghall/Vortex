# Vortex

**A High-Performance, Geo-Distributed Function-as-a-Service (FaaS) Platform**

Vortex is a serverless compute platform designed to execute JavaScript functions at the edge with sub-millisecond cold starts. Built with Rust and Go, it combines the security of V8 isolates with the reliability of Go's orchestration layer.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Vortex Platform                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐     ┌─────────────────────────────────────┐   │
│  │   Client    │────▶│         vortex-api (Go)             │   │
│  └─────────────┘     │  • HTTP API (chi router)            │   │
│                      │  • Function storage (MinIO)         │   │
│                      │  • Worker pool (10 concurrent)      │   │
│                      └──────────────┬──────────────────────┘   │
│                                     │                           │
│                                     ▼                           │
│                      ┌─────────────────────────────────────┐   │
│                      │     vortex-runtime (Rust)           │   │
│                      │  • V8 JavaScript engine             │   │
│                      │  • Sandboxed execution              │   │
│                      │  • Console capture                  │   │
│                      │  • Async/await support              │   │
│                      └─────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Components

### 1. vortex-runtime (Rust)

A secure V8 JavaScript runtime built on `deno_core`.

**Features:**
- 🔒 **Sandboxed Execution** - No file system or network access
- 📝 **Console Capture** - Logs routed through custom ops, not stdout
- ⏱️ **Async Support** - Full `async/await` with setTimeout polyfill
- 📊 **Execution Metrics** - Timing data for performance monitoring

**Tech Stack:**
- Rust 2021 Edition
- `deno_core` (V8 abstraction layer)
- `tokio` (async runtime)
- `serde` / `serde_json` (serialization)

### 2. vortex-api (Go)

A production-grade orchestration service managing function lifecycle.

**Features:**
- 🚀 **Function Deployment** - Upload and store JavaScript functions
- ⚡ **Function Execution** - Invoke functions with managed concurrency
- 🔄 **Worker Pool** - Limits concurrent executions (prevents resource exhaustion)
- 🛡️ **Zombie Prevention** - Context timeouts kill runaway processes

**Tech Stack:**
- Go 1.22+
- `go-chi/chi` (HTTP router)
- `minio-go` (S3-compatible storage)
- MinIO (function storage backend)

---

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (1.70+)
- [Go](https://golang.org/) (1.22+)
- [Docker](https://www.docker.com/) (for MinIO)

### 1. Build the Rust Runtime

```bash
cd vortex-runtime
cargo build --release
```

### 2. Start MinIO (Storage)

```bash
cd vortex-api
docker-compose up -d
```

This creates a MinIO instance with:
- Endpoint: `localhost:9000`
- Console: `localhost:9001`
- Credentials: `minioadmin:minioadmin`
- Bucket: `vortex-functions` (auto-created)

### 3. Start the API Server

```bash
cd vortex-api
go run ./cmd/server
```

The server starts on `localhost:8080`.

---

## API Reference

### Deploy a Function

```bash
curl -X POST http://localhost:8080/deploy \
  -H "Content-Type: application/json" \
  -d '{"code": "console.log(\"Hello Vortex!\"); return 42;"}'
```

**Response:**
```json
{
  "function_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

### Execute a Function

```bash
curl -X POST http://localhost:8080/execute/550e8400-e29b-41d4-a716-446655440000
```

**Response:**
```json
{
  "output": 42,
  "logs": [
    {"timestamp": "2024-01-01T12:00:00Z", "message": "Hello Vortex!"}
  ],
  "execution_time_ms": 5
}
```

### Health Check

```bash
curl http://localhost:8080/health
```

**Response:**
```json
{
  "status": "healthy",
  "active_workers": 0,
  "max_workers": 10
}
```

---

## Project Structure

```
vortex/
├── vortex-runtime/           # Rust V8 runtime
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs            # Public API
│       ├── main.rs           # CLI entrypoint
│       ├── worker.rs         # VortexWorker (JsRuntime wrapper)
│       ├── ops.rs            # Custom V8 operations
│       └── bootstrap.rs      # JavaScript polyfills
│
└── vortex-api/               # Go Orchestrator
    ├── docker-compose.yml    # MinIO setup
    ├── go.mod
    ├── cmd/server/
    │   └── main.go           # Server entrypoint
    └── internal/
        ├── api/
        │   ├── handlers.go   # HTTP endpoints
        │   └── response.go   # JSON helpers
        ├── store/
        │   └── blob_store.go # MinIO client
        └── runner/
            └── process_runner.go  # Process execution
```

---

## Key Design Patterns

### 1. Worker Pool (Bulkheading)

Prevents resource exhaustion by limiting concurrent function executions:

```go
semaphore := make(chan struct{}, 10)  // Max 10 concurrent

select {
case semaphore <- struct{}{}:
    defer func() { <-semaphore }()
default:
    return ErrCapacityExceeded  // 503 response
}
```

### 2. Zombie Process Prevention

Context timeout ensures runaway scripts don't consume resources:

```go
ctx, cancel := context.WithTimeout(parent, 5*time.Second)
defer cancel()

cmd := exec.CommandContext(ctx, binaryPath, filePath)
// When deadline expires, Go sends SIGKILL
```

### 3. Log Capture via Custom Ops

Console output is routed through Rust ops instead of stdout:

```rust
#[op2(fast)]
pub fn op_log(#[state] storage: &LogStorage, #[string] msg: String) {
    storage.borrow_mut().push(LogEntry::new(msg));
}
```

```javascript
// Bootstrap JS
globalThis.console.log = (...args) => {
    Deno.core.ops.op_log(args.join(' '));
};
```

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MINIO_ENDPOINT` | `localhost:9000` | MinIO server address |
| `MINIO_ACCESS_KEY` | `minioadmin` | MinIO access key |
| `MINIO_SECRET_KEY` | `minioadmin` | MinIO secret key |
| `MINIO_BUCKET` | `vortex-functions` | Storage bucket name |
| `VORTEX_RUNTIME_PATH` | Auto-detected | Path to Rust binary |

---

## Testing

### Run Rust Tests

```bash
cd vortex-runtime
cargo test
```

### Run Go Tests

```bash
cd vortex-api
go test ./...
```

### Manual Integration Test

```bash
# Deploy
FUNC_ID=$(curl -s -X POST http://localhost:8080/deploy \
  -H "Content-Type: application/json" \
  -d '{"code": "return 1 + 1;"}' | jq -r .function_id)

# Execute
curl -X POST http://localhost:8080/execute/$FUNC_ID
# Expected: {"output":2,"logs":[],"execution_time_ms":0}
```

---

## Roadmap

- [ ] **Phase 3**: Edge Deployment (Fly.io / Cloudflare integration)
- [ ] **Phase 4**: V8 Snapshots (sub-millisecond cold starts)
- [ ] **Phase 5**: Fetch API (controlled network access)
- [ ] **Phase 6**: KV Storage (Durable state)

---

## License

MIT
