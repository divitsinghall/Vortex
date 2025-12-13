'use client';

import { create } from 'zustand';

/**
 * Log entry from the vortex-runtime
 */
export interface LogEntry {
    timestamp: string;
    message: string;
}

/**
 * Execution status
 */
export type ExecutionStatus = 'idle' | 'deploying' | 'running' | 'success' | 'error';

/**
 * Vortex IDE state
 */
interface VortexState {
    // Editor content
    code: string;
    setCode: (code: string) => void;

    // Execution results
    logs: LogEntry[];
    output: unknown;
    executionTime: number | null;
    error: string | null;
    status: ExecutionStatus;

    // Actions
    startExecution: () => void;
    setDeploying: () => void;
    setRunning: () => void;
    setSuccess: (result: { logs: LogEntry[]; output: unknown; executionTime: number }) => void;
    setError: (error: string) => void;
    reset: () => void;
}

const DEFAULT_CODE = `// Welcome to Vortex! 🌀
// Write your JavaScript function below.
// Use console.log() for logging and 'return' to output a value.

console.log("Hello, Vortex!");

const result = {
  message: "Hello from the edge!",
  timestamp: new Date().toISOString(),
  computation: 21 * 2
};

console.log("Computed result:", result);

return result;
`;

/**
 * Zustand store for Vortex IDE state management
 */
export const useVortexStore = create<VortexState>((set) => ({
    // Initial state
    code: DEFAULT_CODE,
    logs: [],
    output: null,
    executionTime: null,
    error: null,
    status: 'idle',

    // Actions
    setCode: (code) => set({ code }),

    startExecution: () => set({
        status: 'deploying',
        logs: [],
        output: null,
        executionTime: null,
        error: null,
    }),

    setDeploying: () => set({ status: 'deploying' }),

    setRunning: () => set({ status: 'running' }),

    setSuccess: ({ logs, output, executionTime }) => set({
        status: 'success',
        logs,
        output,
        executionTime,
        error: null,
    }),

    setError: (error) => set({
        status: 'error',
        error,
    }),

    reset: () => set({
        logs: [],
        output: null,
        executionTime: null,
        error: null,
        status: 'idle',
    }),
}));
