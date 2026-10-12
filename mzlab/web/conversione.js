"use strict";
// Thermo .raw -> mzML (index.html rawToMzml): the progress of the conversion and the download of the converted files. The DOM part first (only runs in the page), the pure helpers
// below (also run by node in tests/test_conversione.py). Classic script, loaded after i18n.js.

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
    saveBlob(blob, name, 60000);
    return true;
  },
  // all of them, one after the other (a pause between browser downloads, a dialog each in the desktop app)
  async downloadAll() { for (const n of [...this.files.keys()]) { await this.download(n); if (!window.MZLAB_DESKTOP) await new Promise(r => setTimeout(r, 400)); } },
};

// the panel of the running conversions, inside `box`: "File 1 of 2: name", a button to cancel, the phase (1/3 reading, 2/3 converting; 3/3 indexing is the
// row that upload() shows next), a progress bar (role=progressbar) of the file, the numbers (scans, MB written, elapsed, time left), and with several files a
// bar of the total. If no news arrives for 2 s it says it is still working (animated dot, still when the system asks for reduced motion); after 30 s it adds advice.
const ConvUI = {
  group(box, sizes) {
    const T = (k, p) => I18N.t(k, p), n = sizes.length, ctl = new AbortController();
    const el = document.createElement("div"); el.className = "convprog"; el.setAttribute("role", "group"); el.setAttribute("aria-label", T("conv.aria"));
    el.innerHTML = `<div class="cp-head"><b data-r="file"></b> <button type="button" class="sec sm" data-r="cancel"></button></div>
      <div class="cp-phase"><span data-r="phase" role="status" aria-live="polite"></span> <span class="cp-beat muted" data-r="beat" role="status" aria-live="polite"></span></div>
      <div class="prog" role="progressbar" aria-valuemin="0" aria-valuemax="100" data-r="bar"><i></i></div><div class="muted sm" data-r="detail"></div>
      <div class="cp-total" data-r="totbox" hidden><div class="muted sm" data-r="totlab"></div><div class="prog" role="progressbar" aria-valuemin="0" aria-valuemax="100" data-r="tot"><i></i></div></div>`;
    const q = s => el.querySelector(`[data-r=${s}]`);
    const cancel = q("cancel"); cancel.textContent = T("conv.cancel"); cancel.title = cancel.ariaLabel = T("conv.cancel.title");
    cancel.onclick = () => { cancel.disabled = true; q("phase").textContent = T("conv.cancelling"); ctl.abort(); };
    box.appendChild(el);
    return {
      signal: ctl.signal,
      start(i, name) {
        const t0 = Date.now();
        let last = null, phase = "", beat = "", changed = t0, key = "", samples = [];
        q("file").textContent = T("conv.file", { i, n, name });
        q("totbox").hidden = n < 2;
        const bar = q("bar"), fill = bar.querySelector("i"), tot = q("tot");
        const setBar = (b, f, pc) => { if (pc == null) { b.removeAttribute("aria-valuenow"); f.style.width = "0%"; } else { b.setAttribute("aria-valuenow", String(pc)); f.style.width = pc + "%"; } };
        const show = () => {
          const now = Date.now(), ph = convPhase(last), pc = convPercent(last), elapsed = convElapsed(now - t0);
          const lab = T(ph === "reading" ? "conv.phase.reading" : "conv.phase.converting");
          if (ph !== phase && !cancel.disabled) { phase = ph; q("phase").textContent = lab; }
          bar.setAttribute("aria-label", lab);
          setBar(bar, fill, pc);
          let d;
          if (last && last.copied != null) d = T("conv.detail.handover", { done: convMB(last.copied), total: convMB(last.size), elapsed });
          else if (ph === "converting") {
            const eta = convEta(samples, now), parts = [pc + "%", T("conv.detail.scans", { done: I18N.fix(last.scans, 0), total: I18N.fix(last.scansTotal, 0) }),
              T("conv.detail.written", { mb: convMB(last.written) }), T("conv.detail.elapsed", { elapsed })];
            if (eta != null) parts.push(T("conv.detail.eta", { eta: convElapsed(eta * 1000) }));
            d = parts.join(" · ");
          } else d = T("conv.detail.wait", { elapsed });
          q("detail").textContent = d;
          if (n > 1) { const tp = convTotalPercent(sizes, i - 1, convFraction(last)); q("totlab").textContent = T("conv.total", { pc: tp == null ? 0 : tp }); setBar(tot, tot.querySelector("i"), tp); tot.setAttribute("aria-label", T("conv.total", { pc: tp == null ? 0 : tp })); }
          const quiet = now - changed, b = quiet >= 30000 ? "stall" : quiet >= 2000 ? "beat" : "";
          if (b !== beat) { beat = b; q("beat").innerHTML = b ? `<span class="cp-spin" aria-hidden="true"></span> <span data-r="beattxt"></span>` : ""; if (b) q("beattxt").textContent = T(b === "stall" ? "conv.stall" : "conv.beat"); }
        };
        show();
        const timer = setInterval(show, 1000);
        return {
          update(p) {
            const k = [p.scans, p.read, p.written, p.copied].join(",");
            if (k !== key) { key = k; changed = Date.now(); }
            last = p;
            if (p.scansTotal) { const f = convFraction(p), z = samples[samples.length - 1]; if (!z || f !== z.f) { samples.push({ t: Date.now(), f }); if (samples.length > 60) samples.shift(); } }
            show();
          },
          done() { clearInterval(timer); },
          fail(msg) { clearInterval(timer); el.remove(); const f = document.createElement("div"); f.className = "fail"; f.textContent = name + ": " + msg; box.appendChild(f); },
        };
      },
      end() { el.remove(); },
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
// seconds left from samples [{ t (ms), f (fraction 0..1) }] of the last seconds (moving average over the last 15 s); null until the
// samples span 4 s and have moved forward
function convEta(samples, now) {
  const w = (samples || []).filter(x => now - x.t <= 15000);
  if (w.length < 2) return null;
  const a = w[0], z = w[w.length - 1], span = z.t - a.t, df = z.f - a.f;
  if (span < 4000 || df <= 0) return null;
  return Math.ceil((1 - z.f) / (df / span) / 1000 - 1e-6);
}
// name of the mzML made from a .raw: "x.raw" -> "x.mzML"; when that name is taken (a real x.mzML in the same loading or already open) the extension
// stays in the key, "x.raw.mzML", so both files are listed. `taken`: names (any case) that are already in use.
function convName(rawName, taken) {
  const base = String(rawName).replace(/\.raw$/i, ""), used = new Set([...(taken || [])].map(n => String(n).toLowerCase()));
  return used.has((base + ".mzML").toLowerCase()) ? base + ".raw.mzML" : base + ".mzML";
}
// "m:ss" or "h:mm:ss" from milliseconds
function convElapsed(ms) {
  const s = Math.max(0, Math.floor(ms / 1000)), h = Math.floor(s / 3600), m = Math.floor(s % 3600 / 60), r = s % 60;
  return (h ? h + ":" + String(m).padStart(2, "0") : String(m)) + ":" + String(r).padStart(2, "0");
}

if (typeof module !== "undefined") module.exports = { convName, convPhase, convFraction, convPercent, convTotalPercent, convEta, convElapsed, convMB };
