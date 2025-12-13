'use client';

import { Zap } from 'lucide-react';

/**
 * Navbar component with Vortex branding
 */
export function Navbar() {
    return (
        <header className="h-14 border-b border-border bg-card flex items-center px-4 gap-3">
            {/* Logo */}
            <div className="flex items-center gap-2">
                <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-violet-500 to-fuchsia-500 flex items-center justify-center">
                    <Zap className="w-5 h-5 text-white" />
                </div>
                <span className="text-lg font-semibold bg-gradient-to-r from-violet-400 to-fuchsia-400 bg-clip-text text-transparent">
                    Vortex
                </span>
            </div>

            {/* Tagline */}
            <span className="text-sm text-muted-foreground hidden sm:inline">
                Serverless JavaScript Runtime
            </span>

            {/* Spacer */}
            <div className="flex-1" />

            {/* Status indicator */}
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <span className="flex items-center gap-1.5">
                    <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
                    Backend Connected
                </span>
            </div>
        </header>
    );
}
