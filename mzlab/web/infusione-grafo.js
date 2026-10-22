// @ts-check
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

const CSS = `.ig-field{align-items:flex-end;gap:8px}.ig-field input[type=text]{min-width:14ch}.ig-set summary{cursor:pointer;min-height:24px}.ig-set input[type=number]{width:7ch}.ig-bar{display:flex;gap:4px;align-items:center;flex-wrap:wrap;margin:6px 0}.ig-bar .bt{min-width:28px;min-height:28px}.ig-cut{min-height:28px}.ig-cuts{border-collapse:collapse}.ig-cuts th,.ig-cuts td{padding:2px 6px;text-align:left}
@media (pointer:coarse){.ig-bar .bt,.ig-use,.ig-cut{min-width:40px;min-height:40px}}
.ig-wrap{overflow:auto;border:1px solid var(--line);border-radius:6px;max-height:min(60vh,520px);background:var(--bg)}.ig-svg{display:block}
.ig-box{fill:var(--panel);stroke:var(--line);stroke-width:1.5}.ig-node{cursor:pointer;outline:none}.ig-node:focus-visible .ig-box,.ig-node.sel .ig-box{stroke:var(--accent);stroke-width:3}.ig-node.hot .ig-box{stroke:var(--accent2)}
.ig-node.sel .ig-box{fill:var(--sel)}.ig-mz{font:700 12px sans-serif;fill:var(--ink)}.ig-cand{font:11px sans-serif;fill:var(--muted)}.ig-cand.on{fill:var(--ink);font-weight:700}
.ig-arc{fill:none;stroke:var(--muted);opacity:.6}.ig-arc.on{stroke:var(--accent2);opacity:1}.ig-arc.iso{stroke-dasharray:0}.ig-headp{fill:var(--muted)}.ig-loss{font:10px sans-serif;fill:var(--muted)}.ig-amb{fill:var(--warn);stroke:var(--ink)}
.ig-cands{border-collapse:collapse;width:100%;font-size:12px}.ig-cands th,.ig-cands td{border-bottom:1px solid var(--line);padding:2px 6px;text-align:left}.ig-cands tr.on{background:var(--sel)}.ig-cands .num{text-align:right}
.ig-rule{font:10px sans-serif;fill:var(--ink)}.ig-weak{border:1px solid var(--line);border-radius:4px;padding:0 4px;font-weight:700}.ig-cal{color:var(--warn)}.ig-rl{margin-top:8px}.ig-src{display:inline-block;min-height:24px;min-width:24px}.ig-flt{display:inline-flex;align-items:center;gap:4px;min-height:24px}
.ig-warn{color:var(--warn)}.ig-unexp{color:var(--muted)}.ig-draw{border:1px solid var(--line);border-radius:6px;padding:6px;margin-top:8px;max-width:340px}.ig-pic svg{max-width:100%;height:auto}`;

/** @type {{ el: HTMLElement | null, svg: SVGSVGElement | null, scale: number }} */
const V = { el: null, svg: null, scale: 1 };
let tokenNow = 0;

function addStyle() {
  if (document.getElementById("ig-css")) return;
  const st = document.createElement("style"); st.id = "ig-css"; st.textContent = CSS; document.head.appendChild(st);
}

const nodeLabel = (/** @type {any} */ n) => `MS${n.livello} ${n.mz.toFixed(4)}`;

/** The rows of «the data of the chart»: one per ion with the formula and the candidates, then one per loss. */
function dataTable() {
  const j = filtered(G.j, G.rf), byId = new Map(j.nodi.map((/** @type {any} */ n) => [n.id, n]));
  const rows = j.nodi.map((/** @type {any} */ n) => [n.id, n.livello, n.mz.toFixed(5), chosen(n) || "", n.formula ? n.formula.candidati.map((/** @type {any} */ c) => `${c.formula} ${c.ppm} ppm`).join("; ") : "", n.isolato ? I18N.t("msngrafo.yes") : "", n.osservazioni, "", "", ""]);
  const arcs = j.archi.map((/** @type {any} */ a) => [`${a.da}>${a.a}`, "", `${(/** @type {any} */ (byId.get(a.a))).mz.toFixed(5)}`, a.perdita.formula || "", `${a.perdita.da_massa.toFixed(5)} Da`, a.misure.map((/** @type {any} */ m) => `CE ${m.ce ?? ""} ${m.rel}%`).join("; "), a.misure.length, ...arcCells(j, a)]);
  return { title: I18N.t("msngrafo.data.title"), head: ["id", I18N.t("msngrafo.col.level"), "m/z", I18N.t("msngrafo.col.chosen"), I18N.t("msngrafo.col.cand"), I18N.t("msngrafo.col.iso"), I18N.t("msngrafo.col.n"), I18N.t("msngrafo.col.rule"), I18N.t("msngrafo.col.source"), I18N.t("msngrafo.col.priority")], rows: [...rows, ...arcs], rowHeader: true };
}

async function copyData() {
  const t = dataTable(), tsv = [t.head, ...t.rows].map(r => r.join("\t")).join("\n");
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
    h`<button type="button" class="bt" id="ig-data" title="${I18N.t("tdati.btn.title")}">${I18N.t("tdati.btn")}</button><button type="button" class="bt" id="ig-copyd">${I18N.t("msngrafo.copydata")}</button><label class="ig-flt" title="${I18N.t("msngrafo.rule.filter.tip")}"><input type="checkbox" id="ig-rf"${raw(G.rf ? " checked" : "")} aria-label="${I18N.t("msngrafo.rule.filter.aria")}"> ${I18N.t("msngrafo.rule.filter")}</label>`;
  bar.querySelectorAll("[data-z]").forEach(b => b.addEventListener("click", () => /** @type {any} */ (act)[/** @type {string} */ (/** @type {HTMLElement} */ (b).dataset.z)]()));
  bar.querySelector("#ig-data")?.addEventListener("click", () => G.j && TDATI.table(dataTable()));
  bar.querySelector("#ig-copyd")?.addEventListener("click", () => G.j && copyData());
  bar.querySelector("#ig-rf")?.addEventListener("change", e => { G.rf = /** @type {HTMLInputElement} */ (e.target).checked; if (G.j) redraw(); });
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
  const ion = i ? I18N.t("msngrafo.ion.line", { f: i.formula, mz: i.mz_teorico.toFixed(5), ppm: i.ppm }) : I18N.t("msngrafo.masses");
  return h`<p class="sm" id="ig-sum">${ion} · ${I18N.t("msngrafo.counts", { nodes: c.nodi, arcs: c.archi })}</p>`;
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
  V.svg = drawGraph(wrap, filtered(G.j, G.rf), { onSelect: id => { G.sel = id; drawBody(b, file, true); }, onOpen: openSpectrum });
  setScale(V.scale);
  if (keepFocus && G.sel) /** @type {HTMLElement | null} */ (wrap.querySelector(`.ig-node[data-id="${G.sel}"]`))?.focus();
  nodePanel(pan, nodeOf(G.sel), { redo, again: () => drawBody(b, file, false) });
  exportRow(exp, file, G.sel);
  const auto = G.auto; G.auto = false;
  searchRow(find, file, auto);
  if (window.TDATI && V.svg) { V.svg.setAttribute("aria-describedby", "ig-sum"); TDATI.attach(/** @type {any} */ (V.svg), dataTable); }
}

/** Asks for the graph with the state in force and draws the answer. */
async function run(/** @type {HTMLElement} */ b, /** @type {any} */ file) {
  const mine = ++tokenNow;
  const ok = await fetchGraph(file.k);
  if (!ok || mine !== tokenNow || !b.isConnected) return;
  const msg = /** @type {HTMLElement} */ (b.querySelector("#ig-msg"));
  msg.innerHTML = G.error ? h`<p class="fail sm" role="alert">${G.error}</p>` : summary() + warnLines();
  drawBody(b, file, false);
}

/** Body of the tab: called by banco.js with the node to fill, the active cell and the token of the render. */
async function render(/** @type {HTMLElement} */ b, /** @type {any} */ p, /** @type {number} */ tok) {
  addStyle();
  const focusId = (document.activeElement instanceof Element && document.activeElement.closest(".ig-node")?.getAttribute("data-id")) || null;
  const file = E.files.find(INF.isInf);
  if (!file) { b.innerHTML = h`<div class="muted">${I18N.t("inf.none")}</div>`; return; }
  b.innerHTML = h`<div id="ig-field"></div><div id="ig-set">${raw(settings(file))}</div><div id="ig-msg" aria-live="polite"></div><div class="ig-bar" id="ig-bar" role="toolbar" aria-label="${I18N.t("msngrafo.zoom.aria")}"></div><div class="ig-wrap" id="ig-wrap" tabindex="0" role="region" aria-label="${I18N.t("msngrafo.graph.aria")}"></div><div id="ig-node" aria-live="polite"></div><div id="ig-exp"></div><div id="ig-find"></div>`;
  V.el = b;
  const go = () => { G.scelte = []; run(b, file); };
  drawField(/** @type {HTMLElement} */ (b.querySelector("#ig-field")), file, go);
  wireSettings(b, () => run(b, file));
  zoomBar(/** @type {HTMLElement} */ (b.querySelector("#ig-bar")), /** @type {HTMLElement} */ (b.querySelector("#ig-wrap")), () => drawBody(b, file, false));
  if (G.j && G.k === file.k) { const msg = /** @type {HTMLElement} */ (b.querySelector("#ig-msg")); msg.innerHTML = summary() + warnLines(); drawBody(b, file, !!focusId); return; }
  await run(b, file);
  void tok; void p;
}

window.INFG = { render, G };
