// @ts-strict
"use strict";
// Shared panel bar #pbar (study docs/studies/PANEL_TOOLBAR_STUDY.md §3), lazy, loaded only when the setting «Panel bar» is on (settings.js, localStorage mzlab.pbar = "1").
// One row under the header: a chip with the number, kind and file of the selected panel (E.active) and the tools of that panel, drawn by AZIONI.bar (azioni-barra.js).
// The header buttons of every panel stay in the DOM (hidden by CSS, html[data-pbar]): their state is copied to the bar, and a button without its own action runs the header one.
// Classic script, uses by name: E, Q, setActive, tabPanels, plotFiles, ptitleP, LAYOUT, AZIONI, I18N, FRAME.
(() => {
  /** @typedef {any} Pannello */
  const T = (/** @type {string} */ k, /** @type {any} */ a = {}) => I18N.t(k, a), KEY = NOMI.KEY("pbar");
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
    const h = hdr(p), b = host(); if (!h || !b) return;
    for (const t of /** @type {NodeListOf<HTMLElement>} */ (b.querySelectorAll(".pbtools [data-a]"))) {
      const s = /** @type {HTMLElement | null} */ (h.querySelector(`[data-a="${t.dataset.a}"]`)); if (!s) continue;
      if (t.className !== s.className) t.className = s.className;
      if (t.innerHTML !== s.innerHTML) t.innerHTML = s.innerHTML;
      for (const k of ["title", "aria-label", "aria-pressed", "aria-expanded"]) { const v = s.getAttribute(k); if (v == null) t.removeAttribute(k); else if (t.getAttribute(k) !== v) t.setAttribute(k, v); }
      if (t.hidden !== s.hidden) t.hidden = s.hidden;
      if ("disabled" in s && /** @type {HTMLButtonElement} */ (t).disabled !== /** @type {HTMLButtonElement} */ (s).disabled) /** @type {HTMLButtonElement} */ (t).disabled = /** @type {HTMLButtonElement} */ (s).disabled;
      if (s.tagName === "SELECT") /** @type {HTMLSelectElement} */ (t).value = /** @type {HTMLSelectElement} */ (s).value;
    }
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
    if (!p) { b.innerHTML = `<span class="pbhint">${EH(T("pbar.hint"))}</span>`; b.setAttribute("aria-label", T("pbar.aria.none")); delete b.dataset.pair; return; }
    b.innerHTML = `<span class="pbchip" title="${EH(T("pbar.chip.title"))}"></span><span class="pbtools"></span>`;
    AZIONI.bar({ view: p.type, p }, /** @type {HTMLElement} */ (b.querySelector(".pbtools")), "e");
    const tb = /** @type {HTMLElement} */ (b.querySelector(".pbtools"));
    for (const t of /** @type {NodeListOf<HTMLElement>} */ (tb.querySelectorAll("[data-a]"))) {   // a button without its own action runs the header twin (the XIC button of a spectrum)
      const s = /** @type {HTMLElement | null} */ (hdr(p)?.querySelector(`[data-a="${t.dataset.a}"]`));
      if (!t.onclick && !t.onchange && s) t.onclick = e => s.dispatchEvent(new MouseEvent("click", { bubbles: true, shiftKey: e.shiftKey }));
    }
    paintChip(p); mirror(p); rove(null);
    const h = hdr(p); if (h) { mo = new MutationObserver(() => later()); mo.observe(p.el.querySelector(".hd") || h, { subtree: true, childList: true, attributes: true, characterData: true }); }
  }
  /** follows E.active: the first panel when none is; the bar is rebuilt only when the selected panel changes */
  function sync() {
    if (!on) return;
    let p = E.active && E.panels.includes(E.active) && E.active.el ? E.active : null;
    if (!p) { const q = first(); if (q) { setActive(q); return; } }
    if (p !== shown || !host()?.firstElementChild) render(p); else if (p) { paintChip(p); mirror(p); rove(/** @type {HTMLElement | null} */ (document.activeElement)); }
  }
  const later = () => { if (!raf) raf = FRAME.next(() => { raf = 0; sync(); }); };
  function set(/** @type {boolean} */ v) {
    on = v; document.documentElement.toggleAttribute("data-pbar", v);
    const b = host(); if (!b) return;
    b.hidden = !v; shown = null; if (mo) { mo.disconnect(); mo = null; }
    if (v) sync(); else b.textContent = "";
  }
  const key = (/** @type {KeyboardEvent} */ e) => {   // arrows, Home and End move inside the bar (one Tab stop)
    const l = tools(), i = l.indexOf(/** @type {HTMLElement} */ (document.activeElement)); if (i < 0 || /SELECT/.test(document.activeElement?.tagName || "")) return;
    const j = e.key === "ArrowRight" ? (i + 1) % l.length : e.key === "ArrowLeft" ? (i + l.length - 1) % l.length : e.key === "Home" ? 0 : e.key === "End" ? l.length - 1 : -1;
    if (j < 0) return; e.preventDefault(); rove(l[j]); l[j].focus();
  };
  host()?.addEventListener("keydown", key);
  document.addEventListener("focusin", e => {   // focus inside a panel selects it
    if (!on) return; const el = /** @type {Element} */ (e.target).closest?.(".pnl"); if (!el) return;
    const q = E.panels.find((/** @type {Pannello} */ x) => x.el === el); if (q && q !== E.active) setActive(q);
  });
  window.PBAR = { sync, set, on: () => on, height: () => (on && host() && !host()?.hidden ? host()?.offsetHeight || 0 : 0), KEY };
  set((() => { try { return localStorage.getItem(KEY) === "1"; } catch (_) { return false; } })());
})();
