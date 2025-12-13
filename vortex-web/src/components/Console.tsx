'use client';

import { Play, CheckCircle2, XCircle, Loader2, Terminal, Clock, Braces } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useVortexStore, ExecutionStatus } from '@/store/useVortexStore';
import { useRunFunction } from '@/hooks/useRunFunction';

/**
 * Status badge component for showing execution state
 */
function StatusBadge({ status }: { status: ExecutionStatus }) {
    const configs: Record<ExecutionStatus, { label: string; className: string; icon: React.ReactNode }> = {
        idle: {
            label: 'Ready',
            className: 'bg-muted text-muted-foreground',
            icon: <Terminal className="w-3 h-3" />,
        },
        deploying: {
            label: 'Deploying...',
            className: 'bg-violet-500/20 text-violet-400',
            icon: <Loader2 className="w-3 h-3 animate-spin" />,
        },
        running: {
            label: 'Running...',
            className: 'bg-blue-500/20 text-blue-400',
            icon: <Loader2 className="w-3 h-3 animate-spin" />,
        },
        success: {
            label: 'Success',
            className: 'bg-emerald-500/20 text-emerald-400',
            icon: <CheckCircle2 className="w-3 h-3" />,
        },
        error: {
            label: 'Error',
            className: 'bg-red-500/20 text-red-400',
            icon: <XCircle className="w-3 h-3" />,
        },
    };

    const config = configs[status];

    return (
        <span className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium ${config.className}`}>
            {config.icon}
            {config.label}
        </span>
    );
}

/**
 * Format JSON output with syntax highlighting
 */
function JsonOutput({ data }: { data: unknown }) {
    if (data === null || data === undefined) {
        return <span className="text-muted-foreground">null</span>;
    }

    const formatted = JSON.stringify(data, null, 2);

    return (
        <pre className="text-sm font-mono whitespace-pre-wrap break-all">
            <code className="text-emerald-400">{formatted}</code>
        </pre>
    );
}

/**
 * Console component - Terminal-like output display
 * 
 * Features:
 * - Run button with glow effect during execution
 * - Status indicators
 * - Timestamped logs with color coding
 * - JSON tree view for return values  
 * - Execution metrics
 */
export function Console() {
    const { logs, output, executionTime, error, status } = useVortexStore();
    const { run, isRunning } = useRunFunction();

    return (
        <div className="h-full flex flex-col bg-card">
            {/* Console header */}
            <div className="h-14 px-4 border-b border-border flex items-center justify-between gap-4">
                <div className="flex items-center gap-3">
                    <h2 className="text-sm font-semibold flex items-center gap-2">
                        <Terminal className="w-4 h-4 text-violet-400" />
                        Output
                    </h2>
                    <StatusBadge status={status} />
                </div>

                {/* Run button with glow effect */}
                <Button
                    onClick={run}
                    disabled={isRunning}
                    size="sm"
                    className={`
            relative overflow-hidden
            bg-gradient-to-r from-violet-600 to-fuchsia-600
            hover:from-violet-500 hover:to-fuchsia-500
            text-white font-medium
            transition-all duration-300
            ${isRunning ? 'animate-pulse shadow-lg shadow-violet-500/50' : ''}
          `}
                >
                    {isRunning ? (
                        <>
                            <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                            Running...
                        </>
                    ) : (
                        <>
                            <Play className="w-4 h-4 mr-2" />
                            Run
                        </>
                    )}

                    {/* Glow effect overlay */}
                    {isRunning && (
                        <span className="absolute inset-0 bg-gradient-to-r from-violet-400/30 to-fuchsia-400/30 animate-pulse" />
                    )}
                </Button>
            </div>

            {/* Console output */}
            <div className="flex-1 overflow-auto p-4 space-y-4 font-mono text-sm">
                {/* Error message */}
                {error && (
                    <div className="p-3 rounded-lg bg-red-500/10 border border-red-500/20">
                        <div className="flex items-start gap-2">
                            <XCircle className="w-4 h-4 text-red-400 mt-0.5 flex-shrink-0" />
                            <div>
                                <p className="text-red-400 font-medium">Execution Failed</p>
                                <p className="text-red-300/80 mt-1">{error}</p>
                            </div>
                        </div>
                    </div>
                )}

                {/* Logs section */}
                {logs.length > 0 && (
                    <div className="space-y-1">
                        <div className="flex items-center gap-2 text-muted-foreground mb-2">
                            <Terminal className="w-3.5 h-3.5" />
                            <span className="text-xs font-medium uppercase tracking-wider">Console Output</span>
                        </div>
                        <div className="space-y-1 pl-1">
                            {logs.map((log, index) => (
                                <div
                                    key={index}
                                    className="flex items-start gap-2 text-sm group"
                                >
                                    <span className="text-muted-foreground/60 text-xs font-mono min-w-[80px] pt-0.5">
                                        {log.timestamp}
                                    </span>
                                    <span className="text-emerald-400 whitespace-pre-wrap break-all">
                                        {log.message}
                                    </span>
                                </div>
                            ))}
                        </div>
                    </div>
                )}

                {/* Output section */}
                {output !== null && output !== undefined && (
                    <div className="space-y-2">
                        <div className="flex items-center gap-2 text-muted-foreground">
                            <Braces className="w-3.5 h-3.5" />
                            <span className="text-xs font-medium uppercase tracking-wider">Return Value</span>
                        </div>
                        <div className="p-3 rounded-lg bg-muted/30 border border-border/50">
                            <JsonOutput data={output} />
                        </div>
                    </div>
                )}

                {/* Execution time */}
                {executionTime !== null && (
                    <div className="flex items-center gap-2 text-muted-foreground pt-2 border-t border-border/50">
                        <Clock className="w-3.5 h-3.5" />
                        <span className="text-xs">
                            Executed in <span className="text-foreground font-medium">{executionTime}ms</span>
                        </span>
                    </div>
                )}

                {/* Empty state */}
                {status === 'idle' && logs.length === 0 && !error && (
                    <div className="flex flex-col items-center justify-center h-full text-center text-muted-foreground/60">
                        <Terminal className="w-12 h-12 mb-4 opacity-30" />
                        <p className="text-sm">Click <span className="text-violet-400 font-medium">Run</span> to execute your code</p>
                        <p className="text-xs mt-1">Output will appear here</p>
                    </div>
                )}
            </div>
        </div>
    );
}
