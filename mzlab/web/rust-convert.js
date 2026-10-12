// SCIEX .wiff + .wiff.scan -> mzML done in the worker (rust-worker.js), for the page (the Thermo .raw is in rust-raw.js). `rpc(msg, onProgress)` and `base(name)` come from
// the bridge (rust-bridge.js). A failed conversion rejects with an Error whose `json` is { error, error_key, params } like the API errors.
const WIFF_ERRORS = ["err.file.notWiff", "err.file.wiffNoScan", "err.file.wiff", "err.file.mzmlWrite"];        // the keys the Rust reader and writer of the .wiff give
export function wiffApi(rpc, base) {
  const fail = (r, key) => {
    let e = { error_key: key, params: { detail: "" } };
    try { const j = JSON.parse(r.error); if (j.error_key && WIFF_ERRORS.includes(j.error_key)) e = j; } catch (_) { /* the worker was lost */ }
    return Object.assign(new Error(e.error_key), { json: { error: e.error_key, ...e } });
  };
  return {
    // SCIEX .wiff: the numbers of its samples (one injection each); a file that is not a .wiff rejects like a conversion
    async wiffSamples(file) {
      const r = await rpc({ op: "wiffsamples", name: base(file.name), blob: file });
      if (r.dead || r.error) throw fail(r, "err.file.notWiff");
      return JSON.parse(r.info);
    },
    // a .wiff alone: the open mzML files it was made from (same SHA-1) get its method -> { sha1, matched: [{ name, sample }] }; the .wiff is read, not converted
    async metodoWiff(file) {
      const r = await rpc({ op: "lr_metodo_wiff", blob: file });
      if (r.dead || r.error) throw fail(r, "err.file.notWiff");
      return JSON.parse(r.info);
    },
    // one sample of a .wiff (+ its .wiff.scan, null when missing) -> Blob of mzML. Several can be asked at once (the engine converts a few at a time); `key` names the
    // conversion for cancelWiff; `centroid` true writes one point per peak (mzLab centroidi, MS:1000035) instead of the profile. A cancelled one rejects with { cancelled: true }.
    async convertWiff(wiff, scan, sample, onProgress, key, centroid) {
      const r = await rpc({ op: "wiff2mzml", name: base(wiff.name), blob: wiff, scan: scan || null, sample, centroid: centroid === true, key: key || wiff.name + "#" + sample }, onProgress);
      if (r.cancelled) throw Object.assign(new Error("cancelled"), { cancelled: true });
      if (r.dead || r.error) throw fail(r, "err.file.wiff");
      return Object.assign(new Blob(r.parts, { type: "application/xml" }), { mzlabId: null });
    },
    // stops a conversion asked with this key (queued or running)
    cancelWiff(key) { return rpc({ op: "wiffcancel", key }); },
  };
}
