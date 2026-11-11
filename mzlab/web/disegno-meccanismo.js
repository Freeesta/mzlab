// @ts-check
// Mechanism notation of the Drawing: curved arrows (full head = an electron pair, half head = one electron), lone-pair dots and the «•+» radical cation.
// Arrows and lone pairs are data S-groups of Ketcher (fieldName MZLAB_MECH / MZLAB_LP), so they travel in the .ket; they are drawn over the canvas and
// in the exported image here. The Rust engine (`mech` of the grafo .wasm, `mecc/1`) says whether each arrow is well formed, never which mechanism is right.
import { carica } from "./disegno-frammenti.js";
import { ketIndex } from "./disegno-parziali.js";
import { atomAt } from "./disegno-radicale.js";

export const KEY_ARROW = "MZLAB_MECH", KEY_LP = "MZLAB_LP";
const PROBLEMS = /** @type {Record<string, string>} */ ({ noLonePair: "draw.mech.problem.noLonePair", noRadical: "draw.mech.problem.noRadical",
  pairFromRadical: "draw.mech.problem.pairFromRadical", noBond: "draw.mech.problem.noBond", same: "draw.mech.problem.same" });
const ENDS = /** @type {Record<string, string>} */ ({ lone: "draw.mech.end.lone", radical: "draw.mech.end.radical", atom: "draw.mech.end.atom" });
const NS = "http://www.w3.org/2000/svg", INK = "#1a1a1a", BAD = "#d55e00";   // the canvas is white in every theme; vermillion (Okabe-Ito) marks a malformed arrow

/** @typedef {{ t: "lone" | "radical" | "atom", i: number } | { t: "bond", i: number, j: number }} End  (Ketcher atom ids) */
/** @typedef {{ pair: boolean, from: End, to: End, data: string, atoms: number[] }} Arrow */
/** @typedef {{ pieces: () => { ids: number[] }[], pieceMolfile: (ids: number[]) => string | null, t: (k: string, p?: any) => string }} Helpers */

/** «pair L0 B0-1»: the kind, then the start and the end as positions in the S-group's atoms (sorted) */
const code = (/** @type {End} */ e, /** @type {number[]} */ at) => e.t === "bond" ? `B${at.indexOf(e.i)}-${at.indexOf(e.j)}` : { lone: "L", radical: "R", atom: "A" }[e.t] + at.indexOf(e.i);
const sorted = (/** @type {number[]} */ ids) => [...new Set(ids)].sort((a, b) => a - b);
const endIds = (/** @type {End} */ e) => e.t === "bond" ? [e.i, e.j] : [e.i];

/** reads one end back from its code, or null */
function decode(/** @type {string} */ s, /** @type {number[]} */ at) {
  const m = /^([LRAB])(\d+)(?:-(\d+))?$/.exec(s || ""); if (!m) return null;
  const i = at[+m[2]], j = m[3] != null ? at[+m[3]] : undefined;
  if (i == null || (m[1] === "B") !== (j != null)) return null;
  return /** @type {End} */ (m[1] === "B" ? { t: "bond", i, j } : { t: { L: "lone", R: "radical", A: "atom" }[m[1]], i });
}

/** the arrows of the drawing, in the order of their S-groups */
export function arrows(/** @type {any} */ K) {
  /** @type {Arrow[]} */ const out = [];
  K.editor.struct().sgroups.forEach((/** @type {any} */ sg) => {
    if (sg.type !== "DAT" || !sg.data || sg.data.fieldName !== KEY_ARROW) return;
    const data = String(sg.data.fieldValue || "").trim(), [kind, f, t] = data.split(/\s+/), at = sorted([...sg.atoms]);
    const from = decode(f, at), to = decode(t, at);
    if ((kind === "pair" || kind === "single") && from && to && from.t !== "atom" && to.t !== "lone" && to.t !== "radical") out.push({ pair: kind === "pair", from, to, data, atoms: at });
  });
  return out;
}

/** lone pairs drawn on each atom (Ketcher id → count) */
export function lonePairs(/** @type {any} */ K) {
  /** @type {Map<number, number>} */ const out = new Map();
  K.editor.struct().sgroups.forEach((/** @type {any} */ sg) => {
    const m = sg.type === "DAT" && sg.data && sg.data.fieldName === KEY_LP && /^lp (\d)$/.exec(String(sg.data.fieldValue || "").trim());
    if (m && sg.atoms.length === 1) out.set(sg.atoms[0], +m[1]);
  });
  return out;
}

/** changes the .ket with `fn` (on the molecule node of the atoms `ids` and their .ket positions) and loads it back: one undo step */
async function editKet(/** @type {any} */ K, /** @type {number[]} */ ids, /** @type {(mol: any, idx: number[]) => void} */ fn) {
  const j = JSON.parse(await K.getKet()), at = ketIndex(j, K.editor.struct(), ids);
  if (!at || new Set(at.map(a => a.mol)).size !== 1) return false;
  fn(j[at[0].mol], at.map(a => a.idx));
  await K.setMolecule(JSON.stringify(j));
  return true;
}

/** adds an arrow; false when its atoms are not in one structure */
export function addArrow(/** @type {any} */ K, /** @type {boolean} */ pair, /** @type {End} */ from, /** @type {End} */ to) {
  const ids = sorted([...endIds(from), ...endIds(to)]);
  return editKet(K, ids, (mol, idx) => {
    (mol.sgroups = mol.sgroups || []).push({ type: "DAT", atoms: idx, context: "Fragment", fieldName: KEY_ARROW, fieldData: `${pair ? "pair" : "single"} ${code(from, ids)} ${code(to, ids)}` });
  });
}

/** removes the arrow `a` (the first S-group with its atoms and text) */
export function removeArrow(/** @type {any} */ K, /** @type {Arrow} */ a) {
  return editKet(K, a.atoms, (mol, idx) => {
    const want = [...idx].sort((x, y) => x - y).join(","), k = (mol.sgroups || []).findIndex((/** @type {any} */ s) => s.type === "DAT" && s.fieldName === KEY_ARROW && String(s.fieldData).trim() === a.data && [...s.atoms].sort((x, y) => x - y).join(",") === want);
    if (k >= 0) mol.sgroups.splice(k, 1);
  });
}

/** the next number of lone pairs on the atoms `ids` (0 → 1 → 2 → 3 → 0, from the first atom), each atom written separately */
export async function cycleLonePairs(/** @type {any} */ K, /** @type {number[]} */ ids) {
  const now = lonePairs(K), n = ((now.get(ids[0]) || 0) + 1) % 4, j = JSON.parse(await K.getKet()), at = ketIndex(j, K.editor.struct(), ids);
  if (!at) return false;
  for (const { mol, idx } of at) {
    const m = j[mol]; m.sgroups = (m.sgroups || []).filter((/** @type {any} */ s) => !(s.type === "DAT" && s.fieldName === KEY_LP && s.atoms.length === 1 && s.atoms[0] === idx));
    if (n) m.sgroups.push({ type: "DAT", atoms: [idx], context: "Fragment", fieldName: KEY_LP, fieldData: `lp ${n}` });
  }
  await K.setMolecule(JSON.stringify(j));
  return true;
}

/** puts «•+» (charge +1 and a doublet radical) on the atoms `ids`, or takes it off when they all have it */
export async function toggleRadicalCation(/** @type {any} */ K, /** @type {number[]} */ ids) {
  const st = K.editor.struct(), j = JSON.parse(await K.getKet()), at = ketIndex(j, st, ids);
  if (!at) return false;
  const clear = ids.every(id => { const a = st.atoms.get(id); return a && a.charge === 1 && a.radical === 2; });
  for (const { mol, idx } of at) {
    const a = j[mol].atoms[idx];
    if (clear) { delete a.charge; delete a.radical; } else { a.charge = 1; a.radical = 2; }
    delete a.implicitHCount;
  }
  await K.setMolecule(JSON.stringify(j));
  return true;
}

/** the bond of `st` nearest to `p` (within `max`), as its two atom ids, or null */
function bondAt(/** @type {any} */ st, /** @type {{ x: number, y: number }} */ p, max = 0.3) {
  let best = /** @type {[number, number] | null} */ (null), d = max;
  st.bonds.forEach((/** @type {any} */ b) => {
    const a = st.atoms.get(b.begin).pp, c = st.atoms.get(b.end).pp, dx = c.x - a.x, dy = c.y - a.y, L = dx * dx + dy * dy || 1;
    const u = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / L)), e = Math.hypot(a.x + u * dx - p.x, a.y + u * dy - p.y);
    if (e < d) { d = e; best = [b.begin, b.end]; }
  });
  return best;
}

/** where an arrow starts at point `p`: the radical of an atom (half head, when it has one), its lone pair, or a bond */
export function startAt(/** @type {any} */ st, /** @type {{ x: number, y: number }} */ p, /** @type {boolean} */ pair) {
  const i = atomAt(st, p, 0.4);
  if (i != null) return /** @type {End} */ ({ t: !pair && st.atoms.get(i).radical ? "radical" : "lone", i });
  const b = bondAt(st, p); return b ? /** @type {End} */ ({ t: "bond", i: b[0], j: b[1] }) : null;
}

/** where an arrow ends at point `p`: an atom or a bond */
export function endAt(/** @type {any} */ st, /** @type {{ x: number, y: number }} */ p) {
  const i = atomAt(st, p, 0.4); if (i != null) return /** @type {End} */ ({ t: "atom", i });
  const b = bondAt(st, p); return b ? /** @type {End} */ ({ t: "bond", i: b[0], j: b[1] }) : null;
}

/** up to `n` free directions around atom `i` (angles in radians, Ketcher's y down), away from its bonds */
function freeDirs(/** @type {any} */ st, /** @type {number} */ i, /** @type {number} */ n) {
  const p = st.atoms.get(i).pp, used = /** @type {number[]} */ (st.atoms.get(i).radical ? [-Math.PI / 2] : []);   // Ketcher draws the radical dot above the atom
  st.bonds.forEach((/** @type {any} */ b) => { const o = b.begin === i ? b.end : b.end === i ? b.begin : null; if (o != null) { const q = st.atoms.get(o).pp; used.push(Math.atan2(q.y - p.y, q.x - p.x)); } });
  const gap = (/** @type {number} */ a, /** @type {number} */ b) => { const d = Math.abs(a - b) % (2 * Math.PI); return Math.min(d, 2 * Math.PI - d); };
  const out = /** @type {number[]} */ ([]);
  for (let k = 0; k < n; k++) {
    let best = -Math.PI / 2, score = -1;
    for (let c = 0; c < 16; c++) { const a = -Math.PI / 2 + c * Math.PI / 8, s = Math.min(Math.PI, ...[...used, ...out].map(u => gap(a, u))); if (s > score + 1e-6) { score = s; best = a; } }
    out.push(best);
  }
  return out;
}

/** the geometry of the overlay in Ketcher units: dots of every lone pair and the curve and head of every arrow */
function geometry(/** @type {any} */ K) {
  const st = K.editor.struct(), lp = lonePairs(K), dots = /** @type {{ x: number, y: number }[]} */ ([]), firstLp = new Map();
  lp.forEach((n, i) => {
    if (!st.atoms.has(i)) return;
    const p = st.atoms.get(i).pp;
    freeDirs(st, i, n).forEach((a, k) => {
      const cx = p.x + 0.42 * Math.cos(a), cy = p.y + 0.42 * Math.sin(a), ox = -Math.sin(a) * 0.08, oy = Math.cos(a) * 0.08;
      dots.push({ x: cx + ox, y: cy + oy }, { x: cx - ox, y: cy - oy }); if (!k) firstLp.set(i, { x: cx, y: cy });
    });
  });
  const pos = (/** @type {End} */ e) => {
    if (e.t === "bond") { const a = st.atoms.get(e.i).pp, b = st.atoms.get(e.j).pp; return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 }; }
    const a = st.atoms.get(e.i).pp;
    if (e.t === "lone") return firstLp.get(e.i) || (d => ({ x: a.x + 0.42 * Math.cos(d), y: a.y + 0.42 * Math.sin(d) }))(freeDirs(st, e.i, 1)[0]);
    if (e.t === "radical") return { x: a.x, y: a.y - 0.38 };
    return { x: a.x, y: a.y };
  };
  const curves = arrows(K).filter(a => [...endIds(a.from), ...endIds(a.to)].every(i => st.atoms.has(i))).map(a => {
    const p = pos(a.from), q0 = pos(a.to), dx = q0.x - p.x, dy = q0.y - p.y, L = Math.hypot(dx, dy) || 1;
    const back = a.to.t === "atom" ? Math.min(0.3, L / 3) : 0.08, q = { x: q0.x - dx / L * back, y: q0.y - dy / L * back };   // stop short of an atom label
    const bend = Math.max(0.35, L * 0.4), c = { x: (p.x + q.x) / 2 - dy / L * bend, y: (p.y + q.y) / 2 + dx / L * bend };
    return { a, p, c, q };
  });
  return { dots, curves };
}

/**
 * Draws the overlay into the group `g` (units × `sc`, shifted by `off`): the dots, then each arrow, dashed in vermillion when the check named a problem.
 * @param {boolean[]} bad the arrows the last check found malformed (by order)
 */
function paint(/** @type {any} */ K, /** @type {Element} */ g, /** @type {number} */ sc, /** @type {{ ox: number, oy: number }} */ off, /** @type {boolean[]} */ bad) {
  const doc = /** @type {Document} */ (g.ownerDocument), el = (/** @type {string} */ n, /** @type {Record<string, string | number>} */ at) => { const e = doc.createElementNS(NS, n); for (const k in at) e.setAttribute(k, String(at[k])); g.appendChild(e); return e; };
  const X = (/** @type {{ x: number }} */ p) => +(p.x * sc + off.ox).toFixed(2), Y = (/** @type {{ y: number }} */ p) => +(p.y * sc + off.oy).toFixed(2);
  const { dots, curves } = geometry(K), w = Math.max(1.2, sc * 0.035);
  for (const d of dots) el("circle", { cx: X(d), cy: Y(d), r: Math.max(1.4, sc * 0.045), fill: INK });
  curves.forEach(({ a, p, c, q }, k) => {
    const col = bad[k] ? BAD : INK, tx = q.x - c.x, ty = q.y - c.y, tl = Math.hypot(tx, ty) || 1, ux = tx / tl, uy = ty / tl, h = 0.22, s = 0.1;
    el("path", { d: `M${X(p)} ${Y(p)} Q${X(c)} ${Y(c)} ${X(q)} ${Y(q)}`, fill: "none", stroke: col, "stroke-width": w, "stroke-linecap": "round", ...(bad[k] ? { "stroke-dasharray": `${w * 3} ${w * 2}` } : {}), "data-mech": a.pair ? "pair" : "single" });
    const b = { x: q.x - ux * h, y: q.y - uy * h }, l = { x: b.x - uy * s, y: b.y + ux * s }, r = { x: b.x + uy * s, y: b.y - ux * s };
    el("path", { d: a.pair ? `M${X(q)} ${Y(q)} L${X(l)} ${Y(l)} L${X(r)} ${Y(r)} Z` : `M${X(q)} ${Y(q)} L${X(l)} ${Y(l)} L${X(b)} ${Y(b)} Z`, fill: col, stroke: col, "stroke-width": w * 0.6, "stroke-linejoin": "round" });
  });
  return { dots, curves };
}

/** the .ket text without the mechanism S-groups: the exported image gets the arrows and dots from `addExportMech`, not Ketcher's DAT text */
export function ketWithoutMech(/** @type {string} */ ket) {
  const j = JSON.parse(ket);
  for (const k of Object.keys(j)) if (j[k] && j[k].type === "molecule" && j[k].sgroups) j[k].sgroups = j[k].sgroups.filter((/** @type {any} */ s) => !(s.type === "DAT" && (s.fieldName === KEY_ARROW || s.fieldName === KEY_LP)));
  return JSON.stringify(j);
}

/** draws the arrows and lone pairs in an exported SVG (40 px per unit, `off` from `exportOffset` of draw.js) and grows the picture to hold the curves */
export function addExportMech(/** @type {any} */ K, /** @type {string} */ svgText, /** @type {{ ox: number, oy: number }} */ off, sc = 40) {
  if (!arrows(K).length && !lonePairs(K).size) return svgText;
  const doc = new DOMParser().parseFromString(svgText, "image/svg+xml"), root = doc.documentElement, vb = (root.getAttribute("viewBox") || "").split(/[\s,]+/).map(Number);
  if (vb.length !== 4 || vb.some(isNaN)) return svgText;
  const g = doc.createElementNS(NS, "g"); g.setAttribute("id", "mz-meccanismo"); root.appendChild(g);
  const { dots, curves } = paint(K, g, sc, off, []), pts = [...dots, ...curves.flatMap(c => [c.p, c.c, c.q])].map(p => [p.x * sc + off.ox, p.y * sc + off.oy]);
  const x0 = Math.min(vb[0], ...pts.map(p => p[0] - 8)), y0 = Math.min(vb[1], ...pts.map(p => p[1] - 8)), x1 = Math.max(vb[0] + vb[2], ...pts.map(p => p[0] + 8)), y1 = Math.max(vb[1] + vb[3], ...pts.map(p => p[1] + 8));
  root.setAttribute("viewBox", `${x0} ${y0} ${x1 - x0} ${y1 - y0}`);
  const pw = parseFloat(root.getAttribute("width") || ""), ph = parseFloat(root.getAttribute("height") || "");
  if (pw) root.setAttribute("width", String(pw * (x1 - x0) / vb[2])); if (ph) root.setAttribute("height", String(ph * (y1 - y0) / vb[3]));
  const bg = [...doc.querySelectorAll("rect")].find(r => !r.closest("defs") && /^(rgb\(100%,\s*100%,\s*100%\)|#fff(fff)?|white)$/i.test(r.getAttribute("fill") || ""));
  if (bg) { bg.setAttribute("x", String(x0)); bg.setAttribute("y", String(y0)); bg.setAttribute("width", String(x1 - x0)); bg.setAttribute("height", String(y1 - y0)); }
  return new XMLSerializer().serializeToString(doc);
}

/** the `mecc/1` request of the drawing: every atom (Ketcher order, explicit H too) and the arrows and lone pairs as positions in it */
export function requestOf(/** @type {any} */ K, /** @type {Helpers} */ h) {
  const ids = h.pieces().flatMap(p => p.ids), mf = ids.length ? h.pieceMolfile(ids) : null;
  if (!mf) return null;
  const at = (/** @type {number} */ i) => ids.indexOf(i), end = (/** @type {End} */ e) => e.t === "bond" ? { bond: [at(e.i), at(e.j)] } : { [e.t]: at(e.i) };
  const as = arrows(K), lp = [...lonePairs(K)].filter(([i]) => ids.includes(i));
  if (as.some(a => [...endIds(a.from), ...endIds(a.to)].some(i => at(i) < 0))) return null;
  return { ids, arrows: as, req: { molfile: mf, arrows: as.map(a => ({ kind: a.pair ? "pair" : "single", from: end(a.from), to: end(a.to) })), lone: lp.map(([i, n]) => [at(i), n]) } };
}

/** the check of a request, or `{error}` with the engine's key, or null when the engine cannot answer */
export async function mechOf(/** @type {Record<string, any>} */ req) {
  try { return JSON.parse(await (await carica()).mech(JSON.stringify(req))); }
  catch (e) { try { return { error: /** @type {string} */ (JSON.parse(String(e)).error_key) }; } catch (_) { return null; } }
}

/** the overlay layer over the editor frame that takes the pointer while a tool is on */
function makeLayer(/** @type {HTMLElement} */ host) {
  const layer = document.createElement("div");
  layer.id = "mech-layer"; layer.hidden = true; layer.setAttribute("aria-hidden", "true");
  layer.innerHTML = '<svg width="100%" height="100%"><line stroke-dasharray="6 4" stroke-width="2.5" stroke-linecap="round" visibility="hidden"/></svg>';
  host.appendChild(layer);
  return layer;
}

/** atom names (symbol and position in the drawing, «O3») and the text of an end and of a row of the list */
function words(/** @type {() => any} */ K, /** @type {Helpers} */ h) {
  const t = h.t, name = (/** @type {number} */ i) => { const all = h.pieces().flatMap(p => p.ids), a = K().editor.struct().atoms.get(i); return `${a ? a.label : "?"}${all.indexOf(i) + 1}`; };
  const endText = (/** @type {End} */ e) => e.t === "bond" ? t("draw.mech.end.bond", { a: name(e.i), b: name(e.j) }) : t(ENDS[e.t], { atom: name(e.i) });
  const rowText = (/** @type {Arrow} */ a, /** @type {number} */ n) => t("draw.mech.row", { n, kind: t(a.pair ? "draw.mech.row.pair" : "draw.mech.row.single"), from: endText(a.from), to: endText(a.to) });
  return { name, endText, rowText };
}

/** the list: one row per arrow with its check (`r`, the `mecc/1` answer or null) and a button that removes it; the status line under it */
function fillList(/** @type {any} */ K, /** @type {Helpers} */ h, /** @type {HTMLElement} */ list, /** @type {HTMLElement} */ note, /** @type {any} */ r) {
  const t = h.t, { name, rowText } = words(() => K, h);
  const as = arrows(K); list.textContent = "";
  if (!as.length) { const li = document.createElement("li"); li.className = "muted"; li.textContent = t("draw.mech.empty"); list.appendChild(li); }
  as.forEach((a, k) => {
    const li = document.createElement("li"), s = document.createElement("span"), x = document.createElement("button"), c = r && r.arrows && r.arrows[k];
    s.textContent = rowText(a, k + 1) + (c ? " — " + (c.ok ? t("draw.mech.ok") : t(PROBLEMS[c.problem] || "draw.mech.problem.same")) : "");
    if (c && !c.ok) s.style.color = "var(--bad)";
    x.type = "button"; x.className = "sm"; x.textContent = t("draw.mech.remove"); x.setAttribute("aria-label", t("draw.mech.remove.label", { n: k + 1 }));
    x.style.minWidth = x.style.minHeight = "24px"; x.addEventListener("click", () => { removeArrow(K, a); });
    li.append(s, " ", x); list.appendChild(li);
  });
  const lp = r && r.lone ? r.lone.filter((/** @type {any} */ l) => !l.ok) : [], all = h.pieces().flatMap(p => p.ids);
  const lines = lp.map((/** @type {any} */ l) => t("draw.mech.lp.bad", { atom: name(all[l.atom]), pairs: l.pairs, max: l.max }));
  if (r && r.error) lines.push(t(r.error));
  else if (r && as.length) lines.unshift(r.arrows.every((/** @type {any} */ c) => c.ok) ? t("draw.mech.all.ok") : t("draw.mech.all.bad", { n: r.arrows.filter((/** @type {any} */ c) => !c.ok).length }));
  if (lines.length || r) note.textContent = lines.join(" ");
}

/** draws the arrows and dots over Ketcher's canvas and hides Ketcher's own text of the S-groups; returns the key of the drawing (a new key asks for a check) */
function drawOver(/** @type {any} */ K, /** @type {SVGSVGElement} */ svg, /** @type {number} */ sc, /** @type {boolean[]} */ bad) {
  let g = svg.querySelector("#mz-meccanismo");
  if (!g) { g = svg.ownerDocument.createElementNS(NS, "g"); g.id = "mz-meccanismo"; g.setAttribute("pointer-events", "none"); }
  svg.appendChild(g); g.textContent = "";
  const as = arrows(K), lp = lonePairs(K), texts = new Set([...as.map(a => a.data), ...[...lp.values()].map(n => `lp ${n}`)]);
  paint(K, g, sc, { ox: 0, oy: 0 }, bad);
  if (texts.size) svg.querySelectorAll("text").forEach(x => {   // the text and the white box Ketcher puts under it, which would cover bonds
    if (!texts.has((x.textContent || "").trim())) return;
    x.style.visibility = "hidden"; const r = x.previousElementSibling; if (r && r.tagName.toLowerCase() === "rect") /** @type {SVGElement} */ (r).style.visibility = "hidden";
  });
  const st = K.editor.struct();
  return JSON.stringify([as.map(a => a.data + a.atoms), [...lp], [...st.atoms].map(([i, a]) => [i, a.label, a.charge, a.radical]), [...st.bonds.values()].map(b => [b.begin, b.end, b.type])]);
}

/**
 * The «Mechanism» card: arrow kind, the arrow, lone-pair and «•+» tools over the editor frame, the keyboard path (start and end from the selection), the
 * list with one row per arrow and the status of the check. `draw(svg, sc)` is called by the labels of draw.js after every change of the drawing.
 * @param {{ frame: HTMLIFrameElement, K: () => any, h: Helpers, root: HTMLElement }} o
 */
export function mountMech(o) {
  const q = (/** @type {string} */ s) => /** @type {HTMLElement} */ (o.root.querySelector(s));
  const btn = { arrow: q("#mech-arrow"), lone: q("#mech-lone"), ion: q("#mech-ion") }, keyBtn = q("#mech-key"), list = q("#mech-list"), note = q("#mech-note");
  const host = /** @type {HTMLElement} */ (o.frame.parentElement), layer = makeLayer(host), line = /** @type {SVGLineElement} */ (layer.querySelector("line"));
  /** @type {"arrow" | "lone" | "ion" | null} */ let mode = null;
  /** @type {{ x: number, y: number } | null} */ let from = null;
  /** @type {End | null} */ let keyStart = null;
  let bad = /** @type {boolean[]} */ ([]), lastKey = "", run = 0;
  const t = o.h.t, pair = () => /** @type {HTMLInputElement} */ (o.root.querySelector("input[name=mech-kind]:checked"))?.value !== "single";
  const set = (/** @type {typeof mode} */ m) => {
    mode = m; from = null; line.setAttribute("visibility", "hidden"); layer.hidden = !m;
    for (const [k, b] of Object.entries(btn)) b.setAttribute("aria-pressed", String(k === m));
    if (m) { const f = o.frame.getBoundingClientRect(), hb = host.getBoundingClientRect(); Object.assign(layer.style, { left: f.left - hb.left + "px", top: f.top - hb.top + "px", width: f.width + "px", height: f.height + "px" }); }
  };
  const selected = () => { const s = o.K().editor.selection() || {}; return { atoms: /** @type {number[]} */ (s.atoms || []), bonds: /** @type {number[]} */ (s.bonds || []) }; };
  const { endText, rowText } = words(o.K, o.h);
  const add = async (/** @type {End} */ f, /** @type {End} */ e) => {
    const K = o.K(), p = pair();
    if (!(await addArrow(K, p, f, e))) { note.textContent = t("draw.mech.onePiece"); return; }
    note.textContent = t("draw.mech.added", { row: rowText({ pair: p, from: f, to: e, data: "", atoms: [] }, arrows(K).length) });
  };
  const local = (/** @type {PointerEvent} */ e) => { const f = layer.getBoundingClientRect(); return { x: e.clientX - f.left, y: e.clientY - f.top }; };
  const obj = (/** @type {{ x: number, y: number }} */ p) => o.K().editor.render.page2obj({ clientX: p.x, clientY: p.y });
  for (const [k, b] of Object.entries(btn)) b.addEventListener("click", () => {
    const s = selected().atoms;
    if (k !== "arrow" && s.length && mode !== k) { (k === "lone" ? cycleLonePairs : toggleRadicalCation)(o.K(), s); return; }   // with atoms selected the item acts on them
    set(mode === k ? null : /** @type {typeof mode} */ (k));
  });
  layer.addEventListener("pointerdown", e => {
    const p = local(e), K = o.K(), i = atomAt(K.editor.struct(), obj(p), 0.45); e.preventDefault();
    if (mode === "lone" || mode === "ion") { if (i != null) (mode === "lone" ? cycleLonePairs : toggleRadicalCation)(K, [i]); return; }
    from = p; layer.setPointerCapture(e.pointerId);
    for (const [a, v] of [["x1", p.x], ["x2", p.x], ["y1", p.y], ["y2", p.y]]) line.setAttribute(String(a), String(v));
    line.setAttribute("visibility", "visible");
  });
  layer.addEventListener("pointermove", e => { if (!from) return; const p = local(e); line.setAttribute("x2", String(p.x)); line.setAttribute("y2", String(p.y)); });
  layer.addEventListener("pointerup", e => {
    if (!from) return;
    const st = o.K().editor.struct(), f = startAt(st, obj(from), pair()), to = endAt(st, obj(local(e)));
    from = null; line.setAttribute("visibility", "hidden");
    if (!f || !to) { note.textContent = t("draw.mech.miss"); return; }
    add(f, to);
  });
  document.addEventListener("keydown", e => { if (e.key === "Escape" && mode) set(null); });
  // the keyboard path: select one atom or one bond, press for the start; select the end, press again
  keyBtn.addEventListener("click", () => {
    const s = selected(), st = o.K().editor.struct();
    const one = s.atoms.length + s.bonds.length === 1 || (s.bonds.length === 1 && s.atoms.length <= 2);   // a selected bond selects its atoms too
    if (!one) { note.textContent = t("draw.mech.pick"); return; }
    const b = s.bonds.length ? st.bonds.get(s.bonds[0]) : null;
    if (!keyStart) {
      keyStart = b ? { t: "bond", i: b.begin, j: b.end } : { t: !pair() && st.atoms.get(s.atoms[0]).radical ? "radical" : "lone", i: s.atoms[0] };
      keyBtn.textContent = t("draw.mech.key.end"); note.textContent = t("draw.mech.start", { end: endText(keyStart) }); return;
    }
    const f = keyStart; keyStart = null; keyBtn.textContent = t("draw.mech.key");
    add(f, b ? { t: "bond", i: b.begin, j: b.end } : { t: "atom", i: s.atoms[0] });
  });
  const fill = (/** @type {any} */ r) => fillList(o.K(), o.h, list, note, r);
  const check = async (/** @type {() => void} */ redraw) => {
    const me = ++run, K = o.K(), r = requestOf(K, o.h);
    if (!r || (!r.req.arrows.length && !r.req.lone.length)) { bad = []; fill(null); return; }
    const out = await mechOf(r.req); if (me !== run) return;
    bad = out && out.arrows ? out.arrows.map((/** @type {any} */ c) => !c.ok) : [];
    fill(out || { error: "draw.frag.off" }); redraw();
  };
  fill(null);
  return {
    off: () => set(null),
    /** draws the arrows and dots over Ketcher's canvas and hides Ketcher's own text of the S-groups; a changed drawing is checked again */
    draw(/** @type {SVGSVGElement} */ svg, /** @type {number} */ sc) {
      const key = drawOver(o.K(), svg, sc, bad);
      if (key !== lastKey) { lastKey = key; check(() => { const R = o.K().editor.render; if (R) this.draw(R.paper.canvas, R.options.microModeScale || 40); }); }
    },
  };
}
