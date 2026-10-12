import { ensureRoom } from "./archivio.js";
// Before starting, the room left in the browser (navigator.storage.estimate, archivio.js ensureRoom) is checked against the size of the mzML and of its index (about 2x the .raw, 4x with the profile): too little is an error.quota with the MB needed and left.
// Thermo .raw -> mzML for rust-bridge.js: made in the worker, which reports the scans converted and writes the mzML to a file of OPFS (a File backed by the disk, not a Blob in memory;
// rust-opfs.js; without OPFS a Blob). `opts.profile`: keep the profile data (default: the vendor centroids). `opts.anonymous`: leave the computer, user folders, paths and user-label values out of the mzML. `signal` (AbortSignal) cancels it.
// The worker is busy in one long call and cannot hear a message, so cancelling asks the bridge to stop it and start a new one (h.abort).
// Rejects with an Error whose `json` is { error, error_key, params } like the API errors, or with { cancelled: true } after a cancel.
export function makeConvertRaw(h) {
  return async (file, onProgress, signal, opts = {}) => {
    const w = h.current(), cancelled = () => Object.assign(new Error("cancelled"), { cancelled: true });
    if (signal && signal.aborted) throw cancelled();
    try { await ensureRoom(file.size * (opts.profile ? 2.2 : 1.1)); }
    catch (q) { const j = JSON.parse(q); throw Object.assign(new Error(j.error_key), { json: { error: j.error_key, ...j } }); }
    const onAbort = () => { if (w && w === h.current()) h.abort(w); };
    if (signal) signal.addEventListener("abort", onAbort, { once: true });
    const dest = `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}.mzML`;      // in OPFS mzlab/conv/
    const drop = () => navigator.storage.getDirectory().then(d => d.getDirectoryHandle("mzlab")).then(d => d.getDirectoryHandle("conv")).then(d => d.removeEntry(dest)).catch(() => {});
    const r = await h.rpc({ op: "raw2mzml", name: h.base(file.name), blob: file, dest, profile: !!opts.profile, anonymous: !!opts.anonymous }, onProgress);
    if (signal) signal.removeEventListener("abort", onAbort);
    if (r.cancelled || signal && signal.aborted) { drop(); throw cancelled(); }
    if (r.dead || r.error) {
      drop();
      let e = { error_key: "err.file.raw", params: { detail: "" } };
      try { const j = JSON.parse(r.error); if (j.error_key) e = j; } catch (_) { /* the worker was lost */ }
      throw Object.assign(new Error(e.error_key), { json: { error: e.error_key, ...e } });
    }
    return Object.assign(r.file || new Blob(r.parts, { type: "application/xml" }), { mzlabId: r.tid ?? null });       // tid: the number of the mzML on the disk (desktop app only)
  };
}
