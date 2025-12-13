// Package api provides HTTP handlers for the Vortex API.
package api

import (
	"encoding/json"
	"errors"
	"log"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"

	"github.com/vortex/vortex-api/internal/runner"
	"github.com/vortex/vortex-api/internal/store"
)

// Handler holds dependencies for the API handlers.
type Handler struct {
	Store  *store.BlobStore
	Runner *runner.ProcessRunner
}

// NewHandler creates a new Handler with the given dependencies.
func NewHandler(s *store.BlobStore, r *runner.ProcessRunner) *Handler {
	return &Handler{
		Store:  s,
		Runner: r,
	}
}

// RegisterRoutes sets up the API routes on the given router.
func (h *Handler) RegisterRoutes(r chi.Router) {
	r.Post("/deploy", h.HandleDeploy)
	r.Post("/execute/{functionID}", h.HandleExecute)
	r.Get("/health", h.HandleHealth)
}

// DeployRequest is the request body for POST /deploy.
type DeployRequest struct {
	Code string `json:"code"`
}

// DeployResponse is the response body for POST /deploy.
type DeployResponse struct {
	FunctionID string `json:"function_id"`
}

// HandleDeploy handles POST /deploy
//
// Validates the code, generates a UUID, and stores the function in MinIO.
func (h *Handler) HandleDeploy(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()

	// Parse request body
	var req DeployRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		WriteError(w, http.StatusBadRequest, "Invalid JSON body", err)
		return
	}

	// Validate code is not empty
	if req.Code == "" {
		WriteError(w, http.StatusBadRequest, "Code cannot be empty", nil)
		return
	}

	// Generate unique function ID
	functionID := uuid.New().String()

	// Store function in MinIO
	if err := h.Store.SaveFunction(ctx, functionID, req.Code); err != nil {
		WriteError(w, http.StatusInternalServerError, "Failed to store function", err)
		return
	}

	// Return success response
	WriteJSON(w, http.StatusCreated, DeployResponse{
		FunctionID: functionID,
	})

	log.Printf("Deployed function %s (%d bytes)", functionID, len(req.Code))
}

// ExecuteResponse is the response body for POST /execute/{functionID}.
type ExecuteResponse struct {
	Output          interface{}       `json:"output"`
	Logs            []runner.LogEntry `json:"logs"`
	ExecutionTimeMs uint64            `json:"execution_time_ms"`
}

// HandleExecute handles POST /execute/{functionID}
//
// Retrieves the function from MinIO and executes it through the Rust runtime.
func (h *Handler) HandleExecute(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()

	// Extract function ID from URL
	functionID := chi.URLParam(r, "functionID")
	if functionID == "" {
		WriteError(w, http.StatusBadRequest, "Missing function_id", nil)
		return
	}

	// Check if function exists
	exists, err := h.Store.FunctionExists(ctx, functionID)
	if err != nil {
		WriteError(w, http.StatusInternalServerError, "Failed to check function", err)
		return
	}
	if !exists {
		WriteError(w, http.StatusNotFound, "Function not found", nil)
		return
	}

	// Get function code from MinIO
	code, err := h.Store.GetFunction(ctx, functionID)
	if err != nil {
		WriteError(w, http.StatusInternalServerError, "Failed to retrieve function", err)
		return
	}

	// Execute through Rust runtime
	result, err := h.Runner.Execute(ctx, functionID, code)
	if err != nil {
		// Handle specific error types
		if errors.Is(err, runner.ErrCapacityExceeded) {
			WriteError(w, http.StatusServiceUnavailable, "Server at capacity, try again later", err)
			return
		}
		if errors.Is(err, runner.ErrTimeout) {
			WriteError(w, http.StatusGatewayTimeout, "Function execution timed out", err)
			return
		}
		WriteError(w, http.StatusInternalServerError, "Execution failed", err)
		return
	}

	// Return execution result
	WriteJSON(w, http.StatusOK, ExecuteResponse{
		Output:          result.Output,
		Logs:            result.Logs,
		ExecutionTimeMs: result.ExecutionTimeMs,
	})

	log.Printf("Executed function %s in %dms", functionID, result.ExecutionTimeMs)
}

// HealthResponse is the response for GET /health.
type HealthResponse struct {
	Status        string `json:"status"`
	ActiveWorkers int    `json:"active_workers"`
	MaxWorkers    int    `json:"max_workers"`
}

// HandleHealth returns the health status of the server.
func (h *Handler) HandleHealth(w http.ResponseWriter, r *http.Request) {
	WriteJSON(w, http.StatusOK, HealthResponse{
		Status:        "healthy",
		ActiveWorkers: h.Runner.CurrentWorkers(),
		MaxWorkers:    h.Runner.MaxWorkers(),
	})
}
