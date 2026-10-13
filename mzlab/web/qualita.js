// @ts-check
// @ts-strict
// Peak measures from the Rust engine (crates/mzlab-core/src/peakq.rs): quality of an integrated peak and the two methods of finding peaks.
// Loaded as a module after the classic scripts; picchi.js uses window.QUALITA. Every call resolves to null when the .wasm is not there
// (the local development server does not build it) or the worker dies: the page then shows an empty cell, never an error.
const URL_WORKER = new URL("peaks-worker.js", import.meta.url);
let worker = null, dead = false, seq = 0;
const pending = new Map();

function start() {
  if (worker || dead) return;
  try {
    worker = new Worker(URL_WORKER, { type: "module" });
  } catch (_) { dead = true; return; }
  worker.onmessage = ev => { const p = pending.get(ev.data.id); if (p) { pending.delete(ev.data.id); p(ev.data.error ? null : ev.data.v); } };
  worker.onerror = () => { dead = true; worker = null; for (const p of pending.values()) p(null); pending.clear(); };
}

function call(msg) {
  start();
  return new Promise(res => {
    if (!worker) return res(null);
    const id = ++seq;
    pending.set(id, res);
    worker.postMessage({ ...msg, id });
  });
}

const f64 = a => Float64Array.from(a);

window.QUALITA = {
  /** [gaussian similarity, asymmetry at 10 % (null if not measurable), noise score] of the peak between the indices lo and hi of the trace (x in minutes), or null. */
  async quality(x, y, lo, hi) {
    const v = await call({ op: "quality", rt: f64(x), y: f64(y), lo, hi });
    return v ? { gaussian: v[0], asymmetry: Number.isNaN(v[1]) ? null : v[1], noise: v[2] } : null;
  },
  /** Peaks of the trace: mode 0 = CWT, 1 = prominence + local minimum. Each: { apex, lo, hi, area, height, snr, n, gaussian, asymmetry, noise } (indices into the trace), or null. */
  detect(x, y, mode) { return call({ op: "detect", rt: f64(x), y: f64(y), mode }); },
  /** True when the engine can answer. */
  async available() { return (await call({ op: "quality", rt: f64([0, 1, 2]), y: f64([0, 1, 0]), lo: 0, hi: 2 })) !== null; },
};
