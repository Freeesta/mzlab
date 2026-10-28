// @ts-check
// @ts-strict
"use strict";
// Adduct table of the m/z calculator and of the Adducts panel: catalogue, ion m/z, ppm and implied neutral masses come from the Rust engine
// (api/adduct_table); this file asks for them (the groups under the calculator are calc-qui*.js). Classic script, after calc-addotti-html.js.

const CALCADD = (() => {
  let catP = /** @type {Promise<AdRow[]> | null} */ (null);

  /** @param {string} f @param {string} obs @param {string} tol @param {string} delta @returns {Promise<AdTable>} */
  const table = (f, obs, tol, delta) => J(`api/adduct_table?f=${encodeURIComponent(f)}&obs=${encodeURIComponent(obs)}&tol=${encodeURIComponent(tol)}&delta=${encodeURIComponent(delta)}`);
  // the catalogue alone (no formula): the search box and the palette read it
  const catalog = () => catP || (catP = table("", "", "", "").then(t => t.rows));

  /** Opens the calculator with a difference already typed (a pair of peaks of a spectrum): the adduct differences are one of its groups. */
  function showDelta(/** @type {number} */ v) { CALCQUI.apri(v, { gruppo: "diff" }); }
  return { table, catalog, shiftText: CALCADDHTML.shiftText, showDelta };
})();
