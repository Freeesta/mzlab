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
  };
}
