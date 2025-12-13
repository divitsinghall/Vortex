// Package store provides the storage layer for Vortex functions.
//
// This package wraps the MinIO Go SDK to provide a simple interface for
// storing and retrieving JavaScript function code from S3-compatible storage.
package store

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"log"
	"time"

	"github.com/minio/minio-go/v7"
	"github.com/minio/minio-go/v7/pkg/credentials"
)

// BlobStore wraps a MinIO client to store and retrieve function code.
//
// It handles:
// - Connection retries during MinIO startup
// - Automatic bucket verification
// - Simple string-based storage/retrieval
type BlobStore struct {
	client     *minio.Client
	bucketName string
}

// BlobStoreConfig holds configuration for connecting to MinIO.
type BlobStoreConfig struct {
	Endpoint        string
	AccessKeyID     string
	SecretAccessKey string
	BucketName      string
	UseSSL          bool
}

// NewBlobStore creates a new BlobStore with connection retry logic.
//
// The retry logic is crucial because in containerized environments,
// MinIO may not be immediately available when our service starts.
// We implement exponential backoff to handle this gracefully.
func NewBlobStore(ctx context.Context, cfg BlobStoreConfig) (*BlobStore, error) {
	var client *minio.Client
	var err error

	// Retry with exponential backoff: 1s, 2s, 4s, 8s, 16s
	// This handles the case where MinIO is still starting up
	maxRetries := 5
	for i := 0; i < maxRetries; i++ {
		client, err = minio.New(cfg.Endpoint, &minio.Options{
			Creds:  credentials.NewStaticV4(cfg.AccessKeyID, cfg.SecretAccessKey, ""),
			Secure: cfg.UseSSL,
		})
		if err != nil {
			backoff := time.Duration(1<<i) * time.Second
			log.Printf("Failed to create MinIO client (attempt %d/%d): %v. Retrying in %v...",
				i+1, maxRetries, err, backoff)
			select {
			case <-ctx.Done():
				return nil, ctx.Err()
			case <-time.After(backoff):
				continue
			}
		}

		// Verify connection by checking if bucket exists
		exists, err := client.BucketExists(ctx, cfg.BucketName)
		if err != nil {
			backoff := time.Duration(1<<i) * time.Second
			log.Printf("Cannot reach MinIO (attempt %d/%d): %v. Retrying in %v...",
				i+1, maxRetries, err, backoff)
			select {
			case <-ctx.Done():
				return nil, ctx.Err()
			case <-time.After(backoff):
				continue
			}
		}

		if !exists {
			// Bucket should be created by docker-compose, but create if missing
			log.Printf("Bucket %s does not exist, creating...", cfg.BucketName)
			err = client.MakeBucket(ctx, cfg.BucketName, minio.MakeBucketOptions{})
			if err != nil {
				return nil, fmt.Errorf("failed to create bucket: %w", err)
			}
		}

		log.Printf("Connected to MinIO successfully. Bucket: %s", cfg.BucketName)
		return &BlobStore{
			client:     client,
			bucketName: cfg.BucketName,
		}, nil
	}

	return nil, fmt.Errorf("failed to connect to MinIO after %d retries: %w", maxRetries, err)
}

// SaveFunction stores JavaScript code in MinIO with the given function ID.
//
// The function ID is used to construct the object key: functions/{id}.js
// Returns an error if the upload fails.
func (s *BlobStore) SaveFunction(ctx context.Context, functionID, code string) error {
	objectName := fmt.Sprintf("functions/%s.js", functionID)
	reader := bytes.NewReader([]byte(code))

	_, err := s.client.PutObject(
		ctx,
		s.bucketName,
		objectName,
		reader,
		int64(len(code)),
		minio.PutObjectOptions{
			ContentType: "application/javascript",
		},
	)
	if err != nil {
		return fmt.Errorf("failed to save function %s: %w", functionID, err)
	}

	log.Printf("Saved function %s (%d bytes)", functionID, len(code))
	return nil
}

// GetFunction retrieves JavaScript code from MinIO by function ID.
//
// Returns the code as a string, or an error if the function doesn't exist.
func (s *BlobStore) GetFunction(ctx context.Context, functionID string) (string, error) {
	objectName := fmt.Sprintf("functions/%s.js", functionID)

	obj, err := s.client.GetObject(ctx, s.bucketName, objectName, minio.GetObjectOptions{})
	if err != nil {
		return "", fmt.Errorf("failed to get function %s: %w", functionID, err)
	}
	defer obj.Close()

	data, err := io.ReadAll(obj)
	if err != nil {
		return "", fmt.Errorf("failed to read function %s: %w", functionID, err)
	}

	return string(data), nil
}

// FunctionExists checks if a function exists in MinIO.
func (s *BlobStore) FunctionExists(ctx context.Context, functionID string) (bool, error) {
	objectName := fmt.Sprintf("functions/%s.js", functionID)

	_, err := s.client.StatObject(ctx, s.bucketName, objectName, minio.StatObjectOptions{})
	if err != nil {
		// Check if it's a "not found" error
		errResponse := minio.ToErrorResponse(err)
		if errResponse.Code == "NoSuchKey" {
			return false, nil
		}
		return false, fmt.Errorf("failed to check function %s: %w", functionID, err)
	}

	return true, nil
}
