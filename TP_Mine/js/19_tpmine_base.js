// @ts-strict
// TPMINE-PRIVATE  Engine worker of the page (mzFinder): starts tpmine-worker.js and gives the other scripts `window.TPCALL(fn, ...args)` (the identities of the feature tables, the proofs of the origin of an ion).
(() => {
  "use strict";
  const ctx = () => QTOOLS.ctx;
  // ---------------------------------------------------------------- worker and calls
  let worker = null, ready = null, seq = 0;
  const pending = new Map();
  /** @type {(t?: any, f?: any, m?: any) => any} */
  let onProgress = () => {};
  function startWorker() {
    if (ready) return ready;
    ready = (async () => {
      const c = ctx(), base = c.base;
      const url = URL.createObjectURL(new Blob([c.files["tpmine-worker.js"]], { type: "text/javascript" }));
      worker = new Worker(url, { type: "module" });
      worker.onmessage = ev => {
        const m = ev.data;
        if (m.type === "progress") return onProgress(m.text, m.frac, m);
        if (m.type === "step") return onProgress(m.text, null);
        if (m.type === "notice") {            // a short notice of the worker (a profile file reduced to centroids): its text comes from the catalogs
          const t = /** @type {any} */ (window).TE(m.key, m.params);
          const d = document.createElement("div"); d.className = "tp-toast"; d.setAttribute("role", "status"); d.textContent = t; document.body.appendChild(d);
          setTimeout(() => d.classList.add("out"), 4200); setTimeout(() => d.remove(), 4800); return;
        }
        if (m.type === "ask") {               // what only this page can do for the worker: the engine of the Data tab reads the maker's files (docs/agenti/native-reading.md)
          asked(m.what, m.name).then(value => worker.postMessage({ type: "ask-reply", rid: m.rid, value }), () => worker.postMessage({ type: "ask-reply", rid: m.rid, value: null }));
          return;
        }
        if (m.type === "ready") { const p = pending.get("init"); pending.delete("init"); return p && p.res(); }
        const p = pending.get(m.id); if (!p) return; pending.delete(m.id);
        m.error ? p.rej(new Error(m.error)) : p.res(m.result);
      };
      await new Promise((res, rej) => {
        pending.set("init", { res, rej });
        worker.onerror = e => rej(new Error(e.message || "worker"));
        worker.postMessage({ type: "init", desktop: !!window.MZLAB_DESKTOP, base });
      });
      URL.revokeObjectURL(url);
    })();
    ready.catch(() => { ready = null; });
    return ready;
  }
  const send = (msg, transfer) => new Promise((res, rej) => { const id = ++seq; pending.set(id, { res, rej }); worker.postMessage({ ...msg, id }, transfer || []); });
  const call = async (fn, ...args) => { await startWorker(); return JSON.parse(await send({ type: "call", fn, args: args.map(a => typeof a === "string" ? a : JSON.stringify(a)) })); };
  window.TPCALL = call;                // the engine, for the other scripts of mzFinder (the identities of the feature tables)

  // ---------------------------------------------------------------- page files (the browser's own storage of the main program)
  const mzmls = new Map();                  // the mzML written for a sample (once, for the use that needs it: Pyodide, a profile sample to reduce)
  async function stubOf(f) {                // the stub of a sample of a maker's file (.raw, .wiff, .D) in files/, null for anything else
    if (!f || f.size >= 65536) return null;
    try { const j = JSON.parse(await f.text()); return j.mzlab === "mzlab/nat1" ? j : null; } catch (_) { return null; }
  }
  async function asked(what, name) {        // "mzml": the mzML of the sample; "cenidx": its centroid index, built apart (true); "srm": its SRM traces for mzFinder, read from the maker's file without an mzML
    const d = await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab").then(r => r.getDirectoryHandle("files"));
    const j = await stubOf(await (await d.getFileHandle(name)).getFile()), rb = await window.MZLAB_RUST_READY;
    if (!j || !rb) return null;
    if (what === "cenidx") return rb.nativeCentroidIndex ? (await rb.nativeCentroidIndex(name), true) : false;
    if (what === "srm") return rb.nativeSrm ? await rb.nativeSrm(name) : null;
    if (!mzmls.has(name)) mzmls.set(name, rb.nativeToMzml ? await rb.nativeToMzml(name, j.size || 0) : null);
    return mzmls.get(name);
  }
})();
