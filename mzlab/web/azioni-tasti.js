// @ts-strict
// @ts-check
"use strict";
// The shortcuts of the registry azioni/1 that no older module owns, and the panel «Shortcuts» (key ?) built from the registry itself.
// The keys live in the field `scorciatoia` of each action; ONE keydown in azioni.js runs them (letters only with the focus on a chart, never in a text field;
// the combinations with Ctrl/⌘ or Alt anywhere in the view). Existing keys win: a chart handler that already used the key has run (and prevented the default) before.
// Classic script, loaded after azioni.js. Short labels live in lang/ (scorc.*), the explanations in the tooltips.
(() => {
  /** @param {string} k @param {any} [v] */
  const T = (k, v) => I18N.t(k, v);
  /** @param {any} o */
  const reg = o => AZIONI.registra({ menu: false, ...o });
  // the same actions that the right-click menu offers, now with a key (X, Enter): the menu keeps them, the key is a field of their registration
  const V = ["chrom", "xic", "mrm"];
  // integration at the cursor: the automatic one, as the entry «Integrate automatically» of the menu
  reg({ id: "chrom.intAuto", gruppo: "tools", ordine: 1, viste: V, icona: "area", scorciatoia: { tasto: "i", dove: "chart", et: "scorc.integra" },
    nome: () => T("scorc.integra"), tip: () => T("scorc.integra.tip"), disponibile: (/** @type {any} */ c) => typeof autoInt === "function" && typeof intTargets === "function" && !!c.p._a,
    esegui: (/** @type {any} */ c) => { if (!guardInt(c.p)) return; intTargets(c.p, c.x, c.py).forEach((/** @type {any} */ s) => autoInt(c.p, s, c.x)); } });
  // measure from the highest visible peak (like «Measure from this peak» of the menu)
  reg({ id: "spec.measureKey", gruppo: "tools", viste: ["spec"], icona: "measure", scorciatoia: { tasto: "r", dove: "chart", et: "scorc.misura" },
    nome: () => T("scorc.misura"), tip: () => T("scorc.misura.tip"), disponibile: (/** @type {any} */ c) => c.has && typeof measSet === "function", esegui: (/** @type {any} */ c) => measSet(c.p, c.m) });
  // full screen on every panel (the map keeps its own F, which runs first)
  reg({ id: "view.fullscreen", gruppo: "view", viste: ["*"], icona: "wide", scorciatoia: { tasto: "f", dove: "chart", et: "scorc.schermo" },
    nome: () => T("scorc.schermo"), tip: () => T("scorc.schermo.tip"), disponibile: (/** @type {any} */ c) => !!(c.p && c.p._setMax),
    esegui: (/** @type {any} */ c) => c.p._setMax(!c.p.el.classList.contains("max")) });
  const inData = () => typeof S !== "undefined" && S.view === "data";
  reg({ id: "app.apri", gruppo: "tools", viste: ["*"], icona: "plus", scorciatoia: { tasto: "o", mod: true, dove: "view", et: "scorc.apri" },
    nome: () => T("scorc.apri"), tip: () => T("scorc.apri.tip"), disponibile: () => !!document.getElementById("pick"), esegui: () => { const i = document.getElementById("pick"); if (i) i.click(); } });
  reg({ id: "app.esporta", gruppo: "tools", viste: ["*"], icona: "save", scorciatoia: { tasto: "e", mod: true, dove: "view", et: "scorc.esporta" },
    nome: () => T("scorc.esporta"), tip: () => T("scorc.esporta.tip"), disponibile: () => inData() && !!window.RAPPORTO, esegui: () => window.RAPPORTO && RAPPORTO.export("html") });
  /** @param {string} k @param {string} v @param {string} code @param {() => string} nome */
  const vista = (k, v, code, nome) => reg({ id: "app." + k, gruppo: "tools", viste: ["*"], icona: "side", scorciatoia: { code, alt: true, dove: "view", et: "scorc." + k },
    nome, tip: () => T("scorc.vista.tip"), disponibile: () => typeof setView === "function", esegui: () => setView(v) });
  vista("dati", "data", "Digit1", () => T("scorc.dati")); vista("disegno", "draw", "Digit2", () => T("scorc.disegno")); vista("teoria", "theory", "Digit3", () => T("scorc.teoria"));
  reg({ id: "app.scorciatoie", gruppo: "help", viste: ["*"], icona: "book", scorciatoia: { tasto: "?", dove: "view", et: "scorc.titolo" },
    nome: () => T("scorc.titolo"), tip: () => T("scorc.titolo.tip"), disponibile: () => inData(), esegui: () => PANNELLO.apri() });

  /** The panel «Shortcuts»: a dialog generated from the registry, the shortcuts of the active chart first. */
  const PANNELLO = (() => {
    /** @type {HTMLDialogElement | null} */ let dlg = null;
    const riga = (/** @type {any} */ a) => h`<li><span class="sc-i" aria-hidden="true">${raw(AZIONI.icona(a, { vista: "*" }))}</span><span class="sc-n" title="${a.tip ? a.tip({}) : ""}">${T(a.scorciatoia.et)}</span><kbd>${AZIONI.testo(a.scorciatoia)}</kbd></li>`;
    function elenco() {
      const p = typeof E !== "undefined" && E.active && E.active.el ? E.active : null, vista = p ? p.type : "";
      const tutte = AZIONI.voci.filter((/** @type {any} */ a) => a.scorciatoia);
      const qui = vista ? tutte.filter((/** @type {any} */ a) => a.scorciatoia.dove === "chart" && (a.viste.includes(vista) || a.viste.includes("*"))) : [];
      const resto = tutte.filter((/** @type {any} */ a) => !qui.includes(a) && (a.scorciatoia.dove === "view" || !vista));
      return { qui, resto };
    }
    function apri() {
      if (!dlg) {
        dlg = document.createElement("dialog"); dlg.id = "scorc"; dlg.setAttribute("aria-labelledby", "scorc-t");
        const st = document.createElement("style");
        st.textContent = "#scorc{max-width:min(92vw,460px);width:100%}#scorc h2{margin:0 0 8px;font-size:16px}#scorc h3{margin:12px 0 4px;font-size:13px}#scorc ul{list-style:none;margin:0;padding:0}#scorc li{display:flex;align-items:center;gap:8px;min-height:28px;padding:2px 0}#scorc .sc-n{flex:1}#scorc .sc-i svg{width:16px;height:16px;fill:none;stroke:currentColor;stroke-width:1.4;vertical-align:middle}#scorc kbd{font:inherit;border:1px solid var(--line);border-radius:5px;padding:1px 7px;background:var(--panel)}";
        document.head.appendChild(st); document.body.appendChild(dlg);
        dlg.addEventListener("close", () => { const b = /** @type {any} */ (dlg).prev; if (b && document.contains(b)) try { b.focus({ preventScroll: true }); } catch (_) { /* gone */ } });
      }
      const { qui, resto } = elenco();
      dlg.innerHTML = h`<h2 id="scorc-t">${T("scorc.titolo")}</h2>${raw(qui.length ? h`<h3>${T("scorc.qui")}</h3><ul>${raw(qui.map(riga).join(""))}</ul>` : "")}` +
        h`<h3>${T("scorc.ovunque")}</h3><ul>${raw(resto.map(riga).join(""))}</ul>` +
        h`<div class="row"><button type="button" class="bt" id="scorc-altri">${T("scorc.altri")}</button><button type="button" class="bt" id="scorc-x" autofocus>${T("scorc.chiudi")}</button></div>`;
      /** @type {any} */ (dlg).prev = document.activeElement;
      /** @type {HTMLElement} */ (dlg.querySelector("#scorc-x")).onclick = () => /** @type {HTMLDialogElement} */ (dlg).close();
      /** @type {HTMLElement} */ (dlg.querySelector("#scorc-altri")).onclick = () => { /** @type {HTMLDialogElement} */ (dlg).close(); if (typeof info === "function" && typeof HELP !== "undefined") info(HELP.scorrimento[1]); };
      if (!dlg.open) dlg.showModal();
    }
    return { apri, elenco };
  })();
  /** @type {any} */ (window).SCORCIATOIE = PANNELLO;
})();
