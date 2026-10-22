// @ts-check
// The panel of the selected ion of the «Fragmentation graph»: its candidate formulas (error, evidences, «Use»), the isolation window, the peaks of its spectra
// the graph could not explain, the warnings of the node and a drawing box (the fragment the user draws, kept next to the node: a hypothesis, not a result).
import { G, TXT, ionText, useCandidate, fetchStrutture } from "./infusione-grafo-stato.js";
import { rulesPanel } from "./infusione-grafo-regole.js";
import { formulaHtml, prettyFormula, splitCharge } from "./msngrafo-vista.js";
import { molSvg, pieceOf } from "./infusione-grafo-mol.js";

/** A formula of a candidate as HTML with subscript digits and the charge of the graph as a superscript. */
const fHtml = (/** @type {string} */ f) => raw(formulaHtml(...(/** @type {[string, string]} */ (splitCharge(f, G.j && G.j.polarita === -1 ? "-" : "+")))));

const ST = "mzlab.infg.frag";
/** @returns {Record<string, { smiles: string, svg: string }>} */
const frags = () => { try { return JSON.parse(localStorage.getItem(ST) || "{}"); } catch (_) { return {}; } };
const fragKey = (/** @type {any} */ n) => `${(G.j.sorgente && G.j.sorgente.sha256_idx) || ""}|${ionText()}|${n.mz.toFixed(3)}`;

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
  const rows = f.candidati.map((/** @type {any} */ c) => {
    const on = f.scelta === c.formula, prove = c.prove.map((/** @type {string} */ p) => I18N.t(/** @type {any} */ (TXT.prove)[p])).join(", ");
    const btn = f.candidati.length > 1 && !on ? h`<button type="button" class="bt ig-use" data-f="${c.formula}" title="${I18N.t("msngrafo.use.tip", { f: prettyFormula(c.formula) })}">${I18N.t("msngrafo.use")}</button>` : on ? I18N.t(f.fonte === "utente" ? "msngrafo.used" : "msngrafo.chosen") : "";
    return h`<tr${raw(on ? ' class="on"' : "")}><th scope="row">${fHtml(c.formula)}</th><td class="num">${c.mz_teorico.toFixed(5)}</td><td class="num">${ppmTxt(c.ppm)}</td><td class="num">${c.dbe}</td><td>${I18N.t(/** @type {any} */ (TXT.elettroni)[c.elettroni])}</td><td class="num">${c.finestra}</td><td>${prove}</td><td>${raw(btn)}</td></tr>`;
  });
  const head = ["msngrafo.col.cand", "msngrafo.col.mz", "msngrafo.col.err", "DBE", "msngrafo.col.e", "msngrafo.col.win", "msngrafo.col.prove", ""].map(k => raw(h`<th scope="col">${k.startsWith("msngrafo") ? I18N.t(k) : k}</th>`));
  return h`<table class="ig-cands"><caption class="sr-only">${I18N.t("msngrafo.cands.cap")}</caption><thead><tr>${head}</tr></thead><tbody>${rows.map((/** @type {string} */ r) => raw(r))}</tbody></table>`;
}

function isolation(/** @type {any} */ n) {
  const i = n.isolamento; if (!i) return "";
  const w = i.larghezza != null ? I18N.t("msngrafo.iso.win", { w: i.larghezza }) : i.a_monte != null ? I18N.t("msngrafo.iso.up", { w: i.a_monte }) : "";
  return h`<p class="sm">${w}${raw(i.stretto ? ` <span class="muted">${EH(I18N.t("msngrafo.iso.narrow"))}</span>` : "")}</p>`;
}

function unexplained(/** @type {any} */ n) {
  const u = n.non_spiegati; if (!u) return "";
  const top = u.picchi.map((/** @type {any} */ p) => `${p.mz.toFixed(4)} (${p.rel}%)`).join(", ");
  return h`<p class="sm ig-unexp">${I18N.t("msngrafo.unexp", { n: u.n })} ${top}</p>`;
}

/** The neutral loss that led to the node (`interpretazione` of the graph, when the engine gives it): name, formula, mass, error, confidence, activations; evidence, never a verdict. */
function lossLine(/** @type {any} */ n) {
  const it = G.j.interpretazione; if (!it) return "";
  const arcs = it.archi.filter((/** @type {any} */ a) => a.a === n.id && a.perdita);
  if (!arcs.length) return "";
  const rows = arcs.map((/** @type {any} */ a) => {
    const p = a.perdita, name = p.nome || (p.formula ? prettyFormula(p.formula) : "") || I18N.t("msngrafo.int.unnamed");
    const alt = p.alternative.length ? " · " + I18N.t("msngrafo.int.alt", { list: p.alternative.join(", ") }) : "";
    return h`<li>${I18N.t("msngrafo.int.line", { name, f: p.formula ? prettyFormula(p.formula) : "–", mass: p.massa.toFixed(4), ppm: ppmTxt(p.errore_ppm), conf: I18N.t(/** @type {any} */ (TXT.conf)[p.confidenza]), act: a.attivazioni.map((/** @type {string} */ x) => x.toUpperCase()).join("/") })}${alt}</li>`;
  });
  return h`<div class="ig-int"><b class="sm">${I18N.t("msngrafo.int.title")}</b><ul class="sm">${rows.map((/** @type {string} */ r) => raw(r))}</ul><p class="muted sm">${I18N.t("msngrafo.int.hyp")}</p></div>`;
}

/** How the ion was reached: the activation (CID/HCD) and the collision energy of each spectrum where it shows, from `misure` of the arcs that end in the node. */
function activationLine(/** @type {any} */ n) {
  const seen = new Set();
  for (const a of G.j.archi) if (a.a === n.id) for (const m of a.misure || []) {
    const act = m.att ? String(m.att).toUpperCase() : "";
    seen.add(`${act || "?"}${m.ce != null ? " " + m.ce : ""}`);
  }
  if (!seen.size) return "";
  return h`<p class="sm ig-act">${I18N.t("msngrafo.act.line", { list: [...seen].join(" · ") })}</p>`;
}

const warnings = (/** @type {any} */ n) => G.j.avvisi.filter((/** @type {any} */ a) => a.nodo === n.id).map((/** @type {any} */ a) => h`<p class="sm ig-warn">${I18N.t(/** @type {any} */ (TXT.warn)[a.chiave], { node: n.mz.toFixed(4) })}</p>`).join("");

/** The drawing box: the saved fragment (picture from MOLUI) with Draw / Save drawing / Remove. */
function drawBox(/** @type {any} */ n) {
  const v = savedFrag(n);
  const pic = v ? `<div class="ig-pic" role="img" aria-label="${EH(I18N.t("msngrafo.draw.aria", { smi: v.smiles }))}">${window.MOLUI ? MOLUI.svg(v.svg) : ""}</div><code class="sm">${EH(v.smiles)}</code>` : `<p class="muted sm">${EH(I18N.t("msngrafo.draw.none"))}</p>`;
  return `<div class="ig-draw"><b class="sm">${EH(I18N.t("msngrafo.draw.title"))}</b>${pic}<div class="row"><button type="button" class="bt" data-a="draw" title="${EH(I18N.t("msngrafo.draw.go.tip"))}">${EH(I18N.t("msngrafo.draw.go"))}</button><button type="button" class="bt" data-a="save" title="${EH(I18N.t("msngrafo.draw.save.tip"))}">${EH(I18N.t("msngrafo.draw.save"))}</button>${v ? `<button type="button" class="bt" data-a="del">${EH(I18N.t("msngrafo.draw.del"))}</button>` : ""}</div><p class="muted sm">${EH(I18N.t("msngrafo.draw.hyp"))}</p><div class="ig-str"></div></div>`;
}

/** One row per cut of the cleavage candidates (the candidates are ordered by error): bond, atoms of the piece that carries the charge, m/z, error, explained peaks. */
function cutsTable(/** @type {any} */ s) {
  const rows = [];
  s.candidati.forEach((/** @type {any} */ c, /** @type {number} */ i) => c.tagli.forEach((/** @type {any} */ t, /** @type {number} */ k) => rows.push({ i, k, t })));
  if (!rows.length) return h`<p class="muted sm">${I18N.t("msngrafo.str.none")}</p>`;
  const body = rows.slice(0, 20).map(({ i, k, t }) => {
    const peaks = t.picchi.map((/** @type {any} */ p) => p.mz.toFixed(4)).join(", ") || "—";
    return h`<tr><th scope="row">${t.legame.etichetta}</th><td class="num">${t.pezzo.length}</td><td class="num">${t.mz_teorico.toFixed(5)}</td><td class="num">${ppmTxt(t.ppm)}</td><td>${peaks}</td><td><button type="button" class="bt ig-cut" data-i="${i}" data-k="${k}" title="${I18N.t("msngrafo.str.show.tip", { bond: t.legame.etichetta })}" aria-label="${I18N.t("msngrafo.str.show.tip", { bond: t.legame.etichetta })}">${I18N.t("msngrafo.str.show")}</button></td></tr>`;
  });
  const head = ["msngrafo.str.col.bond", "msngrafo.str.col.atoms", "msngrafo.col.mz", "msngrafo.col.err", "msngrafo.str.col.peaks", ""].map(k => raw(h`<th scope="col">${k ? I18N.t(k) : ""}</th>`));
  const more = rows.length > 20 ? h`<p class="muted sm">${I18N.t("msngrafo.more", { n: rows.length - 20 })}</p>` : "";
  return h`<table class="ig-cuts"><caption class="sr-only">${I18N.t("msngrafo.str.cap")}</caption><thead><tr>${head}</tr></thead><tbody>${body.map((/** @type {string} */ r) => raw(r))}</tbody></table>${raw(more)}`;
}

/** The drawing of the molecule with the atoms of the piece of the best candidate highlighted (a hypothesis); empty without a molfile or OpenChemLib. */
function hypothesis(/** @type {any} */ s) {
  const atoms = pieceOf(s), svg = atoms && molSvg(atoms, 260, 150, "igp-" + s.nodo);
  if (!atoms || !svg) return "";
  return h`<div class="ig-hypo"><b class="sm">${I18N.t("msngrafo.v.hypo.title")}</b><div role="img" aria-label="${I18N.t("msngrafo.v.hypo.aria", { n: atoms.length })}">${raw(svg)}</div><p class="muted sm">${I18N.t("msngrafo.v.hypo.note")}</p></div>`;
}

/** Fills `.ig-str` of a node panel with the cleavage candidates, if the engine gives them; a click shows the broken bond in the drawing. */
async function fillCuts(/** @type {HTMLElement} */ host, /** @type {any} */ n) {
  const box = host.querySelector(".ig-str"); if (!box) return;
  const s = await fetchStrutture(n);
  if (!s || !box.isConnected) return;
  const warn = s.avvisi.map((/** @type {string} */ a) => h`<p class="sm ig-warn">${I18N.t(/** @type {any} */ (TXT.warn)[a])}</p>`).join("");
  box.innerHTML = hypothesis(s) + h`<b class="sm">${I18N.t("msngrafo.str.title")}</b>` + warn + cutsTable(s) + h`<p class="muted sm">${I18N.t("msngrafo.str.hyp")}</p>`;
  box.querySelectorAll(".ig-cut").forEach(b => b.addEventListener("click", () => {
    const d = /** @type {HTMLElement} */ (b).dataset;
    showCut(s.candidati[+(d.i || 0)].tagli[+(d.k || 0)].legame.atomi);
  }));
}

/** Opens Draw with the SMILES of the tab and marks the bond between two atoms (their order in the SMILES). */
async function showCut(/** @type {number[]} */ atoms) {
  if (typeof setView === "function") setView("draw");
  try {
    await TPDraw.load(G.smiles);
    if (!(await TPDraw.mark([atoms]))) nearMsg(I18N.t("msngrafo.str.nomark"));
  } catch (e) { console.info("INFG cut", e); nearMsg(I18N.t("msngrafo.draw.notReady")); }
}

async function openDraw(/** @type {any} */ n) {
  const v = savedFrag(n), smi = (v && v.smiles) || (n.livello === 1 ? G.smiles : "");
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
  if (!n) { host.innerHTML = h`<p class="muted sm">${I18N.t("msngrafo.pick")}</p>`; return; }
  host.innerHTML = h`<h4>${I18N.t("msngrafo.node.title", { level: n.livello, mz: n.mz.toFixed(4) })}</h4>` + candidates(n) + activationLine(n) + lossLine(n) + rulesPanel(n) + isolation(n) + unexplained(n) + warnings(n) + drawBox(n);
  fillCuts(host, n);
  host.querySelectorAll(".ig-use").forEach(b => b.addEventListener("click", () => {
    useCandidate(n, /** @type {string} */ (/** @type {HTMLElement} */ (b).dataset.f)); cb.redo();
  }));
  const on = (/** @type {string} */ a, /** @type {() => any} */ fn) => { const b = host.querySelector(`[data-a="${a}"]`); if (b) b.addEventListener("click", fn); };
  on("draw", () => openDraw(n));
  on("save", async () => { await saveDrawing(n); cb.again(); });
  on("del", () => { saveFrag(n, null); cb.again(); });
}
