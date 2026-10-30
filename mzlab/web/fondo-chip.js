// @ts-check
"use strict";
// The chip of the internal blank (ibkChip in grafici.js calls ibkLegend): «Blank 0.1–0.4 min · 2.70e8 cps» in the command bar above the plot, never over the trace.
// A click on the text asks for another stretch, × removes the subtraction. Classic script, loaded after grafici.js.

/** level of the blank: 3 significant digits, the unit once @param {{ lvl: number, lvl2?: number | null }} l */
const ibkLevelShort = l => {
  const f = (/** @type {number} */ v) => Math.abs(v) >= 1e4 ? v.toExponential(2).replace("e+", "e") : String(+v.toPrecision(3));
  return f(l.lvl) + (l.lvl2 != null ? " " + I18N.t("ibk.andShort") + " " + f(l.lvl2) : "") + " cps";
};
/** @param {any} p @param {any} ib @param {any} l0 */
function ibkLegend(p, ib, l0) {
  const rg = (/** @type {number[]} */ a) => `${+a[0].toFixed(1)}–${+a[1].toFixed(1)} min`, ranges = ib.mode === "line" && ib.b ? I18N.t("ibk.and", { a: rg(ib.a), b: rg(ib.b) }) : rg(ib.a);
  const mk = (/** @type {string} */ k, /** @type {string} */ text, /** @type {string} */ title) => {
    const b = document.createElement("button"); b.type = "button"; b.dataset.ib = k; b.textContent = text; b.title = title; if (k === "x") b.setAttribute("aria-label", title); return b;
  };
  const c = document.createElement("span"); c.className = "tg ibkc";
  const ed = mk("edit", I18N.t("ibk.leg", { ranges, level: l0 ? ibkLevelShort(l0) : "" }), I18N.t("ibk.leg.title")), x = mk("x", "×", I18N.t("ibk.leg.off"));
  ed.onclick = () => { p._pickIbk = true; info(I18N.t("ibk.pick")); };
  x.onclick = () => setFondo(p, "");
  c.append(ed, x);
  (p.el.querySelector(".ibkslot") || p.leg).appendChild(c);
}
