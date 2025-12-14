'use client';

import { useCallback } from 'react';
import axios, { AxiosError } from 'axios';
import { useVortexStore, LogEntry } from '@/store/useVortexStore';
import { useLogStream } from './useLogStream';

/**
 * API response types matching the Go backend
 */
interface DeployResponse {
    function_id: string;
}

interface ExecuteResponse {
    output: unknown;
    logs: LogEntry[];
    execution_time_ms: number;
}

interface ApiError {
    error: string;
    message: string;
}

/**
 * Custom hook for running functions through the Vortex platform.
 * 
 * Implements the Two-Step Deploy→Execute flow with real-time log streaming:
 * 1. POST /api/deploy - Deploy the code, receive function_id
 * 2. Connect WebSocket for real-time logs
 * 3. POST /api/execute/{function_id} - Execute and get results
 * 
 * This abstraction makes the two API calls feel like a single "Run" action
 * to the user, providing seamless execution experience with real-time feedback.
 */
export function useRunFunction() {
    const {
        code,
        status,
        functionId,
        startExecution,
        setRunning,
        setFunctionId,
        setSuccess,
        setError,
    } = useVortexStore();

    // Real-time log streaming via WebSocket
    // This hook connects to ws://localhost:8080/ws/{functionId} when status is 'running'
    useLogStream(functionId);

    const isRunning = status === 'deploying' || status === 'running';

    const run = useCallback(async () => {
        if (isRunning) return;

        // Step 0: Reset state and start execution
        startExecution();

        try {
            // Step 1: DEPLOY - Upload code to the platform
            // This stores the function in MinIO and returns a unique function_id
            const deployResponse = await axios.post<DeployResponse>('/api/deploy', {
                code,
            });

            const { function_id } = deployResponse.data;

            // Store the function ID for WebSocket connection
            // This will trigger the useLogStream hook to connect
            setFunctionId(function_id);

            // Update status to show we're now executing
            // This also triggers the WebSocket connection in useLogStream
            setRunning();

            // Step 2: EXECUTE - Run the deployed function
            // While this is running, logs are streamed in real-time via WebSocket
            // The Rust runtime publishes to Redis, Go forwards to WebSocket
            const executeResponse = await axios.post<ExecuteResponse>(
                `/api/execute/${function_id}`
            );

            const { output, execution_time_ms } = executeResponse.data;

            // Step 3: Update store with results
            // Note: We preserve the real-time streamed logs instead of replacing
            // with the final batch. The streamed logs should be identical,
            // but we keep what we already have for better UX.
            setSuccess({
                logs: [], // Keep existing streamed logs, don't overwrite
                output,
                executionTime: execution_time_ms,
            });
        } catch (err) {
            // Handle errors gracefully
            const axiosError = err as AxiosError<ApiError>;

            let errorMessage = 'An unexpected error occurred';

            if (axiosError.response?.data?.message) {
                // API returned a structured error
                errorMessage = axiosError.response.data.message;
            } else if (axiosError.response?.status === 503) {
                errorMessage = 'Server at capacity. Please try again later.';
            } else if (axiosError.response?.status === 504) {
                errorMessage = 'Function execution timed out.';
            } else if (axiosError.code === 'ERR_NETWORK') {
                errorMessage = 'Cannot connect to the Vortex API. Is the backend running?';
            } else if (axiosError.message) {
                errorMessage = axiosError.message;
            }

            setError(errorMessage);
        }
    }, [code, isRunning, startExecution, setRunning, setFunctionId, setSuccess, setError]);

    return {
        run,
        isRunning,
    };
}

