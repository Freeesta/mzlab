// @ts-check
// The fragmentation rules in the «Fragmentation graph» tab: the label of a rule under the loss of an arc, the filter «explained by a rule», the rules of the
// arcs that end in the selected ion (with source, weak tag and warnings) and the columns of the data table. Every number, reason, DOI and warning comes from the
// key `regole` of msngrafo/1; the rules are evidence for the user's own judgement, never a verdict. Used by infusione-grafo*.js.
import { G, TXT } from "./infusione-grafo-stato.js";
import { prettyFormula } from "./msngrafo-vista.js";

/** Names of the numeric thresholds of the table (spelled out so the catalogue check sees every key). */
const THR = { trap_cutoff: "msngrafo.rule.thr.trap", loss_tolerance_mda: "msngrafo.rule.thr.tol" };

/** The rules of the engine for the arc `a` of the graph `j`, or null when the answer has none. */
export const arcRules = (/** @type {any} */ j, /** @type {any} */ a) => (j.regole && j.regole.archi.find((/** @type {any} */ x) => x.da === a.da && x.a === a.a)) || null;

/** A rule or a sum of two rules explains the arc. */
export const isExplained = (/** @type {any} */ r) => !!r && (r.regole.length > 0 || r.composta.length > 0);

const weakTag = (/** @type {any} */ x, /** @type {(k: string) => string} */ t) => (x.priorita === 3 ? ` (${t("msngrafo.rule.weak")})` : "");

/** The short text under the loss of an arc: the ids of its rules (at most two, «+n» for the rest), the weak ones tagged; empty when no rule explains it. */
export function arcLabel(/** @type {any} */ j, /** @type {any} */ a, /** @type {(k: string) => string} */ t = k => I18N.t(/** @type {any} */ (k))) {
  const r = arcRules(j, a);
  if (!isExplained(r)) return "";
  const ids = r.regole.length ? r.regole.map((/** @type {any} */ x) => x.id + weakTag(x, t)) : r.composta.map((/** @type {string[]} */ c) => c.join(" + "));
  return ids.slice(0, 2).join(", ") + (ids.length > 2 ? ` +${ids.length - 2}` : "");
}

/** The graph with only the losses a rule explains and the ions they reach (the roots stay); the same object when the filter is off or the answer has no rules. */
export function filtered(/** @type {any} */ j, /** @type {boolean} */ only) {
  if (!only || !j.regole) return j;
  const archi = j.archi.filter((/** @type {any} */ a) => isExplained(arcRules(j, a)));
  const reached = new Set(archi.map((/** @type {any} */ a) => a.a)), inner = new Set(j.archi.map((/** @type {any} */ a) => a.a));
  return { ...j, archi, nodi: j.nodi.filter((/** @type {any} */ n) => reached.has(n.id) || !inner.has(n.id)) };
}

/** The source of a reference key as a link to its DOI (a new tab, no opener), or the bare key when the table gives no DOI. @returns {string} */
function source(/** @type {string} */ key) {
  const f = (G.j.regole.fonti || []).find((/** @type {any} */ x) => x.key === key);
  return f && f.doi ? h`<a href="${"https://doi.org/" + encodeURI(f.doi)}" target="_blank" rel="noopener noreferrer" class="ig-src" title="DOI ${f.doi}">${key}</a>` : EH(key);
}

/** The rules, the sums and the warnings of one arc that ends in the selected ion, as list items. */
function arcItems(/** @type {any} */ r) {
  const li = r.regole.map((/** @type {any} */ x) => {
    const line = I18N.t("msngrafo.rule.line", { id: x.id, f: prettyFormula(x.formula), err: (x.errore_mda > 0 ? "+" : x.errore_mda < 0 ? "−" : "") + Math.abs(x.errore_mda).toFixed(2), p: x.priorita, e: x.energia });
    const weak = x.priorita === 3 ? h`<span class="ig-weak">${I18N.t("msngrafo.rule.weak")}</span> ` : "";
    const why = h`<span class="muted">${x.perche}</span>`;
    const src = x.refs.length ? " " + I18N.t("msngrafo.rule.src") + " " + x.refs.map(source).join(", ") : "";
    return h`<li>${raw(weak)}${line}${raw(src)}<br>${raw(why)}</li>`;
  });
  const sums = r.composta.map((/** @type {string[]} */ c) => h`<li>${I18N.t("msngrafo.rule.sum", { a: c[0], b: c[1] })}</li>`);
  const stepped = r.stepped ? [h`<li class="ig-warn">${I18N.t("msngrafo.rule.stepped")}</li>`] : [];
  const warns = r.avvisi.map((/** @type {string} */ k) => h`<li class="ig-warn">${I18N.t(/** @type {any} */ (TXT.warn)[k])}</li>`);
  return [...li, ...sums, ...stepped, ...warns];
}

/** The thresholds of the table with where they come from and, when it is so, «to be calibrated». */
function thresholds() {
  const rows = (G.j.regole.soglie || []).map((/** @type {any} */ s) => {
    const name = I18N.t(/** @type {any} */ (THR)[s.id] || s.id), cal = s.da_calibrare ? h` · <b class="ig-cal">${I18N.t("msngrafo.rule.calibrate")}</b>` : "";
    return h`<li>${I18N.t("msngrafo.rule.thr", { name, v: s.valore })}${raw(cal)}<br><span class="muted">${s.origine || ""} ${raw((s.refs || []).map(source).join(", "))}</span></li>`;
  });
  return rows.length ? h`<details class="ig-thr"><summary>${I18N.t("msngrafo.rule.thr.title")}</summary><ul class="sm">${raw(rows.join(""))}</ul></details>` : "";
}

/** The rules of the losses that end in the ion `n`: per arc, the ion it comes from and its rules; empty when the answer has no rules or the ion is a root. */
export function rulesPanel(/** @type {any} */ n) {
  const j = G.j;
  if (!j || !j.regole) return "";
  const arcs = j.archi.filter((/** @type {any} */ a) => a.a === n.id);
  if (!arcs.length) return "";
  const blocks = arcs.map((/** @type {any} */ a) => {
    const from = j.nodi.find((/** @type {any} */ x) => x.id === a.da), r = arcRules(j, a);
    const head = I18N.t("msngrafo.rule.from", { mz: from ? from.mz.toFixed(4) : a.da, loss: a.perdita.formula ? prettyFormula(a.perdita.formula) : a.perdita.da_massa.toFixed(3) + " Da" });
    const items = r ? arcItems(r) : [];
    return h`<p class="sm"><b>${head}</b></p>` + (items.length ? h`<ul class="sm ig-rules">${raw(items.join(""))}</ul>` : h`<p class="muted sm">${I18N.t("msngrafo.rule.none")}</p>`);
  });
  return h`<div class="ig-rl"><b class="sm">${I18N.t("msngrafo.rule.title")}</b>${raw(blocks.join(""))}${raw(thresholds())}<p class="muted sm">${I18N.t("msngrafo.rule.hyp")}</p></div>`;
}

/** The three extra columns of the data table for an arc: rule ids, sources (key and DOI) and the best priority; empty for an ion or an arc no rule explains. */
export function arcCells(/** @type {any} */ j, /** @type {any} */ a) {
  const r = arcRules(j, a);
  if (!isExplained(r)) return ["", "", ""];
  const keys = [...new Set(r.regole.flatMap((/** @type {any} */ x) => x.refs))];
  const src = keys.map(k => { const f = (j.regole.fonti || []).find((/** @type {any} */ x) => x.key === k); return f && f.doi ? `${k} (doi:${f.doi})` : k; }).join("; ");
  const ids = r.regole.length ? r.regole.map((/** @type {any} */ x) => x.id).join("; ") : r.composta.map((/** @type {string[]} */ c) => c.join(" + ")).join("; ");
  return [ids, src, r.regole.length ? Math.min(...r.regole.map((/** @type {any} */ x) => x.priorita)) : ""];
}
