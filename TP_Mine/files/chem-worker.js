// @ts-check
// TPMINE-PRIVATE  Module Worker of the reactivity of the parent (crates/mzlab-chem-wasm, wasm-feat/chem/; docs/agenti/reattivita.md). The .wasm `chem` is fetched and compiled on the first
// request only (never at start), verified against its SHA-256 of wasm-feat/manifest.json, and `reactivity(smiles, opts)` runs here, away from the page. One calculation at a time:
// the page (lazy/reatt-auto.js) terminates this worker to cancel a calculation that is half done and creates a new one.
// Messages: { id, smiles, opts, jsUrl, wasmUrl, sha256 } -> { id, ok: true, json, version, ms } | { id, error: { error_key, params } }.
/** @type {any} */ let mod = null;
/** @type {Promise<any>|null} */ let loading = null;

/** @param {string} jsUrl @param {string} wasmUrl @param {string} sha256 */
async function load(jsUrl, wasmUrl, sha256) {
  if (mod) return mod;
  if (!loading) loading = (async () => {
    const bytes = await (await fetch(wasmUrl)).arrayBuffer();
    if (sha256) {
      const hex = [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))].map(x => x.toString(16).padStart(2, "0")).join("");
      if (hex !== sha256) throw { error_key: "error.featWasmHash", params: {} };
    }
    const m = await import(jsUrl); await m.default({ module_or_path: bytes }); mod = m; return m;
  })().catch(e => { loading = null; throw e; });
  return loading;
}

/** the error of the .wasm is JSON `{error_key, params}`; anything else is a failure to start the engine @param {any} e */
function describe(e) {
  if (e && e.error_key) return { error_key: e.error_key, params: e.params || {} };
  try { const j = JSON.parse(typeof e === "string" ? e : String(e && e.message || e)); if (j && j.error_key) return { error_key: j.error_key, params: j.params || {} }; } catch (_) { /* not JSON */ }
  return { error_key: "error.featFailed", params: { detail: String(e && e.message || e).slice(0, 200) } };
}

self.onmessage = async (/** @type {MessageEvent} */ ev) => {
  const m = ev.data, t0 = performance.now();
  try {
    const c = await load(m.jsUrl, m.wasmUrl, m.sha256 || "");
    const json = c.reactivity(m.smiles, m.opts || "");
    self.postMessage({ id: m.id, ok: true, json, version: c.version(), ms: performance.now() - t0 });
  } catch (e) { self.postMessage({ id: m.id, error: describe(e) }); }
};
