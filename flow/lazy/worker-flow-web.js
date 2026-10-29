// @ts-check
// Flow web runner, worker side: loads the `feat` .wasm of mzFinder and answers `stima`, the memory that aligning a stored series asks for (Rust `stima_series`, the
// rule of `slabs::stima`), and the facts the record of a run (prov/1) asks of the files in OPFS: `meta` (estimates and memory of a job, `flow_job_meta`), `sha` (SHA-256 and size of a file, read
// in pieces), `source` (the file an index was made from). Only these live here; the steps run in the pool of feat-worker.js. Used by flow-web-run.js and flow-web-prov.js.
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

/** OPFS sync handles of the paths (a path twice is one handle) and the reader the .wasm calls: read(k, offset, length) @param {string[]} paths */
async function open(paths) {
  const hs = /** @type {Map<string, any>} */ (new Map());
  for (const path of paths) {
    if (hs.has(path)) continue;
    const parts = path.split("/").filter(Boolean), name = /** @type {string} */ (parts.pop());
    let d = await navigator.storage.getDirectory(); for (const p of parts) d = await d.getDirectoryHandle(p);
    const fh = await d.getFileHandle(name);
    for (let i = 0; ; i++) {   // a handle of a worker that has just died is released a moment later
      try { const h = await fh.createSyncAccessHandle(); hs.set(path, { h, size: h.getSize() }); break; }
      catch (e) { if (i >= 40 || /** @type {any} */ (e).name !== "NoModificationAllowedError") throw e; await new Promise(r => setTimeout(r, 50)); }
    }
  }
  const read = (/** @type {number} */ k, /** @type {number} */ off, /** @type {number} */ len) => { const b = new Uint8Array(len); hs.get(paths[k]).h.read(b, { at: off }); return b; };
  return { read, sizes: paths.map(p => hs.get(p).size), close: () => { for (const x of hs.values()) x.h.close(); } };
}

/** the answer of the facts above @param {any} m */
async function facts(m) {
  const f = await open(m.paths);
  try {
    if (m.op === "sha") return mod.flow_file_sha(f.read, f.sizes[0]);
    if (m.op === "source") return mod.flow_idx_source(f.read, f.sizes[0]);
    const stored = await Promise.all((m.stored || []).map(async (/** @type {string} */ path) => {
      const parts = path.split("/").filter(Boolean), name = /** @type {string} */ (parts.pop());
      let d = await navigator.storage.getDirectory(); for (const p of parts) d = await d.getDirectoryHandle(p);
      return new Uint8Array(await (await (await d.getFileHandle(name)).getFile()).arrayBuffer());
    }));
    return mod.flow_job_meta(m.plan, m.registry, m.instance, f.read, f.sizes, stored, m.budgetMb || 0);
  } finally { f.close(); }
}

self.onmessage = async (/** @type {MessageEvent} */ ev) => {
  const m = ev.data;
  try {
    await load(m.wasm);
    if (m.op) { self.postMessage({ id: m.id, ok: true, json: await facts(m) }); return; }
    const files = m.files.map((/** @type {ArrayBuffer} */ b) => new Uint8Array(b));
    self.postMessage({ id: m.id, ok: true, json: mod.stima_series(files, m.recipe || "", m.budgetMb || 0) });
  } catch (e) { self.postMessage({ id: m.id, error: describe(e) }); }
};
export {};
