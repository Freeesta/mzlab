// @ts-strict
"use strict";
// Shared panel bar #pbar (study docs/studies/PANEL_TOOLBAR_STUDY.md §3), lazy, loaded only when the setting «Panel bar» is on (settings.js, localStorage mzlab.pbar = "1").
// One row under the header: a chip with the number, kind and file of the selected panel (E.active) and the tools of that panel, drawn by AZIONI.bar (azioni-barra.js) in the groups View, Analyse, Extract, Output.
// A group shows only when the kind has an action in it, and an action that is not available for the panel is hidden, never greyed (the rule of AZIONI.clean for the menus).
// The header buttons of every panel stay in the DOM (hidden by CSS, html[data-pbar]): their state is copied to the bar, and a button without its own action runs the header one.
// Expand is in the bar (Output) and in the header, close only in the header. A panel whose smoothing, log scale or zoom link is not the default shows a small dot chip in its header (the state of the others stays visible).
// Classic script, uses by name: E, Q, setActive, tabPanels, plotFiles, ptitleP, fitHdSoon, fitHdNow, LAYOUT, AZIONI, I18N, FRAME, ESP.
(() => {
  /** @typedef {any} Pannello */
  const T = (/** @type {string} */ k, /** @type {any} */ a = {}) => I18N.t(k, a), KEY = NOMI.KEY("pbar");
  /** group of the bar for each header group («s», the spectrum tools, splits by action: peak picking is Analyse); more and close stay in the header. The DDA buttons (dd) and the TIC background (tb) extract a trace or a list from the data, like the XIC: Extract */
  const PB = /** @type {Record<string, string>} */ ({ xb: "x", v: "v", l: "v", s: "v", i: "a", t: "v", x: "x", o: "o", e: "o", dd: "x", tb: "x" });
  /** the groups of the bar: View, Analyse, Extract, Output, each labelled, drawn only when the kind has an action in it */
  const GRP = { of: (/** @type {any} */ a) => a.bid === "more" || a.nodata ? "" : a.bid === "par" ? "a" : PB[a.bar] || "", groups: [["v", "pbar.g.view"], ["a", "pbar.g.analyse"], ["x", "pbar.g.extract"], ["o", "pbar.g.output"]],
    wrap: (/** @type {string} */ k, /** @type {string} */ t, /** @type {string} */ s) => h`<span class="tbg" role="group" data-pg="${k}" aria-label="${T(t)}"><span class="tbl" aria-hidden="true">${T(t)}</span>${raw(s)}</span>` };
  /** @type {Pannello | null} */ let shown = null;
  /** @type {MutationObserver | null} */ let mo = null;
  let on = false, raf = 0, said = "";
  const host = () => /** @type {HTMLElement | null} */ (document.getElementById("pbar"));
  const first = () => /** @type {Pannello | undefined} */ (tabPanels().filter((/** @type {Pannello} */ q) => q.el).sort(LAYOUT.ordine)[0]);
  const hdr = (/** @type {Pannello} */ p) => /** @type {HTMLElement | null} */ (p.el.querySelector(".tbs"));
  /** panel number, kind and file: «2 · TIC · sample A» @param {Pannello} p */
  const label = p => {
    const fs = plotFiles(p), file = fs.length > 1 ? T("pbar.files", { n: fs.length }) : fs[0] ? fs[0].label : "";
    return { n: String(p.num || ""), kind: ptitleP(p), file };
  };
  const live = (/** @type {string} */ msg) => {
    let a = document.getElementById("sb-live");
    if (!a) { a = document.createElement("div"); a.id = "sb-live"; a.setAttribute("role", "status"); a.setAttribute("aria-live", "polite"); a.style.cssText = "position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap"; document.body.appendChild(a); }
    a.textContent = ""; setTimeout(() => { if (a) a.textContent = msg; }, 30);
  };
  /** the state of each header button (class, disabled, hidden, title, aria, content) copied to its twin in the bar @param {Pannello} p */
  function mirror(p) {
    const hd = hdr(p), b = host(); if (!hd || !b) return;
    for (const t of /** @type {NodeListOf<HTMLElement>} */ (b.querySelectorAll(".pbtools [data-a]"))) {
      const s = /** @type {HTMLElement | null} */ (hd.querySelector(`[data-a="${t.dataset.a}"]`)); if (!s) continue;
      const tbg = t.dataset.a === "ticbg" && /** @type {any} */ (window).TICBG; if (tbg) tbg.wire(p, t);   // long press, right click and ↓ open the options from the bar too (the module is lazy: wired once it has loaded)
      if (t.className !== s.className) t.className = s.className;
      if (t.innerHTML !== s.innerHTML) t.innerHTML = s.innerHTML;
      for (const k of ["title", "aria-label", "aria-pressed", "aria-expanded"]) { const v = s.getAttribute(k); if (v == null) t.removeAttribute(k); else if (t.getAttribute(k) !== v) t.setAttribute(k, v); }
      const act = AZIONI.get("panel." + t.dataset.a), gone = s.hidden || !!(act && act.available && !act.available({ view: p.type, p }));
      if (t.hidden !== gone) t.hidden = gone;
      if ("disabled" in s && /** @type {HTMLButtonElement} */ (t).disabled !== /** @type {HTMLButtonElement} */ (s).disabled) /** @type {HTMLButtonElement} */ (t).disabled = /** @type {HTMLButtonElement} */ (s).disabled;
      if (s.tagName === "SELECT") /** @type {HTMLSelectElement} */ (t).value = /** @type {HTMLSelectElement} */ (s).value;
    }
    for (const g of /** @type {NodeListOf<HTMLElement>} */ (b.querySelectorAll(".pbtools .tbg"))) { const e = ![...g.querySelectorAll("[data-a]")].some(x => !(/** @type {HTMLElement} */ (x)).hidden); if (g.hidden !== e) g.hidden = e; }   // a group with nothing to show is not drawn
  }
  /** the states that differ from the default, as [id, text key]: smoothing is on by default but for the SIM/MRM panels @param {Pannello} p */
  const odd = p => {
    const l = /** @type {[string, string, string][]} */ ([]); if (!["chrom", "xic", "mrm"].includes(p.type)) return l;
    if (!!p.smooth !== !ESP.mrm(p)) l.push(p.smooth ? ["smooth", "pbar.dot.smoothon", "pbar.dot.smoothon.title"] : ["smooth", "pbar.dot.smoothoff", "pbar.dot.smoothoff.title"]);
    if (p.log) l.push(["log", "pbar.dot.log", "pbar.dot.log.title"]);
    if (p.tl) l.push(["tl", "pbar.dot.tl", "pbar.dot.tl.title"]);
    return l;
  };
  /** the dot chips of the header of p (none when the bar is off or the panel has the defaults); written only when they changed @param {Pannello} p */
  function dots(p) {
    const hd = p.el?.querySelector(".hd"); if (!hd) return;
    let d = /** @type {HTMLElement | null} */ (hd.querySelector(".pdots")); const l = on ? odd(p) : [], sig = l.map(x => x[1]).join();
    if (!l.length) { d?.remove(); return; }
    if (!d) { d = document.createElement("span"); d.className = "pdots"; d.setAttribute("role", "group"); d.setAttribute("aria-label", T("pbar.dot.group")); hd.insertBefore(d, hd.querySelector(".tbs")); }
    if (d.dataset.sig !== sig) { d.dataset.sig = sig; d.innerHTML = l.map(([id, k, t]) => h`<span class="pdot" data-d="${id}" title="${T(t)}">${T(k)}</span>`).join(""); fitHdSoon(p); }
  }
  /** the chip: number, kind, file; the text is rewritten only when it changed @param {Pannello} p */
  function paintChip(p) {
    const b = host(), c = b && b.querySelector(".pbchip"); if (!b || !c) return;
    const l = label(p), txt = [l.n, l.kind, l.file].filter(Boolean).join(" · "), name = [l.kind, l.file].filter(Boolean).join(" · ");
    if (c.textContent !== txt) c.textContent = txt;
    b.setAttribute("aria-label", T("pbar.aria", { n: l.n, name }));
    const pair = p.el.dataset.pair; if (pair) b.dataset.pair = pair; else delete b.dataset.pair;
    if (said !== txt) { if (said) live(T("pbar.say", { n: l.n, name })); said = txt; }
  }
  /** one tab stop: the first enabled tool takes it, the arrows move it (APG toolbar) */
  const tools = () => /** @type {HTMLElement[]} */ ([...(host()?.querySelectorAll(".pbtools button, .pbtools select") || [])].filter(t => !t.hidden && !(/** @type {HTMLButtonElement} */ (t)).disabled));
  const rove = (/** @type {HTMLElement | null} */ cur) => { const l = tools(), c = cur && l.includes(cur) ? cur : l[0]; host()?.querySelectorAll(".pbtools button, .pbtools select").forEach(t => { /** @type {HTMLElement} */ (t).tabIndex = t === c ? 0 : -1; }); };
  /** the bar for the panel p (null: the hint) */
  function render(/** @type {Pannello | null} */ p) {
    const b = host(); if (!b) return;
    if (mo) { mo.disconnect(); mo = null; }
    shown = p; said = p ? said : "";
    if (!p) { b.innerHTML = h`<span class="pbhint">${T("pbar.hint")}</span>`; b.setAttribute("aria-label", T("pbar.aria.none")); delete b.dataset.pair; return; }
    b.innerHTML = h`<span class="pbchip" title="${T("pbar.chip.title")}"></span><span class="pbtools"></span>`;
    AZIONI.bar({ view: p.type, p }, /** @type {HTMLElement} */ (b.querySelector(".pbtools")), GRP);
    const tb = /** @type {HTMLElement} */ (b.querySelector(".pbtools"));
    for (const t of /** @type {NodeListOf<HTMLElement>} */ (tb.querySelectorAll("[data-a]"))) {   // a button without its own action runs the header twin (the XIC button of a spectrum)
      const s = /** @type {HTMLElement | null} */ (hdr(p)?.querySelector(`[data-a="${t.dataset.a}"]`));
      if (!t.onclick && !t.onchange && s) t.onclick = e => s.dispatchEvent(new MouseEvent("click", { bubbles: true, shiftKey: e.shiftKey }));
    }
    paintChip(p); mirror(p); rove(null);
    const hd = hdr(p); if (hd) { mo = new MutationObserver(() => later()); mo.observe(p.el.querySelector(".hd") || hd, { subtree: true, childList: true, attributes: true, characterData: true }); }
  }
  /** follows E.active: the first panel when none is; the bar is rebuilt only when the selected panel changes */
  function sync() {
    if (!on) return;
    for (const q of E.panels) dots(q);
    let p = E.active && E.panels.includes(E.active) && E.active.el ? E.active : null;
    if (!p) { const q = first(); if (q) { setActive(q); return; } }
    if (p !== shown || !host()?.firstElementChild) render(p); else if (p) { paintChip(p); mirror(p); rove(/** @type {HTMLElement | null} */ (document.activeElement)); }
  }
  const later = () => { if (!raf) raf = FRAME.next(() => { raf = 0; sync(); }); };
  function set(/** @type {boolean} */ v) {
    on = v; document.documentElement.toggleAttribute("data-pbar", v);
    const b = host(); if (!b) return;
    b.hidden = !v; shown = null; if (mo) { mo.disconnect(); mo = null; }
    if (v) sync(); else { b.textContent = ""; for (const q of E.panels) dots(q); }
    for (const q of E.panels) if (q.el) fitHdNow(q);   // the tool groups leave the header or come back: fit it again
  }
  const key = (/** @type {KeyboardEvent} */ e) => {   // arrows, Home and End move inside the bar (one Tab stop)
    const l = tools(), i = l.indexOf(/** @type {HTMLElement} */ (document.activeElement)); if (i < 0 || /SELECT/.test(document.activeElement?.tagName || "")) return;
    const j = e.key === "ArrowRight" ? (i + 1) % l.length : e.key === "ArrowLeft" ? (i + l.length - 1) % l.length : e.key === "Home" ? 0 : e.key === "End" ? l.length - 1 : -1;
    if (j < 0) return; e.preventDefault(); rove(l[j]); l[j].focus();
  };
  host()?.addEventListener("keydown", key);
  // a state button of any panel changed (the header twins keep their class): redraw the dot chips; a new panel joins as a child of #dpanels
  { const st = document.getElementById("dpanels"); if (st) new MutationObserver(() => later()).observe(st, { childList: true, subtree: true, attributes: true, attributeFilter: ["class", "aria-pressed"] }); }
  document.addEventListener("focusin", e => {   // focus inside a panel selects it
    if (!on) return; const el = /** @type {Element} */ (e.target).closest?.(".pnl"); if (!el) return;
    const q = E.panels.find((/** @type {Pannello} */ x) => x.el === el); if (q && q !== E.active) setActive(q);
  });
  window.PBAR = { sync, set, on: () => on, height: () => (on && host() && !host()?.hidden ? host()?.offsetHeight || 0 : 0), KEY };
  set((() => { try { return localStorage.getItem(KEY) === "1"; } catch (_) { return false; } })());
})();
