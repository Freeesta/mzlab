// @ts-strict
// TPMINE-PRIVATE  Engine worker of the page (mzFinder): starts tpmine-worker.js and gives the other scripts `window.TPCALL(fn, ...args)` (the identities of the feature tables, the proofs of the origin of an ion).
(() => {
  "use strict";
  const ctx = () => QTOOLS.ctx;
  // styles of the `.tp` views (tables, row panels): Flow views wrap their content in `.tp`, so they are added once with the package
  const CSS = `
.tp{display:grid;grid-template-columns:minmax(300px,380px) 1fr;gap:14px;align-items:start;padding:12px 16px}
@media(max-width:1000px){.tp{grid-template-columns:1fr}}
.tp.tp-nol{grid-template-columns:1fr}.tp.tp-nol #tp-left{display:none}
.tp-tools{padding:8px 16px 0}.tp-tools button{box-sizing:border-box;min-height:28px;min-width:28px;padding:2px 10px;font:inherit;font-size:12px;cursor:pointer;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink)}.tp-tools button:focus-visible{outline:2px solid var(--accent);outline-offset:1px}@media(pointer:coarse){.tp-tools button{min-height:40px}}
.tp .tp-sr{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap}.tp .tp-rm{box-sizing:border-box;min-width:24px;min-height:24px;padding:0 6px;font:inherit;line-height:1;cursor:pointer;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink)}.tp .tp-rm:focus-visible,.tp .tp-fn:focus-visible{outline:2px solid var(--accent);outline-offset:1px}@media(pointer:coarse){.tp .tp-rm{min-width:40px;min-height:40px}}
.tp h3,.tp .card>h2{margin:0 0 6px;font-size:14px}.tp .card{background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:10px 12px;margin-bottom:10px}
.tp .row{display:flex;gap:6px;align-items:center;flex-wrap:wrap;margin:4px 0}.tp .grow{flex:1;min-width:120px}.tp input[type=text],.tp textarea,.tp input[type=number]{width:100%;box-sizing:border-box}
.tp .mut{color:var(--muted);font-size:12px}.tp table{border-collapse:collapse;width:100%;font-size:12px}.tp th,.tp td{padding:3px 6px;border-bottom:1px solid var(--line);text-align:left;vertical-align:middle}
.tp th{position:sticky;top:0;background:var(--panel);font-weight:600;color:var(--muted)}.tp td.n{text-align:right;font-variant-numeric:tabular-nums}
.tp tr.sel{background:var(--sel)}.tp tr.sel td:first-child{box-shadow:inset 3px 0 0 var(--accent)}.tp tr.clk{cursor:pointer}.tp tr.clk:hover{background:var(--sel)}
.tp input.tp-est{background:color-mix(in srgb,var(--warn) 18%,transparent);border-color:var(--warn)}
.tp .badge{display:inline-block;padding:0 6px;border-radius:9px;font-size:11px;font-weight:600;border:1px solid var(--line)}
.tp .b-forte{background:color-mix(in srgb,var(--ok) 14%,var(--panel));color:var(--ink);border-color:var(--ok)}.tp .b-possibile{background:color-mix(in srgb,var(--warn) 14%,var(--panel));color:var(--ink);border-color:var(--warn)}.tp .b-debole{color:var(--muted)}

.tp .loadbar{margin:4px 0}
.tp .bar{height:6px;background:var(--line);border-radius:3px;overflow:hidden;margin-top:6px}.tp .bar i{display:block;height:100%;background:var(--accent);width:0}
.tp .tbl{max-height:46vh;overflow:auto;border:1px solid var(--line);border-radius:6px}.tp canvas{width:100%;height:210px;display:block}
.tp .crit li{margin:2px 0}.tp .crit .pass{color:var(--ok)}.tp .crit .fail{color:var(--bad)}.tp .crit .na{color:var(--muted)}
.tp .cols{display:grid;grid-template-columns:1fr 1fr;gap:12px}@media(max-width:1300px){.tp .cols{grid-template-columns:1fr}}
.tp .err{color:var(--bad)}.tp [aria-invalid="true"]{border:2px solid var(--bad);box-shadow:inset 4px 0 0 var(--bad)}.tp .row>.grow-in{flex:1;width:auto;min-width:0}.tp details>summary{cursor:pointer;color:var(--muted);font-size:12px}.tp textarea{font:12px ui-monospace,monospace;min-height:150px}
.tp .kv{display:grid;grid-template-columns:auto 1fr;gap:2px 10px;font-size:12px}.tp .kv b{color:var(--muted);font-weight:600}
.tp-wrap{padding:0}
.hero{background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:8px;padding:8px 12px;margin-bottom:10px;display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1fr) auto;gap:8px 18px}
.tp .hero-b{min-width:0}.tp .hero h2{margin:0 0 4px;font-size:12px;font-weight:600;color:var(--muted)}.tp .hero-i{display:flex;flex-wrap:wrap;gap:4px 6px;align-items:center;font-size:13px}.tp #tp-cnt{display:contents}
.tp .hero .hc{display:inline-flex;gap:4px;align-items:baseline;box-sizing:border-box;min-height:24px;padding:2px 8px;border:1px solid var(--line);border-radius:12px}.tp .hero .hl{color:var(--muted);font-size:12px}.tp .hero b{font-variant-numeric:tabular-nums}.tp .hero .hn{font-weight:600;border-color:transparent;padding-left:0}
.tp .hero .hc:focus-visible,.tp .hero button:focus-visible{outline:2px solid var(--accent);outline-offset:1px}.tp .hero-l{box-sizing:border-box;min-height:24px;min-width:24px;padding:0 8px;font:inherit;font-size:12px;cursor:pointer;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink)}
.tp .hero-x{display:flex;gap:6px;flex-wrap:wrap;align-items:flex-start}@media(max-width:900px){.tp .hero{grid-template-columns:minmax(0,1fr)}}@media(pointer:coarse){.tp .hero-x button,.tp .hero-l{min-height:40px}}
.tp #tp-files th{position:static}
.tp #tp-left,.tp #tp-right{min-width:0}.tp #tp-files,.tp #tp-cfiles{max-width:100%;overflow-x:auto}.tp #tp-files table{width:100%;table-layout:auto}.tp #tp-files td:first-child{max-width:min(130px,28vw);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.tp #tp-files select{max-width:100%}.tp .fsum{font-size:12px;color:var(--muted);margin:2px 0 6px}.tp #tp-files td,.tp #tp-files th{padding:1px 6px}.tp #tp-files select,.tp #tp-files input{font-size:11px;padding:0 3px;height:24px}.tp #tp-cond>summary{box-sizing:border-box;min-height:24px;padding:3px 0}
.tp .dig{display:flex;align-items:center;justify-content:center;gap:10px;width:100%;min-height:56px;font-size:18px;font-weight:700}.tp .dig .qi{display:inline-flex}.tp .tp-q{display:inline-grid;place-items:center;width:16px;height:16px;border:1px solid var(--muted);border-radius:50%;font-size:11px;color:var(--muted);cursor:help;margin-left:4px;font-style:normal}

.tp .hero-x button{min-height:24px;min-width:24px}
.tp .tp-mores{margin-top:10px}.tp .tp-mores>details{border:1px solid var(--line);border-radius:6px;background:var(--panel);padding:2px 10px;margin:0 0 4px}.tp .tp-mores>details>summary{min-height:28px;display:flex;align-items:center;color:var(--ink)}
.tp .tp-mores>details[open]{padding-bottom:8px}.tp .tp-mores .card{border:0;padding:0;margin:0}
.tp .lr-sg{display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:6px 14px;align-items:start}.tp .lr-sg canvas{display:block;width:100%;height:180px}
.tp .lr-idx{max-height:240px;overflow:auto;margin-top:6px;border-top:1px solid var(--line);padding-top:4px;font-size:12px}.tp .lr-idx:focus-visible{outline:2px solid var(--accent);outline-offset:1px}
.tp .lr-idx .card{border:0;padding:0;margin:0 0 6px;background:none}.tp .lr-idx h3{font-size:12px;color:var(--muted)}
.vtabs{display:flex;gap:6px;margin:0 0 8px;flex-wrap:wrap}.vt{border-radius:16px;padding:4px 14px}.vt.on{background:var(--accent);color:var(--panel);border-color:var(--accent)}
.mnode.sel{background:var(--sel);border-left-color:var(--accent)}
.tp .tp-alt{background:none;border:0;padding:0 4px;min-width:24px;min-height:24px;font:inherit;color:var(--muted);text-decoration:underline dotted;cursor:pointer;border-radius:4px}.tp .tp-alt:hover{background:var(--sel)}

.mnode{cursor:pointer}.mnode:hover circle{stroke:var(--accent);stroke-width:3}.mnode.sel circle{stroke:var(--accent);stroke-width:4}
.struct{text-align:center;margin-bottom:6px}.struct svg{max-width:100%;height:auto}
.tp-ac{position:relative}.tp-ac input{width:100%}#tp-sug{position:absolute;left:0;right:0;top:100%;z-index:20;margin:2px 0 0;padding:2px;list-style:none;background:var(--panel);border:1px solid var(--line);border-radius:6px;box-shadow:0 4px 14px rgba(0,0,0,.18);max-height:260px;overflow:auto}
#tp-sug li{padding:5px 8px;border-radius:4px;cursor:pointer;display:flex;gap:8px;align-items:baseline;font-size:13px}#tp-sug li span{color:var(--muted);font-size:11px;margin-left:auto;white-space:nowrap}#tp-sug li[aria-selected=true],#tp-sug li:hover{background:var(--sel);box-shadow:inset 3px 0 0 var(--accent)}
#tp-drawbar{position:fixed;left:50%;bottom:14px;transform:translateX(-50%);z-index:95;display:flex;gap:8px;align-items:center;background:var(--panel);border:1px solid var(--accent);border-radius:20px;padding:6px 12px;box-shadow:0 2px 10px rgba(0,0,0,.25);font-size:13px}#tp-drawbar[hidden]{display:none}
.tp #tp-warn{font-size:12px;color:var(--bad,#b3261e)}
.tp #tp-nopar{margin:6px 0;color:var(--bad,#b3261e)}
.words p{font-size:13px}


.tp-toast{position:fixed;left:50%;bottom:28px;transform:translateX(-50%);background:var(--ink);color:var(--panel);padding:8px 18px;border-radius:20px;z-index:99;box-shadow:0 6px 20px rgba(0,0,0,.3);animation:tpIn .3s ease;transition:opacity .5s}.tp-toast.out{opacity:0}
@keyframes tpIn{from{opacity:0}to{opacity:1}}


@media(prefers-reduced-motion:reduce){.tp *{animation:none!important;transition:none!important}}`;
  if (!document.getElementById("tp-css")) { const st = document.createElement("style"); st.id = "tp-css"; st.textContent = CSS; document.head.appendChild(st); }
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
  window.TPCALL = call;
  // hands the bytes of a file to the engine worker (the desktop shim test uses it: the page that did this with a file chooser is gone)
  call.put = async (/** @type {string} */ name, /** @type {ArrayBuffer} */ buf) => { await startWorker(); return send({ type: "put", name, buf }, [buf]); };                // the engine, for the other scripts of mzFinder (the identities of the feature tables)

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
