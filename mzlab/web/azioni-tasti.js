// @ts-strict
// @ts-check
"use strict";
// The shortcuts of the registry actions/1 that no older module owns, and the panel «Shortcuts» (key ?) built from the registry itself.
// The keys live in the field `shortcut` of each action; ONE keydown in azioni.js runs them (letters only with the focus on a chart, never in a text field;
// the combinations with Ctrl/⌘ or Alt anywhere in the view). Existing keys win: a chart handler that already used the key has run (and prevented the default) before.
// Classic script, loaded after azioni.js. Short labels live in lang/ (scorc.*), the explanations in the tooltips.
(() => {
  /** @param {string} k @param {any} [v] */
  const T = (k, v) => I18N.t(k, v);
  /** @param {any} o */
  const reg = o => AZIONI.register({ menu: false, ...o });
  // the same actions that the right-click menu offers, now with a key (X, Enter): the menu keeps them, the key is a field of their registration
  const V = ["chrom", "xic", "mrm"];
  // integration at the cursor: the automatic one, as the entry «Integrate automatically» of the menu
  reg({ id: "chrom.intAuto", group: "tools", order: 1, views: V, iconKey: "area", shortcut: { key: "i", where: "chart", label: "scorc.integra" },
    name: () => T("scorc.integra"), tip: () => T("scorc.integra.tip"), available: (/** @type {any} */ c) => typeof autoInt === "function" && typeof intTargets === "function" && !!c.p._a,
    run: (/** @type {any} */ c) => { if (!guardInt(c.p)) return; intTargets(c.p, c.x, c.py).forEach((/** @type {any} */ s) => autoInt(c.p, s, c.x)); } });
  // measure from the highest visible peak (like «Measure from this peak» of the menu)
  reg({ id: "spec.measureKey", group: "tools", views: ["spec"], iconKey: "measure", shortcut: { key: "r", where: "chart", label: "scorc.misura" },
    name: () => T("scorc.misura"), tip: () => T("scorc.misura.tip"), available: (/** @type {any} */ c) => c.has && typeof measSet === "function", run: (/** @type {any} */ c) => measSet(c.p, c.m) });
  // full screen on every panel (the map keeps its own F, which runs first)
  reg({ id: "view.fullscreen", group: "view", views: ["*"], iconKey: "wide", shortcut: { key: "f", where: "chart", label: "scorc.schermo" },
    name: () => T("scorc.schermo"), tip: () => T("scorc.schermo.tip"), available: (/** @type {any} */ c) => !!(c.p && c.p._setMax),
    run: (/** @type {any} */ c) => c.p._setMax(!c.p.el.classList.contains("max")) });
  // whole view of the active chart (Backspace; with a peak selected it deletes the peak first) and Delete of the selected peak: keys of the view, as the zoom history is
  const attivo = () => { const a = typeof E !== "undefined" ? E.active : null; return a && a.el && document.getElementById("dpanels")?.offsetParent ? a : null; };
  const scelto = (/** @type {any} */ a) => a && a.isel != null && a.ints && a.ints.find((/** @type {any} */ i) => i.id === a.isel);
  reg({ id: "chart.intera", group: "view", views: ["*"], iconKey: "wide", shortcut: { key: "Backspace", where: "view", label: "scorc.intera" },
    name: () => T("scorc.intera"), tip: () => T("scorc.intera.tip"), available: () => !!attivo(),
    run: () => { const a = attivo(), s = scelto(a); if (s) return delInts(a, [s]); pushZh(a); a.zoom = null; a.zoomY = null; if (a.type === "spec") a.lock = null; draw(a); uiSave(); } });
  reg({ id: "chart.elimina", group: "tools", views: ["*"], iconKey: "trash", shortcut: { key: "Delete", where: "view", label: "scorc.elimina" },
    name: () => T("scorc.elimina"), tip: () => T("scorc.elimina.tip"), available: () => !!scelto(attivo()), run: () => delInts(attivo(), [scelto(attivo())]) });
  // Esc is run by azioni-esc.js (one press, one thing, by priority); here it only has its line in the panel
  reg({ id: "app.esc", group: "view", views: ["*"], iconKey: "clear", shortcut: { key: "Escape", where: "view", label: "scorc.esc" },
    name: () => T("scorc.esc"), tip: () => T("scorc.esc.tip"), available: () => false, run: () => {} });
  // Safari on a Mac moves Tab between text fields only (unless set otherwise): the panel says that ⌥Tab reaches buttons and links. Safari runs it itself.
  const ua = navigator.userAgent || "", safariMac = /Mac/.test(navigator.platform || "") && /Safari\//.test(ua) && !/Chrome|Chromium|Edg|Firefox|OPR/.test(ua) && !matchMedia("(hover: none) and (pointer: coarse)").matches;
  if (safariMac) reg({ id: "app.tabSafari", group: "view", views: ["*"], iconKey: "list", shortcut: { key: "Tab", alt: true, where: "view", label: "scorc.tabSafari" },
    name: () => T("scorc.tabSafari"), tip: () => T("scorc.tabSafari.tip"), available: () => false, run: () => {} });
  const inData = () => typeof S !== "undefined" && S.view === "data";
  reg({ id: "app.apri", group: "tools", views: ["*"], iconKey: "plus", shortcut: { key: "o", mod: true, where: "view", label: "scorc.apri" },
    name: () => T("scorc.apri"), tip: () => T("scorc.apri.tip"), available: () => !!document.getElementById("pick"), run: () => { const i = document.getElementById("pick"); if (i) i.click(); } });
  reg({ id: "app.esporta", group: "tools", views: ["*"], iconKey: "save", shortcut: { key: "e", mod: true, where: "view", label: "scorc.esporta" },
    name: () => T("scorc.esporta"), tip: () => T("scorc.esporta.tip"), available: () => inData() && !!window.RAPPORTO && E.files.some(f => !f.gone), run: () => window.RAPPORTO && RAPPORTO.export("html") });
  /** @param {string} k @param {string} v @param {string} code @param {() => string} name */
  const view = (k, v, code, name) => reg({ id: "app." + k, group: "tools", views: ["*"], iconKey: "side", shortcut: { code, alt: true, where: "view", label: "scorc." + k },
    name, tip: () => T("scorc.vista.tip"), available: () => typeof setView === "function", run: () => setView(v) });
  view("dati", "data", "Digit1", () => T("scorc.dati")); view("disegno", "draw", "Digit2", () => T("scorc.disegno"));
  // Shift+H shows/hides the side panels like Ctrl/Cmd+B; Alt+Left/Right step through the files like the arrows next to the file menu (not while typing: the field keeps its keys)
  const inField = () => !!(document.activeElement && document.activeElement.closest("input,textarea,select,[contenteditable='true'],[contenteditable='']"));
  reg({ id: "app.pannelli", group: "view", views: ["*"], iconKey: "side", shortcut: { key: "h", shift: true, where: "view", label: "scorc.pannelli" },
    name: () => T("scorc.pannelli"), tip: () => T("scorc.pannelli.tip"), available: () => inData() && !!window.BARRA, run: () => BARRA.toggleDrawer() });
  const passo = (/** @type {string} */ k, /** @type {string} */ label, /** @type {string} */ key, /** @type {number} */ d) => reg({ id: `app.${k}`, group: "view", views: ["*"], iconKey: "list", shortcut: { key, alt: true, where: "view", label },
    name: () => T(label), tip: () => T("scorc.file.tip"), available: () => inData() && !inField() && typeof goFile === "function" && tabFiles().length > 1, run: () => goFile(d) });
  passo("filePrec", "scorc.filePrec", "ArrowLeft", -1); passo("fileSucc", "scorc.fileSucc", "ArrowRight", 1);
  reg({ id: "app.scorciatoie", group: "help", views: ["*"], iconKey: "book", shortcut: { key: "?", where: "view", label: "scorc.titolo" },
    name: () => T("scorc.titolo"), tip: () => T("scorc.titolo.tip"), available: () => inData(), run: () => PANNELLO.apri() });

  /** The panel «Shortcuts»: a dialog generated from the registry, the shortcuts of the active chart first. */
  const PANNELLO = (() => {
    /** @type {HTMLDialogElement | null} */ let dlg = null;
    const riga = (/** @type {any} */ a) => h`<li><span class="sc-i" aria-hidden="true">${raw(AZIONI.iconOf(a, { view: "*" }))}</span><span class="sc-n" title="${a.tip ? a.tip({}) : ""}">${T(a.shortcut.label)}</span><kbd>${AZIONI.keyText(a.shortcut)}</kbd></li>`;
    function list() {
      const p = typeof E !== "undefined" && E.active && E.active.el ? E.active : null, view = p ? p.type : "";
      const all = AZIONI.actions.filter((/** @type {any} */ a) => a.shortcut);
      // chart keys once per label and key; with no chart in focus under their own heading
      const k = (/** @type {any} */ a) => a.shortcut.label + AZIONI.keyText(a.shortcut);
      const c = all.filter(a => a.shortcut.where === "chart" && (!view || a.views.includes(view) || a.views.includes("*"))), g = c.filter((a, i) => c.findIndex(b => k(b) === k(a)) === i);
      return { qui: view ? g : [], resto: all.filter(a => a.shortcut.where === "view"), grafici: view ? [] : g };
    }
    function apri() {
      if (!dlg) {
        dlg = document.createElement("dialog"); dlg.id = "scorc"; dlg.setAttribute("aria-labelledby", "scorc-t");
        const st = document.createElement("style");
        st.textContent = "#scorc{max-width:min(92vw,460px);width:100%}#scorc h2{margin:0 0 8px;font-size:16px}#scorc h3{margin:12px 0 4px;font-size:13px}#scorc ul{list-style:none;margin:0;padding:0}#scorc li{display:flex;align-items:center;gap:8px;min-height:28px;padding:2px 0}#scorc .sc-n{flex:1}#scorc .sc-i svg{width:16px;height:16px;fill:none;stroke:currentColor;stroke-width:1.4;vertical-align:middle}#scorc kbd{font:inherit;border:1px solid var(--line);border-radius:5px;padding:1px 7px;background:var(--panel)}";
        document.head.appendChild(st); document.body.appendChild(dlg);
        dlg.addEventListener("close", () => { const b = /** @type {any} */ (dlg).prev; if (b && document.contains(b)) try { b.focus({ preventScroll: true }); } catch (_) { /* gone */ } });
      }
      const { qui, resto, grafici } = list();
      const sez = (/** @type {string} */ t, /** @type {any[]} */ l) => l.length ? h`<h3>${T(t)}</h3><ul>${raw(l.map(riga).join(""))}</ul>` : "";
      dlg.innerHTML = h`<h2 id="scorc-t">${T("scorc.titolo")}</h2>` + sez("scorc.qui", qui) + sez("scorc.ovunque", resto) + sez("scorc.grafici", grafici) +
        h`<div class="row"><button type="button" class="bt" id="scorc-altri">${T("scorc.altri")}</button><button type="button" class="bt" id="scorc-x" autofocus>${T("scorc.chiudi")}</button></div>`;
      /** @type {any} */ (dlg).prev = document.activeElement;
      /** @type {HTMLElement} */ (dlg.querySelector("#scorc-x")).onclick = () => /** @type {HTMLDialogElement} */ (dlg).close();
      /** @type {HTMLElement} */ (dlg.querySelector("#scorc-altri")).onclick = () => { /** @type {HTMLDialogElement} */ (dlg).close(); if (typeof info === "function" && typeof HELP !== "undefined") info(HELP.scorrimento[1]); };
      if (!dlg.open) dlg.showModal();
    }
    return { apri, list };
  })();
  /** @type {any} */ (window).SCORCIATOIE = PANNELLO;
})();
