// @ts-check
// @ts-strict
"use strict";
// Markup of the «What can this number be?» block (calc-qui.js): the choices row and the groups of candidates. Pure functions of the groups and of the
// options, shared by the calculator, the Adducts panel and the Neutral losses panel. Classic script, after calc-addotti-html.js.

const CALCQUIHTML = (() => {
  /** @param {string} k @param {Bag} [p] */
  const T = (k, p) => I18N.t(k, p);
  const AH = CALCADDHTML, sgn = AH.sgn;
  const err = (/** @type {number} */ d, /** @type {number} */ dec) => sgn(d) + Math.abs(d).toFixed(dec);
  const loss = (/** @type {Bag} */ l) => (window.QQQRef ? QQQRef.lossName(l) : `<b>${EH(l.f)}</b>`);

  /** Small text saying which resolution was detected and why (never a button). @param {{ hr: boolean, mode: { src: string, name: string } }} o */
  function modeText(o) {
    const n = { name: o.mode.name };
    if (o.mode.src === "files") return o.hr ? T("cq.mode.hr.files", n) : T("cq.mode.lr.files", n);
    if (o.mode.src === "formula") return T("cq.mode.hr.formula");
    if (o.mode.src === "decimals") return T("cq.mode.hr.decimals");
    return T(o.mode.src === "short" ? "cq.mode.lr.short" : "cq.mode.lr.none");
  }
  /** The choices row: detected resolution (text), tolerance, polarity. Ids are `<idp>tol|pol`.
   * @param {string} idp @param {{ hr: boolean, tol: number, pol: number, mode: { src: string, name: string } }} o */
  function ctrlHtml(idp, o) {
    const on = (/** @type {boolean} */ b) => ` class="${b ? "on" : ""}" aria-pressed="${b}"`;
    return `<div class="cadd cq-ctl" role="group" aria-label="${EH(T("cq.ctl"))}">
      <label class="cadd-f"><span class="cq-tl">${T(o.hr ? "calc.tol" : "calc.tol.da")}</span><input id="${idp}tol" inputmode="decimal" autocomplete="off" value="${o.tol}"></label>
      <span class="seg" id="${idp}pol" role="group" aria-label="${EH(T("calc.pol"))}"><button type="button" data-p="1"${on(o.pol > 0)}>ESI+</button><button type="button" data-p="-1"${on(o.pol < 0)}>ESI&minus;</button></span>
      <span class="cq-mode muted" aria-live="polite">${EH(modeText(o))}</span></div>`;
  }

  const rowsOf = (/** @type {string} */ gid, /** @type {string[]} */ lis, /** @type {boolean} */ all) => {
    const lim = CALCQUI.LIMIT, shown = all ? lis : lis.slice(0, lim);
    return `<ul class="cq-l">${shown.join("")}</ul>${lis.length > lim ? `<button type="button" class="cadd-more cq-all" data-g="${gid}" aria-expanded="${all}">${T(all ? "cq.less" : "cq.all", { n: lis.length })}</button>` : ""}`;
  };
  /** @param {CqGroup} g @param {Bag} o */
  function listeRows(g, o) {
    return g.items.map(x => `<li><b>${EH(x.name)}</b> <span class="muted">${x.adduct ? fmtAdduct(x.adduct) + " · " : ""}${x.mz.toFixed(o.dec)} · ${EH(x.listName)} · ${o.hr ? err(x.err, 1) + " ppm" : err(x.diffDa, 1) + " Da"}</span></li>`);
  }
  /** One candidate: the name is a button that opens the detail of its losses (as in the Neutral losses table). @param {Bag} x @param {number} i @param {number} dec */
  function lossRow(x, i, dec) {
    const nm = x.kind === "pair" ? `${loss(x.ls[0])} + ${loss(x.ls[1])}` : x.kind === "rep" ? `${x.n} &times; ${loss(x.ls[0])}` : loss(x.ls[0]);
    const det = x.ls.map((/** @type {Bag} */ l) => (window.QQQRef ? QQQRef.lossDet(l) : "")).join("");
    return `<li><button type="button" class="cq-lb" aria-expanded="false" aria-controls="cq-ld-${i}">${nm}</button> <span class="muted">${x.mass.toFixed(dec)} · ${err(x.err, dec)} Da</span><div class="nldet cq-ld" id="cq-ld-${i}" hidden>${det}</div></li>`;
  }
  /** @param {CqGroup} g @param {Bag} o */
  const lossRows = (g, o) => g.items.map((x, i) => lossRow(x, i, o.hr ? 4 : 1));
  /** @param {CqGroup} g */
  function diffRows(g) {
    return g.items.map(h => `<li>${fmtAdduct(h.a)} &minus; ${fmtAdduct(h.b)} <span class="muted">(${sgn(h.d)}${Math.abs(h.d).toFixed(4)}, ${err(h.err_mda, 2)} mDa)</span></li>`);
  }
  /** The table of the adducts (formula, or the number taken as an observed m/z). @param {Bag} g @param {Bag} o @param {Bag} view */
  function addHtml(g, o, view) {
    const st = { pol: o.pol, more: !!view.more, lr: !o.hr, tolDa: o.tol };
    return AH.mainHtml(g.tab, !!g.formula, !!g.obs || !g.formula, st);
  }
  const TITLE = { liste: "cq.g.liste", perdite: "cq.g.perdite", addotti: "cq.g.addotti", diff: "cq.g.diff" };
  /** The groups, each a `details` (open while it has a close match, or the adduct table of a formula).
   * @param {CqGroup[]} gs @param {Bag} o @param {{ open: Set<string>, all: Set<string>, more?: boolean }} view @param {{ formula: boolean }} ctx */
  function groupsHtml(gs, o, view, ctx) {
    if (!gs.length) return `<p class="muted cq-none">${T("cq.none", { tol: o.hr ? o.tol + " ppm" : o.tol + " Da" })}</p>`;
    return gs.map(g => {
      if (g.id === "diff" && /** @type {Bag} */ (g).catalogue) return AH.diffsHtml(g.tab, { diffs: view.open.has("catdiff"), delta: "" });
      const open = view.open.has(g.id) || (!view.open.has("-" + g.id) && (g.near <= 1 || (g.id === "addotti" && ctx.formula)));
      const all = view.all.has(g.id);
      const body = g.id === "addotti" ? addHtml(g, o, view) : rowsOf(g.id, g.id === "liste" ? listeRows(g, o) : g.id === "perdite" ? lossRows(g, o) : diffRows(g), all);
      const n = g.id === "addotti" ? "" : ` <span class="cq-n">${g.n}</span>`;
      return `<details class="cq-g" data-g="${g.id}"${open ? " open" : ""}><summary>${T(/** @type {Bag} */ (TITLE)[g.id])}${n}</summary>${body}</details>`;
    }).join("");
  }
  return { ctrlHtml, groupsHtml, modeText };
})();
