// @ts-strict
// Any dataset of a maker (Agilent .D folder, SCIEX .wiff + .wiff.scan, Thermo .raw) -> mzML through the readers of the registry (crates/mzlab-lettori-wasm, `LettoreWasm`), done in the
// engine's worker (rust-worker.js, case "lettori2mzml"): the .wasm of the readers is its own file (static/wasm-feat/lettori/, listed in manifest.json with its SHA-256) and is fetched
// only when a dataset of a maker is opened. The members are Blobs read by offset (FileReaderSync, never held); the mzML goes to a file of OPFS (rust-opfs.js).
// `files` = [{ name, blob }]: `name` is the logical path inside the dataset (`AcqData/MSScan.bin`, `x.wiff`, `x.raw`).

/** @typedef {{ name: string, blob: Blob }} Membro */

/** @type {Promise<any> | null} */
let modulo = null;

/** hex SHA-256 of some bytes */
async function sha256(/** @type {ArrayBuffer} */ b) {
  return [...new Uint8Array(await crypto.subtle.digest("SHA-256", b))].map(x => x.toString(16).padStart(2, "0")).join("");
}

/** the readers' module, initialised once; the .wasm is checked against the SHA-256 of the manifest (a different file is refused) */
function carica() {
  if (modulo) return modulo;
  modulo = (async () => {
    const base = new URL("./wasm-feat/", import.meta.url), man = await (await fetch(new URL("manifest.json", base))).json(), e = man.lettori;
    if (!e) throw '{"error_key":"err.route.unavailable","params":{}}';
    const bytes = await (await fetch(new URL(e.wasm, base))).arrayBuffer();
    if ((await sha256(bytes)) !== e.sha256) throw '{"error_key":"err.route.unavailable","params":{}}';
    const mod = await import(new URL(e.js, base).href);
    await mod.default({ module_or_path: bytes });
    return mod;
  })();
  modulo.catch(() => { modulo = null; });               // a failed load (offline) is tried again at the next dataset
  return modulo;
}

// worker side: `readerFor(blob)` gives (offset, length) => Uint8Array; `rawSink` is rust-opfs.js::rawSink. m = { op, name, files, campione, dest }
export async function lettoriOp(m, id, readerFor, rawSink) {
  const files = /** @type {Membro[]} */ (m.files), mod = await carica(), lectors = files.map(f => readerFor(f.blob)), sink = await rawSink(m.dest);
  let written = 0, last = 0, scans = 0, scansTotal = null;
  const tick = (/** @type {boolean} */ force) => { const t = performance.now(); if (force || t - last > 150) { last = t; postMessage({ op: "progress", id, total: files.reduce((a, f) => a + f.blob.size, 0), read: 0, written, scans, scansTotal }); } };
  let l;
  try {
    l = mod.LettoreWasm.open(files.map(f => f.name), Float64Array.from(files.map(f => f.blob.size)), (/** @type {number} */ i, /** @type {number} */ o, /** @type {number} */ n) => lectors[i](o, n), false);
    l.mzml(m.campione | 0, (/** @type {Uint8Array} */ piece) => { sink.write(piece); written += piece.length; tick(false); },
      (/** @type {number} */ done, /** @type {number} */ all) => { scans = done; scansTotal = all; tick(done === 0 || done === all); });
  } catch (e) { const q = await sink.error(); await sink.abort(); throw q || e; }          // q: the disk is full (error.quota)
  finally { if (l) l.free(); }
  const r = await sink.finish();
  postMessage({ id, op: m.op, n: r.parts ? r.parts.length : 1, parts: r.parts || null, file: r.file || null }, r.parts ? r.parts.map((/** @type {Uint8Array} */ a) => a.buffer) : []);
}

// page side, for rust-bridge.js: Blob of mzML (a File of OPFS when there is OPFS) of sample `opts.campione` (0 by default). `signal` (AbortSignal) cancels it as in rust-agilent.js.
// Rejects with an Error whose `json` is { error, error_key, params } like the API errors, or with { cancelled: true }.
export function makeConvertLettori(h) {
  return async (/** @type {string} */ name, /** @type {Membro[]} */ files, /** @type {((p: any) => void) | undefined} */ onProgress, /** @type {AbortSignal | undefined} */ signal, opts = {}) => {
    const w = h.current(), cancelled = () => Object.assign(new Error("cancelled"), { cancelled: true });
    if (signal && signal.aborted) throw cancelled();
    const onAbort = () => { if (w && w === h.current()) h.abort(w); };
    if (signal) signal.addEventListener("abort", onAbort, { once: true });
    const dest = `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}.mzML`;      // in OPFS mzlab/conv/
    const drop = () => navigator.storage.getDirectory().then(d => d.getDirectoryHandle("mzlab")).then(d => d.getDirectoryHandle("conv")).then(d => d.removeEntry(dest)).catch(() => {});
    const r = await h.rpc({ op: "lettori2mzml", name: h.base(name), files, dest, campione: opts.campione | 0 }, onProgress);
    if (signal) signal.removeEventListener("abort", onAbort);
    if (r.cancelled || signal && signal.aborted) { drop(); throw cancelled(); }
    if (r.dead || r.error) {
      drop();
      let e = { error_key: "err.file.notAccepted", params: {} };
      try { const j = JSON.parse(r.error); if (j.error_key) e = j; } catch (_) { /* the worker was lost */ }
      throw Object.assign(new Error(e.error_key), { json: { error: e.error_key, ...e } });
    }
    return Object.assign(r.file || new Blob(r.parts, { type: "application/xml" }), { mzlabId: null });
  };
}
