// @ts-strict
// «Fragmentation graph» tab of an MSn infusion (banco.js, BODY.gra): the Ion formula field (formula or name), the settings, the graph the engine builds from the
// file already loaded (/api/msn_grafo), the panel of the selected ion, the export of the cleaned spectra and «Add to library». JS only draws: the formulas,
// the ppm, the filters and the texts of the exports come from the engine. Parts: infusione-grafo-{stato,campo,svg,nodo,esporta}.js. Loaded by banco.js on first use.
import { G, TXT, fetchGraph, nodeOf, chosen } from "./infusione-grafo-stato.js";
import { drawField } from "./infusione-grafo-campo.js";
import { drawGraph } from "./infusione-grafo-svg.js";
import { nodePanel } from "./infusione-grafo-nodo.js";
import { exportRow } from "./infusione-grafo-esporta.js";
import { searchRow } from "./infusione-grafo-cerca.js";
import { filtered, arcCells } from "./infusione-grafo-regole.js";
import { addStyle } from "./infusione-grafo-css.js";
import { guarda, mostraErrore } from "./infusione-grafo-avanza.js";
import { openRow, dropZone } from "./infusione-grafo-apri.js";
import { wireGestures } from "./infusione-grafo-gesti.js";
import { fullButton, leaveFull } from "./infusione-grafo-schermo.js";
import { prepareMolfile } from "./infusione-grafo-mol.js";
import { formulaHtml, prettyFormula, splitCharge, vistaOf } from "./msngrafo-vista.js";

/** @type {{ el: HTMLElement | null, svg: SVGSVGElement | null, scale: number }} */
const V = { el: null, svg: null, scale: 1 };
let tokenNow = 0;

const nodeLabel = (/** @type {any} */ n) => `MS${n.livello} ${n.mz.toFixed(4)}`;

/** The rows of «the data of the chart»: one per ion with the formula, the candidates, the levels and the intensity, then one per loss. `pretty` writes formulas with Unicode subscripts (what the dialog shows); «Copy data» stays plain ASCII. */
function dataTable(pretty = false) {
  const j = filtered(G.j, G.rf), v = vistaOf(j), byId = new Map(j.nodi.map((/** @type {any} */ n) => [n.id, n])), vx = new Map(v.nodi.map((/** @type {any} */ x) => [x.id, x]));
  const sign = j.polarita === -1 ? "-" : "+", fm = (/** @type {string} */ f) => (pretty ? prettyFormula(...(/** @type {[string, string]} */ (splitCharge(f, sign)))) : f);
  const lv = (/** @type {number[]} */ l) => l.map(x => "MS" + x).join(", ");
  const rows = j.nodi.map((/** @type {any} */ n) => {
    const x = vx.get(n.id), c = chosen(n);
    return [n.id, lv(x.livelli), n.mz.toFixed(5), x.best.da ? String(x.rel) : "", c ? fm(c) : "", n.formula ? n.formula.candidati.map((/** @type {any} */ k) => `${fm(k.formula)} ${k.ppm} ppm`).join("; ") : "", "", "", x.comune ? String(x.entranti) : "", n.isolato ? I18N.t("msngrafo.yes") : "", n.osservazioni, "", "", ""];
  });
  const best = new Set(v.archi.filter((/** @type {any} */ a) => a.best).map((/** @type {any} */ a) => a.da + ">" + a.a));
  const arcs = j.archi.map((/** @type {any} */ a) => {
    const p = a.perdita, loss = p.formula ? fm(p.formula) : `${p.da_massa.toFixed(5)} Da`, comp = !p.formula && p.compatibile.length ? I18N.t("msngrafo.v.compat", { f: p.compatibile.map(fm).join(" / ") }) : "";
    return [`${a.da}>${a.a}`, lv([...new Set(a.misure.map((/** @type {any} */ m) => m.livello))]), `${(/** @type {any} */ (byId.get(a.a))).mz.toFixed(5)}`, String(Math.max(...a.misure.map((/** @type {any} */ m) => m.rel), 0)), "", comp, loss, best.has(a.da + ">" + a.a) ? I18N.t("msngrafo.yes") : "", "", a.isolamento ? I18N.t("msngrafo.yes") : "", a.misure.length, ...arcCells(j, a)];
  });
  return { title: I18N.t("msngrafo.data.title"), head: ["id", I18N.t("msngrafo.v.col.levels"), "m/z", I18N.t("msngrafo.v.col.rel"), I18N.t("msngrafo.col.chosen"), I18N.t("msngrafo.col.cand"), I18N.t("msngrafo.v.col.loss"), I18N.t("msngrafo.v.col.best"), I18N.t("msngrafo.v.col.common"), I18N.t("msngrafo.col.iso"), I18N.t("msngrafo.col.n"), I18N.t("msngrafo.col.rule"), I18N.t("msngrafo.col.source"), I18N.t("msngrafo.col.priority")], rows: [...rows, ...arcs], rowHeader: true };
}

async function copyData() {
  const t = dataTable(false), tsv = [t.head, ...t.rows].map(r => r.join("\t")).join("\n");
  try { await navigator.clipboard.writeText(await PROV.tsv(tsv, { table: "msn-graph" })); nearMsg(I18N.t("tdati.copied")); } catch (_) { nearMsg(I18N.t("msngrafo.copy.fail")); }
}

/** The spectrum of a node in the active cell: its own when it was isolated, else the one of the precursor it comes from. */
function openSpectrum(/** @type {string} */ id) {
  const n = nodeOf(id); if (!n || !G.j) return;
  const arc = G.j.archi.find((/** @type {any} */ a) => a.a === id), tree = n.albero[0] || (arc && arc.misure[0] && arc.misure[0].albero);
  const cell = INF.target(E.active);
  if (tree && cell) INF.pick(cell, tree);
}

function zoomBar(/** @type {HTMLElement} */ bar, /** @type {HTMLElement} */ wrap, /** @type {() => void} */ redraw) {
  const act = { in: () => setScale(V.scale * 1.25), out: () => setScale(V.scale / 1.25), fit: () => setScale(1), l: () => wrap.scrollBy(-80, 0), r: () => wrap.scrollBy(80, 0), u: () => wrap.scrollBy(0, -80), d: () => wrap.scrollBy(0, 80) };
  const defs = [["in", "+", "msngrafo.zoom.in"], ["out", "−", "msngrafo.zoom.out"], ["fit", "1:1", "msngrafo.zoom.fit"], ["l", "←", "msngrafo.pan.l"], ["r", "→", "msngrafo.pan.r"], ["u", "↑", "msngrafo.pan.u"], ["d", "↓", "msngrafo.pan.d"]];
  bar.innerHTML = defs.map(([a, t, k]) => h`<button type="button" class="bt" data-z="${a}" aria-label="${I18N.t(k)}" title="${I18N.t(k)}">${t}</button>`).join("") +
    h`<button type="button" class="bt" id="ig-data" title="${I18N.t("tdati.btn.title")}">${I18N.t("tdati.btn")}</button><button type="button" class="bt" id="ig-copyd">${I18N.t("msngrafo.copydata")}</button><label class="ig-flt" title="${I18N.t("msngrafo.rule.filter.tip")}"><input type="checkbox" id="ig-rf"${raw(G.rf ? " checked" : "")}${raw(G.ro && !(G.j && G.j.regole) ? ' disabled title="' + EH(I18N.t("msngrafo.file.off")) + '"' : "")} aria-label="${I18N.t("msngrafo.rule.filter.aria")}"> ${I18N.t("msngrafo.rule.filter")}</label>`;
  bar.querySelectorAll("[data-z]").forEach(b => b.addEventListener("click", () => /** @type {any} */ (act)[/** @type {string} */ (/** @type {HTMLElement} */ (b).dataset.z)]()));
  bar.querySelector("#ig-data")?.addEventListener("click", () => G.j && TDATI.table(dataTable(true)));
  bar.querySelector("#ig-copyd")?.addEventListener("click", () => G.j && copyData());
  bar.querySelector("#ig-rf")?.addEventListener("change", e => { G.rf = /** @type {HTMLInputElement} */ (e.target).checked; if (G.j) redraw(); });
}

/** The second toolbar: view (tree / radial), filters (min %, levels, best path only), close the subtree of the selected ion, structures. Built once; `syncBar` follows the graph. */
function viewBar(/** @type {HTMLElement} */ bar, /** @type {() => void} */ redraw) {
  const T = (/** @type {string} */ k) => I18N.t(/** @type {any} */ (k));
  bar.innerHTML = h`<span class="ig-mode" role="group" aria-label="${T("msngrafo.v.view.aria")}"><button type="button" class="bt" id="ig-vt" title="${T("msngrafo.v.view.tree.tip")}">${T("msngrafo.v.view.tree")}</button><button type="button" class="bt" id="ig-vr" title="${T("msngrafo.v.view.radial.tip")}">${T("msngrafo.v.view.radial")}</button></span>` +
    h`<label class="ig-num" title="${T("msngrafo.v.minrel.tip")}">${T("msngrafo.v.minrel")} <input type="number" id="ig-minrel" min="0" max="100" step="1" value="${G.minRel}"></label>` +
    h`<label class="ig-flt" title="${T("msngrafo.v.level.tip")}">${T("msngrafo.v.level")} <select id="ig-lvl"></select></label>` +
    h`<label class="ig-flt" title="${T("msngrafo.v.best.tip")}"><input type="checkbox" id="ig-best"> ${T("msngrafo.v.best")}</label>` +
    h`<button type="button" class="bt" id="ig-collapse" title="${T("msngrafo.v.close.tip")}">${T("msngrafo.v.close")}</button>` +
    h`<label class="ig-flt" id="ig-structs-l" title="${T("msngrafo.v.structs.tip")}"><input type="checkbox" id="ig-structs"> ${T("msngrafo.v.structs")}</label>`;
  const q = (/** @type {string} */ id) => /** @type {HTMLInputElement} */ (bar.querySelector("#" + id));
  q("ig-vt").addEventListener("click", () => { G.view = "tree"; redraw(); });
  q("ig-vr").addEventListener("click", () => { G.view = "radial"; redraw(); });
  q("ig-minrel").addEventListener("change", e => { const v = parseFloat(/** @type {HTMLInputElement} */ (e.target).value); G.minRel = Number.isFinite(v) ? Math.min(100, Math.max(0, v)) : 0; redraw(); });
  q("ig-lvl").addEventListener("change", e => { G.maxLevel = +(/** @type {HTMLSelectElement} */ (e.target).value); redraw(); });
  q("ig-best").addEventListener("change", e => { G.bestOnly = /** @type {HTMLInputElement} */ (e.target).checked; redraw(); });
  q("ig-structs").addEventListener("change", e => { G.structs = /** @type {HTMLInputElement} */ (e.target).checked; redraw(); });
  q("ig-collapse").addEventListener("click", () => { if (!G.sel) return; if (!G.collapsed.delete(G.sel)) G.collapsed.add(G.sel); redraw(); });
}

/** Brings the second toolbar in line with the graph and the state: pressed view, levels offered, closed or open subtree, structures only with a molfile. */
function syncBar(/** @type {HTMLElement} */ b) {
  const j = G.j; if (!j) return;
  const q = (/** @type {string} */ id) => /** @type {HTMLInputElement} */ (b.querySelector("#" + id));
  const radial = !!j.vista;
  if (!radial && G.view === "radial") G.view = "tree";
  q("ig-vt").setAttribute("aria-pressed", String(G.view === "tree")); q("ig-vr").setAttribute("aria-pressed", String(G.view === "radial")); q("ig-vr").disabled = !radial;
  q("ig-minrel").value = String(G.minRel); q("ig-best").checked = G.bestOnly; q("ig-structs").checked = G.structs;
  const top = Math.max(1, ...j.nodi.map((/** @type {any} */ n) => n.livello)), sel = q("ig-lvl"), vals = [0, ...Array.from({ length: Math.max(0, top - 2) }, (_, i) => i + 2)];
  if (sel.dataset.top !== String(top)) { sel.dataset.top = String(top); sel.innerHTML = vals.map(v => h`<option value="${v}">${v ? I18N.t("msngrafo.v.level.upto", { n: v }) : I18N.t("msngrafo.v.level.all")}</option>`).join(""); }
  sel.value = String(G.maxLevel);
  const kids = !!G.sel && vistaOf(j).nodi.some((/** @type {any} */ x) => x.best.da === G.sel), shut = !!G.sel && G.collapsed.has(G.sel), btn = q("ig-collapse");
  btn.disabled = !(kids || shut); btn.textContent = I18N.t(shut ? "msngrafo.v.open" : "msngrafo.v.close"); btn.setAttribute("aria-pressed", String(shut));
  /** @type {HTMLElement} */ (b.querySelector("#ig-structs-l")).hidden = !G.molfile;
}

function setScale(/** @type {number} */ s) {
  V.scale = Math.min(4, Math.max(0.4, s));
  const svg = V.svg; if (!svg) return;
  const vb = svg.viewBox.baseVal;
  svg.setAttribute("width", String(vb.width * V.scale)); svg.setAttribute("height", String(vb.height * V.scale));
}

/** Warnings: one line each, with the ion if the warning is about one. */
function warnLines() {
  const j = G.j; if (!j) return "";
  const seen = new Set();
  return j.avvisi.map((/** @type {any} */ a) => {
    const n = nodeOf(a.nodo), txt = I18N.t(/** @type {any} */ (TXT.warn)[a.chiave], { node: n ? n.mz.toFixed(4) : "" }), key = a.chiave + "|" + a.nodo;
    if (seen.has(key)) return ""; seen.add(key);
    return h`<p class="sm ig-warn" role="status">${txt}</p>`;
  }).join("");
}

function summary() {
  const j = G.j; if (!j) return "";
  const i = j.ione, c = j.conteggi;
  const counts = I18N.t("msngrafo.counts", { nodes: c.nodi, arcs: c.archi });
  if (!i) return h`<p class="sm" id="ig-sum">${I18N.t("msngrafo.masses")} · ${counts}</p>`;
  // the formula is drawn with subscripts and the charge as a superscript: the line is split at the place of the formula
  const [pre, post] = I18N.t("msngrafo.ion.line", { f: "\u0001", mz: i.mz_teorico.toFixed(5), ppm: i.ppm }).split("\u0001"), [f, ch] = splitCharge(i.formula, j.polarita === -1 ? "-" : "+");
  return h`<p class="sm" id="ig-sum">${pre}${raw(formulaHtml(f, ch))}${post} · ${counts}</p>`;
}

function settings(/** @type {any} */ file) {
  const { o } = G, num = (/** @type {string} */ id, /** @type {string} */ k, /** @type {number} */ v, /** @type {string} */ tip) => h`<label title="${I18N.t(tip)}">${I18N.t(k)} <input type="number" id="ig-${id}" value="${v}" step="any" min="0"></label>`;
  return h`<details class="ig-set"><summary>${I18N.t("msngrafo.settings")}</summary><div class="row">` +
    num("ppmf", "msngrafo.ppm", o.ppm_formula, "msngrafo.ppm.tip") + num("mzl", "msngrafo.mzlow", o.mz_basso, "msngrafo.mzlow.tip") + num("ppml", "msngrafo.ppmlow", o.ppm_basso, "msngrafo.ppmlow.tip") + num("pm", "msngrafo.proofs", o.prove_min, "msngrafo.proofs.tip") +
    h`<label>${I18N.t("msngrafo.activation")} <select id="ig-act">${["", "cid", "hcd"].map(a => raw(h`<option value="${a}"${raw(a === o.attivazione ? " selected" : "")}>${a ? a.toUpperCase() : I18N.t("msngrafo.act.all")}</option>`))}</select></label>` +
    h`<label><input type="checkbox" id="ig-iso"${raw(o.isotopi ? " checked" : "")}> ${I18N.t("msngrafo.isotopes")}</label></div></details>`;
}

function wireSettings(/** @type {HTMLElement} */ b, /** @type {() => void} */ go) {
  const nums = /** @type {const} */ ([["ppmf", "ppm_formula"], ["mzl", "mz_basso"], ["ppml", "ppm_basso"], ["pm", "prove_min"]]);
  nums.forEach(([id, key]) => { const e = /** @type {HTMLInputElement | null} */ (b.querySelector("#ig-" + id)); if (e) e.onchange = () => { const v = parseFloat(e.value); if (Number.isFinite(v) && v >= 0) { G.o[key] = v; go(); } }; });
  const act = /** @type {HTMLSelectElement | null} */ (b.querySelector("#ig-act")), iso = /** @type {HTMLInputElement | null} */ (b.querySelector("#ig-iso"));
  if (act) act.onchange = () => { G.o.attivazione = act.value; go(); };
  if (iso) iso.onchange = () => { G.o.isotopi = iso.checked; go(); };
}

/** Draws the graph, the panel of the node and the export row from the answer in G.j. */
function drawBody(/** @type {HTMLElement} */ b, /** @type {any} */ file, /** @type {boolean} */ keepFocus) {
  const wrap = /** @type {HTMLElement} */ (b.querySelector("#ig-wrap")), pan = /** @type {HTMLElement} */ (b.querySelector("#ig-node")), exp = /** @type {HTMLElement} */ (b.querySelector("#ig-exp")), find = /** @type {HTMLElement} */ (b.querySelector("#ig-find"));
  if (!G.j || !G.j.nodi.length) { wrap.textContent = ""; pan.innerHTML = ""; exp.innerHTML = ""; find.innerHTML = ""; G.auto = false; return; }
  const redo = () => run(b, file);
  syncBar(b);
  V.svg = drawGraph(wrap, filtered(G.j, G.rf), { onSelect: id => { G.sel = id; drawBody(b, file, true); }, onOpen: G.ro ? () => {} : openSpectrum });
  setScale(V.scale);
  if (keepFocus && G.sel) /** @type {HTMLElement | null} */ (wrap.querySelector(`.ig-node[data-id="${G.sel}"]`))?.focus();
  nodePanel(pan, nodeOf(G.sel), { redo, again: () => drawBody(b, file, false) });
  if (G.ro) { exp.innerHTML = ""; find.innerHTML = ""; } else {
    exportRow(exp, file, G.sel);
    const auto = G.auto; G.auto = false;
    searchRow(find, file, auto);
  }
  if (window.TDATI && V.svg) { V.svg.setAttribute("aria-describedby", "ig-sum"); TDATI.attach(/** @type {any} */ (V.svg), () => dataTable(true)); }
}

/** Asks for the graph with the state in force and draws the answer. */
async function run(/** @type {HTMLElement} */ b, /** @type {any} */ file) {
  const mine = ++tokenNow;
  const msg = /** @type {HTMLElement} */ (b.querySelector("#ig-msg"));
  let hung = false;
  const stop = guarda(msg, (s, phase) => {                                  // no news for 30 s: an error with its diagnosis, the late answer is ignored
    hung = true; tokenNow++;
    const text = I18N.t("msngrafo.stalled", { s, phase: I18N.t(/** @type {Record<string, string>} */ (TXT.stalled)[phase]) });
    mostraErrore(msg, text, { key: "err.msngrafo.stalled", params: { s, phase } }, file, phase);
  });
  await prepareMolfile();
  const ok = await fetchGraph(file.k);
  stop();
  if (hung || !ok || mine !== tokenNow || !b.isConnected) return;
  if (G.error) mostraErrore(msg, G.error, G.errorObj, file);
  else msg.innerHTML = summary() + warnLines();
  drawBody(b, file, false);
}

/** Gestures on the canvas, and (in the tab) the full screen button at the end of the first toolbar. */
function wireStage(/** @type {HTMLElement} */ b, /** @type {boolean} */ full) {
  wireGestures(/** @type {HTMLElement} */ (b.querySelector("#ig-wrap")), { get: () => V.scale, set: setScale });
  if (full) fullButton(/** @type {HTMLElement} */ (b.querySelector("#ig-stage")), /** @type {HTMLElement} */ (b.querySelector("#ig-bar")));
}

/** Body of the tab: called by banco.js with the node to fill, the active cell and the token of the render. */
async function render(/** @type {HTMLElement} */ b, /** @type {any} */ p, /** @type {number} */ tok) {
  addStyle();
  const focusId = (document.activeElement instanceof Element && document.activeElement.closest(".ig-node")?.getAttribute("data-id")) || null;
  const file = E.files.find(INF.isInf);
  if (!file) { b.innerHTML = h`<div class="muted">${I18N.t("inf.none")}</div>`; return; }
  b.innerHTML = h`<div id="ig-open"></div><div id="ig-field"></div><div id="ig-set">${raw(settings(file))}</div><div class="ig-stage" id="ig-stage"><div id="ig-msg" aria-live="polite"></div><div class="ig-bar" id="ig-bar" role="toolbar" aria-label="${I18N.t("msngrafo.zoom.aria")}"></div><div class="ig-bar" id="ig-bar2" role="toolbar" aria-label="${I18N.t("msngrafo.v.view.aria")}"></div><div class="ig-wrap" id="ig-wrap" tabindex="0" role="region" aria-label="${I18N.t("msngrafo.graph.aria")}"></div><div id="ig-node" aria-live="polite"></div></div><div id="ig-exp"></div><div id="ig-find"></div>`;
  V.el = b; leaveFull();
  openRow(/** @type {HTMLElement} */ (b.querySelector("#ig-open"))); dropZone(b);
  const go = () => { G.scelte = []; run(b, file); };
  drawField(/** @type {HTMLElement} */ (b.querySelector("#ig-field")), file, go);
  wireSettings(b, () => run(b, file));
  zoomBar(/** @type {HTMLElement} */ (b.querySelector("#ig-bar")), /** @type {HTMLElement} */ (b.querySelector("#ig-wrap")), () => drawBody(b, file, false));
  viewBar(/** @type {HTMLElement} */ (b.querySelector("#ig-bar2")), () => drawBody(b, file, false));
  wireStage(b, true);
  if (G.j && G.k === file.k) { const msg = /** @type {HTMLElement} */ (b.querySelector("#ig-msg")); msg.innerHTML = summary() + warnLines(); drawBody(b, file, !!focusId); return; }
  await run(b, file);
  void tok; void p;
}

/** Body of the window of a graph file opened without raw data: the same graph and panel of a node, read-only (G.ro), no field, settings, export or search. */
async function renderRo(/** @type {HTMLElement} */ b, /** @type {string} */ name) {
  addStyle();
  b.innerHTML = h`<p class="muted sm" id="ig-ro-note">${I18N.t("msngrafo.file.ro", { name })}</p><div class="ig-stage" id="ig-stage"><div id="ig-msg" aria-live="polite"></div><div class="ig-bar" id="ig-bar" role="toolbar" aria-label="${I18N.t("msngrafo.zoom.aria")}"></div><div class="ig-bar" id="ig-bar2" role="toolbar" aria-label="${I18N.t("msngrafo.v.view.aria")}"></div><div class="ig-wrap" id="ig-wrap" tabindex="0" role="region" aria-label="${I18N.t("msngrafo.graph.aria")}"></div><div id="ig-node" aria-live="polite"></div></div><div id="ig-exp"></div><div id="ig-find"></div>`;
  V.el = b;
  viewBar(/** @type {HTMLElement} */ (b.querySelector("#ig-bar2")), () => drawBody(b, null, false));
  zoomBar(/** @type {HTMLElement} */ (b.querySelector("#ig-bar")), /** @type {HTMLElement} */ (b.querySelector("#ig-wrap")), () => drawBody(b, null, false));
  wireStage(b, false);
  /** @type {HTMLElement} */ (b.querySelector("#ig-msg")).innerHTML = summary() + warnLines();
  drawBody(b, null, false);
}

window.INFG = { render, renderRo, G };
