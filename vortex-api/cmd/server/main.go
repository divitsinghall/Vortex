// Package main is the entrypoint for the Vortex API server.
//
// This server orchestrates the Vortex FaaS platform by:
// - Accepting function deployments via POST /deploy
// - Executing functions via POST /execute/{id}
// - Managing function storage in MinIO (S3-compatible)
// - Invoking the Rust vortex-runtime binary for JavaScript execution
package main

import (
	"context"
	"log"
	"net/http"
	"os"
	"os/signal"
	"path/filepath"
	"syscall"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"

	"github.com/vortex/vortex-api/internal/api"
	"github.com/vortex/vortex-api/internal/runner"
	"github.com/vortex/vortex-api/internal/store"
)

func main() {
	log.SetFlags(log.LstdFlags | log.Lshortfile)
	log.Println("Starting Vortex API Server...")

	// Configuration (could be loaded from env vars in production)
	cfg := Config{
		ServerAddr: ":8080",
		MinIO: store.BlobStoreConfig{
			Endpoint:        getEnv("MINIO_ENDPOINT", "localhost:9000"),
			AccessKeyID:     getEnv("MINIO_ACCESS_KEY", "minioadmin"),
			SecretAccessKey: getEnv("MINIO_SECRET_KEY", "minioadmin"),
			BucketName:      getEnv("MINIO_BUCKET", "vortex-functions"),
			UseSSL:          false,
		},
		Runner: runner.ProcessRunnerConfig{
			BinaryPath:     getRuntimePath(),
			MaxConcurrent:  10,
			DefaultTimeout: 5 * time.Second,
		},
	}

	// Create context that listens for shutdown signals
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Handle graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	// Initialize storage layer (with retry logic for MinIO startup)
	log.Println("Connecting to MinIO...")
	blobStore, err := store.NewBlobStore(ctx, cfg.MinIO)
	if err != nil {
		log.Fatalf("Failed to initialize blob store: %v", err)
	}
	log.Println("Connected to MinIO successfully")

	// Initialize execution engine
	log.Printf("Initializing runner with max %d concurrent workers", cfg.Runner.MaxConcurrent)
	processRunner := runner.NewProcessRunner(cfg.Runner)

	// Initialize API handlers
	handler := api.NewHandler(blobStore, processRunner)

	// Set up router with middleware
	r := chi.NewRouter()

	// Chi middleware stack
	r.Use(middleware.RequestID)
	r.Use(middleware.RealIP)
	r.Use(middleware.Logger)    // Logs all requests
	r.Use(middleware.Recoverer) // Recovers from panics
	r.Use(middleware.Timeout(30 * time.Second))

	// Register API routes
	handler.RegisterRoutes(r)

	// Create HTTP server
	server := &http.Server{
		Addr:         cfg.ServerAddr,
		Handler:      r,
		ReadTimeout:  15 * time.Second,
		WriteTimeout: 15 * time.Second,
		IdleTimeout:  60 * time.Second,
	}

	// Start server in goroutine
	go func() {
		log.Printf("Server listening on %s", cfg.ServerAddr)
		log.Println("Endpoints:")
		log.Println("  POST /deploy          - Deploy a new function")
		log.Println("  POST /execute/{id}    - Execute a function")
		log.Println("  GET  /health          - Health check")

		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("Server failed: %v", err)
		}
	}()

	// Wait for shutdown signal
	sig := <-sigChan
	log.Printf("Received signal %v, shutting down gracefully...", sig)

	// Create shutdown context with timeout
	shutdownCtx, shutdownCancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer shutdownCancel()

	// Attempt graceful shutdown
	if err := server.Shutdown(shutdownCtx); err != nil {
		log.Printf("Server forced to shutdown: %v", err)
	}

	log.Println("Server stopped")
}

// Config holds all configuration for the server.
type Config struct {
	ServerAddr string
	MinIO      store.BlobStoreConfig
	Runner     runner.ProcessRunnerConfig
}

// getEnv returns an environment variable or a default value.
func getEnv(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

// getRuntimePath finds the vortex-runtime binary.
// It looks in several locations:
// 1. VORTEX_RUNTIME_PATH environment variable
// 2. ../vortex-runtime/target/debug/vortex-runtime (development)
// 3. ../vortex-runtime/target/release/vortex-runtime (release)
// 4. ./vortex-runtime (current directory)
func getRuntimePath() string {
	// Check environment variable first
	if path := os.Getenv("VORTEX_RUNTIME_PATH"); path != "" {
		return path
	}

	// Get the directory of the current executable
	execPath, err := os.Executable()
	if err != nil {
		log.Printf("Warning: could not determine executable path: %v", err)
		return "./vortex-runtime"
	}
	execDir := filepath.Dir(execPath)

	// Check common development paths
	candidates := []string{
		filepath.Join(execDir, "..", "vortex-runtime", "target", "debug", "vortex-runtime"),
		filepath.Join(execDir, "..", "vortex-runtime", "target", "release", "vortex-runtime"),
		filepath.Join(execDir, "vortex-runtime"),
		"../vortex-runtime/target/debug/vortex-runtime",
		"../vortex-runtime/target/release/vortex-runtime",
		"./vortex-runtime",
	}

	for _, candidate := range candidates {
		if _, err := os.Stat(candidate); err == nil {
			absPath, _ := filepath.Abs(candidate)
			log.Printf("Found vortex-runtime at: %s", absPath)
			return candidate
		}
	}

	// Default to current directory
	log.Println("Warning: vortex-runtime not found, using default path ./vortex-runtime")
	return "./vortex-runtime"
}
