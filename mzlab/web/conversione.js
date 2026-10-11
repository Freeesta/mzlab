"use strict";
// Thermo .raw -> mzML (index.html rawToMzml): the progress of the conversion and the download of the converted files. Pure helpers first
// (also run by node in tests/test_conversione.py); the DOM part below only runs in the page. Classic script, loaded after i18n.js.

// progress message { total, read, written } (bytes of the .raw read, bytes of the mzML written) or, while the desktop app hands the mzML over,
// { copied, size }: phase = reading (nothing written yet), converting (reading and writing together), writing (the .raw is all read)
function convPhase(p) {
  if (!p) return "reading";
  if (p.copied != null) return "writing";
  if (!p.total) return "reading";
  if (p.read >= p.total) return "writing";
  return p.written > 0 ? "converting" : "reading";
}
// whole percent 0..100 of the file (bytes of the .raw read; bytes handed over while "writing" in the desktop app); null if not known
function convPercent(p) {
  if (!p) return null;
  if (p.copied != null) return p.size ? Math.min(100, Math.floor(100 * p.copied / p.size)) : null;
  if (!p.total) return null;
  return Math.min(100, Math.floor(100 * p.read / p.total));
}
// "m:ss" or "h:mm:ss" from milliseconds
function convElapsed(ms) {
  const s = Math.max(0, Math.floor(ms / 1000)), h = Math.floor(s / 3600), m = Math.floor(s % 3600 / 60), r = s % 60;
  return (h ? h + ":" + String(m).padStart(2, "0") : String(m)) + ":" + String(r).padStart(2, "0");
}
const convMB = b => (b / 1048576).toFixed(b >= 1.048576e7 ? 0 : 1);

// the mzML files made in this page: mzML name -> Blob (with .mzlabId in the desktop app); they are what the program reads, so a saved copy opens without converting again
const CONV = {
  files: new Map(),
  add(name, blob) { this.files.set(name, blob); },
  has(name) { return this.files.has(name); },
  get size() { return this.files.size; },
  // saves one: the system dialog in the desktop app, a normal download in the browser; resolves to false if the user cancelled or could not save
  async download(name) {
    const blob = this.files.get(name); if (!blob) return false;
    if (window.MZLAB_DESKTOP) {
      if (blob.mzlabId == null || !window.MZLAB_SAVE_MZML) return false;
      try { return !!(await window.MZLAB_SAVE_MZML(blob.mzlabId, name)); }
      catch (e) { const n = document.getElementById("up"); if (n) { const d = document.createElement("div"); d.className = "note"; d.textContent = I18N.t("conv.dl.needsApp"); n.appendChild(d); setTimeout(() => d.remove(), 12000); } return false; }
    }
    const a = document.createElement("a"), u = URL.createObjectURL(blob);
    a.href = u; a.download = name; document.body.appendChild(a); a.click(); a.remove();
    setTimeout(() => URL.revokeObjectURL(u), 60000);
    return true;
  },
  // all of them, one after the other (a pause between browser downloads, a dialog each in the desktop app)
  async downloadAll() { for (const n of [...this.files.keys()]) { await this.download(n); if (!window.MZLAB_DESKTOP) await new Promise(r => setTimeout(r, 400)); } },
};

// the panel of one running conversion, inside `box`: "2 of 5: name", the phase, a progress bar (role=progressbar) and the numbers with the elapsed time
const ConvUI = {
  start(box, i, n, name) {
    const t0 = Date.now(), T = (k, p) => I18N.t(k, p);
    const el = document.createElement("div"); el.className = "convprog"; el.setAttribute("role", "group"); el.setAttribute("aria-label", T("conv.aria"));
    el.innerHTML = `<div><b data-r="file"></b> <span class="muted" data-r="phase" role="status" aria-live="polite"></span></div>
      <div class="prog" role="progressbar" aria-valuemin="0" aria-valuemax="100" data-r="bar"><i></i></div><div class="muted sm" data-r="detail"></div>`;
    const q = s => el.querySelector(`[data-r=${s}]`);
    q("file").textContent = T("conv.file", { i, n, name });
    box.appendChild(el);
    let last = null, phase = "";
    const bar = q("bar"), fill = bar.querySelector("i");
    const show = () => {
      const ph = convPhase(last), pc = convPercent(last), el_ = convElapsed(Date.now() - t0);
      const lab = { reading: T("conv.phase.reading"), converting: T("conv.phase.converting"), writing: T("conv.phase.writing") }[ph];
      if (ph !== phase) { phase = ph; q("phase").textContent = lab; }
      bar.setAttribute("aria-label", lab);
      if (pc == null) { bar.removeAttribute("aria-valuenow"); fill.style.width = "0%"; } else { bar.setAttribute("aria-valuenow", String(pc)); fill.style.width = pc + "%"; }
      const d = last && last.copied != null ? T("conv.detail.writing", { done: convMB(last.copied), total: convMB(last.size), elapsed: el_ })
        : last && last.total ? T("conv.detail.read", { pc, done: convMB(last.read), total: convMB(last.total), elapsed: el_ }) : T("conv.detail.wait", { elapsed: el_ });
      q("detail").textContent = d;
    };
    show();
    const timer = setInterval(show, 1000);
    return {
      update(p) { last = p; show(); },
      done() { clearInterval(timer); el.remove(); },
      fail(msg) { clearInterval(timer); el.innerHTML = ""; el.className = "fail"; el.textContent = name + ": " + msg; },
    };
  },
};

if (typeof module !== "undefined") module.exports = { convPhase, convPercent, convElapsed, convMB };
