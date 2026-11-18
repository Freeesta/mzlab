// @ts-strict
// The bond breaks of the SMILES of the tab that explain an ion, in the panel of a node of the «Fragmentation graph». Every hypothesis comes from the key
// `rotture` of msn-graph/1 (formula, error, hydrogens moved); they are evidence for the user's own judgement, never an identification. Used by infusione-grafo-nodo.js.
import { G } from "./infusione-grafo-stato.js";
import { prettyFormula } from "./msngrafo-vista.js";

/** The hypotheses of the node and whether the arc that reaches it is explained by a break of its precursor; null when the answer has no `rotture`. */
function breaksOf(/** @type {any} */ n) {
  const r = G.j && G.j.breaks; if (!r) return null;
  const hit = r.nodes.find((/** @type {any} */ x) => x.node === n.id);
  const arcs = r.edges.filter((/** @type {any} */ a) => a.to === n.id);
  return { r, hit, explained: arcs.some((/** @type {any} */ a) => a.explained), arcs: arcs.length };
}

/** The block of the panel: the number of breaks that fit the m/z, the first ones with formula, error and moved hydrogens, and whether the loss from the precursor is explained. */
export function breaksPanel(/** @type {any} */ n) {
  const b = breaksOf(n); if (!b) return "";
  const t = (/** @type {string} */ k, /** @type {any} */ v) => I18N.t(/** @type {any} */ (k), v);
  const hs = b.hit && b.hit.hypotheses.length ? b.hit.hypotheses : [];
  const rows = hs.map((/** @type {any} */ x) => h`<li>${t("msngrafo.brk.line", { f: prettyFormula(x.formula, ""), mz: x.theoretical_mz.toFixed(4), ppm: x.ppm.toFixed(1), bonds: x.bonds.length, h: (x.h > 0 ? "+" : "") + x.h })}</li>`);
  const more = b.hit && b.hit.n > hs.length ? h`<li class="muted">${t("msngrafo.brk.more", { n: b.hit.n - hs.length })}</li>` : "";
  const state = b.arcs ? h`<p class="sm">${t(b.explained ? "msngrafo.brk.explained" : "msngrafo.brk.notexplained", {})}</p>` : "";
  const body = rows.length ? h`<ul class="sm">${rows.map((/** @type {string} */ r) => raw(r))}${raw(more)}</ul>` : h`<p class="sm muted">${t("msngrafo.brk.none", {})}</p>`;
  return h`<div class="ig-brk"><b class="sm">${t("msngrafo.brk.title", {})}</b>${raw(state)}${raw(body)}<p class="muted sm">${t("msngrafo.brk.hyp", {})}</p></div>`;
}
