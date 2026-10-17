// @ts-strict
"use strict";
// Thermo .raw and SCIEX .wiff -> mzML (index.html rawToMzml, wiffToMzml): the progress of the conversion and the download of the converted files. The DOM part first (only runs in the page), the pure helpers
// below (also run by node in tests/test_conversione.py). Classic script, loaded after i18n.js.

// the mzML files made in this page: mzML name -> Blob (with .mzlabId in the desktop app); they are what the program reads, so a saved copy opens without converting again
const CONV = {
  files: new Map(),
  numpress: false,       // «Download .mzML» with MS-Numpress (lossy, smaller): off unless the user ticks it
  key(name) { return (this.numpress ? "np:" : "") + name; },
  made: new Map(),        // name -> mzML written on request for a sample of a maker's file (nativo.js)
  add(name, blob) { this.files.set(name, blob); },
  // the samples of the maker's files of this page: their mzML is written when it is asked for
  native() { const rb = typeof window !== "undefined" && window.MZLAB_RUST; return rb && rb.nativeNames ? rb.nativeNames() : []; },
  has(name) { return this.files.has(name) || this.native().includes(name); },
  get size() { return new Set([...this.files.keys(), ...this.native()]).size; },
  // the mzML of a sample of a maker's file, written now into the box of the loading (a panel with the scans done and the cancel button); null if it was cancelled or failed
  async make(name) {
    const rb = window.MZLAB_RUST, box = document.getElementById("up");
    if (!rb || !rb.nativeToMzml || !rb.isNative(name) || !box) return null;
    const f = (typeof ST !== "undefined" && ST.files || []).find(x => x.name === name), g = ConvUI.group(box, [1]), ui = g.start(1, name);
    try { const blob = await rb.nativeToMzml(name, f ? f.size : 0, p => ui.update(p), g.signal, this.numpress); ui.done(); g.end(); this.made.set(this.key(name), blob); return blob; }
    catch (e) { if (e.cancelled) { ui.done(); g.end(); } else ui.fail(e.json ? I18N.err(e.json).message : e.message); return null; }
  },
  // saves one: the system dialog in the desktop app, a normal download in the browser; resolves to false if the user cancelled or could not save
  async download(name) {
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
  async downloadAll() { for (const n of new Set([...this.files.keys(), ...this.native()])) { await this.download(n); if (!window.MZLAB_DESKTOP) await new Promise(r => setTimeout(r, 400)); } },
};

// the files being read, converted and indexed (the samples of a maker's file, the files a module converts), as rows of the list of the files (index.html renderFiles) from the moment
// they are known until the user closes them (the running ones in the table, the ended ones in a list under it): { name, state, pc, sub, t0, seen, ... }. Running states: "open" (pair found, reading its .wiff), "wait" (queued), "run" (converting, pc),
// "conv" (converted, waits for the index), "index" / "nread" / "nopen" / "nscan" / "nchrom" (indexing, pc; sub = {done, total} scans). Final states, which stay: "done" (scans, secs),
// "fail" (msg, diag: DIAG with the trace of the loading, «Details» and «Copy diagnosis»), "cancel". A change of state draws the list again; a new percentage only moves the bar of its
// row; while a row runs, a clock of 1 s writes its time and, after CONV_QUIET ms without news, how long it has been quiet (never a still screen). `onCancel` (set by the loading)
// shows «Cancel» in the heading.
const CONV_QUIET = 5000;
const CONV_FINAL = ["done", "fail", "cancel"];
// what the summary counts apart, in this order: the kinds of file (`kind` of the file entry, "mixed" for a file of more parts), then the rows that did not load
const CONV_SUM = { full: "conv.sum.full", ms2: "conv.sum.ms2", mrm: "conv.sum.mrm", infusion: "conv.sum.infusion", mixed: "conv.sum.mixed", other: "conv.sum.other", fail: "conv.sum.fail", cancel: "conv.sum.cancel" };
const CONVROWS = {
  /** @type {{ name: string, state: string, pc: number | null, sub?: { done: number, total: number } | null, t0?: number, seen?: number, tr?: string[], last?: string, scans?: number, kind?: string, missing?: number, secs?: number, msg?: string, diag?: any, mem?: string }[]} */
  rows: [],
  /** @type {null | (() => void)} */
  onCancel: null,
  timer: /** @type {any} */ (0),
  find(name) { return this.rows.find(r => r.name === name); },
  running() { return this.rows.filter(r => !CONV_FINAL.includes(r.state)); },
  // a row; `mem` describes the members without their names (".wiff 1.2 MB + .wiff.scan 57.0 MB", for the diagnosis)
  add(name, state, mem) {
    const r = this.find(name), now = Date.now();
    if (r && !CONV_FINAL.includes(r.state)) return;
    if (r) this.rows.splice(this.rows.indexOf(r), 1);
    this.rows.push({ name, state, pc: null, sub: null, t0: now, seen: now, tr: [`0.0 s ${state}`], last: "", mem: mem || "" }); this.paint(); this.clock();
  },
  set(name, state, pc, sub) {
    const r = this.find(name); if (!r || CONV_FINAL.includes(r.state)) return;
    const was = r.state, was_pc = r.pc, was_sub = r.sub ? r.sub.done : -1, now = Date.now();
    r.state = state; r.pc = pc == null ? null : pc; if (sub !== undefined) r.sub = sub && sub.total ? sub : null;
    if (was !== state || was_pc !== r.pc || (r.sub ? r.sub.done : -1) !== was_sub) r.seen = now;
    if (was !== state) { (r.tr = r.tr || []).push(`${((now - (r.t0 || now)) / 1000).toFixed(1)} s ${state}${r.pc == null ? "" : " " + r.pc + "%"}`); this.paint(); } else if (r.seen === now) this.fill(r);
  },
  // the last message of the worker, for the diagnosis
  note(name, text) { const r = this.find(name); if (r) r.last = String(text).slice(0, 200); },
  // the end of a row: "done" ({ scans }), "fail" ({ msg, diag }) or "cancel"; it stays until «Close» or the next loading
  finish(name, state, extra) {
    const r = this.find(name); if (!r || CONV_FINAL.includes(r.state)) return;
    const now = Date.now(); r.secs = (now - (r.t0 || now)) / 1000; Object.assign(r, extra || {});
    (r.tr = r.tr || []).push(`${r.secs.toFixed(1)} s ${state}`); r.state = state; r.seen = now;
    if (r.diag) r.diag.dettagli.trace = this.trace([r]);
    this.paint();
  },
  drop(name) { const i = this.rows.findIndex(r => r.name === name); if (i >= 0) { this.rows.splice(i, 1); this.paint(); } },
  // the rows that ended go away (a new loading, «Close»)
  clearDone() { const n = this.rows.length; this.rows = this.running(); if (n !== this.rows.length) this.paint(); },
  paint() { if (typeof renderFiles === "function") renderFiles(); },
  clock() {
    if (this.timer || typeof setInterval === "undefined") return;
    this.timer = setInterval(() => {
      const run = this.running();
      for (const r of run) this.fill(r);
      if (!run.length) { clearInterval(this.timer); this.timer = 0; }
    }, 1000);
  },
  label(r) {
    if (r.state === "done") return I18N.t("conv.row.done", { scans: I18N.fix(r.scans || 0, 0), secs: I18N.fix(r.secs || 0, 1) }) + (r.missing ? " · " + I18N.t("conv.row.missing", { n: I18N.fix(r.missing, 0) }) : "");
    if (r.state === "fail") return r.msg || "";
    const lab = I18N.t(convRowKey(r.state), { pc: r.pc == null ? 0 : r.pc });
    return r.sub ? `${lab} · ${I18N.t("conv.row.scans", { done: I18N.fix(r.sub.done, 0), total: I18N.fix(r.sub.total, 0) })}` : lab;
  },
  // the time of a running row and, when it has been quiet for CONV_QUIET ms, for how long (not for a queued row: it waits for a free builder); "" for an ended row
  when(r, now = Date.now()) {
    if (CONV_FINAL.includes(r.state)) return "";
    const t = convElapsed(now - (r.t0 || now)), q = now - (r.seen || now);
    return q >= CONV_QUIET && r.state !== "wait" ? `${t} · ${I18N.t("conv.row.quiet", { s: Math.floor(q / 1000) })}` : t;
  },
  cell(name) { const q = document.getElementById("flist"); return q ? [...q.querySelectorAll("td[data-cvrow]")].find(x => x.dataset.cvrow === name) : null; },
  fill(r) {
    const c = this.cell(r.name); if (!c) return;
    const lab = this.label(r), bar = c.querySelector(".prog"), txt = c.querySelector("[data-cvtxt]"), w = c.querySelector("[data-cvwhen]");
    if (txt && txt.textContent !== lab) txt.textContent = lab;
    if (w) { const t = this.when(r); if (w.textContent !== t) w.textContent = t; }
    if (bar) { bar.setAttribute("aria-label", r.name + ": " + lab); if (r.pc == null) bar.removeAttribute("aria-valuenow"); else bar.setAttribute("aria-valuenow", String(r.pc)); bar.querySelector("i").style.width = (r.pc || 0) + "%"; }
  },
  // the trace of the loading for the diagnosis: no file names (the rows are numbered), the members by extension and size, the phases with their times, the last message of the worker
  trace(rows) {
    const nav = typeof navigator !== "undefined" ? /** @type {any} */ (navigator) : {}, now = Date.now();
    const L = [`device: ${nav.deviceMemory || "?"} GB, ${nav.hardwareConcurrency || "?"} cores${typeof window !== "undefined" && window.MZLAB_STIMA ? ", storage " + window.MZLAB_STIMA : ""}`];
    rows.forEach(r => {
      const k = this.rows.indexOf(r) + 1;
      const end = CONV_FINAL.includes(r.state), secs = end ? r.secs || 0 : (now - (r.t0 || now)) / 1000;
      L.push(`#${k || "?"} ${r.state}${r.pc == null ? "" : " " + r.pc + "%"}${r.sub ? ` scans ${r.sub.done}/${r.sub.total}` : ""}${r.scans ? ` scans ${r.scans}` : ""} · ${secs.toFixed(1)} s${end ? "" : ` · quiet ${((now - (r.seen || now)) / 1000).toFixed(1)} s`}${r.mem ? " · " + r.mem : ""}`);
      L.push(`  phases: ${(r.tr || []).join(", ")}`);
      if (r.last) L.push(`  last: ${r.last}`);
    });
    return L.join("\n");
  },
  // the rows running, for the table of the list (a heading row with the files ended of all, «Cancel» and «Details», then one row per file), `ncol` = the columns of the table
  html(ncol) {
    const run = this.running(); if (!run.length) return "";
    const T = (k, p) => I18N.t(k, p), n = this.rows.length;
    const act = this.onCancel ? ` <button type="button" class="sec sm" data-cvact="cancel" title="${T("load.cancel.title")}">${T("conv.cancel")}</button>` : "";
    const det = `<div class="diag" data-cvdiag><button type="button" class="sec sm" data-a="diagdet" aria-expanded="false">${T("diag.details")}</button><div class="diagbox" hidden><pre data-cvtrace></pre><button type="button" class="sec" data-a="diagcopy">${T("diag.copy")}</button></div></div>`;
    const head = `<tr class="grp"><td colspan="${ncol}">${T("conv.row.group")} <span class="muted">${T("load.prog.count", { done: n - run.length, n })}</span>${act}${det}</td></tr>`;
    return head + run.map(r => {
      const lab = this.label(r), now = r.pc == null ? "" : ` aria-valuenow="${r.pc}"`;
      return `<tr aria-busy="true"><td></td><td>${esc(r.name)}</td><td colspan="${ncol - 2}" data-cvrow="${esc(r.name)}"><div class="muted sm"><span data-cvtxt>${esc(lab)}</span> <span class="cp-num" data-cvwhen>${esc(this.when(r))}</span></div>` +
        `<div class="prog" role="progressbar" aria-valuemin="0" aria-valuemax="100"${now} aria-label="${esc(r.name + ": " + lab)}"><i style="width:${r.pc || 0}%"></i></div></td></tr>`;
    }).join("");
  },
  // the count of the ended rows by kind of file (full scan, MS2, MRM, infusion, mixed, other), the failed, cancelled and truncated ones, and the time from the first start to the last end
  /** @param {CONVROWS["rows"]} end */
  totals(end) {
    const T = (k, p) => I18N.t(k, p), by = new Map();
    const add = k => by.set(k, (by.get(k) || 0) + 1);
    for (const r of end) add(r.state !== "done" ? r.state : r.kind in CONV_SUM && r.kind !== "fail" && r.kind !== "cancel" ? r.kind : "other");
    const parts = Object.keys(CONV_SUM).filter(k => by.has(k)).map(k => T(CONV_SUM[k], { n: I18N.fix(by.get(k), 0) }));
    const miss = end.filter(r => r.missing).length; if (miss) parts.push(T("conv.sum.missing", { n: I18N.fix(miss, 0) }));
    const t0 = Math.min(...end.map(r => r.t0 || 0)), t1 = Math.max(...end.map(r => (r.t0 || 0) + 1000 * (r.secs || 0)));
    return T("conv.sum.head", { n: I18N.fix(end.length, 0), parts: parts.join(", "), time: convElapsed(t1 - t0) });
  },
  // the files whose loading ended, under the table: «Loaded ✓ (scans, time)», the error with «Details» and «Copy diagnosis», «Cancelled»; «Close» once nothing runs
  summary() {
    const end = this.rows.filter(r => CONV_FINAL.includes(r.state)); if (!end.length) return "";
    const T = (k, p) => I18N.t(k, p), run = this.running().length;
    const li = end.map(r => {
      const lab = r.state === "cancel" ? T("conv.row.cancel") : this.label(r);
      return `<li data-cvrow="${esc(r.name)}" data-cvend="${r.state}"${r.missing ? ` data-cvmiss="${r.missing}"` : ""}>${esc(r.name)}: ${r.state === "fail" ? `<span class="fail" data-cvtxt>${esc(lab)}</span>${r.diag && typeof DIAG !== "undefined" ? DIAG.html(r.diag, esc, T) : ""}` : `<span data-cvtxt>${esc(lab)}</span>`}</li>`;
    }).join("");
    return `<div class="cvsum" role="group" aria-label="${esc(T(run ? "conv.row.group" : "conv.row.groupDone"))}"><p class="sm"><b>${T(run ? "conv.row.group" : "conv.row.groupDone")}</b> <span class="muted">${T("load.prog.count", { done: end.length, n: this.rows.length })}</span>` +
      `${run ? "" : ` <button type="button" class="sec sm" data-cvact="close">${T("common.close")}</button>`}</p>${run ? "" : `<p class="sm" data-cvtot>${esc(this.totals(end))}</p>`}<ul class="sm">${li}</ul></div>`;
  },
  // the buttons of the rows, after renderFiles has drawn them in `el`
  wire(el) {
    const T = (k, p) => I18N.t(k, p), v = typeof window !== "undefined" ? window.MZLAB_VERSION : "";
    const c = el.querySelector("[data-cvact=cancel]"); if (c) c.onclick = () => { c.disabled = true; if (this.onCancel) this.onCancel(); };
    const x = el.querySelector("[data-cvact=close]"); if (x) x.onclick = () => this.clearDone();
    const g = el.querySelector("[data-cvdiag]");
    if (g) {      // the trace of the loading still running: written when it is opened and copied as it is at that moment
      const d = () => ({ codice: "load.trace", fase: "index", causa: "unknown", dettagli: { format: "?", size: 0, raw: "", head: "", trace: this.trace(this.rows) } });
      DIAG.cabla(g, d, v, T);
      const b = g.querySelector("[data-a=diagdet]"), pre = g.querySelector("[data-cvtrace]"), o = b.onclick;
      b.onclick = e => { pre.textContent = this.trace(this.rows); if (o) o.call(b, e); };
    }
    this.rows.forEach(r => { if (r.state !== "fail" || !r.diag) return; const li = [...el.querySelectorAll("[data-cvend=fail]")].find(x => /** @type {HTMLElement} */ (x).dataset.cvrow === r.name), dg = li && li.querySelector(".diag"); if (dg) DIAG.cabla(dg, r.diag, v, T); });
  },
};

// the panel of the running conversions, inside `box`: "File 1 of 2: name", a button to cancel, the phase as text ("Converting to mzML", then "Indexing" while the page builds the
// .mzidx: g.index), ONE progress bar (role=progressbar) over the whole job (the conversion of all the files counts 3/4 of it, the indexing 1/4), and the numbers (scans, MB
// written, time elapsed: no estimate of the time left). The digits are tabular and padded to a fixed width, so the text does not move when a number grows. If no news arrives
// for 2 s it says it is still working (animated dot, still when the system asks for reduced motion); after 30 s it adds advice.
const CONV_W = 0.75;         // share of the bar taken by the conversion; the rest is the indexing
const FIG = "\u2007";        // figure space: as wide as a digit
const convPad = (s, w) => String(s).padStart(w, FIG);
const ConvUI = {
  group(box, sizes) {
    const T = (k, p) => I18N.t(k, p), n = sizes.length, ctl = new AbortController(), t0 = Date.now();
    const el = document.createElement("div"); el.className = "convprog"; el.setAttribute("role", "group"); el.setAttribute("aria-label", T("conv.aria"));
    el.innerHTML = `<div class="cp-head"><b data-r="file"></b> <button type="button" class="sec sm" data-r="cancel"></button></div>
      <div class="cp-phase"><span data-r="phase" role="status" aria-live="polite"></span> <span class="cp-beat muted" data-r="beat" role="status" aria-live="polite"></span></div>
      <div class="prog" role="progressbar" aria-valuemin="0" aria-valuemax="100" data-r="bar"><i></i></div><div class="muted sm cp-num" data-r="detail"></div>`;
    const q = s => el.querySelector(`[data-r=${s}]`);
    const cancel = q("cancel"); cancel.textContent = T("conv.cancel"); cancel.title = cancel.ariaLabel = T("conv.cancel.title");
    cancel.onclick = () => { cancel.disabled = true; q("phase").textContent = T("conv.cancelling"); ctl.abort(); };
    box.appendChild(el);
    const bar = q("bar"), fill = bar.querySelector("i");
    const setBar = pc => { if (pc == null) { bar.removeAttribute("aria-valuenow"); fill.style.width = "0%"; } else { bar.setAttribute("aria-valuenow", String(pc)); fill.style.width = pc + "%"; } };
    let beatStage = "", cur = -1, last = null, doneFrac = 0, stage = "converting", beat = "", changed = t0, key = "", phaseKey = "", indexed = new Map(), nIdx = 0, timer = null, maxPc = 0;
    const idxMean = () => nIdx ? [...indexed.values()].reduce((a, b) => a + b, 0) / (100 * nIdx) : 0;
    const show = () => {
      const now = Date.now(), elapsed = convElapsed(now - t0), conv = stage === "indexing" ? 100 : convTotalPercent(sizes, Math.max(0, cur), convFraction(last));
      const known = last && (last.copied != null || last.scansTotal); let pc = (conv == null || !known && cur <= 0) && stage === "converting" ? null : Math.min(100, Math.floor(CONV_W * (conv || 0) + (1 - CONV_W) * 100 * idxMean()));
      if (pc != null) { maxPc = Math.max(maxPc, Math.min(99, pc)); pc = maxPc; }         // forward only and never 100 % before `end()`: the same rules as PROGRESSO
      const lab = T(stage === "indexing" ? "conv.phase.indexing" : "conv.phase.converting");
      if (lab !== phaseKey && !cancel.disabled) { phaseKey = lab; q("phase").textContent = lab; }
      bar.setAttribute("aria-label", lab); setBar(pc);
      let d;
      if (stage === "indexing") d = T("conv.detail.indexed", { done: [...indexed.values()].filter(v => v >= 100).length, total: nIdx, elapsed });
      else if (last && last.copied != null) d = T("conv.detail.handover", { done: convMB(last.copied), total: convMB(last.size), elapsed });
      else if (last && last.scansTotal) {
        const w = String(Math.round(last.scansTotal)).length, fp = convPercent(last);
        const parts = [convPad(fp, 3) + "%", ...(last.several ? [] : [T("conv.detail.scans", { done: convPad(I18N.fix(last.scans, 0), w + Math.floor((w - 1) / 3)), total: I18N.fix(last.scansTotal, 0) })]),
          T("conv.detail.written", { mb: convPad(convMB(last.written), 6) }), T("conv.detail.elapsed", { elapsed: convPad(elapsed, 5) })];
        d = parts.join(" · ");
      } else d = T("conv.detail.wait", { elapsed });
      q("detail").textContent = d;
      const quiet = now - changed, b = quiet >= 30000 ? "stall" : quiet >= 2000 ? "beat" : "";
      if (b !== beat || b === "beat" && beatStage !== stage) { beatStage = stage; beat = b; q("beat").innerHTML = b ? `<span class="cp-spin" aria-hidden="true"></span> <span data-r="beattxt"></span>` : ""; if (b) q("beattxt").textContent = T(b === "stall" ? "conv.stall" : stage === "indexing" ? "conv.beat.indexing" : "conv.beat"); }
    };
    const tick = () => { if (!timer) timer = setInterval(show, 1000); };
    const touch = k => { if (k !== key) { key = k; changed = Date.now(); } };
    show();
    return {
      signal: ctl.signal,
      start(i, name) {
        cur = i - 1; last = null; stage = "converting";
        q("file").textContent = T("conv.file", { i, n, name });
        tick(); show();
        return {
          update(p) { touch([p.scans, p.read, p.written, p.copied].join(",")); last = p; show(); },
          title(text) { q("file").textContent = text; },       // replaces "File i of n: name" (conversions running together: "Samples converted: 3 of 23")
          done() { doneFrac = (cur + 1) / n; },
          fail(msg) { clearInterval(timer); timer = null; el.remove(); const f = document.createElement("div"); f.className = "fail"; f.textContent = name + ": " + msg; box.appendChild(f); },
        };
      },
      // the page builds the index of each converted file (index.html upload): `pc` 0..100 of `name`; the bar goes on from where the conversion left it
      index(name, pc) {
        if (!indexed.has(name)) nIdx++;
        indexed.set(name, Math.max(indexed.get(name) || 0, pc));        // never backwards: the builder reads the file more than once
        if (stage !== "indexing") { stage = "indexing"; q("file").textContent = ""; cancel.hidden = true; }
        touch("idx" + [...indexed.values()].join(","));
        tick(); show();
      },
      end() { clearInterval(timer); timer = null; el.remove(); },
    };
  },
};

const convMB = b => (b / 1048576).toFixed(b >= 1.048576e7 ? 0 : 1);

// progress message { scans, scansTotal } (scans converted / scans in the .raw, from its scan index; scansTotal is 0 or null while the container is being
// opened) plus { total, read, written } (bytes, for the numbers) or, while the desktop app hands the mzML over, { copied, size }.
// Phases of one file: reading (1/3: the container and its index), converting (2/3: scan by scan), indexing (3/3: the page builds the .mzidx, see index.html upload)
function convPhase(p) {
  if (!p || p.copied == null && !p.scansTotal) return "reading";
  return "converting";
}
// fraction 0..1 of the conversion of one file (scans converted / scans); 0 while reading
function convFraction(p) {
  if (!p) return 0;
  if (p.copied != null) return 1;
  return p.scansTotal ? Math.max(0, Math.min(1, p.scans / p.scansTotal)) : 0;
}
// whole percent 0..100 of the phase "converting" (scans converted / scans; bytes handed over while the desktop app passes the mzML); null while reading
function convPercent(p) {
  if (!p) return null;
  if (p.copied != null) return p.size ? Math.min(100, Math.floor(100 * p.copied / p.size)) : null;
  return p.scansTotal ? Math.floor(100 * convFraction(p)) : null;
}
// whole percent 0..100 of all the .raw files: the ones done count for their size, the one running for its fraction; `sizes` are the bytes of the .raw files
function convTotalPercent(sizes, i, frac) {
  const all = sizes.reduce((a, b) => a + b, 0);
  if (!all) return null;
  const before = sizes.slice(0, i).reduce((a, b) => a + b, 0);
  return Math.min(100, Math.floor(100 * (before + sizes[i] * frac) / all));
}
// name of the mzML made from a .raw: "x.raw" -> "x.mzML"; when that name is taken (a real x.mzML in the same loading or already open) the extension
// stays in the key, "x.raw.mzML", so both files are listed. `taken`: names (any case) that are already in use.
function convName(rawName, taken) {
  const base = String(rawName).replace(/\.raw$/i, ""), used = new Set([...(taken || [])].map(n => String(n).toLowerCase()));
  return used.has((base + ".mzML").toLowerCase()) ? base + ".raw.mzML" : base + ".mzML";
}
// name of the mzML of a sample of a SCIEX .wiff: "x.mzML" when the .wiff holds one sample, "x_s3.mzML" (sample number) when it holds several; "x.wiff.mzML" when taken
function wiffName(stem, sample, several, taken) {
  const base = several ? stem + "_s" + sample : String(stem), used = new Set([...(taken || [])].map(n => String(n).toLowerCase()));
  return used.has((base + ".mzML").toLowerCase()) ? base + ".wiff.mzML" : base + ".mzML";
}
// the text key of the state of a row of the files being converted (CONVROWS)
function convRowKey(state) { return { open: "conv.row.open", wait: "conv.row.wait", run: "conv.row.run", conv: "conv.row.conv", index: "conv.row.index", nread: "conv.row.nread", nopen: "conv.row.nopen", nscan: "conv.row.nscan", nchrom: "conv.row.nchrom", cancel: "conv.row.cancel" }[state] || "conv.row.wait"; }
// "m:ss" or "h:mm:ss" from milliseconds
function convElapsed(ms) {
  const s = Math.max(0, Math.floor(ms / 1000)), h = Math.floor(s / 3600), m = Math.floor(s % 3600 / 60), r = s % 60;
  return (h ? h + ":" + String(m).padStart(2, "0") : String(m)) + ":" + String(r).padStart(2, "0");
}

if (typeof module !== "undefined") module.exports = { convName, wiffName, convRowKey, convPhase, convFraction, convPercent, convTotalPercent, convElapsed, convMB };
