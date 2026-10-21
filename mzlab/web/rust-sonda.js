// @ts-check
// Growth probe of the memory the browser grants a WebAssembly instance: a throwaway worker (this same file) makes a `WebAssembly.Memory` with the limit of
// the build and grows it in steps of 256 MB up to 3072 MB or to the first refusal, then closes: a probe inside the engine would leave it gigabytes that
// WASM memory never gives back. The pages are not written: it measures what the browser allows, not the free RAM. Used by rust-budget.js.

const STEP_PAGES = 4096, STEPS = 12, STEP_MB = 256;

/** What the browser lets one memory grow to (MB) and how long that took. */
export function measure() {
  const t = performance.now();
  let mb = 0;
  try {
    const mem = new WebAssembly.Memory({ initial: 1, maximum: 65536 });
    for (let i = 0; i < STEPS; i++) { mem.grow(STEP_PAGES); mb += STEP_MB; }
  } catch (_) { /* the first refusal ends the probe */ }
  return { mb, ms: Math.round(performance.now() - t) };
}

// In the worker the file answers the first message and closes.
const scope = /** @type {any} */ (self);
if (typeof scope.WorkerGlobalScope !== "undefined" && self instanceof scope.WorkerGlobalScope) {
  scope.onmessage = () => { scope.postMessage(measure()); scope.close(); };
}

/**
 * Runs the probe in a worker that is thrown away; null when it cannot run or takes longer than `timeoutMs` (the table of the devices then applies).
 * @param {number} [timeoutMs]
 * @returns {Promise<{ mb: number, ms: number } | null>}
 */
export function probe(timeoutMs = 2000) {
  return new Promise(resolve => {
    /** @type {Worker | null} */ let w = null;
    const end = (/** @type {{ mb: number, ms: number } | null} */ r) => { clearTimeout(timer); if (w) w.terminate(); w = null; resolve(r); };
    const timer = setTimeout(() => end(null), timeoutMs);
    try {
      w = new Worker(new URL("./rust-sonda.js", import.meta.url), { type: "module" });
      w.onmessage = e => end(e.data && typeof e.data.mb === "number" ? e.data : null);
      w.onerror = () => end(null);
      w.postMessage(0);
    } catch (_) { end(null); }
  });
}
