'use client';

import { useCallback } from 'react';
import axios, { AxiosError } from 'axios';
import { useVortexStore, LogEntry } from '@/store/useVortexStore';

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
 * Implements the Two-Step Deploy→Execute flow:
 * 1. POST /api/deploy - Deploy the code, receive function_id
 * 2. POST /api/execute/{function_id} - Execute and get results
 * 
 * This abstraction makes the two API calls feel like a single "Run" action
 * to the user, providing seamless execution experience.
 */
export function useRunFunction() {
    const {
        code,
        status,
        startExecution,
        setRunning,
        setSuccess,
        setError,
    } = useVortexStore();

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

            // Update status to show we're now executing
            setRunning();

            // Step 2: EXECUTE - Run the deployed function
            // This invokes the Rust runtime to execute the JavaScript
            const executeResponse = await axios.post<ExecuteResponse>(
                `/api/execute/${function_id}`
            );

            const { output, logs, execution_time_ms } = executeResponse.data;

            // Step 3: Update store with results
            setSuccess({
                logs,
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
    }, [code, isRunning, startExecution, setRunning, setSuccess, setError]);

    return {
        run,
        isRunning,
    };
}
