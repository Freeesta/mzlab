// Thermo .raw -> Blob of mzML for rust-bridge.js: made in the worker, which reports the scans converted; `signal` (AbortSignal) cancels it.
// The worker is busy in one long call and cannot hear a message, so cancelling asks the bridge to stop it and start a new one (h.abort).
// Rejects with an Error whose `json` is { error, error_key, params } like the API errors, or with { cancelled: true } after a cancel.
export function makeConvertRaw(h) {
  return async (file, onProgress, signal) => {
    const w = h.current(), cancelled = () => Object.assign(new Error("cancelled"), { cancelled: true });
    if (signal && signal.aborted) throw cancelled();
    const onAbort = () => { if (w && w === h.current()) h.abort(w); };
    if (signal) signal.addEventListener("abort", onAbort, { once: true });
    const r = await h.rpc({ op: "raw2mzml", name: h.base(file.name), blob: file }, onProgress);
    if (signal) signal.removeEventListener("abort", onAbort);
    if (r.cancelled || signal && signal.aborted) throw cancelled();
    if (r.dead || r.error) {
      let e = { error_key: "err.file.raw", params: { detail: "" } };
      try { const j = JSON.parse(r.error); if (j.error_key) e = j; } catch (_) { /* the worker was lost */ }
      throw Object.assign(new Error(e.error_key), { json: { error: e.error_key, ...e } });
    }
    return Object.assign(new Blob(r.parts, { type: "application/xml" }), { mzlabId: r.tid ?? null });       // tid: the number of the mzML on the disk (desktop app only)
  };
}
