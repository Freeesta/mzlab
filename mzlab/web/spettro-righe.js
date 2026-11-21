// @ts-check
// Rows of the MS² spectrum card (never an MS¹ card) under its header, one topic each: notes, retention times, collision energy (CE ± spread), sample.
// The fragments row is the header's own controls (`.ctl`, wrapped onto its own line by CSS). CE, spread and sample come from the engine (`ms2_exps`, label).
(() => {
  const T = (/** @type {string} */ k, /** @type {any} */ a) => I18N.t(k, a);
  const num = (/** @type {number} */ v) => String(+v.toFixed(2));
  /** @param {any} p @param {string} cls @param {string} label @param {Node | string} body */
  function row(p, cls, label, body) {
    let r = /** @type {HTMLElement | null} */ (p.el.querySelector(`.srows > .${cls}`));
    if (!r) {
      r = document.createElement("div"); r.className = `sr ${cls}`; r.setAttribute("role", "group");
      const l = document.createElement("span"); l.className = "srl"; r.append(l, document.createElement("span"));
      p.el.querySelector(".srows").appendChild(r);
    }
    r.setAttribute("aria-label", label); r.firstElementChild.textContent = label;
    r.lastElementChild.replaceChildren(body);
    return r;
  }
  /** the RT limits of the integrated peaks of the linked chromatogram that touch the window of the spectrum @param {any} p */
  function peaks(p) {
    const c = E.panels.find((/** @type {any} */ q) => q.id === p.link); if (!c || p.r0 == null) return [];
    return (c.ints || []).filter((/** @type {any} */ i) => i.k === p.k && Math.max(i.a, i.b) >= p.r0 && Math.min(i.a, i.b) <= p.r1)
      .map((/** @type {any} */ i) => T("srow.rt.peak", { a: num(Math.min(i.a, i.b)), b: num(Math.max(i.a, i.b)) }));
  }
  /** @param {any} p */
  function update(p) {
    if (p.type !== "spec" || !p.el) return;
    const hd = p.el.querySelector(".hd"); if (!hd) return;
    // MS² cards only: an MS¹ card keeps its single header row, with no extra rows (the CSS is scoped to .ms2c)
    const on = p.level > 1; p.el.classList.toggle("ms2c", on);
    if (!on) { p.el.querySelector(".srows")?.remove(); return; }
    let box = p.el.querySelector(".srows");
    if (!box) { box = document.createElement("div"); box.className = "srows"; hd.after(box); }
    const ctl = p.el.querySelector(".ctl"); if (ctl) { ctl.setAttribute("role", "group"); ctl.setAttribute("aria-label", T("srow.frag")); }
    const f = E.files[p.k], ms2 = p.level > 1 && !!f && !p.all;
    // notes (kept in p.note, saved with the panel)
    let inp = /** @type {HTMLInputElement | null} */ (box.querySelector(".sr-notes input"));
    if (!inp) {
      inp = document.createElement("input"); inp.type = "text"; inp.maxLength = 300; inp.setAttribute("aria-label", T("srow.notes")); inp.title = T("srow.notes.title"); inp.placeholder = T("srow.notes.ph");
      inp.oninput = () => { p.note = inp.value; clearTimeout(p._nt); p._nt = setTimeout(() => window.uiSave?.(), 600); };
      row(p, "sr-notes", T("srow.notes"), inp);
    }
    if (inp.value !== (p.note || "") && document.activeElement !== inp) inp.value = p.note || "";
    // retention times: the window of the spectrum and the chromatographic peaks it comes from
    if (p.r0 != null) {
      const wide = p.r1 - p.r0 > 1.6 * scanStep(), w = wide ? `${num(p.r0)}–${num(p.r1)} min` : `${num((p.r0 + p.r1) / 2)} min`;
      row(p, "sr-rt", T("srow.rt"), [T("srow.rt.win", { rt: w }), ...peaks(p)].join(" · "));
    } else box.querySelector(".sr-rt")?.remove();
    // collision energy: only from the file; no spread, only CE; neither, no row
    const e = ms2 && p.prec != null ? (f.ms2_exps || []).find((/** @type {any} */ x) => Math.abs(x.prec - p.prec) < 0.6) : null;
    const ce = !e ? null : e.ce_spread != null && e.ce_mid != null ? T("srow.ce.spread", { ce: num(e.ce_mid), sd: num(e.ce_spread) }) : e.ce != null ? T("srow.ce.one", { ce: num(e.ce) }) : null;
    if (ce) row(p, "sr-ce", T("srow.ce"), ce); else box.querySelector(".sr-ce")?.remove();
    if (ms2) row(p, "sr-smp", T("srow.sample"), f.label || ""); else box.querySelector(".sr-smp")?.remove();
  }
  window.SPECROWS = { update };
})();
