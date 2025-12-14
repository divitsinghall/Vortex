'use client';

import { useEffect, useRef, useCallback } from 'react';
import { useVortexStore, LogEntry } from '@/store/useVortexStore';

/**
 * Hook for real-time log streaming via WebSocket
 *
 * Connects to ws://localhost:8080/ws/{functionId} when a function is running
 * and appends incoming log messages to the console in real-time.
 *
 * # Race Condition Note (MVP Acceptable)
 *
 * The WebSocket connection might be established AFTER the function has started
 * executing. This means the client may miss the first few log messages.
 * This is acceptable for MVP as it still provides value for longer-running functions.
 *
 * @param functionId - The function ID to stream logs for, or null if not streaming
 */
export function useLogStream(functionId: string | null) {
    const wsRef = useRef<WebSocket | null>(null);
    const { appendLog, status } = useVortexStore();

    useEffect(() => {
        // Only connect when we have a function ID and the function is running
        if (!functionId || status !== 'running') {
            return;
        }

        // Construct WebSocket URL
        // In production, this would use a secure WebSocket (wss://) and proper host
        const wsUrl = `ws://localhost:8080/ws/${functionId}`;

        console.log(`[useLogStream] Connecting to ${wsUrl}`);
        const ws = new WebSocket(wsUrl);
        wsRef.current = ws;

        ws.onopen = () => {
            console.log(`[useLogStream] Connected to log stream for ${functionId}`);
        };

        ws.onmessage = (event) => {
            try {
                // Parse the log entry JSON from the Rust runtime
                const logEntry: LogEntry = JSON.parse(event.data);
                // Append to the console output in real-time
                appendLog(logEntry);
            } catch (e) {
                console.error('[useLogStream] Failed to parse log message:', e);
            }
        };

        ws.onerror = (error) => {
            console.error('[useLogStream] WebSocket error:', error);
        };

        ws.onclose = (event) => {
            console.log(`[useLogStream] Disconnected from log stream (code: ${event.code})`);
        };

        // Cleanup on unmount or when dependencies change
        return () => {
            if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
                console.log('[useLogStream] Closing WebSocket connection');
                ws.close();
            }
            wsRef.current = null;
        };
    }, [functionId, status, appendLog]);

    /**
     * Manually disconnect the WebSocket
     */
    const disconnect = useCallback(() => {
        if (wsRef.current) {
            wsRef.current.close();
            wsRef.current = null;
        }
    }, []);

    return { disconnect };
}
