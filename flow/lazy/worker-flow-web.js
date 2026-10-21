// @ts-check
// Flow web runner, worker side: loads the `feat` .wasm of mzFinder and answers `stima`, the memory that aligning a stored series asks for (Rust `stima_series`, the
// rule of `slabs::stima`). Only the estimate lives here; the steps run in the pool of feat-worker.js. Used by flow-web-run.js.
/** @type {any} */ let mod = null;

/** @param {any} m the `wasm` entry of the pool: { jsUrl, wasmUrl, sha256? } */
async function load(m) {
  if (mod) return;
  const bytes = await (await fetch(m.wasmUrl)).arrayBuffer();
  if (m.sha256) {
    const h = [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))].map(b => b.toString(16).padStart(2, "0")).join("");
    if (h !== m.sha256) throw Object.assign(new Error("error.featWasmHash"), { key: "error.featWasmHash" });
  }
  const g = await import(m.jsUrl);
  await g.default({ module_or_path: bytes });
  mod = g;
}

/** an error of the .wasm is JSON { error_key, params } @param {any} e */
function describe(e) {
  const text = String(e && e.message || e);
  try { const j = JSON.parse(typeof e === "string" ? e : text); if (j && j.error_key) return { key: j.error_key, params: j.params || {}, text }; } catch (_) { /* not JSON */ }
  return { key: e && e.key || "error.featWorker", params: {}, text };
}

self.onmessage = async (/** @type {MessageEvent} */ ev) => {
  const m = ev.data;
  try {
    await load(m.wasm);
    const files = m.files.map((/** @type {ArrayBuffer} */ b) => new Uint8Array(b));
    self.postMessage({ id: m.id, ok: true, json: mod.stima_series(files, m.recipe || "", m.budgetMb || 0) });
  } catch (e) { self.postMessage({ id: m.id, error: describe(e) }); }
};
export {};
