// @ts-check
// Drawer of the fragmentation graph (msngrafo/1 with its `vista`): pure SVG, no page globals (texts and choices come in `opts`), so the tab, the Flow and the
// stand-alone HTML draw the same figure. Two views of the same data: the tree (columns by MS level) and the radial (precursor in the centre, one ring per step of
// the best path). Imports nothing from the page.
//
// LABEL RULE (the only one; the Rust figure of the CLI repeats it):
//  node  line 1  «m/z» with 4 decimals, then the relative intensity «rel %» with 1 decimal (a root shows none);
//        line 2  the MS levels where the ion shows: «MS2», or «MS2, MS3» (vista.livelli);
//        then    the chosen formula, or up to 4 candidates as the engine gives them, each with the error «+0.12 ppm» and its proofs;
//  arc   the loss as a formula («−H2O»), else «Δ 18.011 Da» and, when the engine names one, «(compatible with H2O)»; the rule ids go below.
//  Formulas are drawn with subscript digits (tspan baseline-shift) and the charge as a superscript; the plain form «C13H25N4O3S+» is used only in aria-label
//  and in the data tables. CSV, MGF, MSP, JSON and «Copy data» stay ASCII (machines read them).
//
// Best path: the arcs with `vista.archi[].best` are solid, the others thin and dashed. A node with two or more precursors (`comune`) has a double border and
// the number of its precursors.

const W = 176, WR = 150, LH = 14, GAP_X = 130, GAP_Y = 20, MAXC = 4, DW = 150, DH = 80;
const NS = "http://www.w3.org/2000/svg";
const SUB = "₀₁₂₃₄₅₆₇₈₉";

/** @typedef {(key: string, p?: Record<string, any>) => string} Tr */
/** @typedef {"tree" | "radial"} Mode */
/** @typedef {{ x: number, y: number, w: number, h: number }} Box */
/**
 * @typedef {object} Opts
 * @property {Tr} t texts of the language in force
 * @property {string | null} [sel] selected ion
 * @property {Mode} [mode] "tree" (default) or "radial"
 * @property {number} [minRel] hide the ions below this % of the base peak (and what hangs from them)
 * @property {number} [maxLevel] hide the ions first seen above this MS level (0 or missing = all)
 * @property {boolean} [bestOnly] draw only the arcs of the best path
 * @property {Set<string>} [collapsed] ions whose best subtree is closed
 * @property {(n: any) => string | null} [chosen] the formula chosen for an ion (default: the chosen one, else the only candidate)
 * @property {(p: string) => string} [proveText] text of a proof
 * @property {(a: any) => string} [ruleLabel] short text of the fragmentation rule under a loss
 * @property {(id: string) => string | null} [drawing] SVG text of the hypothesis drawing of an ion (null = none)
 * @property {string | null} [root] SVG text of the drawing of the molecule (the root)
 */

// ---- formulas ----------------------------------------------------------------------------------------------------------

/** The charge sign of the graph (polarity 1 / −1) as «+» or «-». */
const signOf = (/** @type {any} */ j) => (j && j.polarita === -1 ? "-" : "+");

/** A formula split into its plain text and charge: «C13H25N4O3S+» → [«C13H25N4O3S», «+»]; the charge defaults to the polarity of the graph. */
export function splitCharge(/** @type {string} */ f, /** @type {string} */ dflt = "+") {
  const m = /^(.*?)([+\-−])$/.exec(f);
  return m ? [m[1], m[2] === "−" ? "-" : m[2]] : [f, dflt];
}

/** The formula as pieces to draw: digits are subscripts, the charge is a superscript. @returns {{ t: string, k: "n" | "sub" | "sup" }[]} */
export function formulaParts(/** @type {string} */ f, /** @type {string} */ charge = "") {
  /** @type {{ t: string, k: "n" | "sub" | "sup" }[]} */
  const out = [];
  for (const piece of f.split(/(\d+)/)) if (piece) out.push({ t: piece, k: /^\d+$/.test(piece) ? "sub" : "n" });
  if (charge) out.push({ t: charge === "-" ? "−" : charge, k: "sup" });
  return out;
}

/** The plain form for aria-label and the data tables: «C13H25N4O3S+». */
export const plainFormula = (/** @type {string} */ f, /** @type {string} */ charge = "") => f + charge;

/** The formula with Unicode subscripts and superscript charge, for the places that take text only (tooltips, visible tables). */
export const prettyFormula = (/** @type {string} */ f, /** @type {string} */ charge = "") =>
  f.replace(/\d/g, d => SUB[+d]) + (charge ? (charge === "-" ? "⁻" : "⁺") : "");

const esc = (/** @type {string} */ s) => s.replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c] || c);

/** The formula as HTML with <sub> and <sup> (for tables of the page). */
export const formulaHtml = (/** @type {string} */ f, /** @type {string} */ charge = "") =>
  formulaParts(f, charge).map(p => (p.k === "sub" ? `<sub>${esc(p.t)}</sub>` : p.k === "sup" ? `<sup>${esc(p.t)}</sup>` : esc(p.t))).join("");

// ---- the model ---------------------------------------------------------------------------------------------------------

/** The `vista` of a graph, or a modest one made of the fields of the graph (an answer without `vista`: tree only, no radial view). */
export function vistaOf(/** @type {any} */ j) {
  if (j.vista) return j.vista;
  const into = new Map();
  for (const a of j.archi) (into.get(a.a) || into.set(a.a, []).get(a.a)).push(a);
  return {
    versione: 1, algoritmo: null, regola: "", ordine: [], fallback: true,
    nodi: j.nodi.map((/** @type {any} */ n) => {
      const ins = into.get(n.id) || [], first = ins[0];
      const rel = first ? Math.max(...first.misure.map((/** @type {any} */ m) => m.rel), 0) : null;
      return { id: n.id, livelli: [n.livello], rel, rel_max: n.rel_max, best: { da: first ? first.da : null, passi: n.livello - 1, costo: [] }, entranti: ins.length, comune: ins.length > 1, albero: n.pos, radiale: null };
    }),
    archi: j.archi.map((/** @type {any} */ a, /** @type {number} */ i) => ({ da: a.da, a: a.a, best: into.get(a.a)[0] === a })),
  };
}

/** The ions and arcs to draw after the filters. @returns {{ ids: Set<string>, arcs: any[], hidden: Map<string, number> }} `hidden`: ions closed under a collapsed one. */
export function visible(/** @type {any} */ j, /** @type {Opts} */ o) {
  const v = vistaOf(j), byId = new Map(v.nodi.map((/** @type {any} */ x) => [x.id, x]));
  const present = new Map(j.nodi.map((/** @type {any} */ n) => [n.id, n]));
  const best = new Set(v.archi.filter((/** @type {any} */ a) => a.best).map((/** @type {any} */ a) => a.da + ">" + a.a));
  const arcs = j.archi.filter((/** @type {any} */ a) => present.has(a.da) && present.has(a.a) && (!o.bestOnly || best.has(a.da + ">" + a.a)));
  const out = new Map();
  for (const a of arcs) (out.get(a.da) || out.set(a.da, []).get(a.da)).push(a.a);
  const hasIn = new Set(j.archi.filter((/** @type {any} */ a) => present.has(a.da) && present.has(a.a)).map((/** @type {any} */ a) => a.a));
  const pass = (/** @type {string} */ id) => {
    const n = present.get(id), x = byId.get(id);
    if (!n || !x) return false;
    if (o.maxLevel && n.livello > o.maxLevel && n.livello > 1) return false;
    if (o.minRel && x.best.da && x.rel != null && x.rel < o.minRel) return false;
    return true;
  };
  const ids = new Set(), todo = [];
  for (const n of j.nodi) if (!hasIn.has(n.id) || n.livello === 1) if (pass(n.id)) { ids.add(n.id); todo.push(n.id); }
  const collapsed = o.collapsed || new Set();
  while (todo.length) {
    const id = /** @type {string} */ (todo.pop());
    if (collapsed.has(id)) continue;
    for (const to of out.get(id) || []) if (!ids.has(to) && pass(to)) { ids.add(to); todo.push(to); }
  }
  /** @type {Map<string, number>} */
  const hidden = new Map();
  for (const id of collapsed) {
    if (!ids.has(id)) continue;
    let n = 0;
    const walk = (/** @type {string} */ x) => { for (const y of v.nodi) if (y.best.da === x) { if (!ids.has(y.id)) n++; walk(y.id); } };
    walk(id);
    hidden.set(id, n);
  }
  return { ids, arcs: arcs.filter((/** @type {any} */ a) => ids.has(a.da) && ids.has(a.a)), hidden };
}

/** The chosen formula of an ion: the engine's choice, else the only candidate. */
const chosenOf = (/** @type {any} */ n) => { const f = n.formula; return f ? f.scelta || (f.candidati.length === 1 ? f.candidati[0].formula : null) : null; };

const sgn = (/** @type {number} */ v) => (v > 0 ? "+" : v < 0 ? "−" : "") + Math.abs(v).toFixed(2);
const levelsText = (/** @type {any} */ x) => x.livelli.map((/** @type {number} */ l) => "MS" + l).join(", ");

/** What a box says: the lines of m/z and level, the candidates (the chosen first) and the pieces for the tooltip and the accessible name. */
function boxModel(/** @type {any} */ j, /** @type {any} */ n, /** @type {any} */ x, /** @type {Opts} */ o, /** @type {boolean} */ compact) {
  const t = o.t, chosen = (o.chosen || chosenOf)(n), dflt = signOf(j), f = n.formula;
  const cs = f ? [...f.candidati].sort((/** @type {any} */ a, /** @type {any} */ b) => (b.formula === chosen ? 1 : 0) - (a.formula === chosen ? 1 : 0)) : [];
  const keep = compact ? 1 : MAXC;
  const lines = cs.slice(0, keep).map((/** @type {any} */ c) => {
    const [base, ch] = splitCharge(c.formula, dflt);
    return { parts: formulaParts(base, ch), ppm: sgn(c.ppm) + " ppm", on: c.formula === chosen, prove: (c.prove || []).map((/** @type {string} */ p) => (o.proveText ? o.proveText(p) : p)), plain: plainFormula(base, ch) };
  });
  const more = cs.length > keep ? t("msngrafo.more", { n: cs.length - keep }) : "";
  const rel = x.best.da ? x.rel : null;
  const chosenPlain = chosen ? (([b, c]) => plainFormula(b, c))(splitCharge(chosen, dflt)) : "";
  const aria = [`m/z ${n.mz.toFixed(4)}`, t("msngrafo.v.aria.levels", { levels: levelsText(x) })];
  if (rel != null) aria.push(t("msngrafo.v.aria.rel", { rel: rel.toFixed(1) }));
  if (chosenPlain) aria.push(t("msngrafo.v.aria.formula", { f: chosenPlain }));
  else if (cs.length) aria.push(t("msngrafo.v.aria.cands", { n: cs.length }));
  if (x.comune) aria.push(t("msngrafo.v.aria.common", { n: x.entranti }));
  return { lines, more, rel, chosenPlain, aria, amb: !!f && !chosen };
}

/** The tooltip of an ion: one line per measure of the arcs that end in it (MS level, activation, energy, % of the base peak). */
function nodeTip(/** @type {any} */ j, /** @type {any} */ n, /** @type {any} */ x) {
  const out = [`m/z ${n.mz.toFixed(5)} · ${levelsText(x)}`];
  for (const a of j.archi) if (a.a === n.id) for (const m of a.misure || []) out.push(`MS${m.livello}${m.att ? " " + String(m.att).toUpperCase() : ""}${m.ce != null ? " " + m.ce : ""}: ${m.rel.toFixed(1)} %`);
  return out.join("\n");
}

// ---- layout ------------------------------------------------------------------------------------------------------------

const heightOf = (/** @type {any} */ m, /** @type {boolean} */ withDrawing, /** @type {boolean} */ more) => 36 + LH * Math.max(1, m.lines.length) + (more ? LH : 0) + (withDrawing ? DH + 18 : 0) + 6;

/** @typedef {{ mode: Mode, boxes: Map<string, Box>, ids: string[], col: Map<string, number>, ring: Map<string, number>, angle: Map<string, number>, w: number, h: number, vista: any, centre: { x: number, y: number } }} Layout */

/** Where every visible ion goes. Tree: x by column, rows as the engine ordered them (empty rows closed up). Radial: ring by steps of the best path, angle by the engine, radii grown until no box touches another. */
export function layout(/** @type {any} */ j, /** @type {Opts} */ o, /** @type {{ ids: Set<string> }} */ vis) {
  const v = vistaOf(j), mode = /** @type {Mode} */ (o.mode === "radial" && !v.fallback ? "radial" : "tree");
  const nodes = j.nodi.filter((/** @type {any} */ n) => vis.ids.has(n.id));
  const vx = new Map(v.nodi.map((/** @type {any} */ x) => [x.id, x]));
  const compact = mode === "radial";
  const size = new Map(nodes.map((/** @type {any} */ n) => {
    const m = boxModel(j, n, vx.get(n.id), o, compact), dr = !!(o.drawing && o.drawing(n.id)) || (!!o.root && !(vx.get(n.id).best.da));
    return [n.id, { w: compact ? WR : W, h: heightOf(m, dr, !!m.more) }];
  }));
  /** @type {Map<string, Box>} */
  const boxes = new Map();
  const col = new Map(), ring = new Map(), angle = new Map();
  let w = 0, h = 0, centre = { x: 0, y: 0 };
  if (mode === "tree") {
    const rows = [...new Set(nodes.map((/** @type {any} */ n) => vx.get(n.id).albero.riga))].sort((a, b) => a - b);
    const rowOf = new Map(rows.map((r, i) => [r, i]));
    const rowH = rows.map(() => 0);
    for (const n of nodes) { const r = /** @type {number} */ (rowOf.get(vx.get(n.id).albero.riga)); rowH[r] = Math.max(rowH[r], /** @type {any} */ (size.get(n.id)).h); }
    const top = [20]; rowH.forEach((hh, i) => top.push(top[i] + hh + GAP_Y));
    for (const n of nodes) {
      const a = vx.get(n.id).albero, s = /** @type {any} */ (size.get(n.id));
      boxes.set(n.id, { x: 20 + a.col * (W + GAP_X), y: top[/** @type {number} */ (rowOf.get(a.riga))], w: s.w, h: s.h });
      col.set(n.id, a.col);
    }
  } else {
    // the roots sit in the middle; ring r grows from the ring inside until nothing overlaps
    const placed = /** @type {string[]} */ ([]);
    const maxRing = Math.max(0, ...nodes.map((/** @type {any} */ n) => vx.get(n.id).radiale.anello));
    const at = new Map();
    let radius = 0;
    for (let r = 0; r <= maxRing; r++) {
      const here = nodes.filter((/** @type {any} */ n) => vx.get(n.id).radiale.anello === r);
      if (!here.length) continue;
      const put = (/** @type {number} */ rad) => here.map((/** @type {any} */ n) => {
        const a = vx.get(n.id).radiale.angolo * Math.PI / 180, s = /** @type {any} */ (size.get(n.id)), cx = r === 0 ? 0 : rad * Math.sin(a), cy = r === 0 ? 0 : -rad * Math.cos(a);
        return { id: n.id, box: { x: cx - s.w / 2, y: cy - s.h / 2, w: s.w, h: s.h } };
      });
      const clash = (/** @type {{ id: string, box: Box }[]} */ list) => list.some((p, i) => list.slice(i + 1).some(q => touch(p.box, q.box)) || placed.some(id => touch(p.box, /** @type {Box} */ (at.get(id)))));
      radius = r === 0 ? 0 : Math.max(radius + 40, 120);
      let list = put(radius);
      for (let k = 0; clash(list) && k < 4000; k++) { radius += 10; list = put(radius); }
      for (const p of list) { at.set(p.id, p.box); placed.push(p.id); ring.set(p.id, r); angle.set(p.id, vx.get(p.id).radiale.angolo); }
    }
    const xs = [...at.values()], x0 = Math.min(...xs.map(b => b.x)) - 20, y0 = Math.min(...xs.map(b => b.y)) - 20;
    for (const [id, b] of at) boxes.set(id, { x: b.x - x0, y: b.y - y0, w: b.w, h: b.h });
    centre = { x: -x0, y: -y0 };
  }
  for (const b of boxes.values()) { w = Math.max(w, b.x + b.w + 30); h = Math.max(h, b.y + b.h + 30); }
  return { mode, boxes, ids: nodes.map((/** @type {any} */ n) => n.id), col, ring, angle, w: Math.max(w, 60), h: Math.max(h, 60), vista: v, centre };
}

/** Two boxes closer than 6 px. */
const touch = (/** @type {Box} */ a, /** @type {Box} */ b) => a.x < b.x + b.w + 6 && b.x < a.x + a.w + 6 && a.y < b.y + b.h + 6 && b.y < a.y + a.h + 6;

/** The ion reached from `id` by an arrow key, or null. Tree: ←→ columns, ↑↓ in the column. Radial: → to a ion of the next ring, ← to its precursor, ↑↓ along the ring. */
export function neighbour(/** @type {Layout} */ L, /** @type {string} */ id, /** @type {string} */ key) {
  const b = L.boxes.get(id); if (!b) return null;
  const mid = (/** @type {string} */ k) => { const x = /** @type {Box} */ (L.boxes.get(k)); return x.y + x.h / 2; };
  const near = (/** @type {string[]} */ list) => list.sort((p, q) => Math.abs(mid(p) - mid(id)) - Math.abs(mid(q) - mid(id)))[0] || null;
  const x = L.vista.nodi.find((/** @type {any} */ y) => y.id === id);
  if (L.mode === "tree") {
    const c = /** @type {number} */ (L.col.get(id));
    if (key === "ArrowUp" || key === "ArrowDown") {
      const same = L.ids.filter(k => L.col.get(k) === c).sort((p, q) => mid(p) - mid(q)), i = same.indexOf(id);
      return same[i + (key === "ArrowDown" ? 1 : -1)] || null;
    }
    return near(L.ids.filter(k => L.col.get(k) === c + (key === "ArrowRight" ? 1 : -1)));
  }
  const r = /** @type {number} */ (L.ring.get(id));
  if (key === "ArrowLeft") return x && x.best.da && L.boxes.has(x.best.da) ? x.best.da : null;
  if (key === "ArrowRight") {
    const kids = L.ids.filter(k => (L.vista.nodi.find((/** @type {any} */ y) => y.id === k) || {}).best?.da === id).sort((p, q) => /** @type {number} */ (L.angle.get(p)) - /** @type {number} */ (L.angle.get(q)));
    return kids[0] || null;
  }
  const ringIds = L.ids.filter(k => L.ring.get(k) === r).sort((p, q) => /** @type {number} */ (L.angle.get(p)) - /** @type {number} */ (L.angle.get(q))), i = ringIds.indexOf(id);
  return ringIds[i + (key === "ArrowDown" ? 1 : -1)] || null;
}

/** Ids of the ions on the paths from the roots to an ion and the arcs between them. */
export function pathTo(/** @type {any} */ j, /** @type {string | null} */ id) {
  /** @type {Set<string>} */ const nodes = new Set(), arcs = new Set();
  const up = (/** @type {string} */ x) => { if (nodes.has(x)) return; nodes.add(x); j.archi.filter((/** @type {any} */ a) => a.a === x).forEach((/** @type {any} */ a) => { arcs.add(a.da + ">" + a.a); up(a.da); }); };
  if (id) up(id);
  return { nodes, arcs };
}

// ---- drawing -----------------------------------------------------------------------------------------------------------

function el(/** @type {string} */ tag, /** @type {Record<string, string | number>} */ at, /** @type {Element | null} */ parent, /** @type {string} */ text = "") {
  const e = document.createElementNS(NS, tag);
  for (const [k, v] of Object.entries(at)) e.setAttribute(k, String(v));
  if (text) e.textContent = text;
  if (parent) parent.appendChild(e);
  return e;
}

/** A text made of runs: plain text, subscript digits and superscript charge in tspans. @param {{ t: string, k: string }[]} parts */
function runs(/** @type {Element} */ parent, /** @type {Record<string, string | number>} */ at, /** @type {{ t: string, k: string }[]} */ parts) {
  const e = el("text", at, parent);
  for (const p of parts) {
    if (p.k === "n") e.appendChild(document.createTextNode(p.t));
    else el("tspan", { "baseline-shift": p.k === "sub" ? "sub" : "super", "font-size": "75%", class: p.k === "sub" ? "ig-sub" : "ig-sup" }, e, p.t);
  }
  return e;
}

/** The point where the line from the middle of a box to a point leaves the box. */
function edge(/** @type {Box} */ b, /** @type {number} */ tx, /** @type {number} */ ty) {
  const cx = b.x + b.w / 2, cy = b.y + b.h / 2, dx = tx - cx, dy = ty - cy;
  if (!dx && !dy) return { x: cx, y: cy };
  const k = Math.min(dx ? (b.w / 2) / Math.abs(dx) : Infinity, dy ? (b.h / 2) / Math.abs(dy) : Infinity);
  return { x: cx + dx * k, y: cy + dy * k };
}

/** The label of a loss: [pieces of the first line, pieces of the second line or null]. */
function lossText(/** @type {any} */ a, /** @type {Tr} */ t) {
  const p = a.perdita;
  if (p.formula) return { first: [{ t: "−", k: "n" }, ...formulaParts(p.formula)], second: null };
  const comp = (p.compatibile || []).slice(0, 2);
  /** @type {{ t: string, k: string }[] | null} */
  let second = null;
  if (comp.length) {
    const [pre, post] = t("msngrafo.v.compat", { f: "\u0001" }).split("\u0001");
    second = [{ t: "(" + pre, k: "n" }];
    comp.forEach((/** @type {string} */ c, /** @type {number} */ i) => { if (i) second.push({ t: " / ", k: "n" }); second.push(...formulaParts(c)); });
    second.push({ t: (post || "") + ")", k: "n" });
  }
  return { first: [{ t: `Δ ${p.da_massa.toFixed(3)} Da`, k: "n" }], second };
}

/** The plain text of a loss for tooltips and tables: «−H2O» or «Δ 18.011 Da (compatible with H2O)». */
export function lossPlain(/** @type {any} */ a, /** @type {Tr} */ t) {
  const p = a.perdita;
  if (p.formula) return "−" + prettyFormula(p.formula);
  const comp = (p.compatibile || []).slice(0, 2);
  return `Δ ${p.da_massa.toFixed(3)} Da` + (comp.length ? " (" + t("msngrafo.v.compat", { f: comp.map(prettyFormula).join(" / ") }) + ")" : "");
}

function drawArcs(/** @type {Element} */ g, /** @type {any} */ j, /** @type {Layout} */ L, /** @type {any[]} */ arcs, /** @type {Set<string>} */ on, /** @type {Opts} */ o) {
  const bestSet = new Set(L.vista.archi.filter((/** @type {any} */ a) => a.best).map((/** @type {any} */ a) => a.da + ">" + a.a));
  const t = o.t;
  // the thin ones first so that the best path lies on top
  const sorted = [...arcs].sort((p, q) => Number(bestSet.has(p.da + ">" + p.a)) - Number(bestSet.has(q.da + ">" + q.a)));
  for (const a of sorted) {
    const p = L.boxes.get(a.da), q = L.boxes.get(a.a); if (!p || !q) continue;
    const best = bestSet.has(a.da + ">" + a.a), hot = on.has(a.da + ">" + a.a);
    const rel = Math.max(...a.misure.map((/** @type {any} */ m) => m.rel), 0);
    const cls = "ig-arc" + (best ? " best" : " alt") + (hot ? " on" : "") + (a.isolamento ? " iso" : "");
    const w = best ? (1 + Math.log10(1 + rel) * 1.2).toFixed(2) : "1";
    let mx, my, lx, ly;
    if (L.mode === "tree") {
      const x1 = p.x + p.w, y1 = p.y + p.h / 2, x2 = q.x, y2 = q.y + q.h / 2;
      mx = (x1 + x2) / 2; my = (y1 + y2) / 2;
      el("path", { d: `M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}`, class: cls, "stroke-width": w, "marker-end": "url(#ig-head)" }, g);
      lx = mx; ly = my;
    } else {
      const s = edge(p, q.x + q.w / 2, q.y + q.h / 2), e = edge(q, p.x + p.w / 2, p.y + p.h / 2);
      el("path", { d: `M${s.x},${s.y} L${e.x},${e.y}`, class: cls, "stroke-width": w, "marker-end": "url(#ig-head)" }, g);
      lx = (s.x + e.x) / 2; ly = (s.y + e.y) / 2;
    }
    if (L.mode === "radial" && !best && !hot) continue;                      // the labels of the thin arcs would crowd the rings
    const lt = lossText(a, t), tip = lossPlain(a, t);
    const lab = el("g", { class: "ig-lossg" }, g);
    lab.appendChild(Object.assign(document.createElementNS(NS, "title"), { textContent: tip }));
    runs(lab, { x: lx, y: ly - 3, class: "ig-loss", "text-anchor": "middle" }, lt.first);
    let dy = 9;
    if (lt.second) { runs(lab, { x: lx, y: ly + dy, class: "ig-loss", "text-anchor": "middle" }, lt.second); dy += 11; }
    const rule = o.ruleLabel ? o.ruleLabel(a) : "";
    if (rule) el("text", { x: lx, y: ly + dy, class: "ig-rule", "text-anchor": "middle" }, lab, rule);
  }
}

/** Parses SVG text and puts it at a place of the graph (null when it cannot be read). */
function inline(/** @type {Element} */ parent, /** @type {string} */ text, /** @type {Record<string, string | number>} */ at) {
  const doc = new DOMParser().parseFromString(text, "image/svg+xml"), root = doc.documentElement;
  if (!root || root.nodeName !== "svg" || doc.querySelector("parsererror")) return null;
  const node = /** @type {Element} */ (document.importNode(root, true));
  for (const [k, v] of Object.entries(at)) node.setAttribute(k, String(v));
  parent.appendChild(node);
  return node;
}

function drawNode(/** @type {Element} */ g, /** @type {any} */ j, /** @type {any} */ n, /** @type {any} */ x, /** @type {Box} */ b, /** @type {Layout} */ L, /** @type {Opts} */ o, /** @type {boolean} */ isSel, /** @type {boolean} */ hot, /** @type {number} */ hiddenBelow) {
  const t = o.t, m = boxModel(j, n, x, o, L.mode === "radial");
  const isRoot = !x.best.da, drawingText = isRoot ? o.root || (o.drawing && o.drawing(n.id)) : o.drawing && o.drawing(n.id);
  const aria = m.aria.concat(hiddenBelow ? [t("msngrafo.v.aria.collapsed", { n: hiddenBelow })] : []);
  const box = el("g", { class: "ig-node" + (isSel ? " sel" : "") + (hot ? " hot" : "") + (n.isolato ? " iso" : "") + (x.comune ? " common" : ""), "data-id": n.id, tabindex: -1, role: "button", transform: `translate(${b.x},${b.y})`,
    "aria-pressed": String(isSel), "aria-label": aria.join(", ") }, g);
  box.appendChild(Object.assign(document.createElementNS(NS, "title"), { textContent: nodeTip(j, n, x) }));
  el("rect", { width: b.w, height: b.h, rx: 6, class: "ig-box", "stroke-dasharray": n.isolato ? "0" : "5 3" }, box);
  if (x.comune) {
    el("rect", { x: 3, y: 3, width: b.w - 6, height: b.h - 6, rx: 4, class: "ig-box2" }, box);
    el("text", { x: b.w - 36, y: 17, class: "ig-com", "text-anchor": "end" }, box, `×${x.entranti}`).appendChild(Object.assign(document.createElementNS(NS, "title"), { textContent: t("msngrafo.v.common", { n: x.entranti }) }));
  }
  el("text", { x: 8, y: 17, class: "ig-mz" }, box, `m/z ${n.mz.toFixed(4)}` + (m.rel != null ? `  ·  ${m.rel.toFixed(1)} %` : ""));
  el("text", { x: 8, y: 31, class: "ig-lvl" }, box, levelsText(x) + (hiddenBelow ? `  ·  ${t("msngrafo.v.hidden", { n: hiddenBelow })}` : ""));
  if (m.amb) el("path", { d: `M${b.w - 16},8 l5,9 l-10,0 z`, class: "ig-amb" }, box).appendChild(Object.assign(document.createElementNS(NS, "title"), { textContent: t("msngrafo.amb") }));
  let y = 48;
  m.lines.forEach(l => {
    const row = runs(box, { x: 8, y, class: "ig-cand" + (l.on ? " on" : "") }, [...l.parts, { t: `  ${l.ppm}`, k: "n" }]);
    if (l.prove.length) row.appendChild(document.createTextNode("  " + l.prove.join(" ")));
    y += LH;
  });
  if (m.more) { el("text", { x: 8, y, class: "ig-cand" }, box, m.more); y += LH; }
  if (!m.lines.length && !m.more) y += LH;
  if (drawingText) {
    const s = inline(box, drawingText, { x: (b.w - DW) / 2, y: y - 6, width: DW, height: DH, "aria-hidden": "true", class: "ig-thumb" });
    if (s && !isRoot) el("text", { x: 8, y: y + DH + 6, class: "ig-hyp" }, box, t("msngrafo.v.hyp"));
  }
}

/**
 * Draws the graph into `host` and returns the SVG and its layout. `cb.onSelect(id | null)` on a click, an arrow key or Esc; `cb.onOpen(id)` on a click or Enter.
 * The graph is drawn for the ions that pass the filters of `opts`; the others are not in the SVG.
 */
export function drawVista(/** @type {HTMLElement} */ host, /** @type {any} */ j, /** @type {{ onSelect: (id: string | null) => void, onOpen: (id: string) => void }} */ cb, /** @type {Opts} */ o) {
  host.textContent = "";
  const sel = o.sel || null, vis = visible(j, o), L = layout(j, o, vis), v = L.vista;
  const vx = new Map(v.nodi.map((/** @type {any} */ x) => [x.id, x]));
  const { nodes: hot, arcs: hotArcs } = pathTo({ archi: vis.arcs }, sel);
  const svg = /** @type {SVGSVGElement} */ (el("svg", { class: "ig-svg", viewBox: `0 0 ${L.w} ${L.h}`, width: L.w, height: L.h, role: "group", "data-mode": L.mode, "aria-label": o.t(L.mode === "radial" ? "msngrafo.v.graph.radial" : "msngrafo.graph.aria") }, null));
  const defs = el("defs", {}, svg);
  el("path", { d: "M0,0 L8,4 L0,8 z", class: "ig-headp" }, el("marker", { id: "ig-head", viewBox: "0 0 8 8", refX: 8, refY: 4, markerWidth: 7, markerHeight: 7, orient: "auto" }, defs));
  const g = el("g", { class: "ig-zoom" }, svg);
  if (L.mode === "radial") for (const r of new Set(L.ring.values())) if (r > 0) {
    const sample = [...L.ring].filter(([, k]) => k === r).map(([id]) => /** @type {Box} */ (L.boxes.get(id))), rad = Math.hypot(sample[0].x + sample[0].w / 2 - L.centre.x, sample[0].y + sample[0].h / 2 - L.centre.y);
    el("circle", { cx: L.centre.x, cy: L.centre.y, r: rad.toFixed(1), class: "ig-ring" }, g);
  }
  drawArcs(g, j, L, vis.arcs, hotArcs, o);
  for (const n of j.nodi) {
    const b = L.boxes.get(n.id); if (!b) continue;
    drawNode(g, j, n, vx.get(n.id), b, L, o, n.id === sel, hot.has(n.id), vis.hidden.get(n.id) || 0);
  }
  host.appendChild(svg);
  wire(svg, L, cb, sel);
  return { svg, layout: L, visible: vis };
}

function wire(/** @type {SVGSVGElement} */ svg, /** @type {Layout} */ L, /** @type {any} */ cb, /** @type {string | null} */ sel) {
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
    const id = /** @type {string} */ (b.getAttribute("data-id"));
    if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); cb.onSelect(null); return; }
    if (e.key === "Enter" || e.key === " ") { e.preventDefault(); cb.onSelect(id); cb.onOpen(id); return; }
    if (!e.key.startsWith("Arrow")) return;
    e.preventDefault(); const to = neighbour(L, id, e.key); if (to) focus(to);
  });
}
