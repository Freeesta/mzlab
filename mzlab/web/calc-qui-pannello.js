// @ts-check
// @ts-strict
"use strict";
// The Adducts & losses page of the drawer: the calculator (folds with its toggle), ONE field and ONE choices row, then the tabs Adducts | Neutral losses |
// Contaminants, all fed by calc-qui*.js. Outside the drawer the Adducts window keeps its own field (mountAdducts). Classic script, after tables.js.

(() => {
  const Q1 = (/** @type {string} */ s) => /** @type {HTMLElement | null} */ (document.querySelector(s));
  /** @param {string} k @param {Bag} [p] */
  const T = (k, p) => I18N.t(k, p);
  const SEARCH = '<svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="6.5" cy="6.5" r="4.5"/><path d="M10 10l4 4"/></svg>';
  let seq = 0;
  /** The text of the field is a number (a plain one or the result of a sum) when the block would give losses for it. @param {string} text */
  const numberOf = text => { const a = CALCQUI.arg(text, 0); return a.kind === "num" && a.v > 0 ? String(+a.v.toFixed(4)) : ""; };

  /** @param {HTMLElement} root @param {{ q?: string | number | null }} [opt] */
  function mountAdducts(root, opt = {}) {
    const id = "ad" + ++seq;
    root.innerHTML = h`<div class="nlbar"><div class="sb-search-box">${raw(SEARCH)}<input id="${id}-q" inputmode="decimal" autocomplete="off" placeholder="${T("calc.ph")}" aria-label="${T("cq.ad.title")}" title="${T("cq.ad.title.field")}"></div></div>
      <div id="${id}-ctl"></div>
      <div class="cq-links"><button type="button" class="cq-link" id="${id}-loss" hidden>${raw(T("cq.link.loss"))}</button></div><div id="${id}-out" class="cq-out" aria-live="polite"></div>`;
    const q = /** @type {HTMLInputElement} */ (root.querySelector(`#${id}-q`)), loss = /** @type {HTMLElement} */ (root.querySelector(`#${id}-loss`));
    if (opt.q != null) q.value = String(opt.q);
    const view = CALCQUI.mount({
      ctl: /** @type {HTMLElement} */ (root.querySelector(`#${id}-ctl`)), out: /** @type {HTMLElement} */ (root.querySelector(`#${id}-out`)), idp: id, quiet: true,
      groups: ["addotti", "diff"], catalogue: true, arg: () => ({ text: q.value, ans: 0 }),
    });
    const upd = () => { loss.hidden = !numberOf(q.value); CALCQUI.later(view); };
    q.addEventListener("input", upd);
    loss.onclick = () => { const n = numberOf(q.value); if (n) CALCQUI.apriPerdite(n); };
    root.dataset.cqField = q.id;
    upd();
    if (opt.q != null) setTimeout(() => { q.focus(); q.select(); }, 50);
  }

  // ---------------------------------------------------------------- the page in the drawer
  const SUBS = ["adducts", "losses", "cont"];
  const CALC_KEY = () => (typeof NOMI !== "undefined" ? NOMI.KEY("addloss.calc") : "mzlab.addloss.calc");
  /** @type {{ q: HTMLInputElement, views: any[], losses: { setQ: (q: string) => void } | null } | null} */
  let page = null;
  const field = () => /** @type {HTMLInputElement | null} */ (Q1("#al-q"));
  /** Builds the page once: the field, the choices row (drawn by the Adducts view), the three tab bodies. */
  function ensurePage() {
    if (page) return page;
    const q = field(), ad = Q1("#sb-panel-adducts"), ls = Q1("#sb-panel-losses"), ct = Q1("#sb-panel-cont"), ctl = Q1("#al-ctl");
    if (!q || !ad || !ls || !ct || !ctl) return null;
    const arg = () => ({ text: q.value, ans: 0 });
    const views = [
      CALCQUI.mount({ ctl, out: ad, idp: "al", quiet: true, flat: true, groups: ["addotti", "diff"], catalogue: true, arg }),
      CALCQUI.mount({ ctl: null, out: ct, idp: "alc", quiet: true, flat: true, none: true, numOnly: true, groups: ["liste"], arg, after: (a, gs) => count("cont", a.kind === "num" ? (gs[0] ? gs[0].n : 0) : null) }),
    ];
    const losses = window.QQQRef ? QQQRef.mountLosses(ls) : null;
    page = { q, views, losses };
    q.addEventListener("input", () => { views.forEach(v => CALCQUI.later(v)); if (losses) losses.setQ(q.value); });
    const tg = Q1("#al-calc-tg");
    if (tg) tg.addEventListener("click", () => setCalc(tg.getAttribute("aria-expanded") !== "true", true));
    setCalc(calcStored(), false);
    return page;
  }
  /** The number of matches next to a tab's name (null: nothing typed). @param {string} sub @param {number | null} n */
  function count(sub, n) {
    const b = Q1(`#al-t-${sub} .cq-n`);
    if (b) { b.hidden = n == null; b.textContent = n == null ? "" : String(n); }
  }
  const calcStored = () => { try { return localStorage.getItem(CALC_KEY()) !== "0"; } catch (_) { return true; } };
  /** Unfolds or folds the calculator of the page (remembered). @param {boolean} on @param {boolean} [focus] */
  function setCalc(on, focus) {
    const tg = Q1("#al-calc-tg"), body = Q1("#al-calc-body");
    if (!tg || !body) return;
    tg.setAttribute("aria-expanded", String(on)); body.hidden = !on;
    try { localStorage.setItem(CALC_KEY(), on ? "1" : "0"); } catch (_) { /* blocked: only this visit */ }
    if (on && focus) { const i = Q1("#calcin"); if (i) setTimeout(() => i.focus(), 30); }
  }
  const calcOpen = () => { const tg = Q1("#al-calc-tg"); return !!tg && tg.getAttribute("aria-expanded") === "true"; };
  /** One section of the page was shown (barra.js setSub): build, and the number or formula it was opened with. @param {string} sub @param {{ q?: string | number | null, calc?: boolean, open?: string }} [opt] */
  function showSub(sub, opt = {}) {
    const pg = ensurePage();
    if (!pg) return;
    if (opt.calc) setCalc(true, true);
    if (opt.open) pg.views.forEach(v => { v.open.add(/** @type {string} */ (opt.open)); v.open.delete("-" + opt.open); });
    if (opt.q != null) {
      pg.q.value = String(opt.q); pg.q.dispatchEvent(new Event("input", { bubbles: true }));
      if (!opt.calc) setTimeout(() => { pg.q.focus(); pg.q.select(); }, 50);
    }
  }
  /** «Cerca»: a number, a difference or a formula into the page field, on the tab given (null: the one shown). @param {string | number} q @param {string | null} [sub] @param {{ open?: string }} [opt] */
  function cerca(q, sub = null, opt = {}) {
    if (!window.BARRA) return;
    const s = sub && SUBS.includes(sub) ? sub : BARRA.curSub;
    BARRA.setTab(s, { q: String(q), open: opt.open });
  }
  /** @param {{ q?: string | number | null }} [opt] */
  const ensureAdducts = (opt = {}) => showSub("adducts", opt);

  /** The Adducts tab of the page (the side panel only: from another view it goes back to Dati). The window is kept for a page without the drawer. @param {string | number | null} [q] */
  function apriAddotti(q = null) {
    const opt = q == null ? {} : { q: String(q) };
    if (window.BARRA) { BARRA.setTab("adducts", opt); return; }
    const d = /** @type {HTMLDialogElement | null} */ (Q1("#refdlg")), body = Q1("#refbody"), ttl = Q1("#reftitle"), x = Q1("#refx");
    if (!d || !body || !ttl || !x) return;
    ttl.textContent = T("cq.ad.title"); x.onclick = () => d.close();
    mountAdducts(body, opt);
    if (!d.open) d.showModal();
  }
  /** The Neutral losses with the number typed in the search. @param {string | number} q */
  function apriPerdite(q) {
    if (window.BARRA) BARRA.setTab("losses", { q: String(q) }); else if (window.QQQRef) QQQRef.open("ls", { q: String(q) });
  }
  Object.assign(CALCQUI, { ensureAdducts, mountAdducts, showSub, cerca, setCalc, calcOpen, apriAddotti, apriPerdite, numberOf, SUBS });
})();
