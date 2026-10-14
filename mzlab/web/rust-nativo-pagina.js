// @ts-check
// Page side of the files of a maker read as they are (rust-nativo.js is the worker side): the samples of a dataset, which files of the session are samples of a maker's file,
// and «Download .mzML» (the mzML of one sample written on request to a file of OPFS). `h` = { rpc, base, current, abort } of the bridge (rust-bridge.js).
import { ensureRoom } from "./archivio.js";

/** @param {{ rpc: (m: any, onp?: (p: any) => void) => Promise<any>, base: (n: string) => string, current: () => any, abort: (w: any) => void }} h */
export function makeNativo(h) {
  const names = new Set();                                   // the files of the session that are samples of a maker's file
  const fail = (/** @type {any} */ r, /** @type {string} */ key) => {
    let e = { error_key: key, params: {} };
    try { const j = JSON.parse(r.error); if (j.error_key) e = j; } catch (_) { /* the worker was lost */ }
    return Object.assign(new Error(e.error_key), { json: { error: e.error_key, ...e } });
  };
  return {
    nativo: true,
    markNative: (/** @type {string} */ n) => { names.add(h.base(n)); },
    unmarkNative: (/** @type {string} */ n) => { names.delete(h.base(n)); },
    clearNative: () => { names.clear(); },
    isNative: (/** @type {string} */ n) => names.has(h.base(n)),
    nativeNames: () => [...names],
    /** the samples of a dataset (`members`: [{ name, blob }], names relative to the dataset): { format, samples: [{ id, name, scans }] } */
    async nativeSamples(/** @type {{ name: string, blob: Blob }[]} */ members) {
      const r = await h.rpc({ op: "natsamples", members });
      if (r.dead || r.error) throw fail(r, "err.file.notAccepted");
      return JSON.parse(r.info);
    },
    /** the SRM traces of the sample `name` as `srm::impacchetta` writes them (a Uint8Array): mzFinder integrates an MRM file from them without an mzML */
    async nativeSrm(/** @type {string} */ name) {
      const r = await h.rpc({ op: "natsrm", name: h.base(name) });
      if (r.dead || r.error) throw fail(r, "err.file.notAccepted");
      return r.srm;
    },
    /** the index of the sample `name` with its scans as centroids (`<name>.cen.mzidx` of OPFS idx/, apart from the one of the Data tab), for mzFinder */
    async nativeCentroidIndex(/** @type {string} */ name) {
      const r = await h.rpc({ op: "lr_index_cen", name: h.base(name) });
      if (r.dead || r.error) throw fail(r, "err.engine.index");
    },
    /** the mzML of the sample `name` as a File of OPFS (a Blob without OPFS; in the desktop app an empty Blob that carries `mzlabId`, the number of the mzML on the disk); `onProgress({ scans, scansTotal, written })`; `signal` cancels it (the worker is stopped and started again) */
    async nativeToMzml(/** @type {string} */ name, /** @type {number} */ size, /** @type {((p: any) => void) | undefined} */ onProgress, /** @type {AbortSignal | undefined} */ signal) {
      const w = h.current(), cancelled = () => Object.assign(new Error("cancelled"), { cancelled: true });
      if (signal && signal.aborted) throw cancelled();
      if (!window.MZLAB_DESKTOP) {                                                  // the desktop app writes on the disk, not in the browser storage
        try { await ensureRoom(size * 1.1); }
        catch (q) { const j = JSON.parse(String(q)); throw Object.assign(new Error(j.error_key), { json: { error: j.error_key, ...j } }); }
      }
      const onAbort = () => { if (w && w === h.current()) h.abort(w); };
      if (signal) signal.addEventListener("abort", onAbort, { once: true });
      const dest = `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}.mzML`;      // in OPFS mzlab/conv/
      const drop = () => navigator.storage.getDirectory().then(d => d.getDirectoryHandle("mzlab")).then(d => d.getDirectoryHandle("conv")).then(d => d.removeEntry(dest)).catch(() => {});
      const r = await h.rpc({ op: "nat2mzml", name: h.base(name), dest }, onProgress);
      if (signal) signal.removeEventListener("abort", onAbort);
      if (r.cancelled || signal && signal.aborted) { drop(); throw cancelled(); }
      if (r.dead || r.error) { drop(); throw fail(r, "err.file.mzmlWrite"); }
      if (r.tid != null) return Object.assign(new Blob([]), { mzlabId: r.tid });     // desktop app: the mzML is on the disk, saved from there (conversione.js download)
      return r.file || new Blob(r.parts, { type: "application/xml" });
    },
  };
}
