"use strict";
// Thermo .raw and SCIEX .wiff -> mzML (index.html rawToMzml, wiffToMzml): the progress of the conversion and the download of the converted files. The DOM part first (only runs in the page), the pure helpers
// below (also run by node in tests/test_conversione.py). Classic script, loaded after i18n.js.

// the mzML files made in this page: mzML name -> Blob (with .mzlabId in the desktop app); they are what the program reads, so a saved copy opens without converting again
const CONV = {
  files: new Map(),
  anon: new Set(),                                           // the names made without personal data ("Esporta senza dati personali")
  add(name, blob, anon) { this.files.set(name, blob); if (anon) this.anon.add(name); else this.anon.delete(name); },
  has(name) { return this.files.has(name); },
  get size() { return this.files.size; },
  // saves one: the system dialog in the desktop app, a normal download in the browser; resolves to false if the user cancelled or could not save
  async download(name) {
    const blob = this.files.get(name); if (!blob) return false;
    if (window.MZLAB_DESKTOP) {
      if (blob.mzlabId == null || !window.MZLAB_SAVE_MZML) return false;
      try { const ok = !!(await window.MZLAB_SAVE_MZML(blob.mzlabId, name)); if (ok) this.note(name); return ok; }
      catch (e) { const n = document.getElementById("up"); if (n) { const d = document.createElement("div"); d.className = "note"; d.textContent = I18N.t("conv.dl.needsApp"); n.appendChild(d); setTimeout(() => d.remove(), 12000); } return false; }
    }
    saveBlob(blob, name, 60000);
    this.note(name);
    return true;
  },
  // what the saved file says about the people of the acquisition (a note in the loading area for a few seconds)
  note(name) {
    const n = document.getElementById("up"); if (!n) return;
    const d = document.createElement("div"); d.className = "note"; d.setAttribute("role", "status");
    d.textContent = I18N.t(this.anon.has(name) ? "conv.dl.anon" : "conv.dl.personal"); n.appendChild(d); setTimeout(() => d.remove(), 12000);
  },
  // all of them, one after the other (a pause between browser downloads, a dialog each in the desktop app)
  async downloadAll() { for (const n of [...this.files.keys()]) { await this.download(n); if (!window.MZLAB_DESKTOP) await new Promise(r => setTimeout(r, 400)); } },
};

// the files being converted (the samples of a SCIEX .wiff), as rows of the list of the files (index.html renderFiles) from the moment their .wiff and .wiff.scan are paired
// until their index is ready: { name, state, pc }. state: "open" (pair found, reading its .wiff), "wait" (queued), "run" (converting, pc), "conv" (converted, waits for the
// index), "index" (indexing, pc). A change of state draws the list again; a new percentage only moves the bar of its row.
const CONVROWS = {
  rows: [],
  find(name) { return this.rows.find(r => r.name === name); },
  add(name, state) { if (!this.find(name)) { this.rows.push({ name, state, pc: null }); this.paint(); } },
  set(name, state, pc) {
    const r = this.find(name); if (!r) return;
    const was = r.state, was_pc = r.pc;
    r.state = state; r.pc = pc == null ? null : pc;
    if (was !== state) this.paint(); else if (was_pc !== r.pc) this.fill(r);
  },
  drop(name) { const i = this.rows.findIndex(r => r.name === name); if (i >= 0) { this.rows.splice(i, 1); this.paint(); } },
  paint() { if (typeof renderFiles === "function") renderFiles(); },
  label(r) { return I18N.t(convRowKey(r.state), { pc: r.pc == null ? 0 : r.pc }); },
  cell(name) { const q = document.getElementById("flist"); return q ? [...q.querySelectorAll("[data-cvrow]")].find(x => x.dataset.cvrow === name) : null; },
  fill(r) {
    const c = this.cell(r.name); if (!c) return;
    const lab = this.label(r), bar = c.querySelector(".prog"), txt = c.querySelector("[data-cvtxt]");
    if (txt) txt.textContent = lab;
    if (bar) { bar.setAttribute("aria-label", r.name + ": " + lab); if (r.pc == null) bar.removeAttribute("aria-valuenow"); else bar.setAttribute("aria-valuenow", String(r.pc)); bar.querySelector("i").style.width = (r.pc || 0) + "%"; }
  },
  // the rows for the table of the list (a heading row and one row per file), `ncol` = the columns of the table
  html(ncol) {
    if (!this.rows.length) return "";
    const head = `<tr class="grp"><td colspan="${ncol}">${I18N.t("conv.row.group")} <span class="muted">${I18N.t("load.group.count", { n: this.rows.length })}</span></td></tr>`;
    return head + this.rows.map(r => {
      const lab = this.label(r), now = r.pc == null ? "" : ` aria-valuenow="${r.pc}"`;
      return `<tr aria-busy="true"><td></td><td>${esc(r.name)}</td><td colspan="${ncol - 2}" data-cvrow="${esc(r.name)}"><div class="muted sm" data-cvtxt>${esc(lab)}</div>` +
        `<div class="prog" role="progressbar" aria-valuemin="0" aria-valuemax="100"${now} aria-label="${esc(r.name + ": " + lab)}"><i style="width:${r.pc || 0}%"></i></div></td></tr>`;
    }).join("");
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
    let beatStage = "", cur = -1, last = null, doneFrac = 0, stage = "converting", beat = "", changed = t0, key = "", phaseKey = "", indexed = new Map(), nIdx = 0, timer = null;
    const idxMean = () => nIdx ? [...indexed.values()].reduce((a, b) => a + b, 0) / (100 * nIdx) : 0;
    const show = () => {
      const now = Date.now(), elapsed = convElapsed(now - t0), conv = stage === "indexing" ? 100 : convTotalPercent(sizes, Math.max(0, cur), convFraction(last));
      const known = last && (last.copied != null || last.scansTotal), pc = (conv == null || !known && cur <= 0) && stage === "converting" ? null : Math.min(100, Math.floor(CONV_W * (conv || 0) + (1 - CONV_W) * 100 * idxMean()));
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
function convRowKey(state) { return { open: "conv.row.open", wait: "conv.row.wait", run: "conv.row.run", conv: "conv.row.conv", index: "conv.row.index" }[state] || "conv.row.wait"; }
// "m:ss" or "h:mm:ss" from milliseconds
function convElapsed(ms) {
  const s = Math.max(0, Math.floor(ms / 1000)), h = Math.floor(s / 3600), m = Math.floor(s % 3600 / 60), r = s % 60;
  return (h ? h + ":" + String(m).padStart(2, "0") : String(m)) + ":" + String(r).padStart(2, "0");
}

if (typeof module !== "undefined") module.exports = { convName, wiffName, convRowKey, convPhase, convFraction, convPercent, convTotalPercent, convElapsed, convMB };
