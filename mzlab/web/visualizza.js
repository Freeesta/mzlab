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
  const items = { spec: [], chrom: [] };
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
  register("spec", { id: "rel", label: "disp.rel", kind: "check", get: p => !!specRel(p), set: (p, v) => { p.rel = !!v; redraw(p); } });
  // ---- chromatogram and XIC
  register("chrom", { id: "norm100", label: "disp.norm100", kind: "check", get: p => !!p.norm100, set: (p, v) => { p.norm100 = !!v; redraw(p); } });

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
        if (it.kind === "check") return `<div class="row"><label${title}>${c} ${t(it.label)}</label></div>`;
        return `<div class="row"><label${title}>${t(it.label, { input: c, select: c })}</label></div>`;
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
  return { register, panel, open, items, typeOf };
})();
window.DISP = DISP;
