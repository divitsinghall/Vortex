//! JavaScript bootstrap code that runs before user scripts.
//!
//! This module provides the initialization JavaScript that:
//! - Polyfills `console.log` to route through our `op_log` operation
//! - Sets up the global `vortex` object for future API extensions
//! - Provides a `setTimeout` polyfill for async operations

/// Bootstrap JavaScript code that initializes the runtime environment.
///
/// This code runs once when a VortexWorker is created, before any user code executes.
/// It establishes the bridge between JavaScript's standard APIs and our Rust operations.
pub const BOOTSTRAP_JS: &str = r#"
// Store the core ops reference for faster access
const ops = Deno.core.ops;

// Polyfill console object to capture logs via our custom op
// The real console.log would print to stdout, but we need to capture it
globalThis.console = {
  log: (...args) => {
    const message = args.map(arg => {
      if (arg === null) return 'null';
      if (arg === undefined) return 'undefined';
      if (typeof arg === 'object') {
        try {
          return JSON.stringify(arg);
        } catch (e) {
          return String(arg);
        }
      }
      return String(arg);
    }).join(' ');
    ops.op_log(message);
  },
  error: (...args) => {
    globalThis.console.log('[ERROR]', ...args);
  },
  warn: (...args) => {
    globalThis.console.log('[WARN]', ...args);
  },
  info: (...args) => {
    globalThis.console.log('[INFO]', ...args);
  },
  debug: (...args) => {
    globalThis.console.log('[DEBUG]', ...args);
  }
};

// Global vortex object for future API extensions
// This will be expanded as we add more platform capabilities
globalThis.vortex = {
  version: '0.1.0',
  platform: 'vortex-runtime',
  
  // Future: KV storage, Durable Objects, etc.
};

// Internal timer tracking for setTimeout implementation
let __timerId = 0;
const __activeTimers = new Map();

// setTimeout polyfill that works with our async runtime
// We use a promise-based approach that polls the current time
globalThis.setTimeout = (callback, delay = 0) => {
  const id = ++__timerId;
  const startTime = ops.op_get_time_ms();
  
  const timerPromise = (async () => {
    while (true) {
      const elapsed = ops.op_get_time_ms() - startTime;
      if (elapsed >= delay) {
        __activeTimers.delete(id);
        if (typeof callback === 'function') {
          callback();
        }
        return;
      }
      // Yield to the event loop
      await Promise.resolve();
    }
  })();
  
  __activeTimers.set(id, timerPromise);
  return id;
};

// clearTimeout implementation
globalThis.clearTimeout = (id) => {
  __activeTimers.delete(id);
};

// setInterval polyfill (limited implementation for basic use)
globalThis.setInterval = (callback, delay = 0) => {
  const id = ++__timerId;
  let running = true;
  
  const intervalLoop = async () => {
    while (running && __activeTimers.has(id)) {
      const startTime = ops.op_get_time_ms();
      while (ops.op_get_time_ms() - startTime < delay) {
        await Promise.resolve();
      }
      if (running && __activeTimers.has(id) && typeof callback === 'function') {
        callback();
      }
    }
  };
  
  __activeTimers.set(id, { running: true });
  intervalLoop();
  return id;
};

globalThis.clearInterval = (id) => {
  const timer = __activeTimers.get(id);
  if (timer) {
    timer.running = false;
    __activeTimers.delete(id);
  }
};

// Prevent access to potentially dangerous globals
// These would allow escaping the sandbox
delete globalThis.Deno;
"#;
