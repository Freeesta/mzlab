// @ts-strict
// @ts-noany
"use strict";
// Part D: compare a MS2 spectrum with the spectral libraries (MSP, MGF) the user has loaded. Classic script, loaded after settings.js/explore.js.
// The reading, the archive (OPFS) and the search live in libreria-worker.js (no Pyodide); here: the «Librerie» window (gear), the window with the
// results and the mirror plot (experimental on top, library below). Nothing here touches drawSpec: the mirror is a canvas of its own.
const LIB = (() => {
  // Supervisor: if the worker dies (a trap of the WebAssembly engine, a lost memory) it is replaced and the request is tried once more; a second death within a
  // minute sends the page to the JavaScript path for the rest of the session. `?libjs=1` starts there (tests, browsers where the engine does not load).
  let /** @type {Worker | null} */ worker = null, seq = 0, /** @type {number[]} */ restarts = []; const pend = new Map();
  let forceJs = false; try { forceJs = new URLSearchParams(location.search).get("libjs") === "1"; } catch (e) { /* no location */ }
  const say = (/** @type {string} */ key) => { try { if (typeof toast === "function") toast(I18N.t(key)); } catch (e) { /* no page */ } };
  function w() {
    if (worker) return worker;
    const me = worker = new Worker("static/libreria-worker.js");
    me.onmessage = e => {
      const m = e.data, r = pend.get(m.reqId); if (!r) return;
      if (m.type === "progress") { r.prog && r.prog(m); return; }
      pend.delete(m.reqId); if (m.type === "error") { const er = I18N.err(m); er.fatal = !!m.fatal; r.no(er); } else r.ok(m);
    };
    me.onerror = e => { const er = new Error(I18N.t("lib.err.worker", { message: e.message || "error" })); /** @type {any} */ (er).fatal = true; pend.forEach(r => r.no(er)); pend.clear(); if (worker === me) worker = null; };
    // the engine's .wasm exists only in the built site (window.MZLAB_BROWSER, browser.js) and in the app
    me.postMessage({ cmd: "config", reqId: 0, forceJs, wasm: !!(window.MZLAB_BROWSER || window.MZLAB_DESKTOP) });
    // the folder of the libraries (chosen before the first call: call1 waits for folderStart)
    me.postMessage({ cmd: "folder", reqId: 0, ...folderMsg });
    return me;
  }
  function restart() {
    if (worker) { try { worker.terminate(); } catch (e) { /* gone */ } worker = null; }
    const er = new Error("restarted"); pend.forEach(r => r.no(er)); pend.clear();
    const now = Date.now(); restarts = restarts.filter(t => now - t < 60000); restarts.push(now);
    if (restarts.length > 1) { forceJs = true; say("lib.err.fallback"); } else say("lib.err.restarted");
  }
  /** @param {string} cmd @param {Bag} [payload] @param {(m: Bag) => void} [prog] */ const call1 = async (cmd, payload = {}, prog) => { await folderStart(); return call2(cmd, payload, prog); };
  /** @param {string} cmd @param {Bag} [payload] @param {(m: Bag) => void} [prog] */ const call2 = (cmd, payload = {}, prog) => new Promise((ok, no) => { const reqId = ++seq; pend.set(reqId, { ok, no, prog }); try { w().postMessage({ cmd, reqId, ...payload }); } catch (e) { pend.delete(reqId); no(e); } });
  /** @param {string} cmd @param {Bag} [payload] @param {(m: Bag) => void} [prog] */ const call = async (cmd, payload = {}, prog) => {
    try { return await call1(cmd, payload, prog); }
    catch (e_) { const e = /** @type {Lanciato} */ (e_); if (!(e && e.fatal)) throw e; restart(); return call1(cmd, payload, prog); }
  };
  // for the tests: which engine answers, and a way to kill it
  const engine = { info: () => call1("engine"), async crash() { try { await call1("crash"); } catch (e_) { const e = /** @type {Lanciato} */ (e_); if (e && e.fatal) restart(); else throw e; } }, get fallback() { return forceJs; } };
  const EHt = (/** @type {unknown} */ t) => (typeof EH === "function" ? EH(t) : String(t));
  const fmtN = (/** @type {number | string} */ n) => I18N.num(+n);
  const dlgCss = `<style>#libdlg,#libres{max-width:min(1100px,96vw);width:min(1100px,96vw)}#libdlg{width:min(720px,96vw)}#libdlg .drop{border:2px dashed var(--line);border-radius:8px;padding:14px;text-align:center;margin:8px 0}#libdlg .drop.over{border-color:var(--accent);background:var(--sel)}
    #libdlg table,#libres table{border-collapse:collapse;width:100%}#libdlg td,#libdlg th,#libres td,#libres th{padding:3px 8px;border-bottom:1px solid var(--line);text-align:left;font-size:12px}#libres td.num,#libres th.num,#libdlg td.num,#libdlg th.num{text-align:right;font-variant-numeric:tabular-nums}
    #libres tr.row{cursor:pointer}#libres tr.row.sel{background:var(--sel)}#libres tr.row:hover{background:var(--sel)}#libres .qbar{display:flex;gap:12px;flex-wrap:wrap;align-items:center;margin:6px 0}#libres .qbar input{width:90px}#libres canvas{width:100%;height:300px;display:block;border:1px solid var(--line);border-radius:6px;margin-top:8px}
    #libres .tbl{max-height:260px;overflow:auto;border:1px solid var(--line);border-radius:6px}#libdlg .err{color:var(--bad)}#libdlg progress{width:100%}#libdlg .sbadge{border:1px solid currentColor;border-radius:10px;padding:1px 8px;font-size:11px;white-space:nowrap}#libdlg .st-ok{color:var(--ok)}#libdlg .st-warn{color:var(--warn)}#libdlg .st-bad{color:var(--bad)}#libdlg fieldset{border:1px solid var(--line);border-radius:8px;margin:8px 0}</style>`;
  const ensureCss = () => { if (!document.getElementById("libcss")) { const s = document.createElement("div"); s.id = "libcss"; s.innerHTML = dlgCss; document.head.appendChild(/** @type {Node} */ (s.firstChild)); } };

  // ------------------------------------------------------------ the folder of the libraries (libreria-cartella.js in the worker)
  // desktop: the native folder (the app keeps the path); Chrome / Edge: a folder chosen once (the handle lives in IndexedDB, the permission is asked at the first click of the visit);
  // anywhere else, and by default: the private file system of the browser (OPFS, `librerie/`), where the files are imported once.
  const BASE = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : "mzlab://localhost/";
  const T = () => (window.__TAURI__ && window.__TAURI__.core) || null;
  const LS = { get: (/** @type {string} */ k) => { try { return localStorage.getItem(k); } catch (e) { return null; } }, set: (/** @type {string} */ k, /** @type {string | null} */ v) => { try { if (v == null) localStorage.removeItem(k); else localStorage.setItem(k, v); } catch (e) { /* no storage */ } } };
  const dirDb = () => new Promise((ok, no) => { const r = indexedDB.open("mzlab-libdir", 1); r.onupgradeneeded = () => r.result.createObjectStore("h"); r.onsuccess = () => ok(r.result); r.onerror = () => no(r.error); });
  const dirGet = async () => { try { const db = await dirDb(); return await new Promise(ok => { const r = db.transaction("h").objectStore("h").get("dir"); r.onsuccess = () => { db.close(); ok(r.result || null); }; r.onerror = () => { db.close(); ok(null); }; }); } catch (e) { return null; } };
  const dirPut = async (/** @type {Bag | null} */ h) => { try { const db = await dirDb(); await new Promise(/** @type {(v?: any) => void} */ ok => { const t = db.transaction("h", "readwrite"); if (h) t.objectStore("h").put(h, "dir"); else t.objectStore("h").delete("dir"); t.oncomplete = () => { db.close(); ok(); }; t.onerror = () => { db.close(); ok(); }; }); } catch (e) { /* no storage */ } };
  // what the worker is told; `waiting` = a remembered folder that needs a click to be allowed again
  let /** @type {Bag} */ folderMsg = { mode: "opfs" }, /** @type {Promise<void> | null} */ folderInit = null, /** @type {Bag | null} */ waiting = null;
  const folderInfo = { mode: "opfs", name: "", needsGrant: false };
  // the folder of the computer needs showDirectoryPicker (capacita.js)
  const folderOk = () => !!(window.CAPACITA ? CAPACITA.ha("cartellaLocale") : true) && !!/** @type {any} */ (window).showDirectoryPicker;
  function folderStart() {
    if (!folderInit) folderInit = (async () => {
      if (window.MZLAB_DESKTOP) { folderMsg = { mode: "native", base: BASE }; folderInfo.mode = "native"; return; }
      if (folderOk() && LS.get(NOMI.KEY("lib.cartella")) === "fs") {
        const h = await dirGet();
        if (h) {
          let perm = "prompt"; try { perm = await h.queryPermission({ mode: "readwrite" }); } catch (e) { /* old handle */ }
          folderInfo.name = h.name;
          if (perm === "granted") { folderMsg = { mode: "fs", handle: h }; folderInfo.mode = "fs"; }
          else { waiting = h; folderInfo.needsGrant = true; document.addEventListener("pointerdown", grantOnce, { capture: true, once: true }); document.addEventListener("keydown", grantOnce, { capture: true, once: true }); }
        }
      }
    })();
    return folderInit;
  }
  async function grantOnce() {
    const h = waiting; if (!h) return;
    try { if ((await h.requestPermission({ mode: "readwrite" })) === "granted") { waiting = null; folderInfo.needsGrant = false; folderInfo.mode = "fs"; folderMsg = { mode: "fs", handle: h }; await pushFolder(); changed(); } } catch (e) { /* refused: the browser folder stays */ }
  }
  const pushFolder = () => call1("folder", folderMsg);
  /** @type {(() => void)[]} */ const listeners = []; const changed = () => listeners.forEach(f => { try { f(); } catch (e) { /* a window that is gone */ } });
  async function chooseFolder() {
    if (T()) { const r = await T().invoke("lib_choose_dir"); if (r) { folderMsg = { mode: "native", base: BASE }; await pushFolder(); } return; }
    const h = await /** @type {any} */ (window).showDirectoryPicker({ id: "mzlab-libs", startIn: "desktop", mode: "readwrite" });
    await dirPut(h); LS.set(NOMI.KEY("lib.cartella"), "fs"); waiting = null; folderInfo.needsGrant = false; folderInfo.mode = "fs"; folderInfo.name = h.name; folderMsg = { mode: "fs", handle: h }; await pushFolder();
  }
  async function useBrowserFolder() { await dirPut(null); LS.set(NOMI.KEY("lib.cartella"), null); waiting = null; folderInfo.needsGrant = false; folderInfo.mode = "opfs"; folderInfo.name = ""; folderMsg = { mode: "opfs" }; await pushFolder(); }

  // ------------------------------------------------------------ the «Librerie» window
  const STATES = /** @type {Record<string, string>} */ ({ pronta: "ok", da_indicizzare: "warn", aggiornamento: "warn", danneggiata: "bad" });
  const stLabel = (/** @type {string} */ st) => ({ pronta: I18N.t("lib.st.pronta"), da_indicizzare: I18N.t("lib.st.da_indicizzare"), aggiornamento: I18N.t("lib.st.aggiornamento"), danneggiata: I18N.t("lib.st.danneggiata") })[st] || st;
  async function open() {
    ensureCss(); await folderStart();
    let d = /** @type {any} */ (document.getElementById("libdlg"));
    if (!d) {
      d = document.createElement("dialog"); d.id = "libdlg"; document.body.appendChild(d);
      d.innerHTML = `<div class="top" style="display:flex;align-items:center"><h3 style="margin:0;flex:1">${I18N.t("lib.title")}</h3><button class="x" id="lib-x" title="${I18N.t("common.close")}">&times;</button></div>
        <p class="sm muted" style="margin:6px 0">${I18N.t("lib.intro")}</p>
        <fieldset class="lib-folder" aria-label="${I18N.t("lib.folder")}"><legend class="sm">${I18N.t("lib.folder")}</legend>
          <div class="sm" id="lib-fwhere"></div>
          <div class="sm" style="margin-top:6px"><button id="lib-fchoose" hidden></button> <button id="lib-fgrant" hidden>${I18N.t("lib.folder.grant")}</button> <button id="lib-fbrowser" hidden>${I18N.t("lib.folder.browser")}</button> <button id="lib-fscan">${I18N.t("lib.folder.scan")}</button> <button id="lib-fall" hidden>${I18N.t("lib.indexAll")}</button></div>
          <div class="sm muted" id="lib-fnote" style="margin-top:4px"></div></fieldset>
        <fieldset class="lib-avail" aria-label="${I18N.t("lib.avail")}"><legend class="sm">${I18N.t("lib.avail")}</legend>
          <div class="sm muted">${I18N.t("lib.avail.intro")}</div>
          <div class="sm" style="margin-top:6px"><button id="lib-avail-load">${I18N.t("lib.avail.load")}</button> <button id="lib-avail-go" hidden>${I18N.t("lib.avail.go")}</button> <button id="lib-avail-cancel" hidden>${I18N.t("lib.avail.cancel")}</button></div>
          <div id="lib-avail-list" aria-live="polite"></div><div id="lib-avail-st" class="sm" aria-live="polite"></div></fieldset>
        <div class="drop" id="lib-drop">${I18N.t("lib.drop")} <button id="lib-pick">${I18N.t("lib.pick")}</button> <button id="lib-pickdir">${I18N.t("lib.pickDir")}</button><input type="file" id="lib-file" accept=".msp,.mgf,.json,.jsonl,.ndjson,.txt,.mzlib" multiple hidden><input type="file" id="lib-dirin" webkitdirectory multiple hidden></div>
        <div id="lib-st"></div><div id="lib-list"></div><div class="sm muted" id="lib-space" style="margin-top:6px"></div>`;
      d.querySelector("#lib-x").onclick = () => d.close();
      const input = d.querySelector("#lib-file"), dirin = d.querySelector("#lib-dirin"), drop = d.querySelector("#lib-drop");
      d.querySelector("#lib-pick").onclick = () => input.click();
      d.querySelector("#lib-pickdir").onclick = () => dirin.click();
      d.querySelector("#lib-fchoose").onclick = async (/** @type {Event} */ ev) => { if (!T() && !folderOk()) { CAPACITA.serve("cartella", /** @type {HTMLElement} */ (ev.currentTarget)); return; } try { await chooseFolder(); } catch (e_) { const e = /** @type {Lanciato} */ (e_); if (e && e.name !== "AbortError") d.querySelector("#lib-st").innerHTML = `<div class="err">${EHt(e.message)}</div>`; } refresh(); };
      d.querySelector("#lib-fgrant").onclick = async () => { await grantOnce(); refresh(); };
      d.querySelector("#lib-fbrowser").onclick = async () => { await useBrowserFolder(); refresh(); };
      d.querySelector("#lib-fscan").onclick = () => refresh();
      d.querySelector("#lib-fall").onclick = () => indexAll();
      d.querySelector("#lib-avail-load").onclick = () => loadCatalog();
      d.querySelector("#lib-avail-go").onclick = () => downloadTicked();
      d.querySelector("#lib-avail-cancel").onclick = () => { if (T()) T().invoke("lib_cancel"); else call("cancel"); };
      input.onchange = () => { addFiles([...input.files]); input.value = ""; };
      dirin.onchange = () => { addFiles([...dirin.files].filter(f => /\.(msp|mgf|json|jsonl|ndjson|mzlib)$/i.test(f.name))); dirin.value = ""; };
      drop.ondragover = (/** @type {DragEvent} */ e) => { e.preventDefault(); drop.classList.add("over"); }; drop.ondragleave = () => drop.classList.remove("over");
      drop.ondrop = (/** @type {DragEvent} */ e) => { e.preventDefault(); drop.classList.remove("over"); addFiles([.../** @type {DataTransfer} */ (e.dataTransfer).files]); };
      listeners.push(() => { if (d.open) refresh(); });
    }
    if (!d.open) d.showModal();
    refresh();
  }
  /** @type {string[]} */ let toIndex = [];
  async function refresh() {
    const d = /** @type {any} */ (document.getElementById("libdlg")); if (!d) return;
    const box = d.querySelector("#lib-list");
    try {
      const r = await call("list"), L = r.libs, V = r.voci || [], inV = new Set(V.map((/** @type {Bag} */ v) => v.id));
      const f = r.folder || { mode: folderInfo.mode, name: "" };
      d.querySelector("#lib-fwhere").innerHTML = `<b>${EHt(f.mode === "native" ? I18N.t("lib.folder.native", { name: f.name }) : f.mode === "fs" ? I18N.t("lib.folder.fs", { name: folderInfo.name || f.name }) : I18N.t("lib.folder.opfs"))}</b>${r.folderError ? ` · <span class="err">${EHt(I18N.t("lib.folder.error", { name: r.folderError }))}</span>` : ""}`;
      const bc = d.querySelector("#lib-fchoose"); bc.hidden = false; if (!T() && !folderOk()) CAPACITA.serve("cartella", bc); bc.textContent = I18N.t(T() ? "lib.folder.change" : "lib.folder.choose");
      d.querySelector("#lib-fgrant").hidden = !folderInfo.needsGrant; d.querySelector("#lib-fbrowser").hidden = !(folderOk() && !T() && (folderInfo.mode === "fs" || folderInfo.needsGrant));
      d.querySelector("#lib-fnote").textContent = f.mode === "opfs" ? I18N.t("lib.folder.noteOpfs") : "";
      toIndex = V.filter((/** @type {Bag} */ v) => v.tipo === "sorgente" && (v.stato === "da_indicizzare" || v.stato === "danneggiata")).map((/** @type {Bag} */ v) => v.nome);
      d.querySelector("#lib-fall").hidden = toIndex.length < 2;
      const row = (/** @type {string} */ name, /** @type {string} */ file, /** @type {number | null | undefined} */ n, /** @type {number | undefined} */ size, /** @type {string} */ pol, /** @type {string} */ st, /** @type {string | undefined} */ extra, /** @type {string | undefined} */ rm, /** @type {string | undefined} */ idx, /** @type {string | undefined} */ title, /** @type {number | null | undefined} */ drop) => `<tr><td title="${EHt(file)}">${EHt(name)}</td><td class="num">${n == null ? "" : fmtN(n)}</td><td class="num">${size ? fmtN(Math.round(size / 1e5) / 10) + " MB" : ""}</td><td>${pol}</td><td class="num" ${title ? `title="${EHt(title)}"` : ""}>${drop == null ? "" : fmtN(drop)}</td><td><span class="sbadge st-${STATES[st] || "ok"}">${EHt(stLabel(st))}</span>${extra || ""}</td><td>${idx}${rm}</td></tr>`;
      const polOf = (/** @type {Bag | null | undefined} */ i) => (i ? `ESI+ ${fmtN(i.pos || 0)} · ESI&minus; ${fmtN(i.neg || 0)}${i.n - (i.pos || 0) - (i.neg || 0) > 0 && (i.pos || i.neg) ? " · ? " + fmtN(i.n - i.pos - i.neg) : ""}` : "");
      const rows = V.map((/** @type {Bag} */ v) => row(CARTELLA_stem(v.nome), v.nome, v.info && v.info.n, v.size, polOf(v.info), v.stato,
        v.nuova ? ` <span class="sm">${EHt(I18N.t("lib.st.newer", { v: v.nuova }))}</span>` : "", v.stato === "pronta" || v.stato === "aggiornamento" || v.tipo !== "sorgente" ? `<button data-rm="${EHt(v.id)}">${I18N.t("lib.remove")}</button>` : "",
        v.tipo === "sorgente" && (v.stato === "da_indicizzare" || v.stato === "danneggiata") ? `<button data-idx="${EHt(v.nome)}">${I18N.t("lib.index")}</button> ` : "", v.stato === "danneggiata" ? I18N.t("lib.st.damagedHint") : "", v.info && v.info.dropped));
      const legacy = L.filter((/** @type {Bag} */ l) => !inV.has(l.id) && !l.cartella).map((/** @type {Bag} */ l) => row(l.name, l.file, l.n, l.size, polOf(l), "pronta", "", `<button data-rm="${EHt(l.id)}">${I18N.t("lib.remove")}</button>`, "", I18N.t("lib.droppedTitle", { prec: l.noPrec, peaks: l.noPeaks, broken: l.broken }), l.dropped));
      box.innerHTML = rows.length + legacy.length ? `<table><tr><th>${I18N.t("lib.col.library")}</th><th class="num">${I18N.t("lib.col.spectra")}</th><th class="num">${I18N.t("lib.col.size")}</th><th>${I18N.t("lib.col.polarity")}</th><th class="num">${I18N.t("lib.col.dropped")}</th><th>${I18N.t("lib.col.state")}</th><th></th></tr>${rows.join("")}${legacy.join("")}</table>` : `<p class="muted">${I18N.t("lib.none")}</p>`;
      box.querySelectorAll("[data-rm]").forEach((/** @type {HTMLButtonElement} */ b) => b.onclick = async () => { b.disabled = true; await call("remove", { id: b.dataset.rm }); refresh(); });
      box.querySelectorAll("[data-idx]").forEach((/** @type {HTMLButtonElement} */ b) => b.onclick = () => indexOne(/** @type {string} */ (b.dataset.idx)));
      if (navigator.storage && navigator.storage.estimate) { const s = await navigator.storage.estimate(); d.querySelector("#lib-space").textContent = f.mode === "opfs" || f.mode == null ? I18N.t("lib.space", { mb: Math.round((s.usage || 0) / 1e6) }) + (r.opfs ? "" : " " + I18N.t("lib.noOpfs")) : ""; }
      if (f.mode === "opfs" && navigator.storage && /** @type {any} */ (navigator.storage).persist) { await (await import("./archivio.js")).persistOnce(); const p = await navigator.storage.persisted(); d.querySelector("#lib-fnote").textContent = I18N.t("lib.folder.noteOpfs") + (p ? "" : " " + I18N.t("lib.folder.persistNo")); }
    } catch (e_) { const e = /** @type {Lanciato} */ (e_); box.innerHTML = h`<p class="err">${e.message}</p>`; }
  }
  const CARTELLA_stem = (/** @type {string} */ n) => String(n).replace(/\.(msp|mgf|json|jsonl|ndjson|txt|mzlib)$/i, "");
  async function indexOne(/** @type {string} */ name) {
    const st = document.querySelector("#libdlg #lib-st");
    st.innerHTML = h`<div class="sm">${name}: ${raw(I18N.t("lib.reading"))}</div><progress max="1" value="0"></progress>`;
    try {
      const r = await call("index", { name }, (/** @type {Bag} */ m) => { st.innerHTML = `<div class="sm">${EHt(name)}: ${I18N.t("lib.readN", { n: fmtN(m.n), pct: Math.round(m.read / m.total * 100) })}</div><progress max="${m.total}" value="${m.read}"></progress>`; });
      st.innerHTML = `<div class="sm">${EHt(name)}: ${I18N.t("lib.doneN", { n: fmtN(r.info.n) })}.</div>`;
    } catch (e_) { const e = /** @type {Lanciato} */ (e_); st.innerHTML = h`<div class="err">${name}: ${e.message}</div>`; }
    await refresh();
  }
  async function indexAll() { for (const n of [...toIndex]) await indexOne(n); }

  // ------------------------------------------------------------ the public packages («Disponibili»): the list comes from the site (or, in the app, from the native side) only when the user asks
  let /** @type {any} */ avail = null, busy = false;   // the catalog from the worker: null until first loaded
  const mb = (/** @type {number} */ n) => fmtN(Math.round(n / 1e5) / 10);
  const avLabel = (/** @type {string} */ st) => ({ new: I18N.t("lib.avail.st.new"), update: I18N.t("lib.avail.st.update"), installed: I18N.t("lib.avail.st.installed"), damaged: I18N.t("lib.avail.st.damaged") })[st] || st;
  // answered by the native side (listed so that tools/controlla_i18n.py sees the keys)
  const NATIVE_ERRORS = ["lib.dl.err.host", "lib.dl.err.externalOff"];
  const errText = (/** @type {string} */ key, /** @type {Bag | undefined} */ params) => I18N.t(NATIVE_ERRORS.includes(key) || key.startsWith("lib.dl.err.") ? key : "lib.dl.err.io", params || {});
  // the native side answers a JSON text: the result, or {status:"error", error_key, detail}
  /** @param {string} cmd @param {Bag} [args] */ async function nativeJson(cmd, args) {
    const j = JSON.parse(await T().invoke(cmd, args));
    if (j.status === "error") throw new Error(errText(j.error_key, { detail: j.detail }));
    return j;
  }
  async function loadCatalog() {
    const d = /** @type {any} */ (document.getElementById("libdlg")), box = d.querySelector("#lib-avail-list");
    box.innerHTML = `<p class="sm muted">${I18N.t("lib.reading")}</p>`;
    try {
      avail = T() ? await call("catalog", { cat: (await nativeJson("lib_catalog")).catalog }) : await call("catalog");
      avail.cfg = T() ? await T().invoke("lib_get_config") : null;
      renderAvail();
    } catch (e_) { const e = /** @type {Lanciato} */ (e_); box.innerHTML = h`<p class="err">${e.message}</p>`; d.querySelector("#lib-avail-go").hidden = true; }
  }
  function renderAvail() {
    const d = /** @type {any} */ (document.getElementById("libdlg")), box = d.querySelector("#lib-avail-list"), A = avail, st = new Map(A.avail.map((/** @type {Bag} */ a) => [a.id, a.stato]));
    const rows = A.cat.librerie.map((/** @type {Bag} */ p) => { const s = st.get(p.id) || "new";
      return `<tr><td><input type="checkbox" data-dl="${EHt(p.id)}" ${s === "installed" ? "disabled" : ""} aria-label="${EHt(p.nome)}"></td><td>${EHt(p.nome)}</td><td>${EHt(p.versione)}</td><td class="num">${fmtN(p.spettri)}</td><td class="num">${mb(p.dimensione)} MB</td><td title="${EHt(p.licenza)}">${EHt(p.licenza.split(";")[0].split(":")[0])}</td><td><span class="sbadge st-${s === "installed" ? "ok" : s === "damaged" ? "bad" : "warn"}">${EHt(avLabel(s))}</span></td></tr>`; });
    const ext = A.cfg && A.cfg.external ? A.cat.esterne : [];
    const xrows = ext.map((/** @type {Bag} */ e) => `<tr><td><input type="checkbox" data-dlx="${EHt(e.id)}" aria-label="${EHt(e.nome)}"></td><td>${EHt(e.nome)}</td><td colspan="2" class="num">${e.dimensione ? mb(e.dimensione) + " MB" : ""}</td><td colspan="2" title="${EHt(e.licenza || "")}">${EHt((e.licenza || "").split(":")[0])}</td><td class="sm">${EHt(e.nota || "")}</td></tr>`);
    const head = `<tr><th></th><th>${I18N.t("lib.col.library")}</th><th>${I18N.t("lib.avail.col.version")}</th><th class="num">${I18N.t("lib.col.spectra")}</th><th class="num">${I18N.t("lib.col.size")}</th><th>${I18N.t("lib.avail.col.license")}</th><th>${I18N.t("lib.col.state")}</th></tr>`;
    box.innerHTML = (rows.length ? `<table>${head}${rows.join("")}</table>` : `<p class="muted">${I18N.t("lib.avail.none")}</p>`)
      + (xrows.length ? `<div class="sm" style="margin-top:6px"><b>${I18N.t("lib.avail.ext")}</b> ${I18N.t("lib.avail.ext.hint")}</div><table>${xrows.join("")}</table>` : "")
      + (A.cat.librerie.length ? `<details class="sm" style="margin-top:6px"><summary>${I18N.t("lib.avail.attr")}</summary>${A.cat.librerie.map((/** @type {Bag} */ p) => `<p><b>${EHt(p.nome)}</b>: ${EHt(p.attribuzione || "")} ${p.url_sorgente ? `<span class="muted">(${EHt(p.url_sorgente)})</span>` : ""}</p>`).join("")}</details>` : "");
    d.querySelector("#lib-avail-go").hidden = !(rows.length || xrows.length);
  }
  async function downloadTicked() {
    if (busy) return;
    const d = /** @type {any} */ (document.getElementById("libdlg")), st = d.querySelector("#lib-avail-st"), go = d.querySelector("#lib-avail-go"), cn = d.querySelector("#lib-avail-cancel");
    const ids = [...d.querySelectorAll("[data-dl]:checked")].map(c => c.dataset.dl), xids = [...d.querySelectorAll("[data-dlx]:checked")].map(c => c.dataset.dlx);
    if (!ids.length && !xids.length) return;
    busy = true; go.disabled = true; cn.hidden = false;
    const name = (/** @type {string} */ id) => (avail.cat.librerie.find((/** @type {Bag} */ p) => p.id === id) || avail.cat.esterne.find((/** @type {Bag} */ e) => e.id === id) || { nome: id }).nome;
    const all = [...ids, ...xids]; st.innerHTML = "";
    const A = (await import("./avanzamento.js")).avanza(st, { title: I18N.t("lib.avail.job"), n: all.length, cancel: false });      // the shared progress panel; «Cancel» stays the button of the window
    let cur = "";
    const show = (/** @type {Bag} */ m) => { const k = Math.max(1, all.indexOf(m.id) + 1); if (m.id !== cur) { cur = m.id; A.item(k, name(m.id)); } A.set(m.total ? (k - 1 + m.bytes / m.total) / all.length : null, I18N.t("lib.avail.progress", { name: name(m.id), piece: m.piece || 0, pieces: m.pieces || 0, mb: mb(m.bytes), total: mb(m.total) })); };
    let off = null; const fails = []; let done = 0;
    try {
      if (T() && T().event) off = await T().event.listen("lib-progress", (/** @type {{ payload: Bag }} */ ev) => show(ev.payload));
      if (ids.length) {
        if (T()) { const r = await nativeJson("lib_download", { ids }); done += r.done.length; (r.failed || []).forEach((/** @type {Bag} */ f) => fails.push(`${name(f.id)}: ${errText(f.error_key, { detail: f.detail })}`)); }
        else { const r = await call("download", { ids }, show); done += r.done.length; r.failed.forEach((/** @type {Bag} */ f) => fails.push(`${name(f.id)}: ${errText(f.key, f.params)}`)); }
      }
      for (const id of xids) {
        try { await nativeJson("lib_download_external", { id }); done++; const e = avail.cat.esterne.find((/** @type {Bag} */ x) => x.id === id); await refresh(); await indexOne(`${e.id}.${e.formato}`); }
        catch (e_) { const e = /** @type {Lanciato} */ (e_); fails.push(`${name(id)}: ${e.message}`); }
      }
    } catch (e_) { const e = /** @type {Lanciato} */ (e_); fails.push(e.message); }
    finally { if (off) off(); busy = false; go.disabled = false; cn.hidden = true; }
    fails.forEach(f => A.fail("", f)); for (let i = 0; i < done; i++) A.ok();
    A.end(done ? { head: I18N.t("lib.avail.done", { n: done }) } : undefined);
    try { avail = { ...(await call("catalog", { cached: true })), cfg: avail.cfg }; renderAvail(); } catch (e) { /* the list stays as it was */ }
    await refresh();
  }
  async function addFiles(/** @type {File[]} */ files) {
    const d = /** @type {any} */ (document.getElementById("libdlg")), st = d.querySelector("#lib-st");
    for (const f of files) {
      st.innerHTML = `<div class="sm">${EHt(f.name)}: ${I18N.t("lib.reading")}</div><progress max="1" value="0"></progress>`;
      try {
        const r = await call("add", { file: f }, (/** @type {Bag} */ m) => { st.innerHTML = `<div class="sm">${EHt(f.name)}: ${I18N.t("lib.readN", { n: fmtN(m.n), pct: Math.round(m.read / m.total * 100) })}</div><progress max="${m.total}" value="${m.read}"></progress>`; });
        st.innerHTML = `<div class="sm">${EHt(f.name)}: ${I18N.t("lib.doneN", { n: fmtN(r.info.n) })}${r.info.dropped ? ", " + I18N.t("lib.droppedN", { n: fmtN(r.info.dropped) }) : ""}.</div>`;
      } catch (e_) { const e = /** @type {Lanciato} */ (e_); st.innerHTML = `<div class="err">${EHt(f.name)}: ${EHt(e.message)}</div>`; }
      await refresh();
    }
  }

  // ------------------------------------------------------------ the query: the MS2 spectrum shown in a panel
  // tolerances: the profile of hr.js when it exists, otherwise 10 ppm for a high resolution instrument and 0.5 Da
  function profOf(/** @type {Bag} */ f) {
    try { if (window.HR && HR.prof) { const q = HR.prof(f, 2); if (q) return { hr: !!q.hr, v: q.hr ? q.tol : 0.5, unit: q.hr ? "ppm" : "Da" }; } } catch (e) { /* the fallback below */ }
    const hr = /orbitrap|exploris|fusion|q.?exactive|tof|ft-?icr|astral/i.test(String((f && f.instrument) || ""));
    return hr ? { hr: true, v: 10, unit: "ppm" } : { hr: false, v: 0.5, unit: "Da" };
  }
  function queryOf(/** @type {Bag} */ p) {
    const a = p._a, d0 = a && a.data && a.data[0]; if (!d0 || !d0.d) return null;
    const d = d0.d, y = d.y0 || d.y, f = d0.f, peaks = d.mz.map((/** @type {number} */ m, /** @type {number} */ i) => [m, y[i]]).filter((/** @type {number[]} */ q) => q[1] > 0);
    const prec = [d.prec, p._prec, p.prec, d.scan && d.scan.prec].map(Number).find(v => Number.isFinite(v) && v > 0);
    const pol = f && f.polarity === "positive" ? 1 : f && f.polarity === "negative" ? -1 : 0;
    const rt = p.r0 != null ? (p.r0 + p.r1) / 2 : null;
    return { peaks, prec, pol, f, rt, title: `${(f && f.label) || ""}${rt != null ? " · RT " + rt.toFixed(2) + " min" : ""}` };
  }

  // the peak pairs behind the score of the selected candidate (query peak, library peak, difference); `an` adds the column of the precursor shift
  function pairsTable(/** @type {any[][]} */ ps, /** @type {Bag} */ q, /** @type {Bag} */ l, /** @type {boolean} */ an) {
    if (!ps.length) return `<p class="muted">${I18N.t("lib.pairs.none")}</p>`;
    const rows = ps.slice().sort((/** @type {any[]} */ a, /** @type {any[]} */ b) => q.mz[a[0]] - q.mz[b[0]]).map((/** @type {any[]} */ pr) => {
      const dm = l.mz[pr[1]] - q.mz[pr[0]];
      return `<tr><td class="num">${q.mz[pr[0]].toFixed(4)}</td><td class="num">${l.mz[pr[1]].toFixed(4)}</td><td class="num">${(dm >= 0 ? "+" : "\u2212") + Math.abs(dm).toFixed(4)}</td><td class="num">${q.it[pr[0]].toFixed(3)}</td><td class="num">${l.it[pr[1]].toFixed(3)}</td>${an ? `<td>${pr[2] ? I18N.t("lib.pairs.yes") : ""}</td>` : ""}</tr>`;
    }).join("");
    return `<div class="tbl"><table class="lib-pairs"><caption>${I18N.t("lib.pairs.title", { n: ps.length })}</caption><tr><th scope="col" class="num">${I18N.t("lib.pairs.q")}</th><th scope="col" class="num">${I18N.t("lib.pairs.l")}</th><th scope="col" class="num">&Delta;m/z</th><th scope="col" class="num">${I18N.t("lib.pairs.qi")}</th><th scope="col" class="num">${I18N.t("lib.pairs.li")}</th>${an ? `<th scope="col">${I18N.t("lib.pairs.shift")}</th>` : ""}</tr>${rows}</table></div>`;
  }

  // ------------------------------------------------------------ the results window
  function mirror(/** @type {HTMLCanvasElement} */ cv, /** @type {Bag} */ q, /** @type {Bag} */ r) {
    // description and table: the experimental peaks above, the library ones below
    if (window.TDATI) {
      cv.setAttribute("aria-label", I18N.t("lr.mirror.aria")); TDATI.describe(cv, I18N.t("lr.mirror.desc", { n: q.mz.length, m: r.mz.length, name: r.name || "" }));
      TDATI.addButton(cv, () => ({ title: I18N.t("lr.mirror.aria"), head: [I18N.t("lr.mirror.col.src"), "m/z", I18N.t("tdati.col.rel")], rowHeader: true,
        rows: [...q.mz.map((/** @type {number} */ m, /** @type {number} */ i) => [I18N.t("lr.mirror.exp"), TDATI.num(m, 4), TDATI.num(q.it[i], 4)]), ...r.mz.map((/** @type {number} */ m, /** @type {number} */ i) => [I18N.t("lr.mirror.lib"), TDATI.num(m, 4), TDATI.num(r.it[i], 4)])] }));
    }
    const s = setup(cv), g = s.g, W = s.W, H = s.H, ml = 56, mr = 14, mt = 14, mb = 34, mid = mt + (H - mt - mb) / 2, half = (H - mt - mb) / 2 - 6;
    const all = [...q.mz, ...r.mz], lo = Math.floor(minOf(all) / 10) * 10 - 5, hi = Math.ceil(maxOf(all, r.prec) / 10) * 10 + 5, X = (/** @type {number} */ m) => ml + (m - lo) / (hi - lo) * (W - ml - mr);
    const acc = css("--accent"), ink = css("--ink"), mut = css("--muted"), line = css("--line"), shc = css("--ok");
    g.clearRect(0, 0, W, H); g.fillStyle = css("--panel"); g.fillRect(0, 0, W, H);
    const common = { q: new Set(r.pairs.map((/** @type {Bag} */ p) => p[0])), l: new Set(r.pairs.map((/** @type {Bag} */ p) => p[1])) };
    // lines between the paired peaks (type 0 = same m/z, 1 = shifted by the precursor difference) and between peaks that share a neutral loss without sharing the m/z; what is drawn is recorded in cv.dataset
    const nlp = [], nlq = new Set(), nll = new Set(), /** @type {any[][]} */ lines = [], tolNl = q.frag || 0.01, qy = (/** @type {number} */ i) => mid - q.it[i] * half, ly = (/** @type {number} */ j) => mid + r.it[j] * half, warn = css("--warn");
    if (q.prec > 0 && r.prec > 0) {
      const used = new Set((r.pairs || []).map((/** @type {Bag} */ p) => p[0] + "|" + p[1]));
      for (let i = 0; i < q.mz.length; i++) for (let j = 0; j < r.mz.length; j++) {
        const a = q.prec - q.mz[i], b = r.prec - r.mz[j];
        if (a > 0.5 && b > 0.5 && Math.abs(a - b) <= tolNl && !used.has(i + "|" + j) && !nlq.has(i) && !nll.has(j)) { nlp.push([i, j]); nlq.add(i); nll.add(j); }
      }
    }
    g.save(); g.globalAlpha = 0.55; g.lineWidth = 1;
    (r.pairs || []).forEach((/** @type {any[]} */ pr) => { const t = pr[2] ? 1 : 0; lines.push([pr[0], pr[1], t]); g.beginPath(); g.setLineDash(t ? [5, 3] : []); g.strokeStyle = t ? shc : acc; g.moveTo(X(q.mz[pr[0]]), qy(pr[0])); g.lineTo(X(r.mz[pr[1]]), ly(pr[1])); g.stroke(); });
    nlp.forEach(pr => { g.beginPath(); g.setLineDash([1, 3]); g.strokeStyle = warn; g.lineWidth = 1.5; g.moveTo(X(q.mz[pr[0]]), qy(pr[0])); g.lineTo(X(r.mz[pr[1]]), ly(pr[1])); g.stroke(); });
    g.restore(); cv.dataset.lines = JSON.stringify(lines); cv.dataset.nl = JSON.stringify(nlp);
    g.lineWidth = 1.5; g.font = fpx(12);
    // analogues: peaks matched after the shift
    const shq = new Set((r.pairs || []).filter((/** @type {Bag} */ p) => p[2]).map((/** @type {Bag} */ p) => p[0])), shl = new Set((r.pairs || []).filter((/** @type {Bag} */ p) => p[2]).map((/** @type {Bag} */ p) => p[1]));
    const bars = (/** @type {number[]} */ mz, /** @type {number[]} */ it, /** @type {Set<number>} */ set, /** @type {number} */ dir) => mz.forEach((/** @type {number} */ m, /** @type {number} */ i) => {
      const hh = it[i] * half, sh = (dir > 0 ? shq : shl).has(i), nl = (dir > 0 ? nlq : nll).has(i); g.strokeStyle = sh ? shc : nl ? warn : set.has(i) ? acc : mut; g.lineWidth = set.has(i) ? 2 : 1.3;
      g.beginPath(); g.moveTo(X(m), mid); g.lineTo(X(m), mid - dir * hh); g.stroke();
      if (it[i] > 0.08) { g.fillStyle = sh ? shc : nl ? warn : set.has(i) ? acc : mut; g.textAlign = "center"; g.fillText(m.toFixed(r.prec > 0 && q.hr ? 4 : 2), X(m), dir > 0 ? mid - hh - 4 : mid + hh + 13); }
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
  const assess = (/** @type {Bag[]} */ cands, /** @type {Bag} */ o = {}) => LIVELLI.assess({ cands, analogs: o.analogs, nScans: o.nScans, std: o.std || null, formulaUnique: !!o.formulaUnique });
  const THR_KEYS = ["entropia_min", "picchi_min", "precursore_ppm_max", "scansioni_min", "margine_min", "entropia_provvisorio", "analogo_mcos_min"];
  // «candidate, level N, evidence»: the badge, the evidence (one line per gate), the decisive sentence, the proposed and the decided level
  function levelHtml(/** @type {Bag} */ res) {
    const lv = LIVELLI.shown(res), b = res.best || res.nearest || res.analog;
    return `<div class="vd" style="margin:6px 0;padding:6px 10px;border-radius:6px;border:1px solid var(--line)"><span class="lvb ${LIVELLI.cls(lv)}">${EHt(LIVELLI.sentence(lv))}</span>${b && res.proposed !== "5" && res.proposed !== "0" ? ` <b>${EHt(b.name || "")}</b>` : ""}
      <div class="sm" style="margin-top:4px"><b>${I18N.t("lv.evidence")}</b>: ${res.lines.length ? res.lines.map((/** @type {string} */ x) => EHt(x)).join(" · ") : "–"}</div><div class="sm">${EHt(res.decisive)}</div>
      <div class="sm" style="margin-top:4px">${I18N.t("lv.proposed")}: <b>${EHt(LIVELLI.tag(res.proposed))}</b> · <label>${I18N.t("lv.decided")} <select id="lv-dec"><option value="">${I18N.t("lv.decided.none")}</option>${LIVELLI.ORDER.map((/** @type {string} */ x) => `<option value="${x}"${res.decided === x ? " selected" : ""}>${LIVELLI.tag(x)}</option>`).join("")}</select></label> <span class="sm err" id="lv-msg" role="status"></span></div></div>`;
  }
  function levelBind(/** @type {HTMLElement} */ root, /** @type {Bag} */ res, /** @type {() => void} */ again) {                       // the chemist's decision: 2b is only written here
    const sel = root.querySelector("#lv-dec"); if (!sel) return;
    sel.onchange = () => { const r = LIVELLI.decide(res, sel.value); if (!r.ok) { sel.value = res.decided || ""; root.querySelector("#lv-msg").textContent = r.msg; } else if (again) again(); };
  }
  function thresholdsHtml() {
    const T = LIVELLI.thresholds();
    return `<details id="lr-sog" style="margin-left:auto"><summary class="sm">${I18N.t("lv.thr.title")}</summary><div class="sm" style="padding:4px 0;display:flex;flex-wrap:wrap;gap:6px 14px">${THR_KEYS.map(k => `<label>${I18N.t(`lv.thr.${k}`)} <input data-thr="${k}" style="width:56px" value="${String(T[k])}"></label>`).join("")}<button type="button" id="lr-sog-reset">${I18N.t("lv.thr.reset")}</button></div></details>`;
  }
  function thresholdsBind(/** @type {HTMLElement} */ root, /** @type {() => void} */ again) {
    root.querySelectorAll("[data-thr]").forEach((/** @type {HTMLInputElement} */ el) => el.onchange = () => { const v = numIt(el.value); if (v >= 0) { LIVELLI.setThreshold(el.dataset.thr, v); again(); } });
    const rs = root.querySelector("#lr-sog-reset"); if (rs) rs.onclick = () => { LIVELLI.reset(); const T = LIVELLI.thresholds(); root.querySelectorAll("[data-thr]").forEach((/** @type {HTMLInputElement} */ el) => { el.value = String(T[/** @type {string} */ (el.dataset.thr)]); }); again(); };
  }
  // MS2 scans of the same precursor (10 ppm) and the same peak (RT gaps up to 0.2 min), as «Identify all» groups them; 1 where the file is not DDA
  async function scansOf(/** @type {Bag} */ p) {
    try {
      if (p.avg && p.avg.length > 1) return p.avg.length;
      if (p.sid == null || p.k == null) return 1;
      const d = await J(`api/dda?k=${p.k}` + (window.HR && HR.q ? HR.q() : "")), i0 = d.sid.indexOf(p.sid); if (i0 < 0 || !d.tgt[i0]) return 1;
      const t0 = d.tgt[i0], rts = d.rt.filter((/** @type {unknown} */ _, /** @type {number} */ i) => d.tgt[i] && Math.abs(d.tgt[i] - t0) <= t0 * 10e-6).sort((/** @type {number} */ a, /** @type {number} */ b) => a - b), j = rts.indexOf(d.rt[i0]); let n = 1;
      for (let a = j; a > 0 && rts[a] - rts[a - 1] <= 0.2; a--) n++;
      for (let a = j; a < rts.length - 1 && rts[a + 1] - rts[a] <= 0.2; a++) n++;
      return n;
    } catch (e) { return 1; }
  }
  // neutral change between a query and an analogue: the usual transformations first, then the simplest formula within 3 mDa (the composition of the server)
  const TRANSF = [[15.9949146, I18N.t("lib.tr.hydroxylation")], [-2.01565, I18N.t("lib.tr.dehydrogenation")], [13.97926, "+O −2H"], [-26.01565, "−C₂H₂"], [1.97926, "−CH₂ +O"], [-56.0626, I18N.t("lib.tr.dealkylation")],
    [18.010565, "+H₂O"], [-18.010565, "−H₂O"], [-27.9949146, "−CO"], [2.01565, "+2H"], [-14.01565, I18N.t("lib.tr.demethylation")], [14.01565, I18N.t("lib.tr.methylation")], [-43.98983, "−CO₂"], [31.98983, "+O₂"]];
  const sgnTxt = (/** @type {number} */ v) => (v >= 0 ? "+" : "−") + Math.abs(v).toFixed(4);
  function deltaKnown(/** @type {number} */ dm) { const t = TRANSF.find(([m]) => Math.abs(m - dm) <= 0.003); return t ? t[1] : null; }
  async function deltaFormula(/** @type {number} */ dm) {
    try {
      const j = await J(`api/composition?mz=${Math.abs(dm) - 0.000548579909}&ion=${encodeURIComponent("M+.")}&tol=3&unit=mda&max=30&elements=${encodeURIComponent("C:0-12,H:0-24,N:0-4,O:0-6,S:0-2")}`);
      const atoms = (/** @type {string} */ f) => [...f.matchAll(/([A-Z][a-z]?)(\d*)/g)].reduce((n, m) => n + (+m[2] || 1), 0), best = j.results.map((/** @type {Bag} */ r) => [atoms(r.formula), r.formula]).sort((/** @type {[number, string]} */ a, /** @type {[number, string]} */ b) => a[0] - b[0])[0];
      return best ? (dm < 0 ? "−" : "+") + best[1] + " " + I18N.t("lib.simplest") : null;
    } catch (e) { return null; }
  }
  async function searchFrom(/** @type {Pannello} */ p) {
    ensureCss();
    const q = queryOf(p);
    if (!q || !q.peaks.length) return info(I18N.t("lib.pickMs2"));
    // high resolution: «What is it?» (identity and analogs together, level and evidence)
    if (profOf(q.f).hr) return whatIs(p);
    let d = /** @type {any} */ (document.getElementById("libres"));
    if (!d) { d = document.createElement("dialog"); d.id = "libres"; document.body.appendChild(d); }
    const prof = profOf(q.f), st = /** @type {any} */ ({ prec: q.prec != null ? +q.prec.toFixed(4) : null, v: prof.v, unit: prof.unit, frag: 0.5, res: null, sel: 0 });
    d.innerHTML = `<div class="top" style="display:flex;align-items:center"><h3 style="margin:0;flex:1">${I18N.t("lib.search.title")}</h3><button class="x" id="lr-x" title="${I18N.t("common.close")}">&times;</button></div>
      <div class="sm muted" id="lr-q" style="margin-top:4px">${EHt(q.title)}</div>
      <div class="qbar"><label>${I18N.t("lib.search.prec")} <input id="lr-prec" value="${st.prec ?? ""}"></label><label>${I18N.t("lib.search.tol")} <input id="lr-tol" value="${st.v}"> <select id="lr-unit"><option value="ppm">ppm</option><option value="Da">Da</option></select></label>
        <label>${I18N.t("lib.search.peaksWithin")} <input id="lr-frag" value="${st.frag}"> Da</label><button id="lr-go">${I18N.t("lib.search.go")}</button><button id="lr-copy" title="${I18N.t("lib.copy.title")}">${I18N.t("lib.copy")}</button><button id="lr-xlsx">Excel</button><button id="lr-libs">${I18N.t("lib.libs")}</button><span class="sm muted" id="lr-msg"></span></div>

      <div id="lr-body"></div><canvas id="lr-cv" hidden></canvas>`;
    d.querySelector("#lr-unit").value = st.unit;
    d.querySelector("#lr-x").onclick = () => d.close(); d.querySelector("#lr-libs").onclick = () => { open(); };
    const msg = (/** @type {string | undefined} */ t) => { d.querySelector("#lr-msg").textContent = t || ""; };
    const colHead = ["#", I18N.t("lib.col.name"), I18N.t("lib.col.entropy"), I18N.t("lib.col.cosine"), I18N.t("lib.col.shared"), I18N.t("lib.col.dprec"), "CE", I18N.t("lib.col.instrument"), I18N.t("lib.col.adduct"), I18N.t("lib.col.library")];
    const rowsOf = (/** @type {Bag[]} */ rs) => rs.map((/** @type {Bag} */ r, /** @type {number} */ i) => [i + 1, r.name, r.ent, r.cos, r.shared, r.dprec, r.ce, r.inst, r.type, r.lib]);
    const show = () => {
      const R = st.res, body = d.querySelector("#lr-body"), cv = d.querySelector("#lr-cv");
      if (!R) { body.innerHTML = ""; cv.hidden = true; return; }
      if (!R.libs) { body.innerHTML = `<p>${I18N.t("lib.loadFirst")}</p><p><button id="lr-open">${I18N.t("lib.openLibs")}</button></p>`; body.querySelector("#lr-open").onclick = () => open(); cv.hidden = true; return; }
      if (!R.results.length) { body.innerHTML = `<p>${I18N.t("lib.noPrec", { tol: st.v + " " + st.unit })}</p>`; cv.hidden = true; return; }
      const dp = (/** @type {Bag} */ r) => st.unit === "ppm" ? r.dprec.toFixed(1) + " ppm" : (r.dprec >= 0 ? "+" : "") + r.dprec.toFixed(4) + " Da";
      body.innerHTML = `<div class="tbl"><table><tr><th class="num">#</th><th>${I18N.t("lib.col.name")}</th><th class="num">${I18N.t("lib.col.entropy")}</th><th class="num">${I18N.t("lib.col.cosine")}</th><th class="num">${I18N.t("lib.col.shared")}</th><th class="num">${I18N.t("lib.col.dprecHtml")}</th><th>CE</th><th>${I18N.t("lib.col.instrument")}</th><th>${I18N.t("lib.col.adduct")}</th><th>${I18N.t("lib.col.library")}</th></tr>${R.results.map((/** @type {Bag} */ r, /** @type {number} */ i) => `<tr class="row${i === st.sel ? " sel" : ""}" data-i="${i}"><td class="num">${i + 1}</td><td>${EHt(r.name)}</td><td class="num">${r.ent.toFixed(3)}</td><td class="num">${r.cos.toFixed(3)}</td><td class="num">${r.shared}</td><td class="num">${dp(r)}</td><td>${EHt(r.ce)}</td><td>${EHt(r.inst)}</td><td>${EHt(r.type)}</td><td>${EHt(r.lib)}</td></tr>`).join("")}</table></div>`;
      body.querySelectorAll("tr.row").forEach((/** @type {HTMLElement} */ tr) => tr.onclick = () => { st.sel = Number(tr.dataset.i); body.querySelectorAll("tr.row").forEach((/** @type {Bag} */ x) => x.classList.toggle("sel", x === tr)); draw(); });
      cv.hidden = false; draw();
    };
    const draw = () => { const R = st.res; if (!R || !R.results.length) return; const cv = d.querySelector("#lr-cv"), r = R.results[st.sel]; mirror(cv, { mz: R.query.mz, it: R.query.it, hr: st.hr }, r); };
    const run = async () => {
      const num = (/** @type {string} */ id) => numIt(String(d.querySelector(id).value));
      st.prec = num("#lr-prec"); st.v = num("#lr-tol"); st.unit = d.querySelector("#lr-unit").value; st.frag = num("#lr-frag");
      if (!(st.prec > 0) || !(st.v > 0) || !(st.frag > 0)) return msg(I18N.t("lib.checkTol"));
      msg(I18N.t("lib.searching")); const t0 = performance.now();
      try {
        st.res = await call("search", { peaks: q.peaks, prec: st.prec, pol: q.pol, tol: { v: st.v, unit: st.unit }, frag: st.frag }); st.sel = 0;
        msg(st.res.libs ? I18N.t("lib.results", { n: st.res.results.length, ms: Math.round(performance.now() - t0) }) : "");
      } catch (e_) { const e = /** @type {Lanciato} */ (e_); st.res = null; msg(I18N.t("lib.error", { message: e.message })); }
      show();
    };
    d.querySelector("#lr-go").onclick = run;
    d.querySelectorAll(".qbar input").forEach((/** @type {HTMLInputElement} */ i) => i.onkeydown = (/** @type {KeyboardEvent} */ e) => { if (e.key === "Enter") run(); });
    const table = () => { const h = colHead, rs = st.res ? rowsOf(st.res.results) : []; return { h, rs }; };
    d.querySelector("#lr-copy").onclick = async () => {
      const { h, rs } = table(); if (!rs.length) return msg(I18N.t("lib.nothingCopy"));
      const f = (/** @type {unknown} */ v, /** @type {number} */ k) => typeof v === "number" ? String(k === 5 ? +v.toFixed(st.unit === "ppm" ? 1 : 4) : +v.toFixed(3)) : String(v ?? "");
      const tsv = await PROV.tsv(h.join("\t") + "\n" + rs.map((/** @type {Bag} */ r) => r.map((/** @type {Bag} */ v, /** @type {number} */ k) => f(v, k)).join("\t")).join("\n"), { p, table: "library" });
      const html = `<table><tr>${h.map(x => `<th>${EHt(x)}</th>`).join("")}</tr>${rs.map((/** @type {Bag} */ r) => `<tr>${r.map((/** @type {Bag} */ v, /** @type {number} */ k) => `<td>${EHt(f(v, k))}</td>`).join("")}</tr>`).join("")}</table>`;
      try { if (window.ClipboardItem && navigator.clipboard.write) await navigator.clipboard.write([new ClipboardItem({ "text/plain": new Blob([tsv], { type: "text/plain" }), "text/html": new Blob([html], { type: "text/html" }) })]); else await navigator.clipboard.writeText(tsv); msg(I18N.t("lib.copied", { n: rs.length })); }
      catch (e) { try { await navigator.clipboard.writeText(tsv); msg(I18N.t("lib.copied", { n: rs.length })); } catch (e2) { msg(I18N.t("lib.noCopy")); } }
    };
    d.querySelector("#lr-xlsx").onclick = () => { const { h, rs } = table(); if (!rs.length) return msg(I18N.t("lib.nothingSave")); dlx((typeof plotName === "function" ? plotName(p) : I18N.t("lib.xlsx.fallback")) + I18N.t("lib.xlsx.suffix"), [{ name: I18N.t("lib.xlsx.sheet"), head: h, rows: rs, widths: [5, 30, 10, 10, 16, 14, 8, 14, 12, 20] }], { p, table: "library" }); };
    d.showModal(); window.addEventListener("resize", draw, { once: true });
    if (st.prec) run(); else msg(I18N.t("lib.typePrec"));
  }
  // ------------------------------------------------------------ «Cos'è?» (high resolution only): identity (Flash entropy) and analogues (hybrid) of ONE MS2 spectrum together, with level and evidence
  // the voice of the menu exists only for high-resolution spectra (low resolution gives no answers)
  const isHr = (/** @type {Bag} */ p) => { const q = queryOf(p); return !!(q && profOf(q.f).hr); };
  // cosine on the visible peaks (cos_vis of Attributio): the reference peaks below the lower limit of the MS2 scan window are removed; null when nothing is cut
  function cosVis(/** @type {Bag} */ q, /** @type {Bag} */ r, /** @type {number} */ lo) {
    if (!(lo > 0) || !r.mz.some((/** @type {number} */ m) => m < lo)) return null;
    let dot = 0, na = 0, nb = 0; for (const v of q.it) na += v;
    r.mz.forEach((/** @type {number} */ m, /** @type {number} */ j) => { if (m >= lo) nb += r.it[j]; });
    for (const pr of r.pairs || []) if (r.mz[pr[1]] >= lo) dot += Math.sqrt(q.it[pr[0]] * r.it[pr[1]]);
    return na > 0 && nb > 0 ? dot / Math.sqrt(na * nb) : 0;
  }
  const polTxt = (/** @type {number} */ v) => I18N.t(v > 0 ? "lib.what.pol.pos" : v < 0 ? "lib.what.pol.neg" : "lib.what.pol.unk");
  // the same first InChIKey block (or name) = one structure: the best entry heads the group
  function groupsOf(/** @type {Bag[]} */ rs) {
    const m = new Map(); rs.forEach((/** @type {Bag} */ r, /** @type {number} */ i) => { r._i = i; const k = LIVELLI.ikKey(r); if (!m.has(k)) m.set(k, []); m.get(k).push(r); });
    return [...m.values()];
  }
  async function whatIs(/** @type {Pannello} */ p) {
    ensureCss();
    const q = queryOf(p); if (!q || !q.peaks.length) return info(I18N.t("lib.pickMs2"));
    let d = /** @type {any} */ (document.getElementById("libres"));
    if (!d) { d = document.createElement("dialog"); d.id = "libres"; document.body.appendChild(d); }
    const prof = profOf(q.f), top = q.peaks.reduce((/** @type {number} */ x, /** @type {number[]} */ y) => Math.max(x, y[0]), 0);
    /** @type {any} */
    const st = { prec: q.prec != null ? +q.prec.toFixed(4) : null, v: prof.v, frag: Math.max(0.005, +(10e-6 * top).toFixed(4)), res: null, an: null, sel: { k: "id", i: 0 }, n: 1, lo: null, decided: "", open: new Set() };
    d.innerHTML = `<div class="top" style="display:flex;align-items:center"><h3 style="margin:0;flex:1">${I18N.t("lib.what.title")}</h3><button class="x" id="lr-x" title="${I18N.t("common.close")}">&times;</button></div>
      <div class="sm muted" id="lr-q" style="margin-top:4px">${EHt(q.title)}</div>
      <div class="qbar"><label>${I18N.t("lib.search.prec")} <input id="lr-prec" value="${st.prec ?? ""}"></label><label>${I18N.t("lib.search.tol")} <input id="lr-tol" value="${st.v}"> ppm</label>
        <label>${I18N.t("lib.search.peaksWithin")} <input id="lr-frag" value="${st.frag}"> Da</label><button id="lr-go">${I18N.t("lib.search.go")}</button><button id="lr-copy" title="${I18N.t("lib.copy.title")}">${I18N.t("lib.copy")}</button><button id="lr-xlsx">Excel</button><button id="lr-libs">${I18N.t("lib.libs")}</button><span class="sm muted" id="lr-msg"></span></div>
      <div class="qbar">${thresholdsHtml()}</div><div id="lr-verdict"></div><div id="lr-body"></div><canvas id="lr-cv" hidden></canvas><div id="lr-detail" class="sm"></div>`;
    d.querySelector("#lr-x").onclick = () => d.close(); d.querySelector("#lr-libs").onclick = () => { open(); };
    const msg = (/** @type {string | undefined} */ t) => { d.querySelector("#lr-msg").textContent = t || ""; };
    const sel = () => { const A = st.sel.k === "an" ? st.an : st.res; return A && A.results[st.sel.i] ? A.results[st.sel.i] : null; };
    const dp = (/** @type {Bag} */ r) => (r.dprec >= 0 ? "+" : "") + r.dprec.toFixed(1) + " ppm";
    const pct = (/** @type {number | null | undefined} */ v) => (v == null ? "" : Math.round(v * 100) + "%");
    const idRow = (/** @type {Bag} */ r, /** @type {number} */ g, /** @type {number} */ k) => `<tr class="row${st.sel.k === "id" && st.sel.i === r._i ? " sel" : ""}${k ? " mem" : ""}" data-k="id" data-i="${r._i}"${k && !st.open.has(g) ? " hidden" : ""}><td class="num">${k ? "" : r._i + 1}</td>
      <td>${k ? "&nbsp;&nbsp;↳ " : ""}${EHt(r.name)}${!k && st.groups[g].length > 1 ? ` <button type="button" class="sm" data-grp="${g}" aria-expanded="${st.open.has(g)}" title="${EHt(I18N.t("lib.what.group", { n: st.groups[g].length - 1 }))}">${st.open.has(g) ? "▾" : "▸"} ${I18N.t("lib.what.group", { n: st.groups[g].length - 1 })}</button>` : ""}</td>
      <td class="num">${r.ent.toFixed(3)}</td><td class="num">${r.reverse != null ? r.reverse.toFixed(3) : ""}</td><td class="num">${r.shared}</td><td class="num">${pct(r.explained)}</td><td class="num">${dp(r)}</td><td>${EHt(r.type)}</td><td>${polTxt(r.polarity)}</td><td>${EHt(r.ce)}</td><td>${EHt(r.inst)}</td><td>${EHt(r.lib)}</td><td>${EHt(r.license || "")}</td></tr>`;
    const show = () => {
      const R = st.res, A = st.an, body = d.querySelector("#lr-body"), cv = d.querySelector("#lr-cv"), vd = d.querySelector("#lr-verdict");
      vd.innerHTML = "";
      if (!R) { body.innerHTML = ""; cv.hidden = true; d.querySelector("#lr-detail").innerHTML = ""; return; }
      if (!R.libs) { body.innerHTML = `<p>${I18N.t("lib.loadFirst")}</p><p><button id="lr-open">${I18N.t("lib.openLibs")}</button></p>`; body.querySelector("#lr-open").onclick = () => open(); cv.hidden = true; return; }
      st.groups = groupsOf(R.results);
      st.lv = assess(R.results, { nScans: st.n, analogs: A && A.results }); if (st.decided) LIVELLI.decide(st.lv, st.decided);
      const cvs = st.sel.k === "id" && R.results[st.sel.i] && st.lo != null ? cosVis({ it: R.query.it }, R.results[st.sel.i], st.lo) : null;
      // a proof only: it never changes the level
      if (cvs != null) st.lv.lines = st.lv.lines.concat([I18N.t("lib.what.cosVis", { lo: st.lo, cos: cvs.toFixed(2) })]);
      window.RAPPORTO && RAPPORTO.note(st.lv, q); vd.innerHTML = levelHtml(st.lv); levelBind(vd, st.lv, () => { st.decided = st.lv.decided || ""; show(); });
      body.innerHTML = `<h4 style="margin:8px 0 2px">${I18N.t("lib.what.cand")}</h4>` + (R.results.length ? `<div class="tbl"><table><tr><th class="num">#</th><th>${I18N.t("lib.col.name")}</th><th class="num">${I18N.t("lib.col.entropy")}</th><th class="num">${I18N.t("lib.col.reverse")}</th><th class="num">${I18N.t("lib.col.shared")}</th><th class="num">${I18N.t("lib.col.explained")}</th><th class="num">${I18N.t("lib.col.dprecHtml")}</th><th>${I18N.t("lib.col.adduct")}</th><th>${I18N.t("lib.col.pol")}</th><th>CE</th><th>${I18N.t("lib.col.instrument")}</th><th>${I18N.t("lib.col.library")}</th><th>${I18N.t("lib.col.license")}</th></tr>
        ${st.groups.map((/** @type {any[]} */ g, /** @type {number} */ gi) => g.map((/** @type {Bag} */ r, /** @type {number} */ k) => idRow(r, gi, k)).join("")).join("")}</table></div>` : `<p>${I18N.t("lib.what.none")}</p>`)
        + `<h4 style="margin:10px 0 2px">${I18N.t("lib.what.an")}</h4>` + (!A ? `<p class="muted">${I18N.t("lib.an.searching")}</p>` : !A.results.length ? `<p>${I18N.t("lib.an.none")}</p>` : `<div class="tbl"><table><tr><th class="num">#</th><th>${I18N.t("lib.an.name")}</th><th class="num">&Delta;m</th><th>${I18N.t("lib.an.interp")}</th><th class="num">${I18N.t("lib.an.mcos")}</th><th class="num">${I18N.t("lib.an.matched")}</th><th>${I18N.t("lib.col.library")}</th></tr>
        ${A.results.map((/** @type {Bag} */ r, /** @type {number} */ i) => `<tr class="row${st.sel.k === "an" && st.sel.i === i ? " sel" : ""}" data-k="an" data-i="${i}"><td class="num">${i + 1}</td><td>${EHt(r.name)}</td><td class="num">${sgnTxt(r.delta)}</td><td data-dm="${i}">${EHt(deltaKnown(r.delta) || "…")}</td><td class="num">${r.mcos.toFixed(2)}</td><td class="num">${I18N.t("lib.an.shifted", { matched: r.matched, shifted: r.shifted })}</td><td>${EHt(r.lib)}</td></tr>`).join("")}</table></div><div class="sm muted" style="margin-top:4px">${I18N.t("lib.an.note")}</div>`);
      body.querySelectorAll("tr.row").forEach((/** @type {HTMLElement} */ tr) => tr.onclick = (/** @type {PointerEvent} */ e) => { if (/** @type {HTMLElement} */ (e.target).closest("[data-grp]")) return; st.sel = { k: tr.dataset.k, i: Number(tr.dataset.i) }; body.querySelectorAll("tr.row").forEach((/** @type {Bag} */ x) => x.classList.toggle("sel", x === tr)); draw(); });
      body.querySelectorAll("[data-grp]").forEach((/** @type {HTMLElement} */ b) => b.onclick = () => { const g = Number(b.dataset.grp); if (st.open.has(g)) st.open.delete(g); else st.open.add(g); show(); });
      if (A) A.results.forEach(async (/** @type {Bag} */ r, /** @type {number} */ i) => { if (!deltaKnown(r.delta)) { const t = await deltaFormula(r.delta); const c = body.querySelector(`[data-dm="${i}"]`); if (c) c.textContent = t || I18N.t("lib.an.noFormula"); } });
      cv.hidden = !sel(); draw();
    };
    const draw = () => {
      const r = sel(), cv = d.querySelector("#lr-cv"), det = d.querySelector("#lr-detail"); if (!r) { det.innerHTML = ""; return; }
      const q2 = st.sel.k === "an" ? { mz: r.qa.mz, it: r.qa.it } : { mz: st.res.query.mz, it: st.res.query.it };
      mirror(cv, { ...q2, hr: true, prec: st.prec, frag: st.frag }, st.sel.k === "an" ? { mz: r.la.mz, it: r.la.it, prec: r.prec, name: r.name, shared: r.matched, pairs: r.pairs } : r);
      const /** @type {number[]} */ nls = [], ps = st.sel.k === "an" ? r.pairs : (r.pairs || []), lm = st.sel.k === "an" ? r.la.mz : r.mz;
      JSON.parse(cv.dataset.nl || "[]").forEach((/** @type {[number]} */ [i]) => nls.push(+(st.prec - q2.mz[i]).toFixed(4)));
      ps.forEach((/** @type {any[]} */ pr) => { const v = +(st.prec - q2.mz[pr[0]]).toFixed(4); if (v > 0.5 && !nls.includes(v)) nls.push(v); });
      const attr = [r.authors, r.accession, r.license].some(x => x) ? `<div>${I18N.t("lib.what.attr", { authors: EHt(r.authors || "–"), accession: EHt(r.accession || "–"), license: EHt(r.license || "–") })}</div>` : "";
      det.innerHTML = `<div class="muted">${I18N.t("lib.what.legend")}</div><div>${nls.length ? I18N.t("lib.what.nl", { list: nls.sort((a, b) => a - b).map(v => v.toFixed(4)).join(", ") }) : I18N.t("lib.what.nlNone")}</div>${attr}${pairsTable(ps, q2, st.sel.k === "an" ? r.la : r, st.sel.k === "an")}`;
    };
    const run = async () => {
      const num = (/** @type {string} */ id) => numIt(String(d.querySelector(id).value));
      st.prec = num("#lr-prec"); st.v = num("#lr-tol"); st.frag = num("#lr-frag");
      if (!(st.prec > 0) || !(st.v > 0) || !(st.frag > 0)) return msg(I18N.t("lib.checkTol"));
      msg(I18N.t("lib.searching")); const t0 = performance.now(); st.an = null; st.res = null; st.sel = { k: "id", i: 0 };
      try {
        const [a, b] = await Promise.all([call("search", { peaks: q.peaks, prec: st.prec, pol: q.pol, tol: { v: st.v, unit: "ppm" }, frag: st.frag }), call("analogs", { peaks: q.peaks, prec: st.prec, pol: q.pol, frag: st.frag, maxDelta: 200, max: 10 })]);
        st.res = a; st.an = b; msg(a.libs ? I18N.t("lib.results", { n: a.results.length, ms: Math.round(performance.now() - t0) }) : "");
      } catch (e_) { const e = /** @type {Lanciato} */ (e_); st.res = null; msg(I18N.t("lib.error", { message: e.message })); }
      show();
    };
    thresholdsBind(d, show);
    d.querySelector("#lr-go").onclick = run;
    d.querySelectorAll(".qbar input").forEach((/** @type {HTMLInputElement} */ i) => i.onkeydown = (/** @type {KeyboardEvent} */ e) => { if (e.key === "Enter") run(); });
    const lvTxt = () => (st.lv ? [st.lv.proposed, st.lv.decided || ""] : ["", ""]);
    const head = [ "#", I18N.t("lib.col.name"), I18N.t("lib.col.entropy"), I18N.t("lib.col.reverse"), I18N.t("lib.col.shared"), I18N.t("lib.col.explained"), I18N.t("lib.col.dprec") + " (ppm)", I18N.t("lib.col.adduct"), I18N.t("lib.col.pol"), "CE", I18N.t("lib.col.instrument"), I18N.t("lib.col.library"), I18N.t("lib.col.license"), "InChIKey", I18N.t("lv.proposed"), I18N.t("lv.decided")];
    const rowsOf = () => { const [pr, de] = lvTxt(); return st.res ? st.res.results.map((/** @type {Bag} */ r, /** @type {number} */ i) => [i + 1, r.name, r.ent, r.reverse ?? "", r.shared, r.explained ?? "", r.dprec, r.type, polTxt(r.polarity), r.ce, r.inst, r.lib, r.license || "", r.ik || "", i === 0 ? pr : "", i === 0 ? de : ""]) : []; };
    d.querySelector("#lr-copy").onclick = async () => {
      const rs = rowsOf(); if (!rs.length) return msg(I18N.t("lib.nothingCopy"));
      const f = (/** @type {unknown} */ v) => typeof v === "number" ? String(+v.toFixed(3)) : String(v ?? "");
      const tsv = await PROV.tsv(head.join("\t") + "\n" + rs.map((/** @type {Bag} */ r) => r.map(f).join("\t")).join("\n"), { p, table: "library-analogs" });
      const html = `<table><tr>${head.map(x => `<th>${EHt(x)}</th>`).join("")}</tr>${rs.map((/** @type {Bag} */ r) => `<tr>${r.map((/** @type {Bag} */ v) => `<td>${EHt(f(v))}</td>`).join("")}</tr>`).join("")}</table>`;
      try { if (window.ClipboardItem && navigator.clipboard.write) await navigator.clipboard.write([new ClipboardItem({ "text/plain": new Blob([tsv], { type: "text/plain" }), "text/html": new Blob([html], { type: "text/html" }) })]); else await navigator.clipboard.writeText(tsv); msg(I18N.t("lib.copied", { n: rs.length })); }
      catch (e) { try { await navigator.clipboard.writeText(tsv); msg(I18N.t("lib.copied", { n: rs.length })); } catch (e2) { msg(I18N.t("lib.noCopy")); } }
    };
    d.querySelector("#lr-xlsx").onclick = () => {
      const rs = rowsOf(); if (!rs.length) return msg(I18N.t("lib.nothingSave"));
      const an = st.an ? st.an.results.map((/** @type {Bag} */ r, /** @type {number} */ i) => [i + 1, r.name, +r.delta.toFixed(4), deltaKnown(r.delta) || "", r.mcos, r.matched, r.shifted, r.lib]) : [];
      dlx((typeof plotName === "function" ? plotName(p) : I18N.t("lib.xlsx.fallback")) + I18N.t("lib.xlsx.suffix"), [{ name: I18N.t("lib.xlsx.sheet"), head, rows: rs, widths: [5, 30, 10, 10, 10, 12, 14, 12, 10, 8, 14, 20, 14, 28, 10, 10] },
        { name: I18N.t("lib.what.sheetAn"), head: ["#", I18N.t("lib.an.name"), "Δm", I18N.t("lib.an.interp"), I18N.t("lib.an.mcos"), I18N.t("lib.an.matched"), I18N.t("lib.an.shifted", { matched: "", shifted: "" }).replace(/[()]/g, "").trim() || "shifted", I18N.t("lib.col.library")], rows: an, widths: [5, 30, 10, 24, 12, 12, 10, 20] }], { p, table: "library-analogs" });
    };
    d.showModal(); window.addEventListener("resize", draw, { once: true });
    scansOf(p).then(n => { if (n !== st.n) { st.n = n; if (st.res) show(); } });
    if (p.k != null) J(`api/method?k=${p.k}`).then((/** @type {Bag} */ f) => { const sp = f && f.scan_params, lo = sp && sp.ms2 && sp.ms2.window_lo; if (lo > 0) { st.lo = lo; if (st.res) show(); } }).catch(() => {});
    if (st.prec) run(); else msg(I18N.t("lib.typePrec"));
  }
  // ------------------------------------------------------------ «Identifica tutte le MS2 del file» (WP-H8): every MS2 of a high-resolution file against the libraries
  // the MS2 of each scan are read in blocks of 60 (api/scanbin); the worker answers with the best library entry of each; the scans of one precursor (10 ppm) and one peak (RT within 0.2 min) are grouped
  const phys = (/** @type {Bag} */ f) => (f && f.file ? f.file.split("#")[0] : "");
  function groupRows(/** @type {Bag[]} */ rows, ppm = 10, dRt = 0.2) {
    const R = rows.filter((/** @type {Bag} */ r) => r.prec).sort((/** @type {Bag} */ a, /** @type {Bag} */ b) => a.prec - b.prec), byPrec = [];
    for (const r of R) { const g = byPrec[byPrec.length - 1]; if (g && Math.abs(r.prec - g[g.length - 1].prec) <= r.prec * ppm * 1e-6) g.push(r); else byPrec.push([r]); }
    /** @type {Bag[]} */ const out = [];
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
  // the level of a group of scans of one precursor: its best candidate, the best one of ANOTHER structure (`alt`, from the worker) and the number of scans
  function groupAssess(/** @type {Bag} */ g) {
    const b = g.best && g.best.name ? g.best : null;
    return assess(b ? [b, b.alt].filter(Boolean) : [], { nScans: g.n });
  }
  const lvCell = (/** @type {Bag} */ res) => { const lv = LIVELLI.shown(res); return `<span class="lvb ${LIVELLI.cls(lv)}" title="${EHt(LIVELLI.sentence(lv) + " · " + res.decisive)}">${EHt(LIVELLI.tag(lv))}</span>`; };
  async function identifyAll(/** @type {number} */ k) {
    ensureCss(); await carica("m:libreria-risultati.js");
    const f = E.files[k]; if (!f) return;
    const k2 = (E.files.find((/** @type {Bag} */ x) => phys(x) === phys(f) && x.kind === "ms2" && !x.gone) || {}).k;
    if (k2 == null) return info(I18N.t("lib.noMs2"));
    let d = /** @type {any} */ (document.getElementById("libid"));
    if (!d) { d = document.createElement("dialog"); d.id = "libid"; document.body.appendChild(d); }
    d.innerHTML = `<div class="top" style="display:flex;align-items:center"><h3 style="margin:0;flex:1">${I18N.t("lib.all.title")}</h3><button class="x" id="li-x" title="${I18N.t("common.close")}">&times;</button></div>
      <div class="sm muted" style="margin-top:4px">${EHt(f.label || f.file)} · ${I18N.t("lib.all.sub", { ppm: profOf(f).v })}</div>
      <div id="li-st" style="margin:8px 0"></div><div id="li-body"></div>`;
    d.querySelector("#li-x").onclick = () => d.close(); LIBR.frame(d, { max: I18N.t("lib.all.max"), restore: I18N.t("lib.all.restore") }); d.showModal();
    const st = d.querySelector("#li-st"), say = (/** @type {string} */ t, /** @type {number} */ v, /** @type {number} */ m) => { st.innerHTML = `<div class="sm">${EHt(t)}</div>${m ? `<progress max="${m}" value="${v}"></progress>` : ""}`; };
    try {
      const libs = await call("list"); if (!libs.libs.length) { st.innerHTML = `<p>${I18N.t("lib.loadFirst")}</p><p><button id="li-open">${I18N.t("lib.openLibs")}</button></p>`; d.querySelector("#li-open").onclick = () => { d.close(); open(); }; return; }
      say(I18N.t("lib.all.reading"), 0, 1);
      const dda = await J(`api/dda?k=${k2}`), tgt = new Map(dda.sid.map((/** @type {string | number} */ s, /** @type {number} */ i) => [s, dda.tgt[i]])), pol = f.polarity === "negative" ? -1 : 1, /** @type {Bag[]} */ queries = [];
      let n = 1, i0 = 0;
      for (; i0 < n; i0 += 60) {
        const b = typeof scBin === "function" ? await scBin(`api/scanbin?k=${k2}&i0=${i0}&i1=${i0 + 59}&level=2`) : await J(`api/spectra?k=${k2}&i0=${i0}&i1=${i0 + 59}&level=2`); n = b.n;
        for (const s of b.scans) { const pr = tgt.get(s.sid); if (pr && s.mz.length) queries.push({ key: s.sid, rt: s.rt, prec: pr, pol, peaks: s.mz.map((/** @type {number} */ m, /** @type {number} */ j) => [m, s.y[j]]) }); }
        say(I18N.t("lib.all.readingN", { i: Math.min(i0 + 60, n), n }), Math.min(i0 + 60, n), n);
      }
      const t0 = performance.now();
      say(I18N.t("lib.all.comparing"), 0, queries.length);
      const r = await call("searchAll", { queries, tol: { v: profOf(f).v, unit: "ppm" }, frag: 0.01 }, (/** @type {Bag} */ m) => say(I18N.t("lib.all.comparingN", { i: m.read, n: m.total }), m.read, m.total));
      if (window.LISTE) await LISTE.ready();
      // contaminant exclusions (liste.js): candidates whose precursor is a contaminant the student excluded stay out of the list, counted, until «Mostra»
      let showHid = false;
      const groups = groupRows(r.rows), show = (/** @type {boolean} */ all) => {
        const hidOf = (/** @type {Bag} */ g) => { const c = window.LISTE ? LISTE.compat(g.prec, f.polarity, UIP.hrPpm) : null; return !!(c && c.hid); };
        const base = groups.map(g => ({ g, v: groupAssess(g), k: window.LISTE ? LISTE.textFor(g.prec, f.polarity, UIP.hrPpm) : "", hid: hidOf(g) })).filter(x => all || (x.v.proposed !== "5" && x.v.proposed !== "0"));
        const nHid = base.filter(x => x.hid).length, rows = showHid ? base : base.filter(x => !x.hid), hasK = rows.some(x => x.k);
        d.querySelector("#li-body").innerHTML = (nHid ? `<div class="sm cont-cnt" role="status">${I18N.t(showHid ? "cont.cnt.shown" : "cont.cnt", { n: nHid })} · <button type="button" id="li-chid">${I18N.t(showHid ? "cont.hide" : "cont.show")}</button></div>` : "") + `<div class="bar"><button id="li-xlsx" type="button">${typeof IC_DL !== "undefined" ? IC_DL : ""}Excel</button><button id="li-mgf" type="button">${I18N.t("lib.all.exportMgf")}</button><button id="li-msp" type="button">${I18N.t("lib.all.exportMsp")}</button></div><label class="sm"><input type="checkbox" id="li-all"${all ? " checked" : ""}> ${I18N.t("lib.all.showAll", { n: groups.length })}</label>
          <div id="li-tbl" style="flex:1 1 auto;min-height:0;display:flex;flex-direction:column"></div>`;
        LIBR.fill(d.querySelector("#li-tbl"), [{ h: "RT", w: "7ch", num: true }, { h: "<i>m/z</i>", w: "11ch", num: true }, { h: I18N.t("lib.col.name"), nm: true }, { h: I18N.t("lst.col.formula"), w: "15ch" }, { h: I18N.t("lib.all.bestEntropy"), w: "15ch", num: true }, { h: I18N.t("lib.all.scans"), w: "14ch", num: true }, { h: I18N.t("lv.proposed"), w: "17ch" }, ...(hasK ? [{ h: I18N.t("lib.all.contaminant"), w: "18ch" }] : [])],
          rows.map((x, i) => ({ i, cells: [x.g.rt.toFixed(2), x.g.prec.toFixed(4), EHt(x.g.best.name || ""), x.g.best.formula ? fmtFormula(x.g.best.formula) : "", x.g.best.ent != null ? x.g.best.ent.toFixed(2) : "", `${x.g.n}${x.g.n < 2 ? " " + I18N.t("lib.all.oneScan") : ""}`, lvCell(x.v), ...(hasK ? [EHt(x.k || "")] : [])] })), I18N.t("lib.all.noMatch"), I18N.t("lib.all.openBest"));
        d.querySelector("#li-all").onchange = (/** @type {Event} */ e) => show(/** @type {HTMLInputElement} */ (e.target).checked);
        const chid = d.querySelector("#li-chid"); if (chid) chid.onclick = () => { showHid = !showHid; show(all); };
        const btnMgf = d.querySelector("#li-mgf");
        if (btnMgf) btnMgf.onclick = () => { if (window.WORKFLOW) WORKFLOW.exportQueriesMgf(queries, f); };
        const btnMsp = d.querySelector("#li-msp");
        if (btnMsp) btnMsp.onclick = () => { if (window.WORKFLOW) WORKFLOW.exportQueriesMsp(queries, f); };
        d.querySelector("#li-xlsx").onclick = () => dlx(I18N.t("lib.all.xlsxPrefix") + String(f.label || f.file || "file").replace(/[^\w.-]+/g, "_") + ".xlsx", [
          { name: I18N.t("lib.all.sheetCand"), head: ["RT (min)", I18N.t("lib.all.precMz"), I18N.t("lib.col.name"), I18N.t("lst.col.formula"), I18N.t("lib.all.bestEntropy"), I18N.t("lib.all.scans"), I18N.t("lv.proposed"), I18N.t("lv.decided"), I18N.t("lv.evidence")].concat(hasK ? [I18N.t("lib.all.contaminant")] : []), rows: rows.map(x => [+x.g.rt.toFixed(3), +x.g.prec.toFixed(5), x.g.best.name || "", x.g.best.formula || "", x.g.best.ent != null ? +x.g.best.ent.toFixed(3) : "", x.g.n, x.v.proposed, x.v.decided || "", x.v.lines.concat([x.v.decisive]).join(" | ")].concat(hasK ? [x.k || ""] : [])), widths: [10, 14, 40, 20, 14, 10, 12, 12, 60] },
          { name: I18N.t("lib.all.sheetPar"), head: [I18N.t("lib.all.parameter"), I18N.t("lib.all.value")], rows: [[I18N.t("lib.table.file"), f.label || f.file || ""], [I18N.t("lib.all.precTol"), `${profOf(f).v} ppm`], [I18N.t("lib.all.peakTol"), I18N.t("lib.all.peakTolVal")], [I18N.t("lib.all.shown"), I18N.t("lib.all.shownVal", { n: rows.length, total: groups.length })], [I18N.t("lib.all.level"), I18N.t("lib.all.levelVal")]], widths: [28, 70] }], { table: "library-all", file: String(f.label || f.file || "") });
        d.querySelector("#li-tbl").onclick = (/** @type {MouseEvent} */ e) => { const tr = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest("tr.lr")); if (tr) gotoScan(rows[Number(tr.dataset.i)].g.best.key, d); };
        st.innerHTML = `<div class="sm">${I18N.t("lib.all.summary", { q: queries.length, ms: Math.round(performance.now() - t0), g: groups.length, r: rows.length })}</div>`;
      };
      show(false);
    } catch (e_) { const e = /** @type {Lanciato} */ (e_); st.innerHTML = h`<p class="err">${e.message}</p>`; }
  }
  function gotoScan(/** @type {string | number} */ sid, /** @type {HTMLDialogElement} */ d) {                         // the MS2 panel of the DDA trio shows that scan
    try {
      const s1 = E.panels.find((/** @type {Bag} */ q) => q.type === "spec" && q.duo != null && q.el), s2 = s1 && E.panels.find((/** @type {Bag} */ q) => q.id === s1.duo && q.el);
      if (!s2 || !window.DDA) return toast(I18N.t("lib.all.needDda"));
      d.close(); DDA.selectMs2(s2, sid); s2.el.scrollIntoView({ block: "center", behavior: "smooth" });
    } catch (e) { toast(I18N.t("lib.all.cantOpen")); }
  }
  // the action of the spectrum menu (registry azioni/1): «What is it?» (high resolution) or «Search the libraries» of an MS2 spectrum
  if (typeof AZIONI !== "undefined") AZIONI.registra({ id: "spec.lib", gruppo: "lib", ordine: 1, viste: ["spec"], visibile: (/** @type {Bag} */ c) => c.p.level === 2,
    nome: (/** @type {Bag} */ c) => I18N.t(isHr(c.p) ? "ctx.libWhat" : "ctx.libSearch"), esegui: (/** @type {Bag} */ c) => isHr(c.p) ? whatIs(c.p) : searchFrom(c.p) });
  // the chromatogram menu: the MS2 spectra of a high-resolution file against the libraries
  if (typeof AZIONI !== "undefined") AZIONI.registra({ id: "chrom.identAll", gruppo: "lib", viste: ["chrom"], nome: () => I18N.t("ctx.chrom.identAll"),
    visibile: (/** @type {Bag} */ c) => !!window.HR && HR.isHr(E.files[c.k], 2) && E.files.some((/** @type {Bag} */ x) => x.file.split("#")[0] === E.files[c.k].file.split("#")[0] && x.kind === ESP.ms2), esegui: (/** @type {Bag} */ c) => identifyAll(c.k) });
  return { open, addFiles, queryOf, profOf, searchFrom, whatIs, isHr, mirror, cosVis, call, identifyAll, groupRows, groupAssess, assess, scansOf, engine };
})();
window.LIB = LIB;
