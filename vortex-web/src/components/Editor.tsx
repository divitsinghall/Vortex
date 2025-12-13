'use client';

import { useVortexStore } from '@/store/useVortexStore';
import dynamic from 'next/dynamic';

// Dynamically import Monaco to avoid SSR issues
const MonacoEditor = dynamic(
    () => import('@monaco-editor/react').then((mod) => mod.default),
    {
        ssr: false,
        loading: () => (
            <div className="flex items-center justify-center h-full bg-[#1e1e1e] text-muted-foreground">
                <div className="flex flex-col items-center gap-2">
                    <div className="w-6 h-6 border-2 border-violet-500 border-t-transparent rounded-full animate-spin" />
                    <span className="text-sm">Loading editor...</span>
                </div>
            </div>
        ),
    }
);

/**
 * Monaco Editor wrapper component
 * 
 * Uses VS Code's editor engine for a professional coding experience.
 * Configured with JavaScript as default language and dark theme.
 */
export function Editor() {
    const { code, setCode } = useVortexStore();

    return (
        <div className="h-full flex flex-col bg-[#1e1e1e]">
            {/* Editor header */}
            <div className="h-10 px-4 border-b border-border/50 flex items-center gap-2 bg-[#252526]">
                <div className="flex items-center gap-2">
                    <div className="flex gap-1.5">
                        <span className="w-3 h-3 rounded-full bg-red-500/80" />
                        <span className="w-3 h-3 rounded-full bg-yellow-500/80" />
                        <span className="w-3 h-3 rounded-full bg-green-500/80" />
                    </div>
                </div>
                <span className="text-sm text-muted-foreground ml-2">main.js</span>
                <div className="flex-1" />
                <span className="text-xs text-muted-foreground">JavaScript</span>
            </div>

            {/* Monaco Editor */}
            <div className="flex-1">
                <MonacoEditor
                    height="100%"
                    defaultLanguage="javascript"
                    theme="vs-dark"
                    value={code}
                    onChange={(value) => setCode(value ?? '')}
                    options={{
                        minimap: { enabled: false },
                        fontSize: 14,
                        fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
                        fontLigatures: true,
                        lineNumbers: 'on',
                        rulers: [],
                        renderWhitespace: 'selection',
                        scrollBeyondLastLine: false,
                        automaticLayout: true,
                        tabSize: 2,
                        wordWrap: 'on',
                        padding: { top: 16 },
                        smoothScrolling: true,
                        cursorBlinking: 'smooth',
                        cursorSmoothCaretAnimation: 'on',
                    }}
                />
            </div>
        </div>
    );
}
