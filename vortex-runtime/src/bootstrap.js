// Vortex Runtime Bootstrap JavaScript
// This file is embedded into the V8 snapshot at compile time.

// Store the core ops reference for faster access
const ops = Deno.core.ops;

// Polyfill console object to capture logs via our custom op
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
globalThis.vortex = {
    version: '0.1.0',
    platform: 'vortex-runtime',
};

// Timer tracking
let __timerId = 0;
const __activeTimers = new Map();
const __timerCallbacks = new Map();

// setTimeout using PROPER async sleep (backed by tokio via op_sleep)
globalThis.setTimeout = (callback, delay = 0) => {
    const id = ++__timerId;

    const timerPromise = (async () => {
        await Deno.core.ops.op_sleep(BigInt(Math.max(0, delay)));
        __activeTimers.delete(id);
        __timerCallbacks.delete(id);
        if (typeof callback === 'function') {
            callback();
        }
    })();

    __activeTimers.set(id, timerPromise);
    __timerCallbacks.set(id, callback);
    return id;
};

globalThis.clearTimeout = (id) => {
    __activeTimers.delete(id);
    __timerCallbacks.delete(id);
};

// setInterval using proper async sleep
globalThis.setInterval = (callback, delay = 0) => {
    const id = ++__timerId;
    let running = true;

    const intervalLoop = async () => {
        while (running && __activeTimers.has(id)) {
            await Deno.core.ops.op_sleep(BigInt(Math.max(0, delay)));
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
delete globalThis.Deno;
