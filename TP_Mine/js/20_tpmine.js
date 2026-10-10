// TPMINE-PRIVATE  TP Mine: the tab with the whole automatic search of transformation products (only for the course owner).
(() => {
  "use strict";
  const ctx = () => QTOOLS.ctx;
  const esc = s => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const fmtA = v => v >= 1e6 ? (v / 1e6).toFixed(v >= 1e7 ? 0 : 1) + "M" : v >= 1e3 ? (v / 1e3).toFixed(v >= 1e4 ? 0 : 1) + "k" : String(Math.round(v));
  const TYPES = { sample: "sample", blank: "blank", standard: "standard", control: "control" };
  const KINDS = { full: "Full scan", ms2: "MS2", mrm: "MRM", hr: "LC-HRMS", msn: "MSn (infusion)", empty: "empty", error: "error" };
  const LAB = { forte: "strong", possibile: "possible", debole: "weak", progenitore: "parent" };

  const CSS = `
.tp{display:grid;grid-template-columns:minmax(300px,380px) 1fr;gap:14px;align-items:start;padding:12px 16px}
@media(max-width:1000px){.tp{grid-template-columns:1fr}}
.tp h3{margin:0 0 6px;font-size:14px}.tp .card{background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:10px 12px;margin-bottom:10px}
.tp .row{display:flex;gap:6px;align-items:center;flex-wrap:wrap;margin:4px 0}.tp .grow{flex:1;min-width:120px}.tp input[type=text],.tp textarea,.tp input[type=number]{width:100%;box-sizing:border-box}
.tp .mut{color:var(--muted);font-size:12px}.tp table{border-collapse:collapse;width:100%;font-size:12px}.tp th,.tp td{padding:3px 6px;border-bottom:1px solid var(--line);text-align:left;vertical-align:middle}
.tp th{position:sticky;top:0;background:var(--panel);font-weight:600;color:var(--muted)}.tp td.n{text-align:right;font-variant-numeric:tabular-nums}
.tp tr.sel{background:var(--sel)}.tp tr.sel td:first-child{box-shadow:inset 3px 0 0 var(--accent)}.tp tr.clk{cursor:pointer}.tp tr.clk:hover{background:var(--sel)}
.tp .badge{display:inline-block;padding:0 6px;border-radius:9px;font-size:11px;font-weight:600;border:1px solid var(--line)}
.tp .b-forte{background:#e7f6ee;color:var(--ok);border-color:#b7e1c9}.tp .b-possibile{background:#fdf3e0;color:var(--warn);border-color:#f1d9a8}.tp .b-debole{color:var(--muted)}
.tp .lv{display:inline-block;min-width:18px;text-align:center;border-radius:4px;color:#fff;font-weight:700;font-size:11px;padding:0 4px}.tp .lv3{background:#047857}.tp .lv4{background:#2b5c8a}.tp .lv5{background:#8a6d2b}
.tp .bar{height:6px;background:var(--line);border-radius:3px;overflow:hidden;margin-top:6px}.tp .bar i{display:block;height:100%;background:var(--accent);width:0}
.tp .tbl{max-height:46vh;overflow:auto;border:1px solid var(--line);border-radius:6px}.tp canvas{width:100%;height:210px;display:block}
.tp .crit li{margin:2px 0}.tp .crit .pass{color:var(--ok)}.tp .crit .fail{color:var(--bad)}.tp .crit .na{color:var(--muted)}
.tp .cols{display:grid;grid-template-columns:1fr 1fr;gap:12px}@media(max-width:1300px){.tp .cols{grid-template-columns:1fr}}
.tp .err{color:var(--bad)}.tp details>summary{cursor:pointer;color:var(--muted);font-size:12px}.tp textarea{font:12px ui-monospace,monospace;min-height:150px}
.tp .kv{display:grid;grid-template-columns:auto 1fr;gap:2px 10px;font-size:12px}.tp .kv b{color:var(--muted);font-weight:600}
.tp-wrap{padding:0}
#qt-nav-tab,nav button[data-v="tpmine"]{background:linear-gradient(135deg,#ffe29a,#f4c95d 55%,#d99a1d)!important;color:#3a2a00!important;border-color:#d99a1d!important;font-weight:700}
.hero{background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:8px;padding:12px 14px;margin-bottom:10px}
.tp #tp-files th{position:static}.tp .hero-i{display:inline-flex;color:var(--accent)}.hero-t{display:flex;gap:12px;align-items:center}.hero h2{margin:0;font-size:20px}.hero-s{color:var(--muted);font-size:12px}
.tp-title{margin:12px 16px 2px;font-size:20px}.tp-sub{margin:0 16px;color:var(--muted);font-size:12px}.tp .fsum{font-size:12px;color:var(--muted);margin:2px 0 6px}.tp #tp-files td,.tp #tp-files th{padding:1px 6px}.tp #tp-files select,.tp #tp-files input{font-size:11px;padding:0 3px;height:20px}
.tp .dig{display:flex;align-items:center;justify-content:center;gap:10px;width:100%;min-height:56px;font-size:18px;font-weight:700}.tp .dig .qi{display:inline-flex}.tp .tp-q{display:inline-grid;place-items:center;width:16px;height:16px;border:1px solid var(--muted);border-radius:50%;font-size:11px;color:var(--muted);cursor:help;margin-left:4px;font-style:normal}
.stats{display:flex;gap:10px;flex-wrap:wrap;margin-top:12px}.stat{background:var(--soft);border:1px solid var(--line);border-radius:8px;padding:6px 14px;min-width:110px}
.stat b{display:block;font-size:24px;line-height:1.1;font-variant-numeric:tabular-nums}.stat span{font-size:11px;color:var(--muted)}.stat.g b{color:var(--ok)}.stat.a b{color:var(--warn)}.stat.v b{color:var(--accent)}
.vtabs{display:flex;gap:6px;margin:0 0 8px;flex-wrap:wrap}.vt{border-radius:16px;padding:4px 14px}.vt.on{background:var(--accent);color:var(--panel);border-color:var(--accent)}
.gems{display:grid;grid-template-columns:repeat(auto-fill,minmax(250px,1fr));gap:10px;margin-bottom:10px}
.gem{position:relative;background:var(--panel);border:1px solid var(--line);border-left:3px solid var(--line);border-radius:8px;padding:8px 10px;cursor:pointer}.gem:hover{background:var(--sel)}.gem.sel,.mnode.sel{background:var(--sel);border-left-color:var(--accent)}
.gem.forte{border-left-color:var(--ok)}.gem.possibile{border-left-color:var(--warn)}.gem.sel{border-left-color:var(--accent)}.gem-h{display:flex;justify-content:space-between;gap:6px;align-items:center}

.mnode{cursor:pointer}.mnode:hover circle{stroke:var(--accent);stroke-width:3}.mnode.sel circle{stroke:var(--accent);stroke-width:4}
.frow{display:grid;grid-template-columns:210px 1fr 60px;gap:8px;align-items:center;padding:3px 0;cursor:pointer;font-size:12px}.frow:hover{background:var(--sel)}
.ft{display:block;height:14px;background:var(--line);border-radius:7px;overflow:hidden}.ft i{display:block;height:100%;width:0;border-radius:7px;transition:width .8s cubic-bezier(.3,.9,.3,1)}.fv{text-align:right;color:var(--muted);font-variant-numeric:tabular-nums}
.struct{text-align:center;margin-bottom:6px}.struct svg{max-width:100%;height:auto}
.tp-hd{display:flex;align-items:center;gap:12px;flex-wrap:wrap}.tp-hd .tp-title{margin-right:0}#tp-hd-struct{display:flex;align-items:center;gap:8px;font-size:12px;color:var(--muted)}#tp-hd-struct svg{height:48px;width:auto;max-width:120px}
.tp-ac{position:relative}.tp-ac input{width:100%}#tp-sug{position:absolute;left:0;right:0;top:100%;z-index:20;margin:2px 0 0;padding:2px;list-style:none;background:var(--panel);border:1px solid var(--line);border-radius:6px;box-shadow:0 4px 14px rgba(0,0,0,.18);max-height:260px;overflow:auto}
#tp-sug li{padding:5px 8px;border-radius:4px;cursor:pointer;display:flex;gap:8px;align-items:baseline;font-size:13px}#tp-sug li span{color:var(--muted);font-size:11px;margin-left:auto;white-space:nowrap}#tp-sug li[aria-selected=true],#tp-sug li:hover{background:var(--sel);box-shadow:inset 3px 0 0 var(--accent)}
#tp-drawbar{position:fixed;left:50%;bottom:14px;transform:translateX(-50%);z-index:95;display:flex;gap:8px;align-items:center;background:var(--panel);border:1px solid var(--accent);border-radius:20px;padding:6px 12px;box-shadow:0 2px 10px rgba(0,0,0,.25);font-size:13px}#tp-drawbar[hidden]{display:none}
.tp #tp-warn{font-size:12px;color:var(--bad,#b3261e)}
.words p{font-size:13px}


.tp-toast{position:fixed;left:50%;bottom:28px;transform:translateX(-50%);background:var(--ink);color:var(--panel);padding:8px 18px;border-radius:20px;z-index:99;box-shadow:0 6px 20px rgba(0,0,0,.3);animation:tpIn .3s ease;transition:opacity .5s}.tp-toast.out{opacity:0}
@keyframes tpIn{from{opacity:0}to{opacity:1}}


@media(prefers-reduced-motion:reduce){.tp *{animation:none!important;transition:none!important}}`;

  // ---------------------------------------------------------------- worker and calls
  let worker = null, ready = null, seq = 0;
  const pending = new Map();
  let onProgress = () => {};
  function startWorker() {
    if (ready) return ready;
    ready = (async () => {
      const c = ctx(), base = c.base;
      const [qq, tpz] = [await fetch(base + "mzlab.zip").then(r => { if (!r.ok) throw new Error("the in-browser compute engine is missing (use the website, not the local program)"); return r.arrayBuffer(); }), c.pyZip];
      const url = URL.createObjectURL(new Blob([c.files["tpmine-worker.js"]], { type: "text/javascript" }));
      worker = new Worker(url, { type: "module" });
      worker.onmessage = ev => {
        const m = ev.data;
        if (m.type === "progress") return onProgress(m.text, m.frac, m);
        if (m.type === "step") return onProgress(m.text, null);
        if (m.type === "ready") { const p = pending.get("init"); pending.delete("init"); return p && p.res(); }
        const p = pending.get(m.id); if (!p) return; pending.delete(m.id);
        m.error ? p.rej(new Error(m.error)) : p.res(m.result);
      };
      await new Promise((res, rej) => {
        pending.set("init", { res, rej });
        worker.onerror = e => rej(new Error(e.message || "worker"));
        worker.postMessage({ type: "init", pyodideMjs: new URL("pyodide/pyodide.mjs", base).href, indexURL: new URL("pyodide/", base).href, qqqZip: qq, tpZip: tpz });
      });
      URL.revokeObjectURL(url);
    })();
    ready.catch(() => { ready = null; });
    return ready;
  }
  const send = (msg, transfer) => new Promise((res, rej) => { const id = ++seq; pending.set(id, { res, rej }); worker.postMessage({ ...msg, id }, transfer || []); });
  const call = async (fn, ...args) => { await startWorker(); return JSON.parse(await send({ type: "call", fn, args: args.map(a => typeof a === "string" ? a : JSON.stringify(a)) })); };
  const callRaw = async (fn, ...args) => { await startWorker(); return send({ type: "call", fn, args }); };
  window.TPMINE_MEM = async () => { await startWorker(); return send({ type: "mem" }); };      // size of the worker's WebAssembly memory (tests)

  // ---------------------------------------------------------------- page files (the browser's own storage of the main program)
  async function idbFiles() {
    try {
      // "qqq_lab" is the name of the browser database (kept from the old name: renaming it would lose the users' data)
      if (!indexedDB.databases || !(await indexedDB.databases()).some(d => d.name === "qqq_lab")) return [];
      return await new Promise(res => {
        const r = indexedDB.open("qqq_lab");
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
    }
    g.strokeStyle = ink; g.strokeRect(M.l, M.t, pw, ph);
  }
  function niceStep(r) { const p = Math.pow(10, Math.floor(Math.log10(r))), m = r / p; return (m < 1.5 ? 1 : m < 3.5 ? 2 : m < 7.5 ? 5 : 10) * p; }
  const spark = k => {
    if (!k.length) return ""; const mx = Math.max(...k.map(p => p.area)) || 1, w = 70, h = 18;
    const pts = k.map((p, i) => `${(i / Math.max(1, k.length - 1) * (w - 4) + 2).toFixed(1)},${(h - 2 - p.area / mx * (h - 4)).toFixed(1)}`).join(" ");
    return `<svg width="${w}" height="${h}"><polyline points="${pts}" fill="none" stroke="currentColor" stroke-width="1.3"/></svg>`;
  };

  // ---------------------------------------------------------------- the tab
  function open(sec) {
    if (!document.getElementById("tp-css")) { const st = document.createElement("style"); st.id = "tp-css"; st.textContent = CSS; document.head.appendChild(st); }
    sec.innerHTML = `<div class="tp-wrap">
<div class="tp-hd"><h2 class="tp-title">mzFinder</h2><div id="tp-hd-struct" aria-live="polite"></div></div><p class="tp-sub">Automated transformation-product discovery</p>
<div class="tp">
<div id="tp-left">
 <div class="card"><h3>1 · Files</h3>
  <div class="row"><button id="tp-page" title="Reload the mzML files already open in the Data tab (browser memory)">Reload from Data</button><button id="tp-pick">Choose mzML...</button></div>
  <input type="file" id="tp-in" accept=".mzML,.mzml" multiple hidden>
  <div id="tp-fsum" class="fsum"></div><div id="tp-files" class="mut"></div><div class="row"><button id="tp-more" type="button" aria-expanded="false" hidden></button></div></div>
 <div class="card"><h3>2 · Parent compound</h3><div id="tp-struct"></div>
  <div class="row tp-ac"><input type="text" id="tp-name" placeholder="Name (type 2 letters to search the list)" role="combobox" aria-autocomplete="list" aria-expanded="false" aria-controls="tp-sug" aria-label="Parent compound name" autocomplete="off" spellcheck="false"><ul id="tp-sug" role="listbox" aria-label="Suggestions" hidden></ul></div>
  <div class="row"><input type="text" id="tp-mol" placeholder="Neutral formula (C10H12N2O3S)" aria-label="Neutral molecular formula" spellcheck="false"></div>
  <div class="row"><input type="text" id="tp-smi" placeholder="SMILES (optional)" aria-label="SMILES" spellcheck="false" style="flex:1"><button id="tp-draw" type="button" title="Draw the molecule in the Draw tab and bring the SMILES back here">Draw</button></div>
  <div id="tp-warn" role="status"></div>
  <div class="row"><label class="mut">Adduct</label><select id="tp-add"><option>[M+H]+</option><option>[M+Na]+</option><option>[M+NH4]+</option><option>[M-H]-</option></select><span class="mut" id="tp-prev"></span></div></div>
 <div class="card"><h3>3 · Settings</h3>
  <div class="row"><label class="mut grow" id="tp-tol-l">XIC window ±Da</label><input type="number" id="tp-tol" value="0.35" step="0.05" min="0.05" style="width:70px"></div>
  <div class="row"><label class="mut grow">RT tolerance ±min</label><input type="number" id="tp-rtt" value="0.25" step="0.05" min="0.05" style="width:70px"></div>
  <div class="row"><label class="mut grow">Combined transformations (steps)</label><input type="number" id="tp-steps" value="2" min="1" max="3" style="width:70px"></div>
  <div class="row"><label class="mut grow">Ignore RT before (min)</label><input type="number" id="tp-rtmin" value="0.5" step="0.1" min="0" style="width:70px"></div>
  <div class="row"><label><input type="checkbox" id="tp-disc" checked> Also look for unexpected ions</label><i class="tp-q" id="tp-disc-h" tabindex="0" role="note" aria-label="About unexpected ions" title="Low resolution only. Besides the candidates from the transformation rules, it searches the whole m/z range for ions that grow over time, are missing at t0 and in the blank, and match no candidate. Slower. With high-resolution files this option is hidden: the HR pipeline is already untargeted.">?</i></div>
  <details><summary>List of transformations (name;change)</summary><textarea id="tp-tr" spellcheck="false"></textarea></details></div>
 <div class="card" id="tp-minecard"><button id="tp-go" class="imp dig" type="button" aria-busy="false">${(typeof QICON !== "undefined" && QICON.mine) ? QICON.get("mine", 28) : ""}<span id="tp-go-t">Dig</span></button><div class="bar" id="tp-barbox" role="progressbar" aria-label="Progress" aria-valuemin="0" aria-valuemax="100"><i id="tp-bar"></i></div><div class="mut" id="tp-msg" role="status" style="margin-top:4px"></div></div>
</div>
<div id="tp-right"><div class="card mut" id="tp-empty">Load the files, enter the parent and press <b>Dig</b>: mzFinder calibrates the m/z on the parent, generates the candidates, extracts the XICs, looks for peaks that grow over time and are missing from the blank, checks the isotopes, compares the product ions with those of the parent and assigns a confidence level (Schymanski, as far as the resolution allows).</div></div></div></div>`;
    const $ = s => sec.querySelector(s);
    const st = { files: [], showAll: false, summary: null, sel: null, view: "gems", took: 0, filter: { forte: true, possibile: true, unexp: true, debole: false } };
    const msg = (t, err) => { $("#tp-msg").textContent = t || ""; $("#tp-msg").className = "mut" + (err ? " err" : ""); };
    const bar = f => { const w = Math.round(f * 100); $("#tp-bar").style.width = w + "%"; $("#tp-barbox").setAttribute("aria-valuenow", w); };
    onProgress = (t, f, m) => { if (t) msg(t); if (f != null) bar(f); if (m && window.TPLIVE) TPLIVE.event(m); };

    $("#tp-more").onclick = () => { st.showAll = !st.showAll; renderFiles(); };
    // ---- files
    const renderFiles = () => {
      modeUI(); if (!st.files.length) { $("#tp-files").innerHTML = ""; $("#tp-more").hidden = true; return; }
      $("#tp-files").innerHTML = `<table><tr><th>File</th><th>Data type</th><th>Sample</th><th>t (min)</th></tr>` + st.files.map((f, i) => `<tr><td title="${esc(f.name)}">${esc(f.name.replace(/\.mzml$/i, ""))}</td><td>${KINDS[f.kind] || f.kind}${f.error ? ` <span class="err" title="${esc(f.error)}">!</span>` : ""}</td>
        <td>${f.kind === "full" || f.kind === "mrm" || f.kind === "hr" ? `<select data-i="${i}" data-k="type">${Object.entries(TYPES).map(([k, v]) => `<option value="${k}"${f.type === k ? " selected" : ""}>${v}</option>`).join("")}</select>` : (f.kind === "ms2" ? "fragments" : "-")}</td>
        <td>${f.kind === "full" || f.kind === "ms2" || f.kind === "mrm" || f.kind === "hr" ? `<input type="number" data-i="${i}" data-k="time" value="${f.time ?? ""}" style="width:60px" step="any">` : ""}</td></tr>`).join("") + "</table>";
      const MAXV = 6, many = st.files.length > MAXV;
      $("#tp-files").querySelectorAll("tr").forEach((tr, i) => { if (i > 0) tr.hidden = many && !st.showAll && i > MAXV; });
      const more = $("#tp-more"); more.hidden = !many; more.setAttribute("aria-expanded", st.showAll ? "true" : "false");
      more.textContent = st.showAll ? "Show fewer" : "Show all " + st.files.length + " files";
      const cn = {}; st.files.forEach(f => { const k = f.kind === "hr" ? "LC-HRMS" : f.kind === "msn" ? "MSn" : (KINDS[f.kind] || f.kind); cn[k] = (cn[k] || 0) + 1; });
      const bl = st.files.filter(f => f.type === "blank").length;
      $("#tp-fsum").textContent = Object.entries(cn).map(([k, v]) => v + " " + k).join(" · ") + (bl ? " · " + bl + (bl === 1 ? " blank" : " blanks") : "");
      $("#tp-files").querySelectorAll("[data-k]").forEach(el => el.onchange = () => { const f = st.files[+el.dataset.i]; f[el.dataset.k] = el.dataset.k === "time" ? (el.value === "" ? null : +el.value) : el.value; });
    };
    const BIG = 50 << 20;                   // as in the main program: above this a file is mounted from its Blob, not copied (13 HR files = 1 GB)
    async function addFiles(list, only) {         // list: [{name, buf}] or [{name, file}] (File/Blob: read only if small)
      msg("Reading files...");
      try {
        await startWorker();
        let k = 0;
        for (const f of list) {
          msg(`Reading files... ${k} of ${list.length}`); bar(k++ / list.length);
          if (f.file && f.file.size > BIG) await send({ type: "put", name: f.name, blob: f.file });
          else { const buf = f.file ? await f.file.arrayBuffer() : f.buf; await send({ type: "put", name: f.name, buf }, [buf]); }
        }
        const info = await call("classify", list.map(f => f.name));
        const hint = (() => { try { return Object.fromEntries((window.E && E.files || []).map(x => [String(x.file || x.name || "").split(/[\\/]/).pop(), x])); } catch (_) { return {}; } })();
        st.files = st.files.filter(f => !info.some(i => i.name === f.name));
        for (const i of info) { if (i.hr_kind === "hr" || i.hr_kind === "msn") i.kind = i.hr_kind; const h = hint[i.name]; if (h && h.time != null) i.time = h.time; if (h && ["sample", "blank", "standard", "control"].includes(h.type)) i.type = h.type; st.files.push(i); }
        if (only) st.files = st.files.filter(f => f.kind === "full" || f.kind === "hr");   // automatic start: only the Full Scan / HRMS series
        st.files.sort((a, b) => (a.kind > b.kind ? 1 : a.kind < b.kind ? -1 : (a.time ?? -1) - (b.time ?? -1)));
        renderFiles(); msg(""); bar(0);
      } catch (e) { msg(String(e.message || e), true); bar(0); }
    }
    $("#tp-page").onclick = async () => {
      await loadFromData(false); };
    // default: the Full Scan / HRMS files already loaded in the Data tab (browser memory) are taken at once
    async function loadFromData(quiet) {
      msg("Looking for the files in browser memory...");
      const f = await idbFiles();
      if (!f.length) return msg(quiet ? "" : "No mzML in the Data tab of this browser: open them there or use “Choose mzML...”.", !quiet);
      await addFiles(f.map(x => x.buf instanceof Blob ? { name: x.name, file: x.buf } : { name: x.name, buf: x.buf.slice ? x.buf.slice(0) : x.buf }), quiet);
    }
    $("#tp-pick").onclick = () => $("#tp-in").click();
    $("#tp-in").onchange = async ev => { const fs = [...ev.target.files]; ev.target.value = ""; await addFiles(fs.map(f => ({ name: f.name, file: f }))); };
    // mode: high resolution -> XIC window in ppm (default 5) and RT tolerance from the chromatographic method; unit resolution -> Da and 0.25 min
    let curMode = "";
    function modeUI() {
      const hr = st.files.some(f => f.kind === "hr"), m = !st.files.length ? "" : hr ? "hr" : "lr";
      const t = $("#tp-tol"), r = $("#tp-rtt");
      if (m !== curMode) {
        curMode = m;
        if (m === "hr") {
          $("#tp-tol-l").textContent = "XIC window ±ppm"; t.value = 5; t.step = 1; t.min = 1;
          const span = Math.max(0, ...st.files.filter(f => f.kind === "hr").map(f => f.rt_max || 0));
          r.value = span ? Math.max(0.05, Math.round(span * 0.005 / 0.05) * 0.05).toFixed(2) : "0.10";      // 0.5 % of the gradient length (20 min: 0.10)
          r.title = "From the chromatographic method (0.5 % of the run length)";
        } else { $("#tp-tol-l").textContent = "XIC window ±Da"; t.value = 0.35; t.step = 0.05; t.min = 0.05; r.value = 0.25; r.title = ""; }
      }
      const n = st.files.length;
      $("#tp-disc").closest(".row").hidden = m === "hr";                // the HR pipeline is already untargeted
    }
    $("#tp-mol").addEventListener("input", modeUI);
    setTimeout(() => loadFromData(true), 0);
    sec.addEventListener("tpshow", () => { if (!st.files.length) loadFromData(true); });

    // ---- parent: values, preview, structure
    const FORMULA = /^[A-Za-z0-9()]+$/;
    const parentVals = () => {          // formula field and SMILES field; a SMILES typed in the formula field still counts as a SMILES
      let mol = $("#tp-mol").value.trim(), smi = $("#tp-smi").value.trim();
      if (mol && !FORMULA.test(mol) && !smi) { smi = mol; mol = ""; }
      return { mol, smi };
    };
    let pvT = 0, OCLp = null;
    const ocl = () => (OCLp = OCLp || import(new URL("vendor/openchemlib.js", ctx().base).href));
    const smilesInfo = async t => { try { const m = (await ocl()).Molecule.fromSmiles(t); return { mol: m, formula: m.getMolecularFormula().formula }; } catch (_) { return null; } };
    const preview = () => {
      clearTimeout(pvT);
      pvT = setTimeout(async () => {
        const { mol, smi } = parentVals(), ad = $("#tp-add").value;
        $("#tp-warn").textContent = "";
        if (!mol && !smi) { $("#tp-prev").textContent = ""; structure(""); return; }
        structure(smi);
        let info = null;
        if (smi) info = await smilesInfo(smi);
        let q = mol || (info && info.formula) || smi;       // formula missing: the one computed from the SMILES (OpenChemLib)
        if (!mol && info && !$("#tp-mol").value.trim()) { $("#tp-mol").value = info.formula; }
        try {
          const r = await call("formula_info", q, ad);
          $("#tp-prev").innerHTML = r.ok ? `${esc(r.formula)} · M ${r.neutral} · ${esc(ad)} <i>m/z</i> ${r.mz}` : `<span class="err">${esc(r.error)}</span>`;
          if (r.ok && mol && info && info.formula !== r.formula) $("#tp-warn").textContent = `The formula (${r.formula}) does not match the SMILES (${info.formula}).`;
        } catch (e) { $("#tp-prev").textContent = ""; }
        header(info ? info.formula : (mol || ""), smi);
      }, 350);
    };
    function structure(t) {             // parent structure drawn with OpenChemLib (already in the site for the Disegno tab)
      const el = $("#tp-struct"); if (!el) return;
      if (!t) { el.innerHTML = ""; return; }
      ocl().then(OCL => { el.innerHTML = `<div class="struct">${OCL.Molecule.fromSmiles(t).toSVG(300, 170)}</div>`; }).catch(() => { el.innerHTML = ""; });
    }
    function header(formula, smi) {      // small structure + name + formula next to the title
      const el = $("#tp-hd-struct"); if (!el) return;
      const nm = $("#tp-name").value.trim();
      if (!smi) { el.innerHTML = ""; return; }
      ocl().then(OCL => { el.innerHTML = OCL.Molecule.fromSmiles(smi).toSVG(120, 48) + `<span>${esc(nm)}${nm && formula ? " · " : ""}${esc(formula)}</span>`; }).catch(() => { el.innerHTML = ""; });
    }
    $("#tp-mol").oninput = preview; $("#tp-smi").oninput = preview; $("#tp-add").onchange = preview;

    // ---- name -> formula / SMILES from the curated list data/composti.csv (already loaded with mzFinder; nothing is downloaded here)
    let LIST = null;
    const csvRows = txt => {            // minimal CSV reader (quotes, commas, newlines inside quotes)
      const out = []; let row = [], f = "", q = false;
      for (let i = 0; i < txt.length; i++) {
        const c = txt[i];
        if (q) { if (c === '"') { if (txt[i + 1] === '"') { f += '"'; i++; } else q = false; } else f += c; }
        else if (c === '"') q = true; else if (c === ",") { row.push(f); f = ""; }
        else if (c === "\n") { row.push(f); out.push(row); row = []; f = ""; } else if (c !== "\r") f += c;
      }
      if (f || row.length) { row.push(f); out.push(row); }
      return out;
    };
    const getList = () => LIST || (LIST = csvRows(ctx().files["composti.csv"] || "").slice(1).filter(r => r.length >= 5).map(r => {
      const names = r[0].split(";").map(x => x.trim()).filter(Boolean);
      return { names, low: names.map(x => x.toLowerCase()), formula: r[1], mass: parseFloat(r[2]), smiles: r[3] };
    }));
    const suggest = q => {
      const w = q.trim().toLowerCase(); if (w.length < 2) return [];
      const out = [];
      for (const c of getList()) {
        let best = 9;
        c.low.forEach(n => { const i = n.indexOf(w); if (i < 0) return; const sc = n === w ? 0 : i === 0 ? 1 : /[\s\-(,]/.test(n[i - 1]) ? 2 : 3; if (sc < best) best = sc; });
        if (best < 9) out.push([best, c.names[0].length, c]);
      }
      return out.sort((a, b) => a[0] - b[0] || a[1] - b[1]).slice(0, 8).map(x => x[2]);
    };
    let sugItems = [], sugI = -1;
    const sugBox = $("#tp-sug"), nameIn = $("#tp-name");
    const sugClose = () => { sugBox.hidden = true; sugI = -1; nameIn.setAttribute("aria-expanded", "false"); nameIn.removeAttribute("aria-activedescendant"); };
    const pick = i => {
      const it = sugItems[i]; if (!it) return;
      if (it.link) { window.open(it.link, "_blank", "noopener"); sugClose(); return; }
      nameIn.value = it.c.names[0]; $("#tp-mol").value = it.c.formula; $("#tp-smi").value = it.c.smiles; sugClose(); preview(); nameIn.focus();
    };
    const sugMark = () => { [...sugBox.children].forEach((li, k) => li.setAttribute("aria-selected", k === sugI ? "true" : "false")); if (sugI >= 0) nameIn.setAttribute("aria-activedescendant", "tp-sug-" + sugI); else nameIn.removeAttribute("aria-activedescendant"); };
    const sugShow = () => {
      const w = nameIn.value.trim(); if (w.length < 2) return sugClose();
      sugItems = suggest(w).map(c => ({ c }));
      sugItems.push({ link: "https://pubchem.ncbi.nlm.nih.gov/#query=" + encodeURIComponent(w), word: w });
      sugBox.innerHTML = sugItems.map((it, k) => it.link ? `<li role="option" id="tp-sug-${k}" data-k="${k}" class="mut">Search PubChem for "${esc(it.word)}"</li>` : `<li role="option" id="tp-sug-${k}" data-k="${k}">${esc(it.c.names[0])}<span>${esc(it.c.formula)} · ${it.c.mass.toFixed(4)}</span></li>`).join("");
      sugBox.hidden = false; nameIn.setAttribute("aria-expanded", "true"); sugI = -1; sugMark();
    };
    nameIn.addEventListener("input", () => { sugShow(); header($("#tp-mol").value.trim(), parentVals().smi); });
    nameIn.addEventListener("keydown", e => {
      if (sugBox.hidden) { if (e.key === "ArrowDown") sugShow(); return; }
      if (e.key === "ArrowDown") { sugI = (sugI + 1) % sugItems.length; sugMark(); e.preventDefault(); }
      else if (e.key === "ArrowUp") { sugI = (sugI - 1 + sugItems.length) % sugItems.length; sugMark(); e.preventDefault(); }
      else if (e.key === "Enter" && sugI >= 0) { pick(sugI); e.preventDefault(); }
      else if (e.key === "Escape") { sugClose(); e.preventDefault(); }
    });
    nameIn.addEventListener("blur", () => setTimeout(sugClose, 150));
    sugBox.addEventListener("mousedown", e => { const li = e.target.closest("li"); if (li) { e.preventDefault(); pick(+li.dataset.k); } });

    // ---- Draw: the Draw tab (Ketcher) with a small bar to bring the SMILES back
    const drawBtn = $("#tp-draw");
    if (window.MZLAB_PHONE) drawBtn.hidden = true;
    let dbar = document.getElementById("tp-drawbar");
    if (!dbar) {
      dbar = document.createElement("div"); dbar.id = "tp-drawbar"; dbar.hidden = true; dbar.setAttribute("role", "region"); dbar.setAttribute("aria-label", "mzFinder: parent structure");
      dbar.innerHTML = `<span>Drawing the mzFinder parent</span><button type="button" id="tp-draw-use" class="imp">Use this structure</button><button type="button" id="tp-draw-no">Cancel</button>`;
      document.body.appendChild(dbar);
    }
    let drawing = false;
    const back = () => { drawing = false; dbar.hidden = true; if (typeof setView === "function") setView("tpmine"); };
    drawBtn.onclick = () => { drawing = true; dbar.hidden = false; setView("draw"); };
    document.addEventListener("tpview", e => { dbar.hidden = !(drawing && e.detail.view === "draw"); if (e.detail.view !== "draw" && e.detail.view !== "tpmine") drawing = false; });
    document.getElementById("tp-draw-no").onclick = back;
    document.getElementById("tp-draw-use").onclick = async () => {
      let t = ""; try { t = window.TPDraw ? await window.TPDraw.smiles() : ""; } catch (_) { t = ""; }
      if (!t) { toast("Nothing drawn yet."); return; }
      $("#tp-smi").value = t; $("#tp-mol").value = ""; preview(); back();
    };
    startWorker().then(async () => { $("#tp-tr").value = await callRaw("default_transformations"); }).catch(e => msg(String(e.message || e), true));

    // ---- run
    const setBusy = b => { const g = $("#tp-go"); g.disabled = b; g.setAttribute("aria-busy", b ? "true" : "false"); $("#tp-go-t").textContent = b ? "Digging..." : "Dig"; };
    $("#tp-go").onclick = async () => {
      const pv = parentVals(), mol = pv.mol || pv.smi;      // the engine takes the formula, or the SMILES when the formula is missing
      const hrMode = st.files.some(f => f.kind === "hr");
      if (!hrMode && !st.files.some(f => f.kind === "full")) return msg("Full-scan (MS1) sample files are needed.", true);
      if (!mol) return msg("Enter the neutral molecular formula or the SMILES of the parent.", true);
      setBusy(true); $("#tp-bar").style.width = "2%"; msg("Digging..."); const t0 = performance.now();
      if (window.TPLIVE) TPLIVE.start($("#tp-right"), hrMode, { esc, call });
      try {
        const files = st.files.filter(f => f.kind !== "error" && f.kind !== "empty").map(f => ({ name: f.name, type: f.type, time: f.time, kind: f.kind }));
        const settings = { tol_da: +$("#tp-tol").value, rt_tol_min: +$("#tp-rtt").value, max_steps: +$("#tp-steps").value, rt_min: +$("#tp-rtmin").value, discover: $("#tp-disc").checked };
        const parent = { name: $("#tp-name").value.trim(), neutral: mol, adduct: $("#tp-add").value };
        if (pv.smi) parent.smiles = pv.smi;
        if (hrMode) {          // high resolution: its own engine and view (21_tpmine_hr.js)
          if (parent.adduct !== "[M+H]+") throw new Error("High resolution uses the [M+H]+ ion: choose that adduct.");
          const hs = JSON.parse(await callRaw("run", JSON.stringify(files), JSON.stringify(parent), JSON.stringify({ ppm_prec: +$("#tp-tol").value, rt_tol_min: +$("#tp-rtt").value }), ""));
          st.took = performance.now() - t0; st.summary = null; st.det = {};
          if (window.TPLIVE) TPLIVE.stop(true);
          window.TPHR.render($("#tp-right"), hs, { esc, fmtA, chart, hue, call, callRaw, toast, dl });
          $("#tp-bar").style.width = "100%"; msg("Done."); $("#tp-right").scrollIntoView({ behavior: "smooth", block: "start" });
          setBusy(false); return;
        }
        st.summary = JSON.parse(await callRaw("run", JSON.stringify(files), JSON.stringify(parent), JSON.stringify(settings), $("#tp-tr").value));
        st.took = performance.now() - t0; st.sel = null; st.det = {}; if (window.TPLIVE) TPLIVE.stop(true); render(); $("#tp-bar").style.width = "100%"; msg("Done."); $("#tp-right").scrollIntoView({ behavior: "smooth", block: "start" });
      } catch (e) { if (window.TPLIVE) TPLIVE.stop(false); msg(String(e.message || e), true); $("#tp-bar").style.width = "0"; }
      setBusy(false);
    };

    // ---- results
    const VIEWS = [["tab", "Table"], ["gems", "💎 Gems"], ["map", "🕸 Reaction map"], ["kin", "📈 Kinetics"], ["film", "🎬 Film"]];
    function countUp(el) {
      const to = +el.dataset.count, dec = +(el.dataset.dec || 0), t0 = performance.now(), D = 900;
      const step = t => { const k = Math.min(1, (t - t0) / D), e = 1 - Math.pow(1 - k, 3); el.textContent = (to * e).toFixed(dec); if (k < 1) requestAnimationFrame(step); };
      requestAnimationFrame(step);
    }
    function confetti() {
      const cv = document.createElement("canvas"); cv.style.cssText = "position:fixed;inset:0;width:100%;height:100%;pointer-events:none;z-index:95"; document.body.appendChild(cv);
      cv.width = innerWidth; cv.height = innerHeight; const g = cv.getContext("2d"), cols = ["#2b5c8a", "#047857", "#d99a1d", "#b42318", "#7c3aed", "#0ea5e9"];
      const P = Array.from({ length: 140 }, () => ({ x: innerWidth / 2 + (Math.random() - .5) * 300, y: innerHeight * 0.35, vx: (Math.random() - .5) * 14, vy: -Math.random() * 13 - 3, r: 3 + Math.random() * 4, c: cols[(Math.random() * cols.length) | 0], rot: Math.random() * 6, vr: (Math.random() - .5) * .4 }));
      const t0 = performance.now();
      const f = t => { const k = (t - t0) / 1800; g.clearRect(0, 0, cv.width, cv.height); for (const p of P) { p.vy += .35; p.x += p.vx; p.y += p.vy; p.vx *= .99; p.rot += p.vr; g.save(); g.translate(p.x, p.y); g.rotate(p.rot); g.globalAlpha = Math.max(0, 1 - k); g.fillStyle = p.c; g.fillRect(-p.r, -p.r / 2, p.r * 2, p.r); g.restore(); } k < 1 ? requestAnimationFrame(f) : cv.remove(); };
      requestAnimationFrame(f);
    }
    const toast = t => { const d = document.createElement("div"); d.className = "tp-toast"; d.textContent = t; document.body.appendChild(d); setTimeout(() => d.classList.add("out"), 2200); setTimeout(() => d.remove(), 2800); };
    const good = r => r.label === "forte" || r.label === "possibile";
    const dispName = r => r.kind === "unexpected" ? r.name : r.name;
    const shortN = (t, n = 26) => t.length > n ? t.slice(0, n - 1) + "…" : t;
    const col = r => r.label === "progenitore" ? "#2b5c8a" : r.kind === "unexpected" ? "#7c3aed" : r.label === "forte" ? "#047857" : "#d99a1d";

    function render() {
      const s = st.summary, R = $("#tp-right");
      const par = s.rows.find(r => r.label === "progenitore");
      const cnt = k => s.rows.filter(r => (k === "unexp" ? r.kind === "unexpected" && good(r) : r.kind === "candidate" && r.label === k)).length;
      const nF = cnt("forte"), nP = cnt("possibile"), nU = cnt("unexp");
      R.innerHTML = `<div class="hero"><div class="hero-t"><span class="hero-i">${(typeof QICON !== "undefined" && QICON.mine) ? QICON.get("mine", 44) : "⛏"}</span>
        <div><h2>${nF ? "Vein found!" : nP ? "A few nuggets..." : "Poor vein"}</h2><div class="hero-s">${esc(s.parent.formula || "parent")} · ${esc(s.parent.adduct)} <i>m/z</i> ${s.parent.mz}${par && par.ref_rt ? ` · RT ${par.ref_rt} min` : ""} · dug in ${(st.took / 1000).toFixed(1)} s</div></div></div>
        <div class="stats"><div class="stat g"><b data-count="${nF}">0</b><span>Strong TPs 💎</span></div><div class="stat a"><b data-count="${nP}">0</b><span>Possible TPs</span></div>
        <div class="stat v"><b data-count="${nU}">0</b><span>unexpected ions</span></div>
        <div class="stat"><b data-count="${s.decay && s.decay.half_life_min != null ? s.decay.half_life_min : 0}" data-dec="1">0</b><span>parent t½ (min)</span></div>
        <div class="stat"><b data-count="${s.offset ? Math.abs(s.offset.offset) : 0}" data-dec="2">0</b><span>corrected <i>m/z</i> offset (Da)</span></div>
        <div class="stat"><b data-count="${s.rows.length}">0</b><span>ions analysed</span></div></div>
        <div class="row" style="margin-top:10px"><button id="tp-x-xlsx">Excel (.xlsx)</button><button id="tp-x-csv">CSV</button><button id="tp-x-json">JSON</button><button id="tp-x-txt" title="Copy a summary to paste into the report">📋 Copy summary</button></div></div>
        <div class="vtabs">${VIEWS.map(([k, n]) => `<button class="vt${st.view === k ? " on" : ""}" data-v="${k}">${n}</button>`).join("")}</div>
        <div id="tp-view"></div>${mrmCard(s.mrm)}<div id="tp-det"></div>`;
      R.querySelectorAll(".stat b").forEach(countUp);
      R.querySelectorAll(".vt").forEach(b => b.onclick = () => { st.view = b.dataset.v; R.querySelectorAll(".vt").forEach(x => x.classList.toggle("on", x === b)); view(); });
      $("#tp-x-xlsx").onclick = () => exportAll("xlsx"); $("#tp-x-csv").onclick = () => exportAll("csv"); $("#tp-x-json").onclick = () => exportAll("json");
      $("#tp-x-txt").onclick = async () => { try { await navigator.clipboard.writeText(summaryText()); toast("Summary copied to the clipboard"); } catch (_) { dl("summary_mzFinder.txt", summaryText(), "text/plain"); } };
      view();
      const cc = $("#tp-c-cal");
      if (cc) { const c = JSON.parse(cc.dataset.cal); chart(cc, [{ x: [0, c.hi], y: [c.intercept, c.slope * c.hi + c.intercept], color: "#2b5c8a" }, { x: c.points.map(p => p.conc), y: c.points.map(p => p.area), color: "#b42318", dots: true, w: 0 }], { xlabel: "concentration (mg/L)", x0: 0 }); }
      if (nF) confetti();
    }

    function view() {
      const V = $("#tp-view"), s = st.summary; stopFilm();
      const list = s.rows.filter(r => r.label !== "progenitore" && good(r));
      if (st.view === "tab") {
        const f = st.filter;
        V.innerHTML = `<div class="card"><div class="row"><label><input type="checkbox" data-f="forte" ${f.forte ? "checked" : ""}> strong</label><label><input type="checkbox" data-f="possibile" ${f.possibile ? "checked" : ""}> possible</label><label><input type="checkbox" data-f="unexp" ${f.unexp ? "checked" : ""}> unexpected</label><label><input type="checkbox" data-f="debole" ${f.debole ? "checked" : ""}> weak</label><span class="mut">↑ ↓ to scroll</span></div><div class="tbl"><table id="tp-t"></table></div></div>`;
        V.querySelectorAll("[data-f]").forEach(el => el.onchange = () => { f[el.dataset.f] = el.checked; rows(); });
        rows();
      } else if (st.view === "gems") {
        V.innerHTML = list.length ? `<div class="gems">${list.map((r, i) => gem(r, i)).join("")}</div>` : `<div class="card mut">No gems: no candidate passes the criteria.</div>`;
        V.querySelectorAll(".gem").forEach(g => g.onclick = () => show(+g.dataset.id));
      } else if (st.view === "map") {
        V.innerHTML = `<div class="card"><div class="mut">Each node is an ion that passes the criteria: the radius follows the maximum area, the colour the confidence (green strong, amber possible, purple unexpected). Lines show the transformation (Δ formula). Click a node for details.</div><div id="tp-map"></div></div>`;
        mapView($("#tp-map"));
      } else if (st.view === "kin") {
        V.innerHTML = `<div class="card"><h3>All kinetics, each at its own maximum (100 %)</h3><canvas id="tp-c-all" style="height:320px"></canvas><div class="row" id="tp-leg-all"></div></div>`;
        const top = list.filter(r => r.kind === "candidate").slice(0, 8), par = s.rows.find(r => r.label === "progenitore");
        const ser = [par, ...top].map((r, i) => { const k = r.kinetics, mx = Math.max(...k.map(p => p.area)) || 1; return { x: k.map(p => p.time), y: k.map(p => 100 * p.area / mx), color: i ? hue(i - 1, top.length) : "#555", dash: i ? [] : [6, 4], w: i ? 2 : 2.4, label: r.name, dots: !!i }; });
        chart($("#tp-c-all"), ser, { xlabel: "time (min)" });
        $("#tp-leg-all").innerHTML = ser.map(t => `<span class="mut"><span style="display:inline-block;width:14px;border-top:2px ${t.dash.length ? "dashed" : "solid"} ${t.color};vertical-align:middle"></span> ${esc(shortN(t.label, 34))}</span>`).join("");
      } else if (st.view === "film") {
        filmView(V);
      }
    }

    function gem(r, i) {
      const k = r.kinetics, mx = Math.max(...k.map(p => p.area)) || 1, w = 220, h = 54;
      const pts = k.map((p, j) => [(j / Math.max(1, k.length - 1) * (w - 6) + 3), (h - 4 - p.area / mx * (h - 10))]);
      const line = pts.map(p => p.join(",")).join(" "), area = `3,${h - 3} ${line} ${w - 3},${h - 3}`;
      return `<div class="gem ${r.label}${r.id === st.sel ? " sel" : ""}" data-id="${r.id}" tabindex="0"><div class="gem-h"><b>${esc(shortN(r.name, 34))}</b>${r.level ? `<span class="lv lv${r.level}">${r.level}</span>` : ""}</div>
        <div class="mut">${esc(r.formula || "")} · <i>m/z</i> ${r.mz.toFixed(1)} · RT ${r.ref_rt ?? "?"}${r.insource ? " · in-source fragment" + (r.isf ? " (P " + Math.round(100 * r.isf.probs.isf) + "%" + (r.isf.doubtful ? ", doubtful" : "") + ")" : "?") : (r.isf && r.isf.doubtful ? " · doubtful origin (P ISF " + Math.round(100 * r.isf.probs.isf) + "%)" : "")}</div>
        <svg viewBox="0 0 ${w} ${h}" width="100%" height="${h}"><polygon points="${area}" fill="${col(r)}" opacity=".15"/><polyline points="${line}" fill="none" stroke="${col(r)}" stroke-width="2"/>${pts.map(p => `<circle cx="${p[0]}" cy="${p[1]}" r="2.3" fill="${col(r)}"/>`).join("")}</svg>
        <div class="mut">max at ${r.tmax ?? "?"} min · area ${fmtA(r.max_area)}${r.kind === "candidate" ? ` · ${esc(r.delta)}` : ` · Δ<i>m/z</i> ${r.delta_mz >= 0 ? "+" : ""}${r.delta_mz}`}</div></div>`;
    }

    function mapView(host) {
      const s = st.summary, par = s.rows.find(r => r.label === "progenitore");
      const cand = s.rows.filter(r => r.kind === "candidate" && r.label !== "progenitore" && good(r));
      const unx = s.rows.filter(r => r.kind === "unexpected" && good(r) && !(r.insource && !(r.isf && r.isf.doubtful))).slice(0, 10);
      const cols = [[par], cand.filter(r => r.steps === 1), cand.filter(r => r.steps === 2), [...cand.filter(r => r.steps >= 3), ...unx]];
      const W = 1000, colX = [90, 330, 590, 850], rowsN = Math.max(...cols.map(c => c.length)), H = Math.max(300, rowsN * 56 + 50);
      const maxA = Math.max(...s.rows.map(r => r.max_area)) || 1, pos = {};
      cols.forEach((c, ci) => c.forEach((r, i) => { pos[r.id] = [colX[ci], 30 + (i + 0.5) * (H - 40) / c.length]; }));
      const node = r => pos[r.id];
      const byName = Object.fromEntries(cand.filter(r => r.steps === 1).map(r => [r.name, r]));
      let edges = "";
      for (const r of [...cand, ...unx]) {
        const src = r.kind === "unexpected" ? [par] : r.steps === 1 ? [par] : (() => { const parts = r.name.split(" + ").map(n => byName[n]).filter(Boolean); return parts.length ? parts : [par]; })();
        for (const a of src) {
          const [x1, y1] = node(a), [x2, y2] = node(r), mx = (x1 + x2) / 2, my = (y1 + y2) / 2, lab = r.kind === "unexpected" ? `Δ${r.delta_mz >= 0 ? "+" : ""}${r.delta_mz}` : (r.steps === 1 ? r.delta : "");
          edges += `<path d="M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}" fill="none" stroke="${col(r)}" stroke-opacity=".45" stroke-width="1.6"${r.kind === "unexpected" || src[0] === par && r.steps > 1 ? ' stroke-dasharray="5 4"' : ""}/>${lab ? `<text x="${mx}" y="${(y1 + y2) / 2 - 4}" font-size="10" fill="#687080" text-anchor="middle">${esc(lab)}</text>` : ""}`;
        }
      }
      const nodes = [par, ...cand, ...unx].map(r => { const [x, y] = node(r), rad = 9 + 15 * Math.log10(1 + r.max_area) / Math.log10(1 + maxA);
        return `<g class="mnode${r.id === st.sel ? " sel" : ""}" data-id="${r.id}" transform="translate(${x},${y})"><circle r="${rad.toFixed(1)}" fill="${col(r)}" fill-opacity=".85" stroke="#fff" stroke-width="2"><title>${esc(r.name)}</title></circle>${r.level ? `<text y="4" font-size="11" fill="#fff" font-weight="700" text-anchor="middle">${r.level}</text>` : ""}<text y="${(rad + 13).toFixed(0)}" font-size="11" text-anchor="middle" fill="currentColor">${esc(shortN(r.name, 24))}</text><text y="${(rad + 25).toFixed(0)}" font-size="10" text-anchor="middle" fill="#687080">${r.mz.toFixed(1)}</text></g>`; }).join("");
      host.innerHTML = `<svg viewBox="0 0 ${W} ${H}" width="100%" style="max-height:${H}px">${[["parent", 0], ["1 step", 1], ["2 steps", 2], ["3 steps / unexpected", 3]].map(([t, i]) => `<text x="${colX[i]}" y="14" font-size="11" fill="#687080" text-anchor="middle" font-weight="700">${t}</text>`).join("")}${edges}${nodes}</svg>`;
      host.querySelectorAll(".mnode").forEach(g => g.onclick = () => show(+g.dataset.id));
    }

    let filmT = null;
    const stopFilm = () => { if (filmT) { clearInterval(filmT); filmT = null; } };
    function filmView(V) {
      const s = st.summary, par = s.rows.find(r => r.label === "progenitore"), items = [par, ...s.rows.filter(r => r.kind === "candidate" && good(r)).slice(0, 12)], times = s.times;
      V.innerHTML = `<div class="card"><div class="row"><button id="tp-play">▶ Start the film</button><input type="range" id="tp-sl" min="0" max="${times.length - 1}" value="0" class="grow"><b id="tp-tl">t = ${times[0]} min</b></div>
        <div class="mut">Each bar is the peak area, in % of its maximum: the parent decreases, the transformation products rise (and then, sometimes, fall).</div>
        <div id="tp-film">${items.map(r => `<div class="frow" data-id="${r.id}"><span class="fl" title="${esc(r.name)}">${esc(shortN(r.name, 30))}</span><span class="ft"><i style="background:${col(r)}"></i></span><span class="fv"></span></div>`).join("")}</div></div>`;
      const set = i => {
        $("#tp-sl").value = i; $("#tp-tl").textContent = `t = ${times[i]} min`;
        V.querySelectorAll(".frow").forEach(row => { const r = items.find(x => x.id === +row.dataset.id), mx = Math.max(...r.kinetics.map(p => p.area)) || 1, k = r.kinetics.find(p => p.time === times[i]), v = k ? 100 * k.area / mx : 0;
          row.querySelector("i").style.width = v.toFixed(1) + "%"; row.querySelector(".fv").textContent = k && k.area ? fmtA(k.area) : "-"; });
      };
      $("#tp-sl").oninput = e => { stopFilm(); $("#tp-play").textContent = "▶ Start the film"; set(+e.target.value); };
      $("#tp-play").onclick = () => {
        if (filmT) { stopFilm(); $("#tp-play").textContent = "▶ Start the film"; return; }
        let i = 0; $("#tp-play").textContent = "⏸ Pause"; set(0);
        filmT = setInterval(() => { i++; if (i >= times.length) { stopFilm(); $("#tp-play").textContent = "↺ Replay"; return; } set(i); }, 900);
      };
      V.querySelectorAll(".frow").forEach(r => r.onclick = () => show(+r.dataset.id));
      set(0);
    }

    function summaryText() {
      const s = st.summary, par = s.rows.find(r => r.label === "progenitore"), L = [];
      L.push(`Search for transformation products of ${s.parent.formula || "parent"} (${s.parent.adduct}, m/z ${s.parent.mz}${par && par.ref_rt ? `, RT ${par.ref_rt} min` : ""}).`);
      if (s.decay) L.push(`The parent decays with first-order kinetics: k = ${s.decay.k_per_min} min^-1, t1/2 = ${s.decay.half_life_min} min (R2 = ${s.decay.r2}).`);
      if (s.offset) L.push(`m/z offset measured on the parent: ${s.offset.offset >= 0 ? "+" : ""}${s.offset.offset} Da${s.offset.applied ? " (corrected)" : ""}.`);
      const top = s.rows.filter(r => r.label !== "progenitore" && good(r) && !(r.insource && !(r.isf && r.isf.doubtful)));
      L.push(`Candidates that grow over time and are absent from the blank: ${top.filter(r => r.kind === "candidate").length}; unexpected ions: ${top.filter(r => r.kind === "unexpected").length}.`);
      for (const r of top.slice(0, 15)) L.push(`- ${r.name}${r.formula ? ` (${r.formula}${r.delta ? ", " + r.delta : ""})` : ""}: m/z ${r.mz.toFixed(1)}, RT ${r.ref_rt} min, maximum at ${r.tmax} min, area ${fmtA(r.max_area)}, verdict ${LAB[r.label] || r.label}${r.level ? `, confidence level ${r.level}` : ""}.`);
      L.push("Unit resolution: each row is a candidate, not an identification; levels 2 and 1 require a library spectrum or a standard.");
      return L.join("\n");
    }

    function mrmCard(m) {
      if (!m) return "";
      const q = m.quantifier, qual = m.transitions.slice(1).map(t => t.name), c = m.calibration;
      const body = m.rows.map(r => { const i = r.ion[q]; return `<tr><td>${esc(r.label)}</td><td>${TYPES[r.type] || r.type}</td><td class="n">${r.time ?? ""}</td><td class="n">${r.conc ?? ""}</td><td class="n">${i.detected ? fmtA(i.area) : "-"}</td><td class="n">${i.rt != null ? i.rt.toFixed(2) : ""}</td><td class="n">${i.snr ? i.snr.toFixed(0) : ""}</td>${qual.map(n => `<td class="n">${r.ion[n].detected ? fmtA(r.ion[n].area) : "-"}</td><td class="n">${r.ratios[n] != null ? r.ratios[n].toFixed(2) : ""}</td>`).join("")}<td>${r.ratio_ok == null ? "" : r.ratio_ok ? "✓" : "<span class='err'>out</span>"}</td><td class="n">${r.quant != null ? (+r.quant.toPrecision(3)) : ""}</td><td>${r.in_range == null ? "" : r.in_range ? "" : "<span class='err' title='outside the range of the calibration points'>off line</span>"}</td></tr>`; }).join("");
      return `<div class="card"><h3>MRM: automatic integration</h3><div class="cols"><div><div class="mut">Quantifier ${esc(q)}${qual.length ? ` · qualifier ${qual.map(esc).join(", ")}` : ""} · reference RT ${m.ref_rt ? m.ref_rt.toFixed(2) : "?"} min · ion ratio of the standards ${Object.values(m.ratio_ref).map(v => v == null ? "-" : v.toFixed(2)).join(", ")} (±${Math.round(m.ratio_tol * 100)} %)</div>
        <div class="tbl" style="max-height:300px"><table><tr><th>File</th><th>Type</th><th>t</th><th>conc. (mg/L)</th><th>Quant. area</th><th>RT</th><th>S/N</th>${qual.map(n => `<th>Qual. area</th><th>Qual/Quant</th>`).join("")}<th>Ratio</th><th>Calculated (mg/L)</th><th></th></tr>${body}</table></div></div>
        <div>${c ? `<div class="mut">Line: area = ${c.slope.toPrecision(4)} · conc ${c.intercept >= 0 ? "+" : "−"} ${Math.abs(c.intercept).toPrecision(3)} · weights ${c.weighting} · R² (unweighted) ${c.r2.toFixed(4)} · ${c.points.length} points, ${c.lo}–${c.hi} mg/L</div><canvas id="tp-c-cal" data-cal='${esc(JSON.stringify(c))}'></canvas>` : `<div class="mut">At least three standards with the concentration in the name (e.g. STD_0_6ppm) are needed for the calibration line.</div>`}</div></div></div>`;
    }
    function rows() {
      if (!$("#tp-t")) return;
      const s = st.summary, f = st.filter;
      const keep = s.rows.filter(r => r.label === "progenitore" || (r.kind === "unexpected" ? f.unexp && (r.label === "forte" || r.label === "possibile") : f[r.label]));
      $("#tp-t").innerHTML = `<tr><th>#</th><th>Lvl</th><th>Score</th><th>Candidate</th><th>Formula</th><th><i>m/z</i></th><th>RT</th><th>t max</th><th>Max area</th><th>Kinetics</th></tr>` + keep.map(r => `<tr class="clk${r.id === st.sel ? " sel" : ""}" data-id="${r.id}">
        <td>${r.id >= 100000 ? "n" + (r.id - 100000) : r.id}</td><td>${r.level ? `<span class="lv lv${r.level}" title="${esc(r.level_text)}">${r.level}</span>` : ""}</td>
        <td>${r.score == null ? "" : `<span class="badge b-${r.label}">${r.score} ${LAB[r.label]}</span>`}</td><td>${esc(r.name)}${r.alternatives.length ? ` <span class="mut" title="${esc(r.alternatives.join("\n"))}">+${r.alternatives.length}</span>` : ""}</td>
        <td>${esc(r.formula)}</td><td class="n">${r.mz.toFixed(2)}</td><td class="n">${r.ref_rt ?? ""}</td><td class="n">${r.tmax ?? ""}</td><td class="n">${fmtA(r.max_area)}</td><td>${spark(r.kinetics)}</td></tr>`).join("");
      $("#tp-t").querySelectorAll("tr.clk").forEach(tr => tr.onclick = () => show(+tr.dataset.id));
    }
    function words(d, par) {
      const kin = d.rows.filter(r => r.type === "sample" && r.time != null).sort((a, b) => a.time - b.time), first = kin.find(r => r.detected && r.area > 0), mx = kin.reduce((m, r) => r.detected && r.area > (m ? m.area : 0) ? r : m, null);
      const bl = d.rows.filter(r => r.type === "blank" || r.type === "control"), drt = par && par.ref_rt && d.ref_rt ? d.ref_rt - par.ref_rt : null;
      const P = [];
      const row = st.summary.rows.find(r => r.id === d.id);
      P.push(`<b>${esc(d.name)}</b> ${d.delta ? `(${esc(d.delta)}) ` : ""}has <i>m/z</i> ${d.mz}${row && row.delta_mz != null ? `, i.e. ${row.delta_mz >= 0 ? "+" : ""}${row.delta_mz} relative to the parent` : ""}.`);
      if (first && mx) P.push(`Appears at ${first.time} min and peaks at ${mx.time} min${mx.time === first.time ? "" : ""}; ${bl.length ? (bl.every(b => !b.detected) ? "it is absent from the blank." : "something shows in the blank: be careful.") : "there is no blank to rule out a contaminant."}`);
      if (drt != null) P.push(`Elutes ${Math.abs(drt).toFixed(2)} min ${drt < 0 ? "before" : "after"} the parent${drt < -0.3 ? ": more polar, as expected for an oxidation" : drt > 0.3 ? ": less polar than the parent, unusual for an oxidation" : ", almost together"}.`);
      if (d.ms2 && d.ms2.scans) { const sh = d.ms2.fragments.filter(f => f.note); P.push(`${d.ms2.scans} MS2 scans (precursor ${d.ms2.precursor}): ${d.ms2.fragments.slice(0, 3).map(f => f.mz).join(", ")}${sh.length ? `; ${sh.length} fragments are explained by the parent (${sh.map(f => f.note.startsWith("shared") || f.note.startsWith("condiviso") ? "shared" : "shifted by Δ").join(", ")})` : "; no fragment matches those of the parent"}.`); }
      else P.push("No MS2 with this precursor: a Product Ion Scan on the peak is needed to raise the level.");
      P.push(d.level_text ? esc(d.level_text) : "");
      return P.filter(Boolean).map(p => `<p style="margin:4px 0">${p}</p>`).join("");
    }
    async function getDetail(id) { return st.det[id] || (st.det[id] = await call("detail", id)); }
    async function show(id) {
      st.sel = id; rows();
      $("#tp-right").querySelectorAll(".gem,.mnode").forEach(g => g.classList.toggle("sel", +g.dataset.id === id));
      const D = $("#tp-det"); D.innerHTML = `<div class="card mut">Computing...</div>`;
      let d; try { d = await getDetail(id); } catch (e) { D.innerHTML = `<div class="card err">${esc(e.message || e)}</div>`; return; }
      const par = st.summary.rows.find(r => r.label === "progenitore"), drt = par && par.ref_rt && d.ref_rt ? (d.ref_rt - par.ref_rt) : null;
      const kin = d.rows.filter(r => r.type === "sample" && r.time != null).sort((a, b) => a.time - b.time);
      D.innerHTML = `<div class="card words"><h3>In words</h3>${words(d, par)}</div><div class="card"><h3>${esc(d.name)} ${d.level ? `<span class="lv lv${d.level}">${d.level}</span>` : ""}</h3>
        <div class="kv"><b>Formula</b><span>${esc(d.formula || "-")} ${d.delta ? `<span class="mut">(${esc(d.delta)})</span>` : ""}</span>
        <b><i>m/z</i></b><span>${d.mz} calculated · ${d.mz_extracted} extracted${d.mz_observed ? ` · ${d.mz_observed.toFixed(2)} observed` : ""}</span>
        <b>RT</b><span>${d.ref_rt ? d.ref_rt.toFixed(2) + " min" : "-"}${drt != null ? ` (${drt >= 0 ? "+" : ""}${drt.toFixed(2)} relative to the parent; more polar TPs elute earlier)` : ""}</span>
        ${d.level_text ? `<b>Confidence</b><span>${esc(d.level_text)}</span>` : ""}${d.alternatives.length ? `<b>Same mass</b><span>${d.alternatives.map(esc).join("; ")}</span>` : ""}</div></div>
        <div class="cols"><div class="card"><h3>Criteria</h3><ul class="crit" style="margin:0;padding-left:18px">${d.criteria.map(c => `<li class="${c.status}"><b>${c.status === "pass" ? "✓" : c.status === "fail" ? "✗" : "–"} ${esc(c.name)}</b> <span class="mut">${esc(c.text)}</span></li>`).join("") || "<li class='mut'>parent</li>"}</ul></div>
        <div class="card"><h3>Kinetics (peak area)</h3><canvas id="tp-c-kin"></canvas></div></div>
        <div class="card"><h3>XIC of all samples <span class="mut">(window ±${st.summary.settings.tol_da} Da around ${d.mz_extracted})</span></h3><canvas id="tp-c-xic" style="height:260px"></canvas><div class="row" id="tp-leg"></div></div>
        <div class="cols"><div class="card"><h3>Areas</h3><table><tr><th>Sample</th><th>t</th><th>Area</th><th>Height</th><th>S/N</th><th>RT</th></tr>${d.rows.map(r => `<tr><td>${esc(r.label)}</td><td class="n">${r.time ?? ""}</td><td class="n">${r.detected ? fmtA(r.area) : "<span class='mut'>-</span>"}</td><td class="n">${r.detected ? fmtA(r.height) : ""}</td><td class="n">${r.snr ? r.snr.toFixed(1) : ""}</td><td class="n">${r.apex_rt != null && r.detected ? r.apex_rt.toFixed(2) : ""}</td></tr>`).join("")}</table></div>
        <div class="card"><h3>Product ions (MS2)</h3>${d.ms2.scans ? `<div class="mut">${d.ms2.scans} scans, precursor ${d.ms2.precursor}, CE ${d.ms2.collision_energy ?? "?"}</div><table><tr><th><i>m/z</i></th><th>%</th><th>loss</th><th>note</th></tr>${d.ms2.fragments.map(f => `<tr><td class="n">${f.mz}</td><td class="n">${f.rel}</td><td class="n">${f.loss ?? ""}</td><td>${esc(f.note || "")}</td></tr>`).join("")}</table>` : `<div class="mut">No MS2 scan with this precursor inside the peak.</div>`}
        ${d.transitions.length ? `<h3 style="margin-top:8px">Suggested MRM transitions</h3><table><tr><th>Q1</th><th>Q3</th><th>CE</th><th>RT</th></tr>${d.transitions.map(t => `<tr><td class="n">${t.Q1}</td><td class="n">${t.Q3_consigliato}</td><td class="n">${t.CE}</td><td class="n">${t.RT_attesa_min}</td></tr>`).join("")}</table>` : ""}</div></div>`;
      const isPar = par && id === par.id;
      if (window.TPALBERO && (isPar || (d.ms2 && d.ms2.scans))) { const a = document.createElement("div"); a.className = "card"; a.id = "tp-alb"; D.appendChild(a); window.TPALBERO.mount(a, { esc, call }, isPar ? {} : { compareId: id }); }
      D.scrollIntoView({ behavior: "smooth", block: "nearest" });
      chart($("#tp-c-kin"), [{ x: kin.map(r => r.time), y: kin.map(r => r.detected ? r.area : 0), color: "#2b5c8a", dots: true }], { xlabel: "time (min)" });
      const n = d.traces.length;
      const tr = d.traces.map((t, i) => ({ x: t.rt, y: t.y, color: t.type === "blank" ? "#888" : hue(i, n), dash: t.type === "blank" ? [4, 3] : [], w: 1.3, label: t.label }));
      const pk = d.traces.map(t => t.peak).filter(Boolean), c = d.ref_rt || 0;
      chart($("#tp-c-xic"), tr, { xlabel: "RT (min)", x0: Math.max(0.5, c - 2.5), x1: c + 2.5, shade: pk.length ? [Math.min(...pk.map(p => p.left)), Math.max(...pk.map(p => p.right))] : null });
      $("#tp-leg").innerHTML = tr.map(t => `<span class="mut"><span style="display:inline-block;width:14px;border-top:2px ${t.dash.length ? "dashed" : "solid"} ${t.color};vertical-align:middle"></span> ${esc(t.label)}</span>`).join("");
    }

    document.addEventListener("keydown", ev => {
      if (sec.hidden || !st.summary || st.view !== "tab" || !["ArrowDown", "ArrowUp"].includes(ev.key) || /INPUT|SELECT|TEXTAREA/.test(document.activeElement && document.activeElement.tagName)) return;
      const ids = [...sec.querySelectorAll("#tp-t tr.clk")].map(t => +t.dataset.id); if (!ids.length) return;
      const i = ids.indexOf(st.sel), n = ev.key === "ArrowDown" ? Math.min(ids.length - 1, i + 1) : Math.max(0, i - 1); ev.preventDefault(); show(ids[n < 0 ? 0 : n]);
    });

    // ---- export
    async function exportAll(kind) {
      const s = st.summary; msg("Preparing the export...");
      try {
        const top = s.rows.filter(r => r.label === "forte" || r.label === "possibile");
        const det = []; for (const r of top) det.push(await getDetail(r.id));
        const times = s.times, name = (st.summary.parent.formula || "parent") + "_mzFinder";
        const head = ["#", "Candidate", "Formula", "Change", "m/z", "RT (min)", "Level", "Score", "Verdict", "t max (min)", "Max area", ...times.map(t => "Area t" + t)];
        const line = r => [r.id, r.name, r.formula, r.delta, r.mz, r.ref_rt, r.level ?? "", r.score ?? "", r.label, r.tmax ?? "", Math.round(r.max_area), ...times.map(t => { const k = r.kinetics.find(p => p.time === t); return k ? Math.round(k.area) : ""; })];
        const trans = det.flatMap(d => d.transitions);
        if (kind === "json") return dl(name + ".json", JSON.stringify({ summary: s, details: det }, null, 1), "application/json");
        if (kind === "csv") return dl(name + ".csv", "sep=;\n" + [head, ...s.rows.map(line)].map(r => r.map(v => `"${String(v ?? "").replace(/"/g, '""')}"`).join(";")).join("\n"), "text/csv");
        const sheets = [{ name: "Ranking", head, rows: s.rows.map(line), widths: [6, 38, 16, 14, 10, 9, 8, 9, 11, 10, 12, ...times.map(() => 11)] },
          { name: "Criteria", head: ["#", "Candidate", "Criterion", "Outcome", "Detail"], rows: det.flatMap(d => d.criteria.map(c => [d.id, d.name, c.name, c.status, c.text])), widths: [6, 38, 28, 8, 70] },
          { name: "MS2", head: ["#", "Candidate", "Product m/z", "% rel.", "Loss", "Note"], rows: det.flatMap(d => d.ms2.fragments.map(f => [d.id, d.name, f.mz, f.rel, f.loss ?? "", f.note || ""])), widths: [6, 38, 12, 9, 9, 50] },
          { name: "MRM transitions", head: ["Candidate", "Formula", "Q1", "Q3 observed", "Q3 suggested", "CE", "Expected RT", "RT window", "% rel.", "MS2 scans", "Note"], rows: trans.map(t => [t.compound, t.formula, t.Q1, t.Q3_osservato, t.Q3_consigliato, t.CE, t.RT_attesa_min, t.finestra_RT_min, t["intensita_rel_%"], t.scansioni_ms2, t.nota]), widths: [34, 16, 8, 12, 14, 7, 10, 11, 8, 12, 40] },
          ...(s.mrm ? [mrmSheet(s.mrm)] : []),
          { name: "Information", head: ["Item", "Value"], rows: [["Parent", s.parent.formula], ["Neutral mass", s.parent.neutral], ["Adduct", s.parent.adduct], ["Parent m/z", s.parent.mz], ["m/z offset (Da)", s.offset ? s.offset.offset : ""], ["k (1/min)", s.decay ? s.decay.k_per_min : ""], ["t1/2 (min)", s.decay ? s.decay.half_life_min : ""], ...Object.entries(s.settings).map(([k, v]) => [k, String(v)]), ...s.files.map(f => ["File", `${f.name} (${f.kind}, ${f.type}, t=${f.time ?? ""})`]), ["Note", "Unit resolution: each row is a candidate, not an identification. Levels 2 and 1 require a library spectrum or a standard."]], widths: [22, 90] }];
        dlx(name + ".xlsx", sheets);
      } catch (e) { msg(String(e.message || e), true); return; }
      msg("");
    }
    function mrmSheet(m) {
      const q = m.quantifier, qual = m.transitions.slice(1).map(t => t.name);
      return { name: "MRM", head: ["File", "Type", "t (min)", "Nominal conc. (mg/L)", "Area " + q, "RT", "S/N", ...qual.flatMap(n => ["Area " + n, "Qual/Quant " + n]), "Ion ratio", "Calculated conc. (mg/L)", "Note"],
        rows: m.rows.map(r => [r.file, r.type, r.time ?? "", r.conc ?? "", Math.round(r.ion[q].area), r.ion[q].rt ?? "", Math.round(r.ion[q].snr), ...qual.flatMap(n => [Math.round(r.ion[n].area), r.ratios[n] ?? ""]), r.ratio_ok == null ? "" : r.ratio_ok ? "ok" : "out", r.quant ?? "", r.in_range === false ? "outside the calibration range" : ""]),
        widths: [34, 10, 8, 14, 12, 8, 8, ...qual.flatMap(() => [12, 12]), 12, 16, 34] };
    }
    const dl = (n, text, type) => { const a = document.createElement("a"); a.href = URL.createObjectURL(new Blob([text], { type })); a.download = n; a.click(); setTimeout(() => URL.revokeObjectURL(a.href), 2000); };
  }

  QTOOLS.register({ id: "tpmine", nav: true, name: "mzFinder", desc: "Automatic search for transformation products", open,
    icon: (typeof QICON !== "undefined" && QICON.mine) ? QICON.get("mine", 16) : "⛏" });
})();
