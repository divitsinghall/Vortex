package api

import (
	"encoding/json"
	"log"
	"net/http"
)

// ErrorResponse is a structured error response for the API.
type ErrorResponse struct {
	Error   string `json:"error"`
	Message string `json:"message,omitempty"`
}

// WriteJSON writes a JSON response with the given status code.
func WriteJSON(w http.ResponseWriter, status int, data interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	if err := json.NewEncoder(w).Encode(data); err != nil {
		log.Printf("Failed to encode JSON response: %v", err)
	}
}

// WriteError writes a structured error response.
func WriteError(w http.ResponseWriter, status int, message string, err error) {
	resp := ErrorResponse{
		Error:   http.StatusText(status),
		Message: message,
	}

	// Log the full error for debugging, but don't expose to client
	if err != nil {
		log.Printf("API Error [%d]: %s - %v", status, message, err)
	} else {
		log.Printf("API Error [%d]: %s", status, message)
	}

	WriteJSON(w, status, resp)
}
