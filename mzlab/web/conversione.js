// @ts-strict
// @ts-noany
"use strict";
// Thermo .raw and SCIEX .wiff -> mzML (index.html rawToMzml, wiffToMzml): the progress of the conversion and the download of the converted files. The DOM part first (only runs in the page), the pure helpers
// below (also run by node in tests/test_conversione.py). Classic script, loaded after i18n.js.

// the mzML files made in this page: mzML name -> Blob (with .mzlabId in the desktop app); they are what the program reads, so a saved copy opens without converting again
const CONV = {
  files: new Map(),
  compression: "standard", // «Download .mzML»: "standard" (zlib), "smallest" (zlib and MS-Numpress) or "none"; the same names as `convert --compression`
  // rough size of the mzML as a fraction of the maker's file, by kind of compression (shown as an estimate before the conversion; the real size is shown after)
  RATIO: /** @type {Record<string, number>} */ ({ standard: 1, smallest: 0.55, none: 1.6 }),
  key(/** @type {string} */ name) { return this.compression + ":" + name; },
  // the estimate of the mzML files still to be written, "" when there is nothing to write
  estimate() {
    const todo = (typeof ST !== "undefined" && ST.files || []).filter((/** @type {Bag} */ x) => this.native().includes(x.name) && !this.made.has(this.key(x.name)));
    const total = todo.reduce((/** @type {number} */ a, /** @type {Bag} */ x) => a + (x.size || 0), 0) * (this.RATIO[this.compression] || 1);
    return total ? "≈ " + (total > 1e6 ? (total / 1e6).toFixed(1) + " MB" : Math.max(1, Math.round(total / 1e3)) + " kB") : "";
  },
  // the tooltip of each «Download mzML» button of the table with the real size of the file written for the compression now selected (in place: the table is not drawn again)
  paintSizes() {
    document.querySelectorAll("#flist [data-dl]").forEach((/** @type {HTMLElement} */ b) => { const s = this.realSize(b.dataset.dl || ""); b.title = I18N.t("conv.dl.title") + (s ? " (" + s + ")" : ""); });
  },
  // the real size of the mzML already written for `name` (in the desktop app the file stays on the disk and its size comes from the native side)
  realSize(/** @type {string} */ name) {
    const b = this.files.get(name) || this.made.get(this.key(name));
    const n = b ? b.size || /** @type {any} */ (b).mzlabSize || 0 : 0;
    return n ? (n > 1e6 ? (n / 1e6).toFixed(1) + " MB" : Math.max(1, Math.round(n / 1e3)) + " kB") : "";
  },
  made: new Map(),        // name -> mzML written on request for a sample of a maker's file (nativo.js)
  add(/** @type {string} */ name, /** @type {Blob} */ blob) { this.files.set(name, blob); },
  // the samples of the maker's files of this page: their mzML is written when it is asked for
  native() { const rb = typeof window !== "undefined" && window.MZLAB_RUST; return rb && rb.nativeNames ? rb.nativeNames() : []; },
  has(/** @type {string} */ name) { return this.files.has(name) || this.native().includes(name); },
  get size() { return new Set([...this.files.keys(), ...this.native()]).size; },
  // the mzML of a sample of a maker's file, written now into the box of the loading (a panel with the scans done and the cancel button); null if it was cancelled or failed
  async make(/** @type {string} */ name) {
    const rb = window.MZLAB_RUST, box = document.getElementById("up");
    if (!rb || !rb.nativeToMzml || !rb.isNative(name) || !box) return null;
    const f = (typeof ST !== "undefined" && ST.files || []).find((/** @type {Bag} */ x) => x.name === name), g = await ConvUI.card(box, [1]), ui = g.start(1, name);
    try { const blob = await rb.nativeToMzml(name, f ? f.size : 0, (/** @type {Bag} */ p) => ui.update(p), g.signal, this.compression); ui.done(); g.end(); this.made.set(this.key(name), blob); this.paintSizes(); return blob; }
    catch (e_) { const e = /** @type {Lanciato} */ (e_); if (e.cancelled) { ui.done(); g.end(); } else { ui.fail(e.json ? I18N.err(e.json).message : e.message); g.end(); } return null; }
  },
  // saves one: the system dialog in the desktop app, a normal download in the browser; resolves to false if the user cancelled or could not save
  async download(/** @type {string} */ name) {
    let blob = this.files.get(name) || this.made.get(this.key(name));
    if (!blob && this.native().includes(name)) blob = await this.make(name);
    if (!blob) return false;
    if (window.MZLAB_DESKTOP) {
      if (blob.mzlabId == null || !/** @type {any} */ (window).MZLAB_SAVE_MZML) return false;
      try { const ok = !!(await /** @type {any} */ (window).MZLAB_SAVE_MZML(blob.mzlabId, name)); return ok; }
      catch (e) { const n = document.getElementById("up"); if (n) { const d = document.createElement("div"); d.className = "note"; d.textContent = I18N.t("conv.dl.needsApp"); n.appendChild(d); setTimeout(() => d.remove(), 12000); } return false; }
    }
    saveBlob(blob, name, 60000);
    return true;
  },
  // all of them, one after the other (a pause between browser downloads, a dialog each in the desktop app)
  // the icon of the download buttons (the label is the aria-label)
  ICON: '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M8 2v8M4.5 7 8 10.5 11.5 7M3 13.5h10"/></svg>',
  // the small menu of the download icons: the two formats the program writes (plain mzML, mzML with MS-Numpress); the compression of the advanced options follows the pick. `name` null = all the files
  menu(/** @type {HTMLElement} */ anchor, /** @type {string | null} */ name) {
    document.querySelector(".dlmenu")?.remove();
    const m = document.createElement("div"); m.className = "dlmenu"; m.setAttribute("role", "menu"); m.setAttribute("aria-label", I18N.t(name == null ? "conv.dlall" : "conv.dl"));
    const close = (/** @type {boolean} */ back) => { m.remove(); document.removeEventListener("pointerdown", away, true); anchor.setAttribute("aria-expanded", "false"); if (back) anchor.focus(); };
    const away = (/** @type {Event} */ e) => { if (!m.contains(/** @type {Node} */ (e.target)) && !anchor.contains(/** @type {Node} */ (e.target))) close(false); };
    for (const [k, key] of [["standard", "conv.fmt.mzml"], ["smallest", "conv.fmt.numpress"]]) {
      const b = document.createElement("button"); b.type = "button"; b.className = "sec sm"; b.dataset.dlfmt = k; b.setAttribute("role", "menuitem"); b.textContent = I18N.t(key);
      b.onclick = () => { close(true); this.saveAs(k, name); }; m.append(b);
    }
    m.onkeydown = (/** @type {KeyboardEvent} */ e) => {
      const it = [.../** @type {NodeListOf<HTMLElement>} */ (m.querySelectorAll("button"))], i = it.indexOf(/** @type {HTMLElement} */ (document.activeElement));
      if (e.key === "Escape") { e.preventDefault(); close(true); } else if (e.key === "ArrowDown" || e.key === "ArrowUp") { e.preventDefault(); it[(i + (e.key === "ArrowDown" ? 1 : it.length - 1)) % it.length].focus(); } else if (e.key === "Tab") close(false);
    };
    document.body.append(m); anchor.setAttribute("aria-expanded", "true"); document.addEventListener("pointerdown", away, true);
    const r = anchor.getBoundingClientRect(), w = m.offsetWidth;
    m.style.left = Math.max(8, Math.min(r.left, innerWidth - w - 8)) + "px"; m.style.top = r.bottom + 4 + "px";
    /** @type {HTMLElement} */ (m.firstElementChild).focus();
  },
  // saves in the format picked in the menu (the advanced select and the estimate follow)
  async saveAs(/** @type {string} */ fmt, /** @type {string | null} */ name) {
    this.compression = fmt; const s = /** @type {HTMLSelectElement | null} */ (document.getElementById("cvcomp")); if (s) s.value = fmt;
    this.paintSizes(); const e = document.getElementById("cvest"); if (e) e.textContent = this.estimate();
    if (name == null) await this.downloadAll(); else await this.download(name);
  },
  async downloadAll() { for (const n of new Set([...this.files.keys(), ...this.native()])) { await this.download(n); if (!window.MZLAB_DESKTOP) await new Promise(r => setTimeout(r, 400)); } },
};

// desktop app: the mzML files the background queue (conv_coda.rs) makes of the maker's files of the list. State of each file: queued | run | done | failed | cancelled.
const CONVQ = {
  st: /** @type {Map<string, { state: string, error: string }>} */ (new Map()),
  timer: /** @type {any} */ (0),
  on() { return typeof window !== "undefined" && !!window.MZLAB_DESKTOP && !!window.MZLAB_RUST && !!window.MZLAB_RUST.convQueue; },
  /** @param {string} op @param {string} [name] */ async call(op, name) {
    let j = null; try { j = await window.MZLAB_RUST.convQueue(op, name); } catch (_) { /* the queue is optional: the file loads without it */ }
    if (!j) return;
    const prev = [...this.st];
    this.st = new Map(j.jobs.map((/** @type {Bag} */ x) => [x.name, { state: x.state, error: x.error }]));
    const next = [...this.st];
    if (next.length !== prev.length || next.some(([n, v], i) => n !== prev[i][0] || v.state !== prev[i][1].state || v.error !== prev[i][1].error)) CONVROWS.paint();
  },
  // a maker's file was loaded: its mzML is made in the background
  add(/** @type {string} */ name) {
    if (!this.on() || !window.MZLAB_RUST.isNative(name)) return;
    this.call("conv_add", name).then(() => this.poll());
  },
  poll() {
    if (this.timer) return;
    this.timer = setInterval(async () => {
      await this.call("conv_list");
      if (![...this.st.values()].some(x => x.state === "queued" || x.state === "run")) { clearInterval(this.timer); this.timer = 0; }
    }, 1500);
  },
  stop() { return this.call("conv_cancel"); },
  ready() { return [...this.st.values()].some(x => x.state === "done"); },
  // the text beside a file of the list; "" while nothing is known
  label(/** @type {string} */ name) {
    const s = this.st.get(name); if (!s) return "";
    return s.state === "done" ? I18N.t("conv.q.done") : s.state === "failed" ? I18N.t("conv.q.fail", { msg: DIAG.pulisci(s.error, 160) }) : s.state === "cancelled" ? "" : I18N.t(s.state === "run" ? "conv.q.run" : "conv.q.wait");
  },
  // one folder, one click: every ready mzML goes there
  async saveAll() {
    const f = typeof window !== "undefined" && /** @type {any} */ (window).MZLAB_SAVE_MZML_DIR; if (!f) return;
    try { const n = await f(); if (n != null) { const box = document.getElementById("up"); if (box) { const d = document.createElement("div"); d.className = "note"; d.setAttribute("role", "status"); d.textContent = I18N.t("conv.q.saved", { n }); box.appendChild(d); setTimeout(() => d.remove(), 8000); } } }
    catch (e) { const box = document.getElementById("up"); if (box) { const d = document.createElement("div"); d.className = "note"; d.textContent = I18N.t("conv.dl.needsApp"); box.appendChild(d); setTimeout(() => d.remove(), 12000); } }
  },
};

// the files being read, converted and indexed (the samples of a maker's file, the files a module converts), as rows of the list of the files (index.html renderFiles) from the moment
// they are known until the user closes them (the running ones in the table, the ended ones in a list under it): { name, state, pc, sub, t0, seen, ... }. Running states: "open" (pair found, reading its .wiff), "wait" (queued), "run" (converting, pc),
// "conv" (converted, waits for the index), "index" / "nread" / "nopen" / "nscan" / "nchrom" (indexing, pc; sub = {done, total} scans). Final states, which stay: "done" (scans, secs),
// "fail" (msg, diag: DIAG with the trace of the loading, «Details» and «Copy diagnosis»), "cancel". A change of state draws the list again; a new percentage only moves the bar of its
// row. The rows are drawn in the side list
// (upload-rows.js); the work area has no heading nor row of the loading.
const CONV_FINAL = ["done", "fail", "cancel"];
// what the summary counts apart, in this order: the kinds of file (`kind` of the file entry, "mixed" for a file of more parts), then the rows that did not load
const CONV_SUM = /** @type {Record<string, string>} */ ({ full: "conv.sum.full", ms2: "conv.sum.ms2", mrm: "conv.sum.mrm", infusion: "conv.sum.infusion", mixed: "conv.sum.mixed", other: "conv.sum.other", fail: "conv.sum.fail", cancel: "conv.sum.cancel" });
const CONVROWS = {
  /** @type {ConvRow[]} */
  rows: [],
  /** @type {null | (() => void)} */
  onCancel: null,
  timer: /** @type {any} */ (0),
  liveN: "",
  pq: false,
  // order of arrival of the files (name -> number), the order of the rows of the loading table until the user edits a value (thaw)
  pos: /** @type {Map<string, number>} */ (new Map()),
  dirty: /** @type {Set<any>} */ (new Set()),
  tq: /** @type {any} */ (0),
  rank(/** @type {string} */ name) { const k = this.pos.get(name); return k == null ? -1 : k; },
  thaw() { this.pos.clear(); },
  find(/** @type {string} */ name) { return this.rows.find(r => r.name === name); },
  running() { return this.rows.filter(r => !CONV_FINAL.includes(r.state)); },
  // a row; `mem` describes the members without their names (".wiff 1.2 MB + .wiff.scan 57.0 MB", for the diagnosis)
  add(/** @type {string} */ name, /** @type {string} */ state, /** @type {string | undefined} */ mem) {
    const r = this.find(name), now = Date.now();
    if (r && !CONV_FINAL.includes(r.state)) return;
    if (r) this.rows.splice(this.rows.indexOf(r), 1);
    if (!this.pos.has(name)) this.pos.set(name, this.pos.size);
    this.rows.push({ name, state, pc: null, sub: null, t0: now, seen: now, tr: [`0.0 s ${state}`], last: "", mem: mem || "" });
    this.paint(); this.clock();
  },
  // `det` = bytes { done, total } of a download or a copy; `sub` = scans { done, total }. A row of a loading (`base`) never goes back
  set(/** @type {string} */ name, /** @type {string} */ state, /** @type {number | null | undefined} */ pc, /** @type {{ done: number, total: number } | null | undefined} */ sub, /** @type {{ done: number, total: number } | null | undefined} */ det) {
    const r = this.find(name); if (!r || CONV_FINAL.includes(r.state)) return;
    if (r.mono && pc != null) pc = Math.max(pc, r.pc || 0, r.floor || 0);
    const was = r.state, was_pc = r.pc, was_sub = r.sub ? r.sub.done : -1, now = Date.now();
    r.state = state; r.pc = pc == null ? null : pc; if (sub !== undefined) r.sub = sub && sub.total ? sub : null; if (det !== undefined) r.det = det && det.total ? det : null;
    if (was !== state || was_pc !== r.pc || (r.sub ? r.sub.done : -1) !== was_sub || det) r.seen = now;
    if (was !== state) { (r.tr = r.tr || []).push(`${((now - (r.t0 || now)) / 1000).toFixed(1)} s ${state}${r.pc == null ? "" : " " + r.pc + "%"}`); this.fill(r); } else if (r.seen === now) this.queue(r);
  },
  // at most one repaint of the bars every 100 ms: the first change goes at once, the others of the window together at its end
  queue(/** @type {ConvRow} */ r) {
    this.dirty.add(r);
    if (this.tq) return;
    const go = () => { const d = [...this.dirty]; this.dirty.clear(); for (const x of d) this.fill(x); };
    // every window that ends with changes flushes them and opens the next one, so a change is never left waiting for the clock tick
    const arm = () => { this.tq = setTimeout(() => { this.tq = 0; if (this.dirty.size) { go(); arm(); } }, 100); };
    go(); arm();
  },
  // the part of the bar already done (the download of an example file): the next phase starts from here
  base(/** @type {string} */ name, /** @type {number} */ pc) { const r = this.find(name); if (r) { r.floor = pc; r.mono = true; } },
  // the last message of the worker, for the diagnosis
  note(/** @type {string} */ name, /** @type {string | number} */ text) { const r = this.find(name); if (r) r.last = String(text).slice(0, 200); },
  // the end of a row: "done" ({ scans }), "fail" ({ msg, diag }) or "cancel"; it stays until «Close» or the next loading
  finish(/** @type {string} */ name, /** @type {string} */ state, /** @type {Bag | undefined} */ extra) {
    const r = this.find(name); if (!r || CONV_FINAL.includes(r.state)) return;
    const now = Date.now(); r.secs = (now - (r.t0 || now)) / 1000; Object.assign(r, extra || {});
    (r.tr = r.tr || []).push(`${r.secs.toFixed(1)} s ${state}`); r.state = state; r.seen = now;
    if (state === "done") CONVQ.add(name);
    if (r.diag) r.diag.dettagli.trace = this.trace([r]);
    if (this.running().length) this.fill(r); else this.paint();
  },
  drop(/** @type {string} */ name) { const i = this.rows.findIndex(r => r.name === name); if (i >= 0) { this.rows.splice(i, 1); this.paint(); } },
  // the rows that ended go away (a new loading, «Close»)
  clearDone() { const n = this.rows.length; this.rows = this.running(); if (n !== this.rows.length) this.paint(); },
  // the table is drawn again at most once per task, however many rows change together (a whole batch is added in one loop); the rows themselves change in place (fill)
  paint() {
    if (this.pq) return;
    this.pq = true;
    Promise.resolve().then(() => { this.pq = false; this.draw(); });
  },
  draw() { if (typeof renderFiles === "function") renderFiles(); },
  clock() {
    if (this.timer || typeof setInterval === "undefined") return;
    this.timer = setInterval(() => {
      const run = this.running();
      for (const r of run) this.fill(r);
      if (run.length) this.head();
      if (!run.length) { clearInterval(this.timer); this.timer = 0; }
    }, 1000);
  },
  label(/** @type {ConvRow} */ r) {
    if (r.state === "done") return (r.cached ? I18N.t("conv.row.cached", { scans: I18N.fix(r.scans || 0, 0) }) : I18N.t("conv.row.done", { scans: I18N.fix(r.scans || 0, 0), secs: I18N.fix(r.secs || 0, 1) })) + (r.missing ? " · " + I18N.t("conv.row.missing", { n: I18N.fix(r.missing, 0) }) : "");
    if (r.state === "fail") return r.msg || "";
    const T = (/** @type {string} */ k, /** @type {any} */ p) => I18N.t(k, p), mb = (/** @type {number} */ v) => I18N.fix(v / 1e6, 1);
    if ((r.state === "get" || r.state === "load") && r.det) return T(r.state === "get" ? "conv.row.getmb" : "conv.row.loadmb", { done: mb(r.det.done), total: mb(r.det.total) });
    if (r.sub && (r.state === "nscan" || r.state === "index")) return T("conv.row.readn", { done: I18N.fix(r.sub.done, 0), total: I18N.fix(r.sub.total, 0) });
    const lab = T(convRowKey(r.state), { pc: r.pc == null ? 0 : r.pc });
    return r.sub ? `${lab} · ${T("conv.row.scans", { done: I18N.fix(r.sub.done, 0), total: I18N.fix(r.sub.total, 0) })}` : lab;
  },
  // the short state of a row, always the same size: a word for the phase, the percent (or the scans of a finished file) and the time. The long text (MB, scans, «no news for…») is the title and the aria-label
  short(/** @type {ConvRow} */ r) {
    const T = (/** @type {string} */ k) => I18N.t(k);
    if (r.state === "done") return { txt: T(r.cached ? "conv.st.cached" : "conv.st.done"), num: I18N.fix(r.scans || 0, 0) };
    if (r.state === "fail") return { txt: r.msg || T("conv.st.fail"), num: "" };
    if (r.state === "cancel") return { txt: T("conv.st.cancel"), num: "" };
    return { txt: T(convStKey(r.state)), num: r.pc == null ? "" : r.pc + "%" };
  },
  // a new percent of `r`: the side list redraws its row (upload-rows.js wraps this), here only the count and the buttons
  fill(/** @type {ConvRow} */ r) { void r; this.head(); },
  // the count and clock of the side list (upload-rows.js) and the buttons that appear while the loading goes on: they only change what they show, the space is always there
  head() {
    if (typeof document === "undefined") return;
    const h = document.querySelector("#fl-up [data-cvhead]"); if (h && this.running().length) { const t = this.ready(); if (h.textContent !== t) h.textContent = t; }
    const st = this.toolState();
    for (const k of Object.keys(st)) { const e = document.querySelector(`#flist [data-cvtool=${k}]`); if (e) this.show(e, st[k]); }
  },
  show(/** @type {Element} */ e, /** @type {boolean} */ on) { if (on) { e.removeAttribute("data-off"); e.removeAttribute("inert"); } else { e.setAttribute("data-off", ""); e.setAttribute("inert", ""); } },
  toolState() { return /** @type {Record<string, boolean>} */ ({ adv: CONV.native().length > 0, save: CONVQ.ready(), dl: !CONVQ.ready() && CONV.size > 1 }); },
  // the trace of the loading for the diagnosis: no file names (the rows are numbered), the members by extension and size, the phases with their times, the last message of the worker
  trace(/** @type {ConvRow[]} */ rows) {
    const nav = typeof navigator !== "undefined" ? /** @type {any} */ (navigator) : {}, now = Date.now();
    const L = [`device: ${nav.deviceMemory || "?"} GB, ${nav.hardwareConcurrency || "?"} cores${typeof window !== "undefined" && window.MZLAB_STIMA ? ", storage " + window.MZLAB_STIMA : ""}`];
    rows.forEach((/** @type {ConvRow} */ r) => {
      const k = this.rows.indexOf(r) + 1;
      const end = CONV_FINAL.includes(r.state), secs = end ? r.secs || 0 : (now - (r.t0 || now)) / 1000;
      L.push(`#${k || "?"} ${r.state}${r.pc == null ? "" : " " + r.pc + "%"}${r.sub ? ` scans ${r.sub.done}/${r.sub.total}` : ""}${r.scans ? ` scans ${r.scans}` : ""} · ${secs.toFixed(1)} s${end ? "" : ` · quiet ${((now - (r.seen || now)) / 1000).toFixed(1)} s`}${r.mem ? " · " + r.mem : ""}`);
      L.push(`  phases: ${(r.tr || []).join(", ")}`);
      if (r.last) L.push(`  last: ${r.last}`);
    });
    return L.join("\n");
  },
  // the one summary of the loading: «3 of 5 ready · 0:12»; the screen readers hear it (a polite region of its own) only when the count of the ready files changes, never the clock
  ready() {
    const n = this.rows.length, done = n - this.running().length, t0 = minOf(this.rows.map(r => r.t0 || Date.now()));
    if (typeof document !== "undefined" && this.liveN !== done + "/" + n) {
      this.liveN = done + "/" + n;
      let el = document.getElementById("cvlive");
      if (!el) { el = document.createElement("div"); el.id = "cvlive"; el.className = "sr-only"; el.setAttribute("role", "status"); el.setAttribute("aria-live", "polite"); document.body.appendChild(el); }
      el.textContent = I18N.t("load.prog.count", { done, n });
    }
    return I18N.t("conv.row.ready", { done, n, time: convElapsed(Date.now() - t0) });
  },
  // the bar over the table: the buttons of the converted files and the options. It is always there while a table is, so nothing appears or goes away at the phases; what is not
  // needed is invisible (data-off) and cannot be reached
  tools() {
    /** @param {string} k @param {Bag} [p] */ const T = (k, p) => I18N.t(k, p);
    const st = this.toolState(), off = (/** @type {boolean} */ on) => (on ? "" : " data-off inert");
    return `<div class="cvtool"><div class="cvr2"><details class="muted sm" id="advconv" data-cvtool="adv"${off(st.adv)}><summary>${T("conv.adv")}</summary><label>${T("conv.comp")} <select id="cvcomp">${[["standard", T("conv.comp.standard")], ["smallest", T("conv.comp.smallest")], ["none", T("conv.comp.none")]].map(([k, t]) => `<option value="${k}"${CONV.compression === k ? " selected" : ""}>${t}</option>`).join("")}</select></label> <button type="button" class="hq" data-help="compressione" title="${I18N.t("common.help")}" aria-label="${I18N.t("common.help")}">?</button> <span id="cvest" class="muted" aria-live="polite">${CONV.estimate()}</span></details>` +
      `<button type="button" class="sec sm" id="saveall" data-cvtool="save"${off(st.save)} title="${T("conv.q.saveTitle")}">${T("conv.q.save")}</button><button type="button" class="sec sm dlic" id="dlall" data-cvtool="dl"${off(st.dl)} aria-haspopup="menu" aria-label="${T("conv.dlall")}" title="${T("conv.dlall")}">${CONV.ICON}</button><button type="button" class="sec sm" id="rmall" title="${T("load.table.removeAll")}">${T("load.table.removeAll")}</button></div></div>`;
  },
  // the table of the loading: the columns have a fixed width from the first row, so no phase moves them; `head` = the row of headings, `body` = the rows, `conc` = the concentration column is there
  table(/** @type {string} */ head, /** @type {string} */ body, conc = true, extra = 0) {
    return `<div class="ldwrap"><table class="ldtab"><colgroup><col class="c-use"><col><col class="c-exp"><col class="c-typ"><col class="c-tim">${conc ? '<col class="c-con">' : ""}<col class="c-siz">${'<col class="c-x">'.repeat(extra)}<col class="c-act"></colgroup><thead>${head}</thead><tbody>${body}</tbody></table></div>`;
  },
  // the count of the ended rows by kind of file (full scan, MS2, MRM, infusion, mixed, other), the failed, cancelled and truncated ones, and the time from the first start to the last end
  /** @param {ConvRow[]} end */
  totals(end) {
    /** @param {string} k @param {Bag} [p] */ const T = (k, p) => I18N.t(k, p), by = new Map();
    const add = (/** @type {string} */ k) => by.set(k, (by.get(k) || 0) + 1);
    for (const r of end) add(r.state !== "done" ? r.state : /** @type {string} */ (r.kind) in CONV_SUM && r.kind !== "fail" && r.kind !== "cancel" ? /** @type {string} */ (r.kind) : "other");
    const parts = Object.keys(CONV_SUM).filter(k => by.has(k)).map(k => T(CONV_SUM[k], { n: I18N.fix(by.get(k), 0) }));
    const hit = end.filter((/** @type {ConvRow} */ r) => r.cached).length; if (hit) parts.push(T("conv.sum.cached", { n: I18N.fix(hit, 0) }));
    const miss = end.filter((/** @type {ConvRow} */ r) => r.missing).length; if (miss) parts.push(T("conv.sum.missing", { n: I18N.fix(miss, 0) }));
    const t0 = minOf(end.map((/** @type {ConvRow} */ r) => r.t0 || 0)), t1 = maxOf(end.map((/** @type {ConvRow} */ r) => (r.t0 || 0) + 1000 * (r.secs || 0)));
    return T("conv.sum.head", { n: I18N.fix(end.length, 0), parts: parts.join(", "), time: convElapsed(t1 - t0) });
  },
  // under the table only what needs attention: the error with «Details» and «Copy diagnosis», «Cancelled», a file with scans missing; a loading that went well shows nothing; «Close» once nothing runs
  summary() {
    if (this.running().length) return "";
    const all = this.rows.filter(r => CONV_FINAL.includes(r.state)), end = all.filter(r => r.state !== "done" || r.missing); if (!end.length) return "";
    /** @param {string} k @param {Bag} [p] */ const T = (k, p) => I18N.t(k, p);
    const li = end.map(r => {
      const lab = r.state === "cancel" ? T("conv.row.cancel") : this.label(r);
      return `<li data-cvrow="${esc(r.name)}" data-cvend="${r.state}"${r.missing ? ` data-cvmiss="${r.missing}"` : ""}>${esc(r.name)}: ${r.state === "fail" ? `<span class="fail" data-cvtxt>${esc(lab)}</span>${r.diag && typeof DIAG !== "undefined" ? DIAG.html(r.diag, esc, T) : ""}` : `<span data-cvtxt>${esc(lab)}</span>`}</li>`;
    }).join("");
    return `<div class="cvsum" role="group" aria-label="${esc(T("conv.sum.aria"))}"><p class="sm" data-cvtot>${esc(this.totals(all))} <button type="button" class="sec sm" data-cvact="close">${T("common.close")}</button></p><ul class="sm">${li}</ul></div>`;
  },
  // the buttons of the summary, after renderFiles has drawn it in `el`
  wire(/** @type {HTMLElement} */ el) {
    /** @param {string} k @param {Bag} [p] */ const T = (k, p) => I18N.t(k, p), v = typeof window !== "undefined" ? window.MZLAB_VERSION : "";
    const x = el.querySelector("[data-cvact=close]"); if (x) x.onclick = () => this.clearDone();
    this.rows.forEach(r => { if (r.state !== "fail" || !r.diag) return; const li = [...el.querySelectorAll("[data-cvend=fail]")].find(x => /** @type {HTMLElement} */ (x).dataset.cvrow === r.name), dg = li && li.querySelector(".diag"); if (dg) DIAG.cabla(dg, r.diag, v, T); });
  },
};

// the stops of the conversions (CONVROWS rows, × on a side row cancels one file). The panel of a conversion asked from the table («Download .mzML» of a maker's
// sample, CONV.make) is conversione-carta.js, loaded on first use; the conversions of a loading (moduloToMzml) draw nothing here: their progress is the side list's.
const ConvUI = {
  /** @type {Map<string, AbortController>} */
  stops: new Map(),
  /** stops the conversion of `name` if one runs (× on its row of the side list); true if there was one */
  stop(/** @type {string} */ name) { const c = this.stops.get(name); if (c) c.abort(); return !!c; },
  /** one stop for the whole loading and one for each file, with nothing drawn */
  group() {
    const ctl = new AbortController(), stops = this.stops;
    return {
      signal: ctl.signal,
      start(/** @type {number} */ i, /** @type {string} */ name) {
        void i; const c = new AbortController(), off = () => { if (stops.get(name) === c) stops.delete(name); };
        ctl.signal.addEventListener("abort", () => c.abort()); stops.set(name, c);
        return { signal: c.signal, update(/** @type {Bag} */ p) { void p; }, done: off, fail: off };
      },
      // the indexing of a converted file (shell.js upload) and the end of the loading: nothing to draw
      index(/** @type {string} */ name, /** @type {number} */ pc) { void name; void pc; },
      end() {},
    };
  },
  /** the panel of one conversion inside `box` (conversione-carta.js) */
  async card(/** @type {HTMLElement} */ box, /** @type {number[]} */ sizes) { await carica("m:conversione-carta.js"); return /** @type {any} */ (window).CONVCARD(box, sizes, this.stops); },
};

const convMB = (/** @type {number} */ b) => (b / 1048576).toFixed(b >= 1.048576e7 ? 0 : 1);

// progress message { scans, scansTotal } (scans converted / scans in the .raw, from its scan index; scansTotal is 0 or null while the container is being
// opened) plus { total, read, written } (bytes, for the numbers) or, while the desktop app hands the mzML over, { copied, size }.
// Phases of one file: reading (1/3: the container and its index), converting (2/3: scan by scan), indexing (3/3: the page builds the .mzidx, see index.html upload)
function convPhase(/** @type {Bag | null | undefined} */ p) {
  if (!p || p.copied == null && !p.scansTotal) return "reading";
  return "converting";
}
// fraction 0..1 of the conversion of one file (scans converted / scans); 0 while reading
function convFraction(/** @type {Bag | null | undefined} */ p) {
  if (!p) return 0;
  if (p.copied != null) return 1;
  return p.scansTotal ? Math.max(0, Math.min(1, p.scans / p.scansTotal)) : 0;
}
// whole percent 0..100 of the phase "converting" (scans converted / scans; bytes handed over while the desktop app passes the mzML); null while reading
function convPercent(/** @type {Bag | null | undefined} */ p) {
  if (!p) return null;
  if (p.copied != null) return p.size ? Math.min(100, Math.floor(100 * p.copied / p.size)) : null;
  return p.scansTotal ? Math.floor(100 * convFraction(p)) : null;
}
// whole percent 0..100 of all the .raw files: the ones done count for their size, the one running for its fraction; `sizes` are the bytes of the .raw files
function convTotalPercent(/** @type {number[]} */ sizes, /** @type {number} */ i, /** @type {number} */ frac) {
  const all = sizes.reduce((/** @type {number} */ a, /** @type {number} */ b) => a + b, 0);
  if (!all) return null;
  const before = sizes.slice(0, i).reduce((/** @type {number} */ a, /** @type {number} */ b) => a + b, 0);
  return Math.min(100, Math.floor(100 * (before + sizes[i] * frac) / all));
}
// name of the mzML made from a .raw: "x.raw" -> "x.mzML"; when that name is taken (a real x.mzML in the same loading or already open) the extension
// stays in the key, "x.raw.mzML", so both files are listed. `taken`: names (any case) that are already in use.
function convName(/** @type {string} */ rawName, /** @type {Iterable<string> | null | undefined} */ taken) {
  const base = String(rawName).replace(/\.raw$/i, "");
  return freeMzml(base, ".raw", new Set([...(taken || [])].map(n => String(n).toLowerCase())));
}
// "base.mzML", else "base<ext>.mzML", else "base_2<ext>.mzML", "base_3<ext>.mzML"...: a third file with the same name never takes the name of the second
function freeMzml(/** @type {string} */ base, /** @type {string} */ ext, /** @type {Set<string>} */ used) {
  if (!used.has((base + ".mzML").toLowerCase())) return base + ".mzML";
  for (let i = 1; ; i++) {
    const name = (i === 1 ? base : base + "_" + i) + ext + ".mzML";
    if (!used.has(name.toLowerCase())) return name;
  }
}
// name of the mzML of a sample of a SCIEX .wiff: its DATA name, else "x.mzML" or "x_s3.mzML" (sample number, for a sample without a name in a .wiff with several);
// a name already taken goes through the counter of freeMzml ("S.wiff.mzML", "S_2.wiff.mzML"), the same as a .raw
function wiffName(/** @type {string} */ stem, /** @type {number | string} */ sample, /** @type {boolean} */ several, /** @type {Iterable<string> | null | undefined} */ taken, /** @type {string | null | undefined} */ sampleName) {
  const given = sampleName != null && String(sampleName) !== stem + ".wiff #" + sample;      // the reader names a sample without a DATA name "x.wiff #3"
  const clean = given ? String(sampleName).trim().replace(/[/\\:*?"<>|]/g, "_").slice(0, 120) : "";
  const used = new Set([...(taken || [])].map(n => String(n).toLowerCase()));
  const base = clean || (several ? stem + "_s" + sample : String(stem));
  return freeMzml(base, ".wiff", used);
}
// the short word of the state of a row (CONVROWS.short): the phase without numbers
function convStKey(/** @type {string} */ state) { return /** @type {Record<string, string>} */ ({ open: "conv.st.open", nopen: "conv.st.open", get: "conv.st.get", load: "conv.st.load", idx: "conv.st.index", index: "conv.st.index", wait: "conv.st.wait", run: "conv.st.run", conv: "conv.st.conv", nread: "conv.st.read", nscan: "conv.st.scans", nchrom: "conv.st.chrom" })[state] || "conv.st.wait"; }
// the text key of the state of a row of the files being converted (CONVROWS)
function convRowKey(/** @type {string} */ state) { return /** @type {Record<string, string>} */ ({ open: "conv.row.open", get: "conv.row.get", load: "conv.row.loadn", idx: "conv.row.idx", wait: "conv.row.wait", run: "conv.row.run", conv: "conv.row.conv", index: "conv.row.index", nread: "conv.row.nread", nopen: "conv.row.nopen", nscan: "conv.row.nscan", nchrom: "conv.row.nchrom", cancel: "conv.row.cancel" })[state] || "conv.row.wait"; }
// "m:ss" or "h:mm:ss" from milliseconds
function convElapsed(/** @type {number} */ ms) {
  const s = Math.max(0, Math.floor(ms / 1000)), h = Math.floor(s / 3600), m = Math.floor(s % 3600 / 60), r = s % 60;
  return (h ? h + ":" + String(m).padStart(2, "0") : String(m)) + ":" + String(r).padStart(2, "0");
}

if (typeof module !== "undefined") module.exports = { CONVROWS, convName, wiffName, convRowKey, convStKey, convPhase, convFraction, convPercent, convTotalPercent, convElapsed, convMB };
