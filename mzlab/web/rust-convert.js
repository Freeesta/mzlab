// @ts-strict
// A SCIEX .wiff dropped without its .wiff.scan: read only for its method (rust-worker.js). `rpc(msg, onProgress)` and `base(name)` come from the bridge (rust-bridge.js).
// A failure rejects with an Error whose `json` is { error, error_key, params } like the API errors.
const WIFF_ERRORS = ["err.file.notWiff", "err.file.wiffNoScan", "err.file.wiff", "err.file.mzmlWrite"];        // the keys the Rust reader and writer of the .wiff give
export function wiffApi(rpc, base) {
  const fail = (r, key) => {
    let e = { error_key: key, params: { detail: "" } };
    try { const j = JSON.parse(r.error); if (j.error_key && WIFF_ERRORS.includes(j.error_key)) e = j; } catch (_) { /* the worker was lost */ }
    return Object.assign(new Error(e.error_key), { json: { error: e.error_key, ...e } });
  };
  return {
    // a .wiff alone: the open mzML files it was made from (same SHA-1) get its method -> { sha1, matched: [{ name, sample }] }; the .wiff is read, not converted
    async metodoWiff(file) {
      const r = await rpc({ op: "lr_metodo_wiff", blob: file });
      if (r.dead || r.error) throw fail(r, "err.file.notWiff");
      return JSON.parse(r.info);
    },
    // an Agilent *.m method folder (files = [File] with the folder they came from): its metodo/1 document, the files read whole (those above 4 MB are left out) and sent in one buffer
    async metodoM(name, files) {
      const parts = [], names = [];
      for (const f of files) if (f.size <= 4 << 20) { parts.push(new Uint8Array(await f.arrayBuffer())); names.push(f.rel); }
      const data = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
      parts.reduce((at, p) => { data.set(p, at); return at + p.length; }, 0);
      const r = await rpc({ op: "lr_metodo_m", name, names: names.join("\n"), lens: Uint32Array.from(parts, p => p.length), data });
      if (r.dead || r.error) throw fail(r, "err.file.notWiff");
      return JSON.parse(r.info);
    },
  };
}
