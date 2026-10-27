// @ts-strict
// TPMINE-PRIVATE  Tabui base: the helpers, the styles, the worker with its calls, the files of the Data tab and the small charts (used by 20_tpmine.js).
(() => {
  "use strict";
  const ctx = () => QTOOLS.ctx;
  const esc = s => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const specHtml = sp => !sp ? "" : `<div class="card"><h3>Specificity <span class="mut">(${esc(sp.file)}, ${sp.from}-${sp.to} min)</span></h3><div class="mut" title="Signal in the same file and time window at the m/z of the ion, at ±1 and ±2 Da (neighbours) and at ±0.5 Da (decoy windows). A neighbour or a decoy as strong as the ion says the signal is not specific to its m/z.">ion ${fmtA(sp.signal)}</div><table><tr><th>Δ m/z</th><th>Signal</th></tr>${[...sp.neighbours, ...sp.decoys].map(w => `<tr><td class="n">${w.dm > 0 ? "+" : ""}${w.dm}</td><td class="n">${fmtA(w.signal)}</td></tr>`).join("")}</table><div class="mut">As strong as the ion: ${sp.stronger.neighbours} of ${sp.neighbours.length} neighbours, ${sp.stronger.decoys} of ${sp.decoys.length} decoys.</div></div>`;
  const fmtA = v => /** @type {any} */ (window).TPTAB ? /** @type {any} */ (window).TPTAB.area(v) : String(Math.round(v));   // engineering notation, e.g. 821.43e+6
  /** the same sample read from two sources (Data tab, chosen files) has one key: base name, no .mzML (repeated or not), lower case @param {string} n */
  const fileKey = n => (String(n || "").split(/[\\/]/).pop() || "").replace(/(\.mzml)+$/i, "").toLowerCase();
  /** @param {string} n the name shown in a row: without the extension */
  const fileLabel = n => String(n || "").replace(/(\.mzml)+$/i, "");
  const TYPES = { sample: "sample", blank: "blank", standard: "standard", control: "control" };
  const KINDS = { full: "Full scan", ms2: "MS2", mrm: "MRM", hr: "LC-HRMS", msn: "MSn (infusion)", empty: "empty", error: "error" };
  /** the reactivity of the parent as told by TPREATT (`tp-reatt`): the heavy atoms with the highest Prev. f(−) go in the summary @type {{ smiles: string, atoms: any[] }|null} */
  let fukR = null;
  const fukHtml = () => {
    const host = document.getElementById("tp-reatt");
    if (!fukR || !host || host.dataset.smiles !== fukR.smiles) return "";
    const top = fukR.atoms.filter(a => a.heavy_index != null && Number.isFinite(+a.f_minus)).sort((a, b) => b.f_minus - a.f_minus).slice(0, 3);
    if (!top.length) return "";
    const txt = top.map(a => `${esc(a.el)}${a.heavy_index + 1} ${(+a.f_minus).toFixed(3)}`).join(" · ");
    return `<span class="hc" data-k="fukui"${/** @type {any} */ (window).TPHELP ? ' data-tt="ft.sum.fukui"' : ""} tabindex="0" aria-label="Prev. Fukui f minus, highest heavy atoms: ${txt}"><span class="hl">Prev. f(−)</span> <b>${txt}</b></span>`;
  };
  window.addEventListener("tp-reatt", (/** @type {any} */ e) => {
    const d = e.detail || {};
    fukR = d.reatt && Array.isArray(d.reatt.atoms) ? { smiles: d.smiles, atoms: d.reatt.atoms } : null;
    const h = document.getElementById("tp-fuk"); if (h) h.innerHTML = fukHtml();
  });
  const LAB = { forte: "strong", possibile: "possible", debole: "weak", progenitore: "parent" };

  const CSS = `
.tp{display:grid;grid-template-columns:minmax(300px,380px) 1fr;gap:14px;align-items:start;padding:12px 16px}
@media(max-width:1000px){.tp{grid-template-columns:1fr}}
.tp.tp-nol{grid-template-columns:1fr}.tp.tp-nol #tp-left{display:none}
.tp-tools{padding:8px 16px 0}.tp-tools button{box-sizing:border-box;min-height:28px;min-width:28px;padding:2px 10px;font:inherit;font-size:12px;cursor:pointer;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink)}.tp-tools button:focus-visible{outline:2px solid var(--accent);outline-offset:1px}@media(pointer:coarse){.tp-tools button{min-height:40px}}
.tp .tp-sr{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap}.tp .tp-rm{box-sizing:border-box;min-width:24px;min-height:24px;padding:0 6px;font:inherit;line-height:1;cursor:pointer;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink)}.tp .tp-rm:focus-visible,.tp .tp-fn:focus-visible{outline:2px solid var(--accent);outline-offset:1px}@media(pointer:coarse){.tp .tp-rm{min-width:40px;min-height:40px}}
.tp h3{margin:0 0 6px;font-size:14px}.tp .card{background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:10px 12px;margin-bottom:10px}
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
#qt-nav-tab,nav button[data-v="tpmine"]{background:linear-gradient(135deg,#ffe29a,#f4c95d 55%,#d99a1d)!important;color:#3a2a00!important;border-color:#d99a1d!important;font-weight:700}
.hero{background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:8px;padding:8px 12px;margin-bottom:10px;display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1fr) auto;gap:8px 18px}
.tp .hero-b{min-width:0}.tp .hero h2{margin:0 0 4px;font-size:12px;font-weight:600;color:var(--muted)}.tp .hero-i{display:flex;flex-wrap:wrap;gap:4px 6px;align-items:center;font-size:13px}.tp #tp-cnt{display:contents}
.tp .hero .hc{display:inline-flex;gap:4px;align-items:baseline;box-sizing:border-box;min-height:24px;padding:2px 8px;border:1px solid var(--line);border-radius:12px}.tp .hero .hl{color:var(--muted);font-size:12px}.tp .hero b{font-variant-numeric:tabular-nums}.tp .hero .hn{font-weight:600;border-color:transparent;padding-left:0}
.tp .hero .hc:focus-visible,.tp .hero button:focus-visible{outline:2px solid var(--accent);outline-offset:1px}.tp .hero-l{box-sizing:border-box;min-height:24px;min-width:24px;padding:0 8px;font:inherit;font-size:12px;cursor:pointer;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink)}
.tp .hero-x{display:flex;gap:6px;flex-wrap:wrap;align-items:flex-start}@media(max-width:900px){.tp .hero{grid-template-columns:minmax(0,1fr)}}@media(pointer:coarse){.tp .hero-x button,.tp .hero-l{min-height:40px}}
.tp #tp-files th{position:static}
.tp #tp-left,.tp #tp-right{min-width:0}.tp #tp-files,.tp #tp-cfiles{max-width:100%;overflow-x:auto}.tp #tp-files table{width:100%;table-layout:auto}.tp #tp-files td:first-child{max-width:min(130px,28vw);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.tp #tp-files select{max-width:100%}.tp .fsum{font-size:12px;color:var(--muted);margin:2px 0 6px}.tp #tp-files td,.tp #tp-files th{padding:1px 6px}.tp #tp-files select,.tp #tp-files input{font-size:11px;padding:0 3px;height:20px}
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
        if (m.type === "ask") {               // what only this page can do for the worker: the engine of the Data tab reads the maker's files (docs/agenti/lettura-nativa.md)
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
  const callRaw = async (fn, ...args) => { await startWorker(); return send({ type: "call", fn, args }); };
  window.TPMINE_MEM = async () => { await startWorker(); return send({ type: "mem" }); };      // size of the worker's WebAssembly memory (tests)

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
  async function opfsFiles() {        // the files of the Data tab kept by the Rust engine (low resolution): OPFS mzlab/files/, as File objects (never read whole here)
    try {
      const d = await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab").then(r => r.getDirectoryHandle("files"));
      const out = [];
      for await (const [name, h] of /** @type {any} */ (d).entries()) if (h.kind === "file" && /\.mzml$/i.test(name)) { const f = await h.getFile(); out.push({ name, buf: f, nat: !!(await stubOf(f)) }); }
      return out;
    } catch (_) { return []; }
  }
  async function pageFiles() {
    const o = await opfsFiles(), have = new Set(o.map(f => fileKey(f.name)));
    // by name: the order of the entries of OPFS depends on which write finished first, the one of IndexedDB is by key; the Dig must not depend on it. A sample kept by both is one file
    return o.concat((await idbFiles()).filter(f => !have.has(fileKey(f.name)))).sort((a, b) => a.name.localeCompare(b.name, "en", { numeric: true }));
  }
  async function idbFiles() {
    try {
      // "qqq_lab" is the name of the browser database (kept from the old name: renaming it would lose the users' data)
      const dbName = typeof NOMI !== "undefined" ? NOMI.DB : "qqq_lab";
      if (!indexedDB.databases || !(await indexedDB.databases()).some(d => d.name === dbName)) return [];
      return await new Promise(res => {
        const r = indexedDB.open(dbName);
        r.onerror = () => res([]);
        r.onsuccess = () => {
          const db = r.result;
          if (!db.objectStoreNames.contains("files")) { db.close(); return res([]); }
          const tx = db.transaction("files", "readonly"), s = tx.objectStore("files"), ks = s.getAllKeys(), vs = s.getAll();
          tx.oncomplete = () => { db.close(); res(ks.result.map((k, i) => ({ name: String(k), buf: vs.result[i] })).filter(f => f.buf && /\.mzml$/i.test(f.name))); };
          tx.onerror = () => { db.close(); res([]); };
        };
      });
    } catch (_) { return []; }
  }

  // ---------------------------------------------------------------- charts (plain canvas)
  function hue(i, n) { return `hsl(${Math.round(220 - 220 * (n > 1 ? i / (n - 1) : 0))},70%,45%)`; }
  function chart(cv, series, o = {}) {
    const dpr = devicePixelRatio || 1, w = cv.clientWidth || 400, h = cv.clientHeight || 210;
    cv.width = w * dpr; cv.height = h * dpr;
    const g = cv.getContext("2d"); g.scale(dpr, dpr); g.clearRect(0, 0, w, h);
    const M = { l: 52, r: 10, t: 10, b: 30 }, pw = w - M.l - M.r, ph = h - M.t - M.b;
    let x0 = Infinity, x1 = -Infinity, y1 = 0;
    for (const s of series) s.x.forEach((x, i) => { x0 = Math.min(x0, x); x1 = Math.max(x1, x); y1 = Math.max(y1, s.y[i]); });
    if (o.x0 != null) x0 = o.x0; if (o.x1 != null) x1 = o.x1; if (!(x1 > x0)) { x0 -= 1; x1 += 1; } if (!(y1 > 0)) y1 = 1;
    const X = x => M.l + (x - x0) / (x1 - x0) * pw, Y = y => M.t + ph - y / (y1 * 1.05) * ph;
    const css = getComputedStyle(document.documentElement), ink = css.getPropertyValue("--muted") || "#666", line = css.getPropertyValue("--line") || "#ddd";
    g.font = "11px system-ui"; g.fillStyle = ink; g.strokeStyle = line; g.lineWidth = 1;
    for (let i = 0; i <= 4; i++) { const v = y1 * 1.05 * i / 4, y = Y(v); g.beginPath(); g.moveTo(M.l, y); g.lineTo(w - M.r, y); g.stroke(); g.textAlign = "right"; g.fillText(fmtA(v), M.l - 4, y + 3); }
    const step = niceStep((x1 - x0) / 6);
    for (let v = Math.ceil(x0 / step) * step; v <= x1 + 1e-9; v += step) { g.textAlign = "center"; g.fillText(+v.toFixed(2), X(v), h - M.b + 14); }
    g.textAlign = "center"; g.fillText(o.xlabel || "", M.l + pw / 2, h - 3);
    if (o.shade) { g.fillStyle = "rgba(43,92,138,.10)"; g.fillRect(X(o.shade[0]), M.t, X(o.shade[1]) - X(o.shade[0]), ph); }
    for (const s of series) {
      g.strokeStyle = s.color; g.lineWidth = s.w || 1.4; g.setLineDash(s.dash || []); g.beginPath();
      s.x.forEach((x, i) => i ? g.lineTo(X(x), Y(s.y[i])) : g.moveTo(X(x), Y(s.y[i]))); g.stroke(); g.setLineDash([]);
      if (s.dots) { g.fillStyle = s.color; s.x.forEach((x, i) => { g.beginPath(); g.arc(X(x), Y(s.y[i]), 3, 0, 7); g.fill(); }); }
      if (s.hollow) {                                         // points that are not found peaks (filled areas): an open circle, never a dot
        const bg = css.getPropertyValue("--panel") || "#fff"; g.lineWidth = 1.8;
        s.hollow.forEach(i => { g.beginPath(); g.arc(X(s.x[i]), Y(s.y[i]), 3.6, 0, 7); g.fillStyle = bg; g.fill(); g.stroke(); });
      }
    }
    g.strokeStyle = ink; g.strokeRect(M.l, M.t, pw, ph);
  }
  function niceStep(r) { const p = Math.pow(10, Math.floor(Math.log10(r))), m = r / p; return (m < 1.5 ? 1 : m < 3.5 ? 2 : m < 7.5 ? 5 : 10) * p; }
  const spark = k => {
    if (!k.length) return ""; const mx = Math.max(...k.map(p => p.area)) || 1, w = 70, h = 18;
    const pts = k.map((p, i) => `${(i / Math.max(1, k.length - 1) * (w - 4) + 2).toFixed(1)},${(h - 2 - p.area / mx * (h - 4)).toFixed(1)}`).join(" ");
    return `<svg width="${w}" height="${h}"><polyline points="${pts}" fill="none" stroke="currentColor" stroke-width="1.3"/></svg>`;
  };

  /** the worker is stopped for good (Cancel that did not answer); false when none runs. The caller starts a new one */
  const killWorker = () => {
    if (!worker) return false;
    worker.terminate(); worker = null; ready = null;
    for (const p of pending.values()) p.rej(new Error("Cancelled"));
    pending.clear();
    return true;
  };
  /** @param {(t?: any, f?: any, m?: any) => any} f who shows the progress of the worker (the open tab) */
  const setProgress = f => { onProgress = f; };
  window.TPBASE = { ctx, esc, specHtml, fmtA, fileKey, fileLabel, TYPES, KINDS, fukHtml, LAB, CSS, startWorker, send, call, callRaw, killWorker, setProgress,
    mzmls, stubOf, asked, opfsFiles, pageFiles, idbFiles, hue, chart, niceStep, spark };
})();
