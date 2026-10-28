// @ts-check
// @ts-strict
"use strict";
// The Adducts panel (drawer tab, and a window outside the Dati view): a field for a number or a formula, then the same groups, resolution and polarity
// choices as under the calculator (calc-qui*.js). Links to and from the Neutral losses panel carry the number across. Classic script, after tables.js.

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
    root.innerHTML = `<div class="nlbar"><div class="sb-search-box">${SEARCH}<input id="${id}-q" inputmode="decimal" autocomplete="off" placeholder="${EH(T("calc.ph"))}" aria-label="${EH(T("cq.ad.title"))}" title="${EH(T("cq.ad.title.field"))}"></div></div>
      <div class="nlhint muted">ⓘ ${T("cq.ad.hint")}</div><div id="${id}-ctl"></div>
      <div class="cq-links"><button type="button" class="cq-link" id="${id}-loss" hidden>${T("cq.link.loss")}</button></div><div id="${id}-out" class="cq-out" aria-live="polite"></div>`;
    const q = /** @type {HTMLInputElement} */ (root.querySelector(`#${id}-q`)), loss = /** @type {HTMLElement} */ (root.querySelector(`#${id}-loss`));
    if (opt.q != null) q.value = String(opt.q);
    const view = CALCQUI.mount({
      ctl: /** @type {HTMLElement} */ (root.querySelector(`#${id}-ctl`)), out: /** @type {HTMLElement} */ (root.querySelector(`#${id}-out`)), idp: id, obs: true, quiet: true,
      groups: ["addotti", "diff"], arg: () => ({ text: q.value, ans: 0 }),
    });
    const upd = () => { loss.hidden = !numberOf(q.value); CALCQUI.later(view); };
    q.addEventListener("input", upd);
    loss.onclick = () => { const n = numberOf(q.value); if (n) CALCQUI.apriPerdite(n); };
    root.dataset.cqField = q.id;
    upd();
    if (opt.q != null) setTimeout(() => { q.focus(); q.select(); }, 50);
  }

  /** @param {string | number} q  the sidebar panel is already mounted: fill its field */
  function setAdductsQuery(q) {
    const f = Q1("#sb-panel-adducts [id$='-q']");
    if (!(f instanceof HTMLInputElement)) return;
    f.value = String(q); f.dispatchEvent(new Event("input", { bubbles: true }));
    setTimeout(() => { f.focus(); f.select(); }, 50);
  }

  /** The drawer panel: built the first time, later only its field is filled. @param {{ q?: string | number | null }} [opt] */
  function ensureAdducts(opt = {}) {
    const pnl = Q1("#sb-panel-adducts");
    if (!pnl) return;
    if (!pnl.dataset.mounted) { mountAdducts(pnl, opt); pnl.dataset.mounted = "1"; } else if (opt.q != null) setAdductsQuery(opt.q);
  }

  /** The Adducts: the drawer tab inside the Dati view, a window elsewhere. @param {string | number | null} [q] */
  function apriAddotti(q = null) {
    const opt = q == null ? {} : { q: String(q) };
    if (window.BARRA && BARRA.isDataView()) { BARRA.setTab("adducts", opt); return; }
    const d = /** @type {HTMLDialogElement | null} */ (Q1("#refdlg")), body = Q1("#refbody"), ttl = Q1("#reftitle"), x = Q1("#refx");
    if (!d || !body || !ttl || !x) return;
    ttl.textContent = T("cq.ad.title"); x.onclick = () => d.close();
    mountAdducts(body, opt);
    if (!d.open) d.showModal();
  }
  /** The Neutral losses with the number typed in the search. @param {string | number} q */
  function apriPerdite(q) {
    if (window.BARRA && BARRA.isDataView()) BARRA.setTab("losses", { q: String(q) }); else if (window.QQQRef) QQQRef.open("ls", { q: String(q) });
  }
  Object.assign(CALCQUI, { ensureAdducts, mountAdducts, setAdductsQuery, apriAddotti, apriPerdite, numberOf });
})();
