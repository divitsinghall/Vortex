'use client';

import { Navbar } from '@/components/Navbar';
import { Editor } from '@/components/Editor';
import { Console } from '@/components/Console';
import {
  ResizablePanelGroup,
  ResizablePanel,
  ResizableHandle,
} from '@/components/ui/resizable';

/**
 * Main Vortex IDE Page
 * 
 * Split-pane layout with:
 * - Left: Monaco Code Editor
 * - Right: Output Console
 */
export default function Home() {
  return (
    <main className="h-screen flex flex-col bg-background overflow-hidden">
      {/* Navbar */}
      <Navbar />

      {/* IDE Split Layout */}
      <div className="flex-1 overflow-hidden">
        <ResizablePanelGroup direction="horizontal">
          {/* Left Panel - Code Editor */}
          <ResizablePanel defaultSize={55} minSize={30}>
            <Editor />
          </ResizablePanel>

          {/* Resizable Handle */}
          <ResizableHandle withHandle className="bg-border/50 hover:bg-violet-500/50 transition-colors" />

          {/* Right Panel - Console Output */}
          <ResizablePanel defaultSize={45} minSize={25}>
            <Console />
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>
    </main>
  );
}
