// Package ws provides WebSocket handlers for real-time log streaming.
//
// This package implements the WebSocket endpoint that allows clients to
// subscribe to real-time log streams from function executions. It uses
// Redis Pub/Sub to receive log messages from the Rust runtime and forwards
// them to connected WebSocket clients.
package ws

import (
	"context"
	"log"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/gorilla/websocket"
	"github.com/redis/go-redis/v9"
)

// Handler manages WebSocket connections for log streaming.
type Handler struct {
	// Redis client for subscribing to log channels
	Redis *redis.Client
	// WebSocket upgrader configuration
	Upgrader websocket.Upgrader
}

// NewHandler creates a new WebSocket handler with the given Redis client.
func NewHandler(redisClient *redis.Client) *Handler {
	return &Handler{
		Redis: redisClient,
		Upgrader: websocket.Upgrader{
			// Allow all origins for development
			// TODO: Restrict to specific origins in production
			CheckOrigin: func(r *http.Request) bool {
				return true
			},
			// Buffer sizes for reading/writing
			ReadBufferSize:  1024,
			WriteBufferSize: 1024,
		},
	}
}

// RegisterRoutes registers the WebSocket routes on the given router.
func (h *Handler) RegisterRoutes(r chi.Router) {
	r.Get("/ws/{functionID}", h.HandleLogStream)
}

// HandleLogStream handles GET /ws/{functionID}
//
// This endpoint upgrades the HTTP connection to a WebSocket and subscribes
// to the Redis channel `logs:{functionID}`. All messages published to that
// channel are forwarded to the WebSocket client.
//
// # Race Condition Note (MVP Acceptable)
//
// There is an inherent race condition: the WebSocket connection might be
// established AFTER the function has started executing. This means the
// client may miss the first few log messages. This is acceptable for MVP
// as it still provides value for longer-running functions.
//
// Potential solutions for future versions:
// - Buffer recent logs in Redis with TTL
// - Store logs in a list and replay on connect
// - Use Redis Streams instead of Pub/Sub
//
// # Connection Lifecycle
//
//  1. Upgrade HTTP to WebSocket
//  2. Subscribe to Redis channel `logs:{functionID}`
//  3. Forward messages from Redis to WebSocket
//  4. On disconnect: close Redis subscription, close WebSocket
func (h *Handler) HandleLogStream(w http.ResponseWriter, r *http.Request) {
	functionID := chi.URLParam(r, "functionID")
	if functionID == "" {
		http.Error(w, "Missing function_id", http.StatusBadRequest)
		return
	}

	// Upgrade HTTP connection to WebSocket
	conn, err := h.Upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("WebSocket upgrade failed for function %s: %v", functionID, err)
		return
	}
	defer conn.Close()

	log.Printf("WebSocket connected for function %s", functionID)

	// Create a context that cancels when the connection closes
	ctx, cancel := context.WithCancel(r.Context())
	defer cancel()

	// Subscribe to the Redis channel for this function's logs
	channel := "logs:" + functionID
	pubsub := h.Redis.Subscribe(ctx, channel)
	defer func() {
		if err := pubsub.Close(); err != nil {
			log.Printf("Error closing Redis subscription for %s: %v", functionID, err)
		}
	}()

	// Start a goroutine to detect WebSocket disconnection
	// This is needed because the pubsub.Channel() loop blocks
	go func() {
		for {
			// Read from WebSocket to detect disconnection
			_, _, err := conn.ReadMessage()
			if err != nil {
				log.Printf("WebSocket disconnected for function %s: %v", functionID, err)
				cancel() // This will cause pubsub.Channel() to close
				return
			}
		}
	}()

	// Get the message channel from the subscription
	ch := pubsub.Channel()

	// Forward messages from Redis to WebSocket
	for {
		select {
		case <-ctx.Done():
			log.Printf("Context cancelled for function %s, closing stream", functionID)
			return
		case msg, ok := <-ch:
			if !ok {
				log.Printf("Redis channel closed for function %s", functionID)
				return
			}
			// Write the log message to the WebSocket
			if err := conn.WriteMessage(websocket.TextMessage, []byte(msg.Payload)); err != nil {
				log.Printf("WebSocket write failed for function %s: %v", functionID, err)
				return
			}
		}
	}
}
