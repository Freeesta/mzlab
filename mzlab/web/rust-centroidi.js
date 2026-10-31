// @ts-strict
import { ensureRoom } from "./archivio.js";
// A profile mzML already in the archive -> a copy with one point per peak (rust-worker.js, case "mzml2cen"; the algorithm is crates/mzlab-core/src/centroidi/). The copy is written to a file of OPFS
// (mzlab/conv/) and returned as a File; the page then loads it like any file. `onProgress({ scans, scansTotal, written })`, `signal` (AbortSignal) cancels it: the worker is busy in one long call, so cancelling
// asks the bridge to stop it and start a new one (h.abort). Resolves with the File, or null when the file has no profile spectrum. Rejects with { cancelled: true } or an Error whose `json` is { error, error_key, params }.
export function makeConvertCentroids(h) {
  return async (name, size, onProgress, signal) => {
    const cancelled = () => Object.assign(new Error("cancelled"), { cancelled: true });
    if (signal && signal.aborted) throw cancelled();
    try { await ensureRoom(size * 1.1); }
    catch (q) {
      // ensureRoom throws JSON strings; anything else is passed on as it is
      let j; try { j = JSON.parse(/** @type {string} */ (q)); } catch (_) { throw q; }
      throw Object.assign(new Error(j.error_key), { json: { error: j.error_key, ...j } });
    }
    if (signal && signal.aborted) throw cancelled();
    // the worker is read when Cancel is pressed: it may have been recycled while the quota was checked
    const onAbort = () => { const w = h.current(); if (w) h.abort(w); };
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

/** `spectra.centroid` of the Flow in this browser: the index of the centroid copy of a profile mzML, kept as `<name>.cen.mzidx` (what features.detect reads for a profile file). Reuses what is there for the same
 * source, parameters and engine; else makes the copy with `convert` (the call of Settings) and indexes it. `params` is "" for the estimated values. Resolves "flow" | "settings" (reused), "made", or "none" (no profile spectrum). */
export function makeFlowCentroid(h, convert) {
  return async (name, size, params, onProgress, signal) => {
    const n = h.base(name), have = JSON.parse((await h.rpc({ op: "lr_cen_have", name: n, params })).info || "{}");
    if (have.have) return have.have;
    const blob = await convert(name, size, onProgress, signal);
    if (!blob) return "none";
    const r = await h.rpc({ op: "lr_cen_adopt", name: n, params, blob: blob instanceof Blob ? blob : new Blob([blob]) });
    if (r.dead || r.error) throw Object.assign(new Error("err.engine.index"), { json: { error: "err.engine.index", error_key: "err.engine.index", params: {} } });
    return "made";
  };
}
