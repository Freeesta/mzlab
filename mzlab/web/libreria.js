"use strict";
// Part D: compare a MS2 spectrum with the spectral libraries (MSP, MGF) the user has loaded. Classic script, loaded after settings.js/explore.js.
// The reading, the archive (OPFS) and the search live in libreria-worker.js (no Pyodide); here: the «Librerie» window (gear), the window with the
// results and the mirror plot (experimental on top, library below). Nothing here touches drawSpec: the mirror is a canvas of its own.
const LIB = (() => {
  // Supervisor: if the worker dies (a trap of the WebAssembly engine, a lost memory) it is replaced and the request is tried once more; a second death within a
  // minute sends the page to the JavaScript path for the rest of the session. `?libjs=1` starts there (tests, browsers where the engine does not load).
  let worker = null, seq = 0, restarts = []; const pend = new Map();
  let forceJs = false; try { forceJs = new URLSearchParams(location.search).get("libjs") === "1"; } catch (e) { /* no location */ }
  const say = key => { try { if (typeof toast === "function") toast(I18N.t(key)); } catch (e) { /* no page */ } };
  function w() {
    if (worker) return worker;
    const me = worker = new Worker("static/libreria-worker.js");
    me.onmessage = e => {
      const m = e.data, r = pend.get(m.reqId); if (!r) return;
      if (m.type === "progress") { r.prog && r.prog(m); return; }
      pend.delete(m.reqId); if (m.type === "error") { const er = I18N.err(m); er.fatal = !!m.fatal; r.no(er); } else r.ok(m);
    };
    me.onerror = e => { const er = new Error(I18N.t("lib.err.worker", { message: e.message || "error" })); er.fatal = true; pend.forEach(r => r.no(er)); pend.clear(); if (worker === me) worker = null; };
    me.postMessage({ cmd: "config", reqId: 0, forceJs, wasm: !!window.MZLAB_BROWSER });      // the engine's .wasm exists only in the built site (window.MZLAB_BROWSER, browser.js)
    return me;
  }
  function restart() {
    if (worker) { try { worker.terminate(); } catch (e) { /* gone */ } worker = null; }
    const er = new Error("restarted"); pend.forEach(r => r.no(er)); pend.clear();
    const now = Date.now(); restarts = restarts.filter(t => now - t < 60000); restarts.push(now);
    if (restarts.length > 1) { forceJs = true; say("lib.err.fallback"); } else say("lib.err.restarted");
  }
  const call1 = (cmd, payload = {}, prog) => new Promise((ok, no) => { const reqId = ++seq; pend.set(reqId, { ok, no, prog }); try { w().postMessage({ cmd, reqId, ...payload }); } catch (e) { pend.delete(reqId); no(e); } });
  const call = async (cmd, payload = {}, prog) => {
    try { return await call1(cmd, payload, prog); }
    catch (e) { if (!(e && e.fatal)) throw e; restart(); return call1(cmd, payload, prog); }
  };
  // for the tests: which engine answers, and a way to kill it
  const engine = { info: () => call1("engine"), async crash() { try { await call1("crash"); } catch (e) { if (e && e.fatal) restart(); else throw e; } }, get fallback() { return forceJs; } };
  const EHt = t => (typeof EH === "function" ? EH(t) : String(t));
  const loc = () => (I18N.lang === "it" ? "it-IT" : "en-US"), fmtN = n => Number(n).toLocaleString(loc());
  const dlgCss = `<style>#libdlg,#libres{max-width:min(1100px,96vw);width:min(1100px,96vw)}#libdlg{width:min(720px,96vw)}#libdlg .drop{border:2px dashed var(--line);border-radius:8px;padding:14px;text-align:center;margin:8px 0}#libdlg .drop.over{border-color:var(--accent);background:var(--sel)}
    #libdlg table,#libres table{border-collapse:collapse;width:100%}#libdlg td,#libdlg th,#libres td,#libres th{padding:3px 8px;border-bottom:1px solid var(--line);text-align:left;font-size:12px}#libres td.num,#libres th.num,#libdlg td.num,#libdlg th.num{text-align:right;font-variant-numeric:tabular-nums}
    #libres tr.row{cursor:pointer}#libres tr.row.sel{background:var(--sel)}#libres tr.row:hover{background:var(--sel)}#libres .qbar{display:flex;gap:12px;flex-wrap:wrap;align-items:center;margin:6px 0}#libres .qbar input{width:90px}#libres canvas{width:100%;height:300px;display:block;border:1px solid var(--line);border-radius:6px;margin-top:8px}
    #libres .tbl{max-height:260px;overflow:auto;border:1px solid var(--line);border-radius:6px}#libdlg .err{color:var(--bad)}#libdlg progress{width:100%}</style>`;
  const ensureCss = () => { if (!document.getElementById("libcss")) { const s = document.createElement("div"); s.id = "libcss"; s.innerHTML = dlgCss; document.head.appendChild(s.firstChild); } };

  // ------------------------------------------------------------ the «Librerie» window
  async function open() {
    ensureCss();
    let d = document.getElementById("libdlg");
    if (!d) {
      d = document.createElement("dialog"); d.id = "libdlg"; document.body.appendChild(d);
      d.innerHTML = `<div class="top" style="display:flex;align-items:center"><h3 style="margin:0;flex:1">${I18N.t("lib.title")}</h3><button class="x" id="lib-x" title="${I18N.t("common.close")}">&times;</button></div>
        <p class="sm muted" style="margin:6px 0">${I18N.t("lib.intro")}</p>
        <div class="drop" id="lib-drop">${I18N.t("lib.drop")} <button id="lib-pick">${I18N.t("lib.pick")}</button><input type="file" id="lib-file" accept=".msp,.mgf,.lib,.mzvault,.txt" multiple hidden>
          ${window.showDirectoryPicker ? `<div class="sm" style="margin-top:8px"><button id="lib-dir">${I18N.t("lib.dir")}</button> <button id="lib-dir-new">${I18N.t("lib.dirNew")}</button></div>` : ""}</div>
        <div id="lib-st"></div><div id="lib-list"></div><div class="sm muted" id="lib-space" style="margin-top:6px"></div>`;
      d.querySelector("#lib-x").onclick = () => d.close();
      const input = d.querySelector("#lib-file"), drop = d.querySelector("#lib-drop");
      d.querySelector("#lib-pick").onclick = () => input.click();
      const bd = d.querySelector("#lib-dir"); if (bd) { bd.onclick = () => reloadFolder(false); d.querySelector("#lib-dir-new").onclick = () => reloadFolder(true); }
      input.onchange = () => { addFiles([...input.files]); input.value = ""; };
      drop.ondragover = e => { e.preventDefault(); drop.classList.add("over"); }; drop.ondragleave = () => drop.classList.remove("over");
      drop.ondrop = e => { e.preventDefault(); drop.classList.remove("over"); addFiles([...e.dataTransfer.files]); };
    }
    if (!d.open) d.showModal();
    refresh();
  }
  async function refresh() {
    const d = document.getElementById("libdlg"); if (!d) return;
    const box = d.querySelector("#lib-list");
    try {
      const r = await call("list"), L = r.libs;
      box.innerHTML = L.length ? `<table><tr><th>${I18N.t("lib.col.library")}</th><th class="num">${I18N.t("lib.col.spectra")}</th><th class="num">${I18N.t("lib.col.size")}</th><th>${I18N.t("lib.col.polarity")}</th><th class="num">${I18N.t("lib.col.dropped")}</th><th>${I18N.t("lib.col.loaded")}</th><th></th></tr>${L.map(l => `<tr><td title="${EHt(l.file)}">${EHt(l.name)}</td><td class="num">${fmtN(l.n)}</td><td class="num">${l.size ? fmtN(Math.round(l.size / 1e6)) + " MB" : ""}</td><td>ESI+ ${fmtN(l.pos)} · ESI&minus; ${fmtN(l.neg)}${l.n - l.pos - l.neg ? " · ? " + fmtN(l.n - l.pos - l.neg) : ""}</td>
        <td class="num" title="${I18N.t("lib.droppedTitle", { prec: l.noPrec, peaks: l.noPeaks, broken: l.broken })}">${fmtN(l.dropped)}</td><td>${new Date(l.date).toLocaleDateString(loc())}</td><td><button data-rm="${EHt(l.id)}">${I18N.t("lib.remove")}</button></td></tr>`).join("")}</table>` : `<p class="muted">${I18N.t("lib.none")}</p>`;
      box.querySelectorAll("[data-rm]").forEach(b => b.onclick = async () => { b.disabled = true; await call("remove", { id: b.dataset.rm }); refresh(); });
      if (navigator.storage && navigator.storage.estimate) { const s = await navigator.storage.estimate(); d.querySelector("#lib-space").textContent = I18N.t("lib.space", { mb: Math.round((s.usage || 0) / 1e6) }) + (r.opfs ? "" : " " + I18N.t("lib.noOpfs")); }
    } catch (e) { box.innerHTML = `<p class="err">${EHt(e.message)}</p>`; }
  }
  // Optional shortcut (Chrome / Edge, File System Access API): the folder is remembered and its MSP / MGF files that are not archived yet are imported again. Never the only way.
  const dirDb = () => new Promise((ok, no) => { const r = indexedDB.open("mzlab-libdir", 1); r.onupgradeneeded = () => r.result.createObjectStore("h"); r.onsuccess = () => ok(r.result); r.onerror = () => no(r.error); });
  const dirGet = async () => { const db = await dirDb(); return new Promise(ok => { const r = db.transaction("h").objectStore("h").get("dir"); r.onsuccess = () => { db.close(); ok(r.result || null); }; r.onerror = () => { db.close(); ok(null); }; }); };
  const dirPut = async h => { const db = await dirDb(); return new Promise(ok => { const t = db.transaction("h", "readwrite"); t.objectStore("h").put(h, "dir"); t.oncomplete = () => { db.close(); ok(); }; t.onerror = () => { db.close(); ok(); }; }); };
  async function reloadFolder(choose) {
    const st = document.querySelector("#libdlg #lib-st");
    try {
      let h = choose ? null : await dirGet();
      if (h && (await h.requestPermission({ mode: "read" })) !== "granted") h = null;
      if (!h) { h = await window.showDirectoryPicker({ id: "mzlab-libs", mode: "read" }); await dirPut(h); }
      const have = new Set((await call("list")).libs.map(l => l.file + "|" + l.size)), todo = [];
      for await (const [name, fh] of h.entries()) if (fh.kind === "file" && /\.(msp|mgf)$/i.test(name)) { const f = await fh.getFile(); if (!have.has(f.name + "|" + f.size)) todo.push(f); }
      if (!todo.length) { st.innerHTML = `<div class="sm">${I18N.t("lib.dirNone", { name: EHt(h.name) })}</div>`; return; }
      await addFiles(todo);
    } catch (e) { if (e && e.name !== "AbortError") st.innerHTML = `<div class="err">${EHt(e.message)}</div>`; }
  }
  async function addFiles(files) {
    const d = document.getElementById("libdlg"), st = d.querySelector("#lib-st");
    for (const f of files) {
      st.innerHTML = `<div class="sm">${EHt(f.name)}: ${I18N.t("lib.reading")}</div><progress max="1" value="0"></progress>`;
      try {
        const r = await call("add", { file: f }, m => { st.innerHTML = `<div class="sm">${EHt(f.name)}: ${I18N.t("lib.readN", { n: fmtN(m.n), pct: Math.round(m.read / m.total * 100) })}</div><progress max="${m.total}" value="${m.read}"></progress>`; });
        st.innerHTML = `<div class="sm">${EHt(f.name)}: ${I18N.t("lib.doneN", { n: fmtN(r.info.n) })}${r.info.dropped ? ", " + I18N.t("lib.droppedN", { n: fmtN(r.info.dropped) }) : ""}.</div>`;
      } catch (e) { st.innerHTML = `<div class="err">${EHt(f.name)}: ${EHt(e.message)}</div>`; }
      await refresh();
    }
  }

  // ------------------------------------------------------------ the query: the MS2 spectrum shown in a panel
  function profOf(f) {                              // tolerances: the profile of hr.js when it exists, otherwise 10 ppm for a high resolution instrument and 0.5 Da
    try { if (window.HR && HR.prof) { const q = HR.prof(f, 2); if (q) return { hr: !!q.hr, v: q.hr ? q.tol : 0.5, unit: q.hr ? "ppm" : "Da" }; } } catch (e) { /* the fallback below */ }
    const hr = /orbitrap|exploris|fusion|q.?exactive|tof|ft-?icr|astral/i.test(String((f && f.instrument) || ""));
    return hr ? { hr: true, v: 10, unit: "ppm" } : { hr: false, v: 0.5, unit: "Da" };
  }
  function queryOf(p) {
    const a = p._a, d0 = a && a.data && a.data[0]; if (!d0 || !d0.d) return null;
    const d = d0.d, y = d.y0 || d.y, f = d0.f, peaks = d.mz.map((m, i) => [m, y[i]]).filter(q => q[1] > 0);
    const prec = [d.prec, p._prec, p.prec].map(Number).find(v => Number.isFinite(v) && v > 0);
    const pol = f && f.polarity === "positive" ? 1 : f && f.polarity === "negative" ? -1 : 0;
    const rt = p.r0 != null ? (p.r0 + p.r1) / 2 : null;
    return { peaks, prec, pol, f, rt, title: `${(f && f.label) || ""}${rt != null ? " · RT " + rt.toFixed(2) + " min" : ""}` };
  }

  // ------------------------------------------------------------ the results window
  function mirror(cv, q, r) {
    const s = setup(cv), g = s.g, W = s.W, H = s.H, ml = 56, mr = 14, mt = 14, mb = 34, mid = mt + (H - mt - mb) / 2, half = (H - mt - mb) / 2 - 6;
    const all = [...q.mz, ...r.mz], lo = Math.floor(Math.min(...all) / 10) * 10 - 5, hi = Math.ceil(Math.max(...all, r.prec) / 10) * 10 + 5, X = m => ml + (m - lo) / (hi - lo) * (W - ml - mr);
    const acc = css("--accent"), ink = css("--ink"), mut = css("--muted"), line = css("--line"), shc = css("--ok");
    g.clearRect(0, 0, W, H); g.fillStyle = css("--panel"); g.fillRect(0, 0, W, H);
    const common = { q: new Set(r.pairs.map(p => p[0])), l: new Set(r.pairs.map(p => p[1])) };
    g.lineWidth = 1.5; g.font = fpx(12);
    const shq = new Set((r.pairs || []).filter(p => p[2]).map(p => p[0])), shl = new Set((r.pairs || []).filter(p => p[2]).map(p => p[1]));      // analogues: peaks matched after the shift
    const bars = (mz, it, set, dir) => mz.forEach((m, i) => {
      const hh = it[i] * half, sh = (dir > 0 ? shq : shl).has(i); g.strokeStyle = sh ? shc : set.has(i) ? acc : mut; g.lineWidth = set.has(i) ? 2 : 1.3;
      g.beginPath(); g.moveTo(X(m), mid); g.lineTo(X(m), mid - dir * hh); g.stroke();
      if (it[i] > 0.08) { g.fillStyle = sh ? shc : set.has(i) ? acc : mut; g.textAlign = "center"; g.fillText(m.toFixed(r.prec > 0 && q.hr ? 4 : 2), X(m), dir > 0 ? mid - hh - 4 : mid + hh + 13); }
    });
    bars(q.mz, q.it, common.q, 1); bars(r.mz, r.it, common.l, -1);
    g.strokeStyle = line; g.lineWidth = 1; g.beginPath(); g.moveTo(ml, mid); g.lineTo(W - mr, mid); g.stroke();
    g.fillStyle = mut; g.textAlign = "center"; g.beginPath(); g.strokeStyle = mut; g.moveTo(ml, H - mb + 6); g.lineTo(W - mr, H - mb + 6); g.stroke();
    const step = [1, 2, 5, 10, 20, 25, 50, 100, 200].find(v => (hi - lo) / v <= 9) || 500;
    for (let v = Math.ceil(lo / step) * step; v <= hi; v += step) { g.beginPath(); g.moveTo(X(v), H - mb + 6); g.lineTo(X(v), H - mb + 10); g.stroke(); g.fillText(String(v), X(v), H - mb + 22); }
    g.fillStyle = ink; g.textAlign = "left"; g.fillText(I18N.t("lib.plot.exp"), ml + 4, mt + 10); g.fillText(r.name || I18N.t("lib.plot.lib"), ml + 4, H - mb - 6);
    g.save(); g.fillStyle = mut; g.textAlign = "center"; g.translate(14, mid); g.rotate(-Math.PI / 2); g.fillText(I18N.t("lib.plot.rel"), 0, 0); g.restore();
    g.fillStyle = acc; g.textAlign = "right"; g.fillText(I18N.t("lib.plot.shared", { n: r.shared }), W - mr - 4, mt + 10);
  }
  // confidence levels: livelli.js (rules and thresholds in the data file livelli-1.js, the user's thresholds in qqq.lib.soglie). The candidates carry ik, ent, cos, shared, dprec (ppm), nlib
  const assess = (cands, o = {}) => LIVELLI.assess({ cands, analogs: o.analogs, nScans: o.nScans, std: o.std || null, formulaUnique: !!o.formulaUnique });
  const THR_KEYS = ["entropia_min", "picchi_min", "precursore_ppm_max", "scansioni_min", "margine_min", "entropia_provvisorio", "analogo_mcos_min"];
  function levelHtml(res) {                                    // «candidate, level N, evidence»: the badge, the evidence (one line per gate), the decisive sentence, the proposed and the decided level
    const lv = LIVELLI.shown(res), b = res.best || res.nearest || res.analog;
    return `<div class="vd" style="margin:6px 0;padding:6px 10px;border-radius:6px;border:1px solid var(--line)"><span class="lvb ${LIVELLI.cls(lv)}">${EHt(LIVELLI.sentence(lv))}</span>${b && res.proposed !== "5" && res.proposed !== "0" ? ` <b>${EHt(b.name || "")}</b>` : ""}
      <div class="sm" style="margin-top:4px"><b>${I18N.t("lv.evidence")}</b>: ${res.lines.length ? res.lines.map(x => EHt(x)).join(" · ") : "–"}</div><div class="sm">${EHt(res.decisive)}</div>
      <div class="sm" style="margin-top:4px">${I18N.t("lv.proposed")}: <b>${EHt(res.proposed)}</b> · <label>${I18N.t("lv.decided")} <select id="lv-dec"><option value="">${I18N.t("lv.decided.none")}</option>${LIVELLI.ORDER.map(x => `<option value="${x}"${res.decided === x ? " selected" : ""}>${x}</option>`).join("")}</select></label> <span class="sm err" id="lv-msg" role="status"></span></div></div>`;
  }
  function levelBind(root, res, again) {                       // the chemist's decision: 2b is only written here
    const sel = root.querySelector("#lv-dec"); if (!sel) return;
    sel.onchange = () => { const r = LIVELLI.decide(res, sel.value); if (!r.ok) { sel.value = res.decided || ""; root.querySelector("#lv-msg").textContent = r.msg; } else if (again) again(); };
  }
  function thresholdsHtml() {
    const T = LIVELLI.thresholds();
    return `<details id="lr-sog" style="margin-left:auto"><summary class="sm">${I18N.t("lv.thr.title")}</summary><div class="sm" style="padding:4px 0;display:flex;flex-wrap:wrap;gap:6px 14px">${THR_KEYS.map(k => `<label>${I18N.t(`lv.thr.${k}`)} <input data-thr="${k}" style="width:56px" value="${String(T[k]).replace(".", ",")}"></label>`).join("")}<button type="button" id="lr-sog-reset">${I18N.t("lv.thr.reset")}</button></div></details>`;
  }
  function thresholdsBind(root, again) {
    root.querySelectorAll("[data-thr]").forEach(el => el.onchange = () => { const v = parseFloat(el.value.replace(",", ".")); if (v >= 0) { LIVELLI.setThreshold(el.dataset.thr, v); again(); } });
    const rs = root.querySelector("#lr-sog-reset"); if (rs) rs.onclick = () => { LIVELLI.reset(); const T = LIVELLI.thresholds(); root.querySelectorAll("[data-thr]").forEach(el => { el.value = String(T[el.dataset.thr]).replace(".", ","); }); again(); };
  }
  async function scansOf(p) {                                  // MS2 scans of the same precursor (10 ppm) and the same peak (RT gaps up to 0.2 min), as «Identify all» groups them; 1 where the file is not DDA
    try {
      if (p.avg && p.avg.length > 1) return p.avg.length;
      if (p.sid == null || p.k == null) return 1;
      const d = await J(`api/dda?k=${p.k}` + (window.HR && HR.q ? HR.q() : "")), i0 = d.sid.indexOf(p.sid); if (i0 < 0 || !d.tgt[i0]) return 1;
      const t0 = d.tgt[i0], rts = d.rt.filter((_, i) => d.tgt[i] && Math.abs(d.tgt[i] - t0) <= t0 * 10e-6).sort((a, b) => a - b), j = rts.indexOf(d.rt[i0]); let n = 1;
      for (let a = j; a > 0 && rts[a] - rts[a - 1] <= 0.2; a--) n++;
      for (let a = j; a < rts.length - 1 && rts[a + 1] - rts[a] <= 0.2; a++) n++;
      return n;
    } catch (e) { return 1; }
  }
  // neutral change between a query and an analogue: the usual transformations first, then the simplest formula within 3 mDa (the composition of the server)
  const TRANSF = [[15.9949146, I18N.t("lib.tr.hydroxylation")], [-2.01565, I18N.t("lib.tr.dehydrogenation")], [13.97926, "+O −2H"], [-26.01565, "−C₂H₂"], [1.97926, "−CH₂ +O"], [-56.0626, I18N.t("lib.tr.dealkylation")],
    [18.010565, "+H₂O"], [-18.010565, "−H₂O"], [-27.9949146, "−CO"], [2.01565, "+2H"], [-14.01565, I18N.t("lib.tr.demethylation")], [14.01565, I18N.t("lib.tr.methylation")], [-43.98983, "−CO₂"], [31.98983, "+O₂"]];
  const sgnTxt = v => (v >= 0 ? "+" : "−") + Math.abs(v).toFixed(4);
  function deltaKnown(dm) { const t = TRANSF.find(([m]) => Math.abs(m - dm) <= 0.003); return t ? t[1] : null; }
  async function deltaFormula(dm) {
    try {
      const j = await J(`api/composition?mz=${Math.abs(dm) - 0.000548579909}&ion=${encodeURIComponent("M+.")}&tol=3&unit=mda&max=30&elements=${encodeURIComponent("C:0-12,H:0-24,N:0-4,O:0-6,S:0-2")}`);
      const atoms = f => [...f.matchAll(/([A-Z][a-z]?)(\d*)/g)].reduce((n, m) => n + (+m[2] || 1), 0), best = j.results.map(r => [atoms(r.formula), r.formula]).sort((a, b) => a[0] - b[0])[0];
      return best ? (dm < 0 ? "−" : "+") + best[1] + " " + I18N.t("lib.simplest") : null;
    } catch (e) { return null; }
  }
  async function searchFrom(p) {
    ensureCss();
    const q = queryOf(p);
    if (!q || !q.peaks.length) return info(I18N.t("lib.pickMs2"));
    let d = document.getElementById("libres");
    if (!d) { d = document.createElement("dialog"); d.id = "libres"; document.body.appendChild(d); }
    const prof = profOf(q.f), st = { prec: q.prec != null ? +q.prec.toFixed(4) : null, v: prof.v, unit: prof.unit, frag: prof.hr ? 0.01 : 0.5, hr: prof.hr, res: null, sel: 0, tab: "id", an: null, n: 1, decided: "" };
    d.innerHTML = `<div class="top" style="display:flex;align-items:center"><h3 style="margin:0;flex:1">${I18N.t("lib.search.title")}</h3><button class="x" id="lr-x" title="${I18N.t("common.close")}">&times;</button></div>
      <div class="sm muted" id="lr-q" style="margin-top:4px">${EHt(q.title)}</div>
      <div class="qbar"><label>${I18N.t("lib.search.prec")} <input id="lr-prec" value="${st.prec ?? ""}"></label><label>${I18N.t("lib.search.tol")} <input id="lr-tol" value="${st.v}"> <select id="lr-unit"><option value="ppm">ppm</option><option value="Da">Da</option></select></label>
        <label>${I18N.t("lib.search.peaksWithin")} <input id="lr-frag" value="${st.frag}"> Da</label><button id="lr-go">${I18N.t("lib.search.go")}</button><button id="lr-copy" title="${I18N.t("lib.copy.title")}">${I18N.t("lib.copy")}</button><button id="lr-xlsx">Excel</button><button id="lr-libs">${I18N.t("lib.libs")}</button><span class="sm muted" id="lr-msg"></span></div>
      ${st.hr ? `<div class="qbar" id="lr-tabs"><button data-t="id" class="on">${I18N.t("lib.tab.id")}</button><button data-t="an">${I18N.t("lib.tab.an")}</button>${thresholdsHtml()}</div>
      <div id="lr-verdict"></div>` : ""}
      <div id="lr-body"></div><canvas id="lr-cv" hidden></canvas>`;
    d.querySelector("#lr-unit").value = st.unit;
    d.querySelector("#lr-x").onclick = () => d.close(); d.querySelector("#lr-libs").onclick = () => { open(); };
    const msg = t => { d.querySelector("#lr-msg").textContent = t || ""; };
    const colHead = ["#", I18N.t("lib.col.name"), I18N.t("lib.col.entropy"), I18N.t("lib.col.cosine"), I18N.t("lib.col.shared"), I18N.t("lib.col.dprec"), "CE", I18N.t("lib.col.instrument"), I18N.t("lib.col.adduct"), I18N.t("lib.col.library")];
    const rowsOf = rs => rs.map((r, i) => [i + 1, r.name, r.ent, r.cos, r.shared, r.dprec, r.ce, r.inst, r.type, r.lib]);
    const vdEl = () => d.querySelector("#lr-verdict");
    const show = () => {
      const R = st.res, body = d.querySelector("#lr-body"), cv = d.querySelector("#lr-cv");
      if (vdEl()) vdEl().innerHTML = "";
      if (st.tab === "an") return showAn();
      if (!R) { body.innerHTML = ""; cv.hidden = true; return; }
      if (!R.libs) { body.innerHTML = `<p>${I18N.t("lib.loadFirst")}</p><p><button id="lr-open">${I18N.t("lib.openLibs")}</button></p>`; body.querySelector("#lr-open").onclick = () => open(); cv.hidden = true; return; }
      if (!R.results.length) { body.innerHTML = `<p>${I18N.t("lib.noPrec", { tol: st.v + " " + st.unit })}</p>`; cv.hidden = true; return; }
      const dp = r => st.unit === "ppm" ? r.dprec.toFixed(1) + " ppm" : (r.dprec >= 0 ? "+" : "") + r.dprec.toFixed(4) + " Da";
      body.innerHTML = `<div class="tbl"><table><tr><th class="num">#</th><th>${I18N.t("lib.col.name")}</th><th class="num">${I18N.t("lib.col.entropy")}</th><th class="num">${I18N.t("lib.col.cosine")}</th><th class="num">${I18N.t("lib.col.shared")}</th><th class="num">${I18N.t("lib.col.dprecHtml")}</th><th>CE</th><th>${I18N.t("lib.col.instrument")}</th><th>${I18N.t("lib.col.adduct")}</th><th>${I18N.t("lib.col.library")}</th></tr>${R.results.map((r, i) => `<tr class="row${i === st.sel ? " sel" : ""}" data-i="${i}"><td class="num">${i + 1}</td><td>${EHt(r.name)}</td><td class="num">${r.ent.toFixed(3)}</td><td class="num">${r.cos.toFixed(3)}</td><td class="num">${r.shared}</td><td class="num">${dp(r)}</td><td>${EHt(r.ce)}</td><td>${EHt(r.inst)}</td><td>${EHt(r.type)}</td><td>${EHt(r.lib)}</td></tr>`).join("")}</table></div>`;
      body.querySelectorAll("tr.row").forEach(tr => tr.onclick = () => { st.sel = +tr.dataset.i; body.querySelectorAll("tr.row").forEach(x => x.classList.toggle("sel", x === tr)); draw(); });
      if (st.hr && vdEl()) {                                   // levels only for high-resolution data (the low-resolution world gives candidates, never answers)
        st.lv = assess(R.results, { nScans: st.n, analogs: st.an && st.an.results }); if (st.decided) LIVELLI.decide(st.lv, st.decided);
        vdEl().innerHTML = levelHtml(st.lv); levelBind(vdEl(), st.lv, () => { st.decided = st.lv.decided || ""; show(); });
      }
      cv.hidden = false; draw();
    };
    // analogues (WP-H9): the library spectra with another precursor (up to 200 Da apart) that match the query after a shift of the precursor difference
    const showAn = () => {
      const A = st.an, body = d.querySelector("#lr-body"), cv = d.querySelector("#lr-cv");
      if (!A) { body.innerHTML = `<p class="muted">${I18N.t("lib.an.searching")}</p>`; cv.hidden = true; return; }
      if (!A.libs) { body.innerHTML = `<p>${I18N.t("lib.loadFirst")}</p>`; cv.hidden = true; return; }
      if (!A.results.length) { body.innerHTML = `<p>${I18N.t("lib.an.none")}</p>`; cv.hidden = true; return; }
      body.innerHTML = `<div class="tbl"><table><tr><th class="num">#</th><th>${I18N.t("lib.an.name")}</th><th class="num">&Delta;m</th><th>${I18N.t("lib.an.interp")}</th><th class="num">${I18N.t("lib.an.mcos")}</th><th class="num">${I18N.t("lib.an.matched")}</th><th>${I18N.t("lib.col.library")}</th></tr>
        ${A.results.map((r, i) => `<tr class="row${i === st.sel ? " sel" : ""}" data-i="${i}"><td class="num">${i + 1}</td><td>${EHt(r.name)}</td><td class="num">${sgnTxt(r.delta)}</td><td data-dm="${i}">${EHt(deltaKnown(r.delta) || "…")}</td><td class="num">${r.mcos.toFixed(2)}</td><td class="num">${I18N.t("lib.an.shifted", { matched: r.matched, shifted: r.shifted })}</td><td>${EHt(r.lib)}</td></tr>`).join("")}</table></div>
        <div class="sm muted" style="margin-top:4px">${I18N.t("lib.an.note")}</div>`;
      body.querySelectorAll("tr.row").forEach(tr => tr.onclick = () => { st.sel = +tr.dataset.i; body.querySelectorAll("tr.row").forEach(x => x.classList.toggle("sel", x === tr)); draw(); });
      A.results.forEach(async (r, i) => { if (!deltaKnown(r.delta)) { const t = await deltaFormula(r.delta); const c = body.querySelector(`[data-dm="${i}"]`); if (c) c.textContent = t || I18N.t("lib.an.noFormula"); } });
      cv.hidden = false; draw();
    };
    const draw = () => { if (st.tab === "an") { const A = st.an; if (!A || !A.results.length) return; const r = A.results[st.sel] || A.results[0]; mirror(d.querySelector("#lr-cv"), { mz: r.qa.mz, it: r.qa.it, hr: true }, { mz: r.la.mz, it: r.la.it, prec: r.prec, name: r.name, shared: r.matched, pairs: r.pairs }); return; }
      const R = st.res; if (!R || !R.results.length) return; const cv = d.querySelector("#lr-cv"), r = R.results[st.sel]; mirror(cv, { mz: R.query.mz, it: R.query.it, hr: st.hr }, r); };
    const run = async () => {
      const num = id => parseFloat(String(d.querySelector(id).value).replace(",", "."));
      st.prec = num("#lr-prec"); st.v = num("#lr-tol"); st.unit = d.querySelector("#lr-unit").value; st.frag = num("#lr-frag");
      if (!(st.prec > 0) || !(st.v > 0) || !(st.frag > 0)) return msg(I18N.t("lib.checkTol"));
      msg(I18N.t("lib.searching")); const t0 = performance.now();
      try {
        st.res = await call("search", { peaks: q.peaks, prec: st.prec, pol: q.pol, tol: { v: st.v, unit: st.unit }, frag: st.frag }); st.sel = 0;
        msg(st.res.libs ? I18N.t("lib.results", { n: st.res.results.length, ms: Math.round(performance.now() - t0) }) : "");
      } catch (e) { st.res = null; msg(I18N.t("lib.error", { message: e.message })); }
      show();
    };
    const runAn = async () => {
      st.an = null; show();
      try { st.an = await call("analogs", { peaks: q.peaks, prec: st.prec, pol: q.pol, frag: st.frag, maxDelta: 200, max: 10 }); st.sel = 0; msg(st.an.libs ? I18N.t("lib.analogsN", { n: st.an.results.length }) : ""); } catch (e) { st.an = null; msg(I18N.t("lib.error", { message: e.message })); }
      show();
    };
    d.querySelectorAll("#lr-tabs [data-t]").forEach(b => b.onclick = () => { st.tab = b.dataset.t; st.sel = 0; d.querySelectorAll("#lr-tabs [data-t]").forEach(x => x.classList.toggle("on", x === b)); if (st.tab === "an") runAn(); else show(); });
    if (d.querySelector("#lr-sog")) thresholdsBind(d, show);
    d.querySelector("#lr-go").onclick = () => { if (st.tab === "an") { st.prec = parseFloat(String(d.querySelector("#lr-prec").value).replace(",", ".")); st.frag = parseFloat(String(d.querySelector("#lr-frag").value).replace(",", ".")); runAn(); } else run(); };
    d.querySelectorAll(".qbar input").forEach(i => i.onkeydown = e => { if (e.key === "Enter") run(); });
    const table = () => { const h = colHead, rs = st.res ? rowsOf(st.res.results) : []; return { h, rs }; };
    d.querySelector("#lr-copy").onclick = async () => {
      const { h, rs } = table(); if (!rs.length) return msg(I18N.t("lib.nothingCopy"));
      const it = /^it/i.test(navigator.language || ""), f = (v, k) => typeof v === "number" ? String(k === 5 ? +v.toFixed(st.unit === "ppm" ? 1 : 4) : +v.toFixed(3)).replace(".", it ? "," : ".") : String(v ?? "");
      const tsv = h.join("\t") + "\n" + rs.map(r => r.map((v, k) => f(v, k)).join("\t")).join("\n");
      const html = `<table><tr>${h.map(x => `<th>${EHt(x)}</th>`).join("")}</tr>${rs.map(r => `<tr>${r.map((v, k) => `<td>${EHt(f(v, k))}</td>`).join("")}</tr>`).join("")}</table>`;
      try { if (window.ClipboardItem && navigator.clipboard.write) await navigator.clipboard.write([new ClipboardItem({ "text/plain": new Blob([tsv], { type: "text/plain" }), "text/html": new Blob([html], { type: "text/html" }) })]); else await navigator.clipboard.writeText(tsv); msg(I18N.t("lib.copied", { n: rs.length })); }
      catch (e) { try { await navigator.clipboard.writeText(tsv); msg(I18N.t("lib.copied", { n: rs.length })); } catch (e2) { msg(I18N.t("lib.noCopy")); } }
    };
    d.querySelector("#lr-xlsx").onclick = () => { const { h, rs } = table(); if (!rs.length) return msg(I18N.t("lib.nothingSave")); dlx((typeof plotName === "function" ? plotName(p) : I18N.t("lib.xlsx.fallback")) + I18N.t("lib.xlsx.suffix"), [{ name: I18N.t("lib.xlsx.sheet"), head: h, rows: rs, widths: [5, 30, 10, 10, 16, 14, 8, 14, 12, 20] }]); };
    d.showModal(); window.addEventListener("resize", draw, { once: true });
    if (st.hr) scansOf(p).then(n => { if (n !== st.n) { st.n = n; if (st.res) show(); } });
    if (st.prec) run(); else msg(I18N.t("lib.typePrec"));
  }
  // ------------------------------------------------------------ «Identifica tutte le MS2 del file» (WP-H8): every MS2 of a high-resolution file against the libraries
  // the MS2 of each scan are read in blocks of 60 (api/scanbin); the worker answers with the best library entry of each; the scans of one precursor (10 ppm) and one peak (RT within 0.2 min) are grouped
  const phys = f => (f && f.file ? f.file.split("#")[0] : "");
  function groupRows(rows, ppm = 10, dRt = 0.2) {
    const R = rows.filter(r => r.prec).sort((a, b) => a.prec - b.prec), byPrec = [];
    for (const r of R) { const g = byPrec[byPrec.length - 1]; if (g && Math.abs(r.prec - g[g.length - 1].prec) <= r.prec * ppm * 1e-6) g.push(r); else byPrec.push([r]); }
    const out = [];
    for (const g of byPrec) {
      g.sort((a, b) => a.rt - b.rt); let cur = [g[0]];
      const flush = () => {
        const named = cur.filter(r => r.name && r.ent != null), best = (named.length ? named : cur).reduce((x, y) => ((y.ent ?? -1) > (x.ent ?? -1) ? y : x));
        out.push({ rt: best.rt, prec: cur.reduce((x, r) => x + r.prec, 0) / cur.length, n: cur.length, best, rows: cur });
      };
      for (let i = 1; i < g.length; i++) { if (g[i].rt - g[i - 1].rt <= dRt) cur.push(g[i]); else { flush(); cur = [g[i]]; } }
      flush();
    }
    return out.sort((a, b) => a.rt - b.rt);
  }
  function groupAssess(g) {                          // the level of a group of scans of one precursor: its best candidate, the best one of ANOTHER structure (`alt`, from the worker) and the number of scans
    const b = g.best && g.best.name ? g.best : null;
    return assess(b ? [b, b.alt].filter(Boolean) : [], { nScans: g.n });
  }
  const lvCell = res => { const lv = LIVELLI.shown(res); return `<span class="lvb ${LIVELLI.cls(lv)}" title="${EHt(LIVELLI.sentence(lv) + " · " + res.decisive)}">${EHt(lv)}</span>`; };
  async function identifyAll(k) {
    ensureCss();
    const f = E.files[k]; if (!f) return;
    const k2 = (E.files.find(x => phys(x) === phys(f) && x.kind === "ms2" && !x.gone) || {}).k;
    if (k2 == null) return info(I18N.t("lib.noMs2"));
    let d = document.getElementById("libid");
    if (!d) { d = document.createElement("dialog"); d.id = "libid"; document.body.appendChild(d); }
    d.innerHTML = `<div class="top" style="display:flex;align-items:center"><h3 style="margin:0;flex:1">${I18N.t("lib.all.title")}</h3><button class="x" id="li-x" title="${I18N.t("common.close")}">&times;</button></div>
      <div class="sm muted" style="margin-top:4px">${EHt(f.label || f.file)} · ${I18N.t("lib.all.sub", { ppm: profOf(f).v })}</div>
      <div id="li-st" style="margin:8px 0"></div><div id="li-body"></div>`;
    d.querySelector("#li-x").onclick = () => d.close(); d.showModal();
    const st = d.querySelector("#li-st"), say = (t, v, m) => { st.innerHTML = `<div class="sm">${EHt(t)}</div>${m ? `<progress max="${m}" value="${v}"></progress>` : ""}`; };
    try {
      const libs = await call("list"); if (!libs.libs.length) { st.innerHTML = `<p>${I18N.t("lib.loadFirst")}</p><p><button id="li-open">${I18N.t("lib.openLibs")}</button></p>`; d.querySelector("#li-open").onclick = () => { d.close(); open(); }; return; }
      say(I18N.t("lib.all.reading"), 0, 1);
      const dda = await J(`api/dda?k=${k2}`), tgt = new Map(dda.sid.map((s, i) => [s, dda.tgt[i]])), pol = f.polarity === "negative" ? -1 : 1, queries = [];
      let n = 1, i0 = 0;
      for (; i0 < n; i0 += 60) {
        const b = typeof scBin === "function" ? await scBin(`api/scanbin?k=${k2}&i0=${i0}&i1=${i0 + 59}&level=2`) : await J(`api/spectra?k=${k2}&i0=${i0}&i1=${i0 + 59}&level=2`); n = b.n;
        for (const s of b.scans) { const pr = tgt.get(s.sid); if (pr && s.mz.length) queries.push({ key: s.sid, rt: s.rt, prec: pr, pol, peaks: s.mz.map((m, j) => [m, s.y[j]]) }); }
        say(I18N.t("lib.all.readingN", { i: Math.min(i0 + 60, n), n }), Math.min(i0 + 60, n), n);
      }
      const t0 = performance.now();
      say(I18N.t("lib.all.comparing"), 0, queries.length);
      const r = await call("searchAll", { queries, tol: { v: profOf(f).v, unit: "ppm" }, frag: 0.01 }, m => say(I18N.t("lib.all.comparingN", { i: m.read, n: m.total }), m.read, m.total));
      if (window.LISTE) await LISTE.ready();
      let showHid = false;                                   // contaminant exclusions (liste.js): candidates whose precursor is a contaminant the student excluded stay out of the list, counted, until «Mostra»
      const groups = groupRows(r.rows), show = all => {
        const hidOf = g => { const c = window.LISTE ? LISTE.compat(g.prec, f.polarity, UIP.hrPpm) : null; return !!(c && c.hid); };
        const base = groups.map(g => ({ g, v: groupAssess(g), k: window.LISTE ? LISTE.textFor(g.prec, f.polarity, UIP.hrPpm) : "", hid: hidOf(g) })).filter(x => all || (x.v.proposed !== "5" && x.v.proposed !== "0"));
        const nHid = base.filter(x => x.hid).length, rows = showHid ? base : base.filter(x => !x.hid), hasK = rows.some(x => x.k);
        d.querySelector("#li-body").innerHTML = (nHid ? `<div class="sm cont-cnt" role="status">${I18N.t(showHid ? "cont.cnt.shown" : "cont.cnt", { n: nHid })} · <button type="button" id="li-chid">${I18N.t(showHid ? "cont.hide" : "cont.show")}</button></div>` : "") + `<div class="bar"><button id="li-xlsx" type="button">${typeof IC_DL !== "undefined" ? IC_DL : ""}Excel</button><button id="li-mgf" type="button">${I18N.t("lib.all.exportMgf")}</button><button id="li-msp" type="button">${I18N.t("lib.all.exportMsp")}</button></div><label class="sm"><input type="checkbox" id="li-all"${all ? " checked" : ""}> ${I18N.t("lib.all.showAll", { n: groups.length })}</label>
          <div class="tbl" style="max-height:60vh"><table><tr><th class="num">RT</th><th class="num"><i>m/z</i></th><th>${I18N.t("lib.col.name")}</th><th>${I18N.t("lst.col.formula")}</th><th class="num">${I18N.t("lib.all.bestEntropy")}</th><th class="num">${I18N.t("lib.all.scans")}</th><th>${I18N.t("lv.proposed")}</th>${hasK ? "<th>" + I18N.t("lib.all.contaminant") + "</th>" : ""}</tr>
          ${rows.map((x, i) => `<tr class="row" data-i="${i}" title="${I18N.t("lib.all.openBest")}"><td class="num">${x.g.rt.toFixed(2)}</td><td class="num">${x.g.prec.toFixed(4)}</td><td>${EHt(x.g.best.name || "")}</td><td>${x.g.best.formula ? fmtFormula(x.g.best.formula) : ""}</td><td class="num">${x.g.best.ent != null ? x.g.best.ent.toFixed(2) : ""}</td><td class="num">${x.g.n}${x.g.n < 2 ? " " + I18N.t("lib.all.oneScan") : ""}</td><td>${lvCell(x.v)}</td>${hasK ? `<td class="sm">${EHt(x.k || "")}</td>` : ""}</tr>`).join("") || `<tr><td colspan="7" class="muted">${I18N.t("lib.all.noMatch")}</td></tr>`}</table></div>`;
        d.querySelector("#li-all").onchange = e => show(e.target.checked);
        const chid = d.querySelector("#li-chid"); if (chid) chid.onclick = () => { showHid = !showHid; show(all); };
        const btnMgf = d.querySelector("#li-mgf");
        if (btnMgf) btnMgf.onclick = () => { if (window.WORKFLOW) WORKFLOW.exportQueriesMgf(queries, f); };
        const btnMsp = d.querySelector("#li-msp");
        if (btnMsp) btnMsp.onclick = () => { if (window.WORKFLOW) WORKFLOW.exportQueriesMsp(queries, f); };
        d.querySelector("#li-xlsx").onclick = () => dlx(I18N.t("lib.all.xlsxPrefix") + String(f.label || f.file || "file").replace(/[^\w.-]+/g, "_") + ".xlsx", [
          { name: I18N.t("lib.all.sheetCand"), head: ["RT (min)", I18N.t("lib.all.precMz"), I18N.t("lib.col.name"), I18N.t("lst.col.formula"), I18N.t("lib.all.bestEntropy"), I18N.t("lib.all.scans"), I18N.t("lv.proposed"), I18N.t("lv.decided"), I18N.t("lv.evidence")].concat(hasK ? [I18N.t("lib.all.contaminant")] : []), rows: rows.map(x => [+x.g.rt.toFixed(3), +x.g.prec.toFixed(5), x.g.best.name || "", x.g.best.formula || "", x.g.best.ent != null ? +x.g.best.ent.toFixed(3) : "", x.g.n, x.v.proposed, x.v.decided || "", x.v.lines.concat([x.v.decisive]).join(" | ")].concat(hasK ? [x.k || ""] : [])), widths: [10, 14, 40, 20, 14, 10, 12, 12, 60] },
          { name: I18N.t("lib.all.sheetPar"), head: [I18N.t("lib.all.parameter"), I18N.t("lib.all.value")], rows: [[I18N.t("lib.table.file"), f.label || f.file || ""], [I18N.t("lib.all.precTol"), `${profOf(f).v} ppm`], [I18N.t("lib.all.peakTol"), I18N.t("lib.all.peakTolVal")], [I18N.t("lib.all.shown"), I18N.t("lib.all.shownVal", { n: rows.length, total: groups.length })], [I18N.t("lib.all.level"), I18N.t("lib.all.levelVal")]], widths: [28, 70] }]);
        d.querySelectorAll("#li-body tr.row").forEach(tr => tr.onclick = () => gotoScan(rows[+tr.dataset.i].g.best.key, d));
        st.innerHTML = `<div class="sm">${I18N.t("lib.all.summary", { q: queries.length, ms: Math.round(performance.now() - t0), g: groups.length, r: rows.length })}</div>`;
      };
      show(false);
    } catch (e) { st.innerHTML = `<p class="err">${EHt(e.message)}</p>`; }
  }
  function gotoScan(sid, d) {                         // the MS2 panel of the DDA trio shows that scan
    try {
      const s1 = E.panels.find(q => q.type === "spec" && q.duo != null && q.el), s2 = s1 && E.panels.find(q => q.id === s1.duo && q.el);
      if (!s2 || !window.DDA) return toast(I18N.t("lib.all.needDda"));
      d.close(); DDA.selectMs2(s2, sid); s2.el.scrollIntoView({ block: "center", behavior: "smooth" });
    } catch (e) { toast(I18N.t("lib.all.cantOpen")); }
  }
  return { open, searchFrom, call, identifyAll, groupRows, groupAssess, assess, scansOf, engine };
})();
window.LIB = LIB;
