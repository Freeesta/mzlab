import { ensureRoom } from "./archivio.js";
// A profile mzML already in the archive -> a copy with one point per peak (rust-worker.js, case "mzml2cen"; the algorithm is crates/mzlab-core/src/centroidi/). The copy is written to a file of OPFS
// (mzlab/conv/) and returned as a File; the page then loads it like any file. `onProgress({ scans, scansTotal, written })`, `signal` (AbortSignal) cancels it: the worker is busy in one long call, so cancelling
// asks the bridge to stop it and start a new one (h.abort). Resolves with the File, or null when the file has no profile spectrum. Rejects with { cancelled: true } or an Error whose `json` is { error, error_key, params }.
export function makeConvertCentroids(h) {
  return async (name, size, onProgress, signal) => {
    const w = h.current(), cancelled = () => Object.assign(new Error("cancelled"), { cancelled: true });
    if (signal && signal.aborted) throw cancelled();
    try { await ensureRoom(size * 1.1); }
    catch (q) { const j = JSON.parse(q); throw Object.assign(new Error(j.error_key), { json: { error: j.error_key, ...j } }); }
    const onAbort = () => { if (w && w === h.current()) h.abort(w); };
    if (signal) signal.addEventListener("abort", onAbort, { once: true });
    const dest = `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}.mzML`;
    const drop = () => navigator.storage.getDirectory().then(d => d.getDirectoryHandle("mzlab")).then(d => d.getDirectoryHandle("conv")).then(d => d.removeEntry(dest)).catch(() => {});
    const r = await h.rpc({ op: "mzml2cen", name: h.base(name), dest }, onProgress);
    if (signal) signal.removeEventListener("abort", onAbort);
    if (r.cancelled || signal && signal.aborted) { drop(); throw cancelled(); }
    if (r.dead || r.error) {
      drop();
      let e = { error_key: "err.file.mzmlWrite", params: { detail: "" } };
      try { const j = JSON.parse(r.error); if (j.error_key) e = j; } catch (_) { /* the worker was lost */ }
      throw Object.assign(new Error(e.error_key), { json: { error: e.error_key, ...e } });
    }
    if (r.none) return null;
    return r.file || new Blob(r.parts, { type: "application/xml" });
  };
}
