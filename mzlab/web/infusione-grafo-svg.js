// @ts-check
// The graph of the «Fragmentation graph» tab as an SVG: columns by MS level and rows as the engine placed them (pos), a box per ion with its m/z and its
// candidate formulas with ppm, an arrow per loss. Selection, path from the root, keys ← → ↑ ↓ Enter Esc, zoom and move with buttons. Colours: CSS variables.
import { G, TXT, chosen } from "./infusione-grafo-stato.js";
import { arcLabel } from "./infusione-grafo-regole.js";

const W = 176, LH = 14, GAP_X = 96, GAP_Y = 20, MAXC = 4;
const NS = "http://www.w3.org/2000/svg";
/** @typedef {(key: string, p?: Record<string, any>) => string} Tr */
/** The texts of the tab (the language of the page); the Flow hands in its own English function. @type {Tr} */
const TR = (key, p) => I18N.t(/** @type {any} */ (key), p);
const f4 = (/** @type {number} */ v) => v.toFixed(4);
const sgn = (/** @type {number} */ v) => (v > 0 ? "+" : v < 0 ? "−" : "") + Math.abs(v).toFixed(2);

/** The lines of a box: m/z, then the candidates (at most MAXC, the chosen first) with their error, and «+n more». */
export function boxLines(/** @type {any} */ n, /** @type {Tr} */ t = TR) {
  const f = n.formula, c = chosen(n);
  /** @type {{ t: string, on: boolean, prove: string[] }[]} */
  const out = [];
  if (!f) return out;
  const cs = [...f.candidati].sort((/** @type {any} */ a, /** @type {any} */ b) => (b.formula === c ? 1 : 0) - (a.formula === c ? 1 : 0));
  cs.slice(0, MAXC).forEach((/** @type {any} */ x) => out.push({ t: `${x.formula}  ${sgn(x.ppm)} ppm`, on: x.formula === c, prove: x.prove }));
  if (cs.length > MAXC) out.push({ t: t("msngrafo.more", { n: cs.length - MAXC }), on: false, prove: [] });
  return out;
}

const heightOf = (/** @type {any} */ n) => 24 + LH * Math.max(1, boxLines(n).length) + 6;

/** Ids of the nodes on a path from the roots to a node, and the arcs between them. */
export function pathTo(/** @type {any} */ j, /** @type {string | null} */ id) {
  const nodes = new Set(), arcs = new Set();
  const up = (/** @type {string} */ x) => { if (nodes.has(x)) return; nodes.add(x); j.archi.filter((/** @type {any} */ a) => a.a === x).forEach((/** @type {any} */ a) => { arcs.add(a.da + ">" + a.a); up(a.da); }); };
  if (id) up(id);
  return { nodes, arcs };
}

function el(/** @type {string} */ tag, /** @type {Record<string, string | number>} */ at, /** @type {Element | null} */ parent, /** @type {string} */ text = "") {
  const e = document.createElementNS(NS, tag);
  for (const [k, v] of Object.entries(at)) e.setAttribute(k, String(v));
  if (text) e.textContent = text;
  if (parent) parent.appendChild(e);
  return e;
}

/** Position of every node: x by column, y by row; rows share one pitch (the tallest box). */
function place(/** @type {any} */ j) {
  const pitch = Math.max(...j.nodi.map(heightOf), 40) + GAP_Y;
  /** @type {Map<string, { x: number, y: number, h: number }>} */
  const at = new Map();
  j.nodi.forEach((/** @type {any} */ n) => at.set(n.id, { x: 20 + n.pos.col * (W + GAP_X), y: 20 + n.pos.riga * pitch, h: heightOf(n) }));
  return at;
}

function drawArcs(/** @type {Element} */ g, /** @type {any} */ j, /** @type {Map<string, any>} */ at, /** @type {Set<string>} */ on, /** @type {Tr} */ t) {
  j.archi.forEach((/** @type {any} */ a) => {
    const p = at.get(a.da), q = at.get(a.a); if (!p || !q) return;
    const x1 = p.x + W, y1 = p.y + p.h / 2, x2 = q.x, y2 = q.y + q.h / 2, mx = (x1 + x2) / 2;
    const rel = Math.max(...a.misure.map((/** @type {any} */ m) => m.rel), 0);
    const hot = on.has(a.da + ">" + a.a);
    el("path", { d: `M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}`, class: "ig-arc" + (hot ? " on" : "") + (a.isolamento ? " iso" : ""), "stroke-width": (1 + Math.log10(1 + rel) * 1.2).toFixed(2), "marker-end": "url(#ig-head)" }, g);
    const lbl = a.perdita.formula ? `−${a.perdita.formula}` : `−${a.perdita.da_massa.toFixed(3)} Da`;
    el("text", { x: mx, y: (y1 + y2) / 2 - 3, class: "ig-loss", "text-anchor": "middle" }, g, lbl);
    const rule = arcLabel(j, a, t);
    if (rule) el("text", { x: mx, y: (y1 + y2) / 2 + 9, class: "ig-rule", "text-anchor": "middle" }, g, rule);
  });
}

function drawNode(/** @type {Element} */ g, /** @type {any} */ n, /** @type {{ x: number, y: number, h: number }} */ pos, /** @type {boolean} */ isSel, /** @type {boolean} */ hot, /** @type {Tr} */ t) {
  const lines = boxLines(n, t), amb = !!n.formula && !chosen(n);
  const box = el("g", { class: "ig-node" + (isSel ? " sel" : "") + (hot ? " hot" : "") + (n.isolato ? " iso" : ""), "data-id": n.id, tabindex: -1, role: "button", transform: `translate(${pos.x},${pos.y})`,
    "aria-pressed": String(isSel), "aria-label": t("msngrafo.node.aria", { mz: f4(n.mz), level: n.livello, n: lines.length }) }, g);
  el("rect", { width: W, height: pos.h, rx: 6, class: "ig-box", "stroke-dasharray": n.isolato ? "0" : "5 3" }, box);
  el("text", { x: 8, y: 17, class: "ig-mz" }, box, `m/z ${f4(n.mz)}`);
  if (amb) el("path", { d: `M${W - 16},8 l5,9 l-10,0 z`, class: "ig-amb" }, box).appendChild(Object.assign(document.createElementNS(NS, "title"), { textContent: t("msngrafo.amb") }));
  lines.forEach((l, i) => {
    const row = el("text", { x: 8, y: 34 + i * LH, class: "ig-cand" + (l.on ? " on" : "") }, box, l.t);
    if (l.prove.length) row.textContent += "  " + l.prove.map(p => t(/** @type {any} */ (TXT.prove)[p])).join(" ");
  });
}

/** Draws the graph in the host (`opts.t` and `opts.sel` replace the texts and the selected ion of the tab): returns the SVG; `onPick(id)` on a click or Enter, `onSelect(id|null)` on a click, arrow keys or Esc. `opts.t` and `opts.sel` replace the texts and the selected ion of the tab. */
export function drawGraph(/** @type {HTMLElement} */ host, /** @type {any} */ j, /** @type {{ onSelect: (id: string | null) => void, onOpen: (id: string) => void }} */ cb, /** @type {{ t?: Tr, sel?: string | null }} */ opts = {}) {
  host.textContent = "";
  const t = opts.t || TR, sel = "sel" in opts ? opts.sel || null : G.sel;
  const at = place(j), { nodes: hot, arcs } = pathTo(j, sel);
  const w = Math.max(...[...at.values()].map(p => p.x + W)) + 30, h = Math.max(...[...at.values()].map(p => p.y + p.h)) + 30;
  const svg = /** @type {SVGSVGElement} */ (el("svg", { class: "ig-svg", viewBox: `0 0 ${w} ${h}`, width: w, height: h, role: "group", "aria-label": t("msngrafo.graph.aria") }, null));
  const defs = el("defs", {}, svg);
  el("path", { d: "M0,0 L8,4 L0,8 z", class: "ig-headp" }, el("marker", { id: "ig-head", viewBox: "0 0 8 8", refX: 8, refY: 4, markerWidth: 7, markerHeight: 7, orient: "auto" }, defs));
  const g = el("g", { class: "ig-zoom" }, svg);
  drawArcs(g, j, at, arcs, t);
  j.nodi.forEach((/** @type {any} */ n) => drawNode(g, n, /** @type {any} */ (at.get(n.id)), n.id === sel, hot.has(n.id), t));
  host.appendChild(svg);
  wire(host, svg, j, cb, sel);
  return svg;
}

/** The node reached from a node by an arrow: the nearest row of the next column, or the neighbour in the column. */
export function neighbour(/** @type {any} */ j, /** @type {any} */ n, /** @type {string} */ key) {
  const same = j.nodi.filter((/** @type {any} */ x) => x.pos.col === n.pos.col).sort((/** @type {any} */ a, /** @type {any} */ b) => a.pos.riga - b.pos.riga);
  if (key === "ArrowDown" || key === "ArrowUp") return same[same.indexOf(n) + (key === "ArrowDown" ? 1 : -1)] || null;
  const col = n.pos.col + (key === "ArrowRight" ? 1 : -1);
  const there = j.nodi.filter((/** @type {any} */ x) => x.pos.col === col);
  return there.sort((/** @type {any} */ a, /** @type {any} */ b) => Math.abs(a.pos.riga - n.pos.riga) - Math.abs(b.pos.riga - n.pos.riga))[0] || null;
}

function wire(/** @type {HTMLElement} */ host, /** @type {SVGSVGElement} */ svg, /** @type {any} */ j, /** @type {any} */ cb, /** @type {string | null} */ sel) {
  /** @type {NodeListOf<SVGGElement>} */
  const boxes = svg.querySelectorAll(".ig-node");
  const first = (sel && [...boxes].find(b => b.dataset.id === sel)) || boxes[0];
  boxes.forEach(b => { b.tabIndex = b === first ? 0 : -1; });
  const focus = (/** @type {string} */ id) => { const b = [...boxes].find(x => x.dataset.id === id); if (!b) return; boxes.forEach(x => { x.tabIndex = -1; }); b.tabIndex = 0; b.focus(); cb.onSelect(id); };
  svg.addEventListener("click", e => {
    const b = /** @type {Element} */ (e.target).closest(".ig-node");
    if (b) { cb.onSelect(/** @type {string} */ (b.getAttribute("data-id"))); cb.onOpen(/** @type {string} */ (b.getAttribute("data-id"))); } else cb.onSelect(null);
  });
  svg.addEventListener("keydown", e => {
    const b = /** @type {Element} */ (e.target).closest(".ig-node"); if (!b) return;
    const n = j.nodi.find((/** @type {any} */ x) => x.id === b.getAttribute("data-id"));
    if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); cb.onSelect(null); return; }
    if (e.key === "Enter" || e.key === " ") { e.preventDefault(); cb.onSelect(n.id); cb.onOpen(n.id); return; }
    if (!e.key.startsWith("Arrow")) return;
    e.preventDefault(); const to = neighbour(j, n, e.key); if (to) focus(to.id);
  });
}
