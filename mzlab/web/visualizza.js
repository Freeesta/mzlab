// @ts-strict
"use strict";
// «Opzioni di visualizzazione» of the high-resolution world: right click on a chromatogram or a spectrum → a panel of the drawer with the display options of that
// kind of chart, instead of buttons and fields scattered in the bars. EXTENDABLE: another work package adds its options with DISP.register(type, item), one list per
// kind of chart ("spec" for spectra, "chrom" for chromatograms and XIC):
//   { id, label: i18n key (may have a {input}, {select} or {check} slot that receives the control), title?: i18n key,
//     kind: "number" | "select" | "check" | "button", min, max, step, opts: p => [[value, text]], get: p => value, set: (p, value) => …, when?: p => bool }
// Nothing here exists in the low-resolution world (its own parameters popover stays). Classic script, loaded after banco.js; BODY.disp of banco.js calls DISP.panel.
const DISP = (() => {
  const css0 = document.createElement("style");
  css0.textContent = `.dsp{display:flex;flex-direction:column;gap:8px}.dsp .row{display:flex;flex-wrap:wrap;gap:4px 10px;align-items:center}.dsp label{display:inline-flex;align-items:center;gap:4px;flex-wrap:wrap}.dsp input[type=number]{width:56px;padding:2px 5px}.dsp select{padding:2px 4px;max-width:100%}.dsp button{padding:2px 8px}`;
  document.head.appendChild(css0);
  const t = (k, p) => I18N.t(k, p);
  /** @type {{ spec: any[], chrom: any[] }} */ const items = { spec: [], chrom: [] };
  const register = (type, it) => { const l = items[type] || (items[type] = []), i = l.findIndex(x => x.id === it.id); if (i >= 0) l[i] = it; else l.push(it); };
  const typeOf = p => (!p ? null : p.type === "spec" ? "spec" : p.type === "chrom" || p.type === "xic" ? "chrom" : null);
  const redraw = p => { draw(p); if (typeof uiSave === "function") uiSave(); };
  const clamp = (v, a, b) => Math.max(a, Math.min(b, v));

  // ---- spectrum: what the ▾ button of the spectrum and the decimals field of the bar had (the low-resolution popover is in spettro.js)
  register("spec", { id: "dec", label: "spec.par.dec", kind: "select", opts: () => [3, 4, 5].map(n => [n, String(n)]), get: () => Math.round(UIP.hrDec || 4),
    set: (p, v) => { UIP.hrDec = clamp(+v || 4, 3, 5); try { uipSave(); } catch (e) { /* no storage */ } redrawAll(); } });
  register("spec", { id: "thr", label: "spec.par.thr", title: "spec.par.thr.title", kind: "number", min: 0, max: 100, step: 1, get: p => p.thr ?? SPEC_DEF.thr, set: (p, v) => { p.thr = clamp(Math.round(+v || 0), 0, 100); redraw(p); } });
  register("spec", { id: "nlab", label: "spec.par.nlab", title: "spec.par.nlab.title", kind: "number", min: 1, max: 60, step: 1, get: p => p.nlab ?? SPEC_DEF.nlab, set: (p, v) => { p.nlab = clamp(Math.round(+v || 1), 1, 60); redraw(p); } });
  register("spec", { id: "sticks", label: "spec.par.sticks", title: "spec.par.sticks.title", kind: "select", when: p => (p._a && p._a.data || []).some(x => x.d.pmz),
    opts: () => [["0", t("spec.par.sticks.profile")], ["1", t("spec.par.sticks.sticks")]], get: p => (p.sticks ? "1" : "0"), set: (p, v) => { p.sticks = v === "1"; redraw(p); } });
  register("spec", { id: "marker", label: "disp.marker", title: "disp.marker.title", kind: "check", when: p => !!(p._a && p._a.markN != null), get: p => markOn(p), set: (p, v) => { p.mk = !!v; redraw(p); } });
  register("spec", { id: "nocont", label: "disp.noCont", title: "disp.noCont.title", kind: "check", when: p => !!p._a && !!p._a.hrp, get: p => !!p.noCont, set: (p, v) => { p.noCont = !!v; redraw(p); } });
  register("spec", { id: "rel", label: "disp.rel", kind: "check", get: p => !!specRel(p), set: (p, v) => { p.rel = !!v; redraw(p); } });
  // ---- chromatogram and XIC
  register("chrom", { id: "norm100", label: "disp.norm100", kind: "check", get: p => !!p.norm100, set: (p, v) => { p.norm100 = !!v; redraw(p); } });

  // ---- PDA in series before the mass spectrometer: the delay is measured by the server (cross-correlation of PDA and TIC of the file) and can be corrected by hand;
  // «Allinea PDA e massa» only moves the time axis of the PDA trace on the screen (data and results stay as they are)
  const pdel = new Map();
  const dfile = p => (typeof shownFor === "function" ? shownFor(p).find(f => f.pda) : null) || null;
  function measured(p) {
    const f = dfile(p); if (!f) return null;
    if (!pdel.has(f.k)) { pdel.set(f.k, null); J(`api/delay?k=${f.k}`).then(j => { pdel.set(f.k, j.delay || false); if (window.BANCO) BANCO.renderInfo(); if (p.pdaAlign) draw(p); }).catch(() => pdel.set(f.k, false)); }
    return pdel.get(f.k) || null;
  }
  const pdaShift = p => { if (!p.pdaAlign) return 0; const m = measured(p); return p.pdaDelay != null ? p.pdaDelay : m ? m.delay : 0; };
  register("chrom", { id: "pdadelay", label: "disp.pdaDelay", title: "disp.pdaDelay.title", kind: "number", min: -2, max: 2, step: 0.01, when: p => p.kind === "pda" && !!dfile(p),
    get: p => { const m = measured(p); return p.pdaDelay != null ? p.pdaDelay : m ? m.delay : 0; },
    set: (p, v) => { const n = numIt(v); p.pdaDelay = isFinite(n) ? clamp(n, -2, 2) : null; redraw(p); },
    note: p => { const m = measured(p); return m ? t("disp.pdaDelay.found", { delay: m.delay.toFixed(2), n: m.peaks, corr: m.corr.toFixed(2) }) : t("disp.pdaDelay.none"); } });
  register("chrom", { id: "pdaalign", label: "disp.pdaAlign", title: "disp.pdaAlign.title", kind: "check", when: p => p.kind === "pda" && !!dfile(p), get: p => !!p.pdaAlign, set: (p, v) => { p.pdaAlign = !!v; redraw(p); } });
  // ---- gradient over the chromatogram: the delay at the column is typed by the student (the program does not estimate the dwell volume)
  register("chrom", { id: "gdelay", label: "disp.gdelay", title: "disp.gdelay.title", kind: "number", min: 0, max: 10, step: 0.1, when: p => p.overlay === "grad", get: p => p.gdelay || 0,
    set: (p, v) => { p.gdelay = clamp(Math.round((numIt(v) || 0) * 10) / 10, 0, 10); redraw(p); } });
  // ---- XIC in high resolution: the ion and its interval (the header only says «XIC • m/z value»); the interval is per trace, in ppm or in Da
  const xt = p => (p && p.type === "xic" ? p.traces[Math.min(p._xt || 0, p.traces.length - 1)] : null);
  const xhr = p => !!(window.HR && p && p.type === "xic" && HR.xicFiles(p.tab).some(x => HR.isHr(x, x.lv)));
  const xunit = (p, tr) => (tr.ion ? (tr.wda != null ? "da" : "ppm") : (p._xu || "da"));
  const xrefresh = (p, tr) => {
    const hf = HR.xicFiles(p.tab).find(x => HR.isHr(x, x.lv));
    tr.label = tr.ion && hf ? HR.ionText(tr.mz, hf, tr) : `m/z ${(+tr.mz).toFixed(HR.dec(HR.xicFiles(p.tab), 1))}`;
    ctl(p); redraw(p); if (window.BANCO) BANCO.renderInfo();
  };
  register("chrom", { id: "xmz", label: "disp.xmz", title: "disp.xmz.title", kind: "number", min: 0, step: 0.0001, when: p => xhr(p) && !!xt(p), get: p => (+xt(p).mz).toFixed(HR.dec(HR.xicFiles(p.tab), 1) + 1),
    set: (p, v) => { const tr = xt(p), n = numIt(v); if (!(n > 0)) return; tr.mz = rh(n, 5); xrefresh(p, tr); } });
  register("chrom", { id: "xunit", label: "disp.xunit", kind: "select", when: p => xhr(p) && !!xt(p), opts: () => [["ppm", "ppm"], ["da", "Da"]], get: p => xunit(p, xt(p)),
    set: (p, v) => {                                          // same interval, written in the other unit
      const tr = xt(p), cur = xunit(p, tr); if (cur === v) return;
      const f = HR.xicFiles(p.tab).find(x => HR.isHr(x, x.lv)), tol = tr.ion ? (tr.ppm ?? HR.prof(f, f.lv).tol) : (tr.w ?? p.tol) / tr.mz * 1e6;
      if (tr.ion) { if (v === "da") tr.wda = rh(tr.mz * tol * 1e-6, 5); else { delete tr.wda; tr.ppm = tol; } } else p._xu = v;
      xrefresh(p, tr); } });
  register("chrom", { id: "xtol", label: "disp.xtol", title: "disp.xtol.title", kind: "number", min: 0, step: 0.1, when: p => xhr(p) && !!xt(p),
    get: p => { const tr = xt(p), u = xunit(p, tr), f = HR.xicFiles(p.tab).find(x => HR.isHr(x, x.lv)); return u === "da" ? +(tr.ion ? tr.wda : (tr.w ?? p.tol)).toFixed(5) : +(tr.ion ? (tr.ppm ?? HR.prof(f, f.lv).tol) : (tr.w ?? p.tol) / tr.mz * 1e6).toFixed(2); },
    set: (p, v) => { const tr = xt(p), n = numIt(v); if (!(n > 0)) return; const u = xunit(p, tr);
      if (tr.ion) { if (u === "da") tr.wda = rh(n, 5); else tr.ppm = n; } else tr.w = u === "da" ? rh(n, 5) : rh(tr.mz * n * 1e-6, 5);
      xrefresh(p, tr); } });

  function control(it, p, k) {
    const v = it.get(p), id = `dsp-${k}`;
    if (it.kind === "number") return `<input id="${id}" data-k="${k}" type="number" min="${it.min}" max="${it.max}" step="${it.step || 1}" value="${EH(v)}">`;
    if (it.kind === "select") return `<select id="${id}" data-k="${k}">${it.opts(p).map(([o, l]) => `<option value="${EH(o)}"${String(o) === String(v) ? " selected" : ""}>${EH(l)}</option>`).join("")}</select>`;
    if (it.kind === "check") return `<input id="${id}" data-k="${k}" type="checkbox"${v ? " checked" : ""}>`;
    return `<button id="${id}" data-k="${k}" type="button">${t(it.label)}</button>`;
  }
  // the body of the drawer panel for the cell p
  function panel(b, p) {
    const ty = typeOf(p), list = (ty ? items[ty] : []).filter(it => !it.when || it.when(p));
    if (!ty) { b.innerHTML = `<div class="muted">${t("disp.none")}</div>`; return; }
    b.innerHTML = `<div class="dsp"><div class="muted sm">${t("disp.for", { type: t(ty === "spec" ? "disp.type.spec" : "disp.type.chrom") })}</div>
      ${list.map((it, k) => {
        const c = control(it, p, k), title = it.title ? ` title="${EH(t(it.title))}"` : "";
        if (it.kind === "button") return `<div class="row">${c}</div>`;
        const note = it.note ? `<div class="sm muted">${EH(it.note(p))}</div>` : "";
        if (it.kind === "check") return `<div class="row"><label${title}>${c} ${t(it.label)}</label></div>${note}`;
        return `<div class="row"><label${title}>${t(it.label, { input: c, select: c })}</label></div>${note}`;
      }).join("")}</div>`;
    b.querySelectorAll("[data-k]").forEach(el => {
      const it = list[+el.dataset.k];
      if (it.kind === "button") el.onclick = () => it.set(p);
      else el.onchange = () => it.set(p, it.kind === "check" ? el.checked : el.value);
    });
  }
  // right click entry / palette command: the panel for the chart p (default: the active cell)
  function open(p) {
    if (p && typeof setActive === "function") setActive(p);
    if (window.BARRA) BARRA.setTab("disp");
    if (window.BANCO) BANCO.renderInfo();
  }
  return { register, panel, open, items, typeOf, pdaShift, pdaHere: p => !!dfile(p), pdaDelayValue: p => { const m = measured(p); return p.pdaDelay != null ? p.pdaDelay : m ? m.delay : 0; } };
})();
window.DISP = DISP;
