// @ts-strict
// The panel of the selected ion of the «Fragmentation graph»: its candidate formulas (error, evidences, «Use»), the isolation window, the peaks of its spectra
// the graph could not explain, the warnings of the node and a drawing box (the fragment the user draws, kept next to the node: a hypothesis, not a result).
import { G, TXT, ionText, useCandidate, fetchStrutture } from "./infusione-grafo-stato.js";
import { rulesPanel } from "./infusione-grafo-regole.js";
import { breaksPanel } from "./infusione-grafo-rotture.js";
import { formulaHtml, prettyFormula, splitCharge } from "./msngrafo-vista.js";
import { molSvg, pieceOf } from "./infusione-grafo-mol.js";

/** A formula of a candidate as HTML with subscript digits and the charge of the graph as a superscript. */
const fHtml = (/** @type {string} */ f) => raw(formulaHtml(...(/** @type {[string, string]} */ (splitCharge(f, G.j && G.j.polarity === -1 ? "-" : "+")))));

const ST = "mzlab.infg.frag";
/** @returns {Record<string, { smiles: string, svg: string }>} */
const frags = () => { try { return JSON.parse(localStorage.getItem(ST) || "{}"); } catch (_) { return {}; } };
const fragKey = (/** @type {any} */ n) => `${(G.j.source && G.j.source.sha256_idx) || ""}|${ionText()}|${n.mz.toFixed(3)}`;

/** The saved drawing of a node, or null. */
export const savedFrag = (/** @type {any} */ n) => frags()[fragKey(n)] || null;

function saveFrag(/** @type {any} */ n, /** @type {{ smiles: string, svg: string } | null} */ v) {
  const all = frags();
  if (v) all[fragKey(n)] = v; else delete all[fragKey(n)];
  try { localStorage.setItem(ST, JSON.stringify(all)); } catch (_) { /* storage full or blocked: the drawing stays in the editor */ }
}

const ppmTxt = (/** @type {number} */ v) => `${v > 0 ? "+" : v < 0 ? "−" : ""}${Math.abs(v).toFixed(2)} ppm`;

/** The table of the candidates of a node; a «Use» button on each when there is more than one. */
function candidates(/** @type {any} */ n) {
  const f = n.formula;
  if (!f) return h`<p class="muted sm">${I18N.t("msngrafo.nof")}</p>`;
  const rows = f.candidates.map((/** @type {any} */ c) => {
    const on = f.choice === c.formula, prove = c.proofs.map((/** @type {string} */ p) => I18N.t(/** @type {any} */ (TXT.proofs)[p])).join(", ");
    const btn = f.candidates.length > 1 && !on && G.ro ? h`<button type="button" class="bt ig-use" disabled aria-disabled="true" title="${I18N.t("msngrafo.file.off")}">${I18N.t("msngrafo.use")}</button>` : f.candidates.length > 1 && !on ? h`<button type="button" class="bt ig-use" data-f="${c.formula}" title="${I18N.t("msngrafo.use.tip", { f: prettyFormula(c.formula) })}">${I18N.t("msngrafo.use")}</button>` : on ? I18N.t(f.source === "user" ? "msngrafo.used" : "msngrafo.chosen") : "";
    return h`<tr${raw(on ? ' class="on"' : "")}><th scope="row">${fHtml(c.formula)}</th><td class="num">${c.theoretical_mz.toFixed(5)}</td><td class="num">${ppmTxt(c.ppm)}</td><td class="num">${c.dbe}</td><td>${I18N.t(/** @type {any} */ (TXT.electrons)[c.electrons])}</td><td class="num">${c.window}</td><td>${prove}</td><td>${raw(btn)}</td></tr>`;
  });
  const head = ["msngrafo.col.cand", "msngrafo.col.mz", "msngrafo.col.err", "DBE", "msngrafo.col.e", "msngrafo.col.win", "msngrafo.col.proofs", ""].map(k => raw(h`<th scope="col">${k.startsWith("msngrafo") ? I18N.t(k) : k}</th>`));
  return h`<table class="ig-cands"><caption class="sr-only">${I18N.t("msngrafo.cands.cap")}</caption><thead><tr>${head}</tr></thead><tbody>${rows.map((/** @type {string} */ r) => raw(r))}</tbody></table>`;
}

function isolation(/** @type {any} */ n) {
  const i = n.isolation; if (!i) return "";
  const w = i.width_da != null ? I18N.t("msngrafo.iso.win", { w: i.width_da }) : i.upstream_da != null ? I18N.t("msngrafo.iso.up", { w: i.upstream_da }) : "";
  return h`<p class="sm">${w}${raw(i.narrow ? ` <span class="muted">${EH(I18N.t("msngrafo.iso.narrow"))}</span>` : "")}</p>`;
}

function unexplained(/** @type {any} */ n) {
  const u = n.unexplained; if (!u) return "";
  const top = u.peaks.map((/** @type {any} */ p) => `${p.mz.toFixed(4)} (${p.rel}%)`).join(", ");
  return h`<p class="sm ig-unexp">${I18N.t("msngrafo.unexp", { n: u.n })} ${top}</p>`;
}

/** The neutral loss that led to the node (`interpretation` of the graph, when the engine gives it): name, formula, mass, error, confidence, activations; evidence, never a verdict. */
function lossLine(/** @type {any} */ n) {
  const it = G.j.interpretation; if (!it) return "";
  const arcs = it.edges.filter((/** @type {any} */ a) => a.to === n.id && a.loss);
  if (!arcs.length) return "";
  const rows = arcs.map((/** @type {any} */ a) => {
    const p = a.loss, name = p.name || (p.formula ? prettyFormula(p.formula) : "") || I18N.t("msngrafo.int.unnamed");
    const alt = p.alternative.length ? " · " + I18N.t("msngrafo.int.alt", { list: p.alternative.join(", ") }) : "";
    return h`<li>${I18N.t("msngrafo.int.line", { name, f: p.formula ? prettyFormula(p.formula) : "–", mass: p.mass.toFixed(4), ppm: ppmTxt(p.error_ppm), conf: I18N.t(/** @type {any} */ (TXT.conf)[p.confidence]), act: a.activations.map((/** @type {string} */ x) => x.toUpperCase()).join("/") })}${alt}</li>`;
  });
  return h`<div class="ig-int"><b class="sm">${I18N.t("msngrafo.int.title")}</b><ul class="sm">${rows.map((/** @type {string} */ r) => raw(r))}</ul><p class="muted sm">${I18N.t("msngrafo.int.hyp")}</p></div>`;
}

/** How the ion was reached: the activation (CID/HCD) and the collision energy of each spectrum where it shows, from `measurements` of the arcs that end in the node. */
function activationLine(/** @type {any} */ n) {
  const seen = new Set();
  for (const a of G.j.edges) if (a.to === n.id) for (const m of a.measurements || []) {
    const act = m.activation ? String(m.activation).toUpperCase() : "";
    seen.add(`${act || "?"}${m.ce != null ? " " + m.ce : ""}`);
  }
  if (!seen.size) return "";
  return h`<p class="sm ig-act">${I18N.t("msngrafo.act.line", { list: [...seen].join(" · ") })}</p>`;
}

const warnings = (/** @type {any} */ n) => G.j.warnings.filter((/** @type {any} */ a) => a.node === n.id).map((/** @type {any} */ a) => h`<p class="sm ig-warn">${I18N.t(/** @type {any} */ (TXT.warn)[a.key], { node: n.mz.toFixed(4) })}</p>`).join("");

/** In a graph file opened without raw data: why the spectrum of the ion cannot be opened. */
const roNote = () => (G.ro ? h`<p class="muted sm ig-ro-note">${I18N.t("msngrafo.file.nospec")}</p>` : "");

/** The drawing box: the saved fragment (picture from MOLUI) with Draw / Save drawing / Remove. */
function drawBox(/** @type {any} */ n) {
  const v = savedFrag(n);
  const pic = v ? `<div class="ig-pic" role="img" aria-label="${EH(I18N.t("msngrafo.draw.aria", { smi: v.smiles }))}">${window.MOLUI ? MOLUI.svg(v.svg) : ""}</div><code class="sm">${EH(v.smiles)}</code>` : `<p class="muted sm">${EH(I18N.t("msngrafo.draw.none"))}</p>`;
  return `<div class="ig-draw"><b class="sm">${EH(I18N.t("msngrafo.draw.title"))}</b>${pic}<div class="row"><button type="button" class="bt" data-a="draw" title="${EH(I18N.t("msngrafo.draw.go.tip"))}">${EH(I18N.t("msngrafo.draw.go"))}</button><button type="button" class="bt" data-a="save" title="${EH(I18N.t("msngrafo.draw.save.tip"))}">${EH(I18N.t("msngrafo.draw.save"))}</button>${v ? `<button type="button" class="bt" data-a="del">${EH(I18N.t("msngrafo.draw.del"))}</button>` : ""}</div><p class="muted sm">${EH(I18N.t("msngrafo.draw.hyp"))}</p><div class="ig-str"></div></div>`;
}

/** One row per cut of the cleavage candidates (the candidates are ordered by error): bond, atoms of the piece that carries the charge, m/z, error, explained peaks. */
function cutsTable(/** @type {any} */ s) {
  const rows = [];
  s.candidates.forEach((/** @type {any} */ c, /** @type {number} */ i) => c.cuts.forEach((/** @type {any} */ t, /** @type {number} */ k) => rows.push({ i, k, t })));
  if (!rows.length) return h`<p class="muted sm">${I18N.t("msngrafo.str.none")}</p>`;
  const body = rows.slice(0, 20).map(({ i, k, t }) => {
    const peaks = t.peaks.map((/** @type {any} */ p) => p.mz.toFixed(4)).join(", ") || "—";
    return h`<tr><th scope="row">${t.bond.label}</th><td class="num">${t.piece.length}</td><td class="num">${t.theoretical_mz.toFixed(5)}</td><td class="num">${ppmTxt(t.ppm)}</td><td>${peaks}</td><td><button type="button" class="bt ig-cut" data-i="${i}" data-k="${k}" title="${I18N.t("msngrafo.str.show.tip", { bond: t.bond.label })}" aria-label="${I18N.t("msngrafo.str.show.tip", { bond: t.bond.label })}">${I18N.t("msngrafo.str.show")}</button></td></tr>`;
  });
  const head = ["msngrafo.str.col.bond", "msngrafo.str.col.atoms", "msngrafo.col.mz", "msngrafo.col.err", "msngrafo.str.col.peaks", ""].map(k => raw(h`<th scope="col">${k ? I18N.t(k) : ""}</th>`));
  const more = rows.length > 20 ? h`<p class="muted sm">${I18N.t("msngrafo.more", { n: rows.length - 20 })}</p>` : "";
  return h`<table class="ig-cuts"><caption class="sr-only">${I18N.t("msngrafo.str.cap")}</caption><thead><tr>${head}</tr></thead><tbody>${body.map((/** @type {string} */ r) => raw(r))}</tbody></table>${raw(more)}`;
}

/** The drawing of the molecule with the atoms of the piece of the best candidate highlighted (a hypothesis); empty without a molfile or OpenChemLib. */
function hypothesis(/** @type {any} */ s) {
  const atoms = pieceOf(s), svg = atoms && molSvg(atoms, 260, 150, "igp-" + s.node);
  if (!atoms || !svg) return "";
  return h`<div class="ig-hypo"><b class="sm">${I18N.t("msngrafo.v.hypo.title")}</b><div role="img" aria-label="${I18N.t("msngrafo.v.hypo.aria", { n: atoms.length })}">${raw(svg)}</div><p class="muted sm">${I18N.t("msngrafo.v.hypo.note")}</p></div>`;
}

/** Fills `.ig-str` of a node panel with the cleavage candidates, if the engine gives them; a click shows the broken bond in the drawing. */
async function fillCuts(/** @type {HTMLElement} */ host, /** @type {any} */ n) {
  const box = host.querySelector(".ig-str"); if (!box) return;
  const s = await fetchStrutture(n);
  if (!s || !box.isConnected) return;
  const warn = s.warnings.map((/** @type {string} */ a) => h`<p class="sm ig-warn">${I18N.t(/** @type {any} */ (TXT.warn)[a])}</p>`).join("");
  box.innerHTML = hypothesis(s) + h`<b class="sm">${I18N.t("msngrafo.str.title")}</b>` + warn + cutsTable(s) + h`<p class="muted sm">${I18N.t("msngrafo.str.hyp")}</p>`;
  box.querySelectorAll(".ig-cut").forEach(b => b.addEventListener("click", () => {
    const d = /** @type {HTMLElement} */ (b).dataset;
    const cut = s.candidates[+(d.i || 0)].cuts[+(d.k || 0)];
    showCut(cut.bond.atoms, cut.bond.label);
  }));
}

/** Opens Draw with the SMILES of the tab, marks the bond between two atoms (their order in the SMILES) and colours it as a cut (evidence colours, `highlight/1`). */
async function showCut(/** @type {number[]} */ atoms, /** @type {string} */ bond) {
  if (typeof setView === "function") setView("draw");
  try {
    if (!(await TPDraw.load(G.smiles))) return;                 // the user kept the drawing on the canvas: nothing to mark
    if (!(await TPDraw.mark([atoms]))) { nearMsg(I18N.t("msngrafo.str.nomark")); return; }
    await TPDraw.highlight([{ kind: "cuts", smiles: G.smiles, bonds: [atoms], label: I18N.t("msngrafo.str.ev.label", { bond }) }]);
  } catch (e) { console.info("INFG cut", e); nearMsg(I18N.t("msngrafo.draw.notReady")); }
}

async function openDraw(/** @type {any} */ n) {
  const v = savedFrag(n), smi = (v && v.smiles) || (n.level === 1 ? G.smiles : "");
  if (typeof setView === "function") setView("draw");
  try { if (window.TPDraw && smi) await TPDraw.load(smi); } catch (e) { console.info("INFG draw", e); }
}

async function saveDrawing(/** @type {any} */ n) {
  if (!window.TPDraw || !TPDraw.ready()) return nearMsg(I18N.t("msngrafo.draw.notReady"));
  try {
    const smiles = await TPDraw.smiles(), blob = await TPDraw.image("svg");
    if (!smiles) return nearMsg(I18N.t("msngrafo.draw.empty"));
    saveFrag(n, { smiles, svg: await blob.text() });
  } catch (e) { console.info("INFG save", e); nearMsg(I18N.t("msngrafo.draw.notReady")); }
}

/** Fills the panel of a node. `redo` asks for the graph again (a candidate was chosen), `again` draws the panel again. */
export function nodePanel(/** @type {HTMLElement} */ host, /** @type {any} */ n, /** @type {{ redo: () => void, again: () => void }} */ cb) {
  if (!n) { host.innerHTML = ""; return; }
  host.innerHTML = h`<h4>${I18N.t("msngrafo.node.title", { level: n.level, mz: n.mz.toFixed(4) })}</h4>` + candidates(n) + activationLine(n) + lossLine(n) + rulesPanel(n) + breaksPanel(n) + isolation(n) + unexplained(n) + warnings(n) + roNote() + drawBox(n);
  fillCuts(host, n);
  host.querySelectorAll(".ig-use").forEach(b => b.addEventListener("click", () => {
    useCandidate(n, /** @type {string} */ (/** @type {HTMLElement} */ (b).dataset.f)); cb.redo();
  }));
  const on = (/** @type {string} */ a, /** @type {() => any} */ fn) => { const b = host.querySelector(`[data-a="${a}"]`); if (b) b.addEventListener("click", fn); };
  on("draw", () => openDraw(n));
  on("save", async () => { await saveDrawing(n); cb.again(); });
  on("del", () => { saveFrag(n, null); cb.again(); });
}
