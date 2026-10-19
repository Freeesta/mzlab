// @ts-check
// Partial structures of the drawing: a set of atoms inside brackets with a delta «+O -H2» (a change whose position is unknown: a hydroxylation somewhere on
// the ring). Stored in the drawing as a data S-group of Ketcher on the atoms (fieldName «MZLAB», fieldData = the delta, context «Fragment»), so it is saved in
// the .ket and travels with the molfile. Brackets and text are drawn by parentesi.js over the canvas; formula, mass and m/z with the delta come from the Rust
// graph (`partial`, parz/1). A multicentre attachment (a substituent on «any of these ring atoms») is a second data S-group, «MZLAB_ENDPTS», on the candidate
// atoms and the substituent atom; it is exported as a V3000 molfile with ENDPTS written here (Ketcher has no bond with a variable position).
// The export for mzFinder is {smiles, delta, level: 3} (level 3 of Schymanski: a generic class with many plausible isomers, never one structure), plus positionUnknown and the V3000.
import { drawBrackets, drawMulticenter, bracketBox } from "./parentesi.js";
import { carica } from "./disegno-frammenti.js";

export const KEY = "MZLAB", KEY_M = "MZLAB_ENDPTS", ANY = "ANY";

/** Ketcher's SMILES without the CXSMILES extension «|SgD:…|», that the SMILES reader of the engine refuses */
export const plainSmiles = (/** @type {unknown} */ s) => String(s || "").replace(/\s+\|[^|]*\|\s*$/, "").trim();

/** @typedef {{ atoms: number[], delta: string }} Partial */

/** the partial structures of the drawing: Ketcher atom ids and delta of every data S-group «MZLAB» */
export function partials(/** @type {any} */ K) {
  /** @type {Partial[]} */ const out = [];
  K.editor.struct().sgroups.forEach((/** @type {any} */ sg) => {
    if (sg.type === "DAT" && sg.data && sg.data.fieldName === KEY) out.push({ atoms: [...sg.atoms], delta: String(sg.data.fieldValue || "").trim() });
  });
  return out;
}

/** true when bond `bid` lies on a ring: its two atoms stay connected without it */
function ringBond(/** @type {any} */ st, /** @type {number} */ bid) {
  const b = st.bonds.get(bid), seen = new Set([b.begin]), todo = [b.begin];
  while (todo.length) {
    const u = todo.pop();
    st.bonds.forEach((/** @type {any} */ c, /** @type {number} */ k) => {
      if (k === bid || (c.begin !== u && c.end !== u)) return;
      const v = c.begin === u ? c.end : c.begin; if (!seen.has(v)) { seen.add(v); todo.push(v); }
    });
  }
  return seen.has(b.end);
}

/**
 * The substituent atom and the candidate atoms of a set of atoms: the substituent is the only atom of the set with no ring bond, joined by a chain bond to one
 * candidate; the others (at least two) are the candidates. Null when the set is not shaped like that.
 * @returns {{ sub: number, ends: number[], bond: number } | null}
 */
export function multiShape(/** @type {any} */ st, /** @type {number[]} */ ids) {
  /** @type {Map<number, boolean>} */ const onRing = new Map(ids.map(i => [i, false]));
  st.bonds.forEach((/** @type {any} */ b, /** @type {number} */ k) => { if ((onRing.has(b.begin) || onRing.has(b.end)) && ringBond(st, k)) { if (onRing.has(b.begin)) onRing.set(b.begin, true); if (onRing.has(b.end)) onRing.set(b.end, true); } });
  const subs = ids.filter(i => !onRing.get(i)), ends = ids.filter(i => onRing.get(i));
  if (subs.length !== 1 || ends.length < 2) return null;
  let bond = -1;
  st.bonds.forEach((/** @type {any} */ b, /** @type {number} */ k) => { if ((b.begin === subs[0] && ends.includes(b.end)) || (b.end === subs[0] && ends.includes(b.begin))) bond = k; });
  return bond < 0 ? null : { sub: subs[0], ends, bond };
}

/** @typedef {{ atoms: number[], sub: number, ends: number[], bond: number }} Multi */

/** the multicentre attachments of the drawing (data S-groups «MZLAB_ENDPTS» whose atoms have the right shape) */
export function multis(/** @type {any} */ K) {
  const st = K.editor.struct(); /** @type {Multi[]} */ const out = [];
  st.sgroups.forEach((/** @type {any} */ sg) => {
    if (sg.type !== "DAT" || !sg.data || sg.data.fieldName !== KEY_M) return;
    const m = multiShape(st, [...sg.atoms]); if (m) out.push({ atoms: [...sg.atoms], ...m });
  });
  return out;
}

/** the delta checked by the engine; null when it is not readable (the text of the error is shown by the caller) */
async function checkDelta(/** @type {string} */ molfile, /** @type {string} */ delta) {
  try { return JSON.parse(await (await carica()).partial(molfile, delta)); } catch (e) { return typeof e === "string" ? { error: JSON.parse(e).error_key } : null; }
}

/** for every Ketcher atom id, its molecule node and position in the .ket (matched on the coordinates: the .ket has its own numbering) */
function ketIndex(/** @type {any} */ j, /** @type {any} */ st, /** @type {number[]} */ ids) {
  const nodes = Object.keys(j).filter(k => j[k] && j[k].type === "molecule");
  /** @type {{ mol: string, idx: number }[]} */ const out = [];
  for (const id of ids) {
    const a = st.atoms.get(id); let hit = null;
    for (const m of nodes) {
      const i = j[m].atoms.findIndex((/** @type {any} */ x) => x.location && Math.abs(x.location[0] - a.pp.x) < 1e-3 && Math.abs(x.location[1] + a.pp.y) < 1e-3);
      if (i >= 0) { hit = { mol: m, idx: i }; break; }
    }
    if (!hit) return null;
    out.push(hit);
  }
  return out;
}

/** changes the .ket of the drawing with `fn` (true = changed) and loads it back: the S-groups have no other door in Ketcher's API */
async function editKet(/** @type {any} */ K, /** @type {number[]} */ ids, /** @type {(mol: any, idx: number[]) => boolean} */ fn) {
  const j = JSON.parse(await K.getKet()), at = ketIndex(j, K.editor.struct(), ids);
  if (!at || new Set(at.map(a => a.mol)).size !== 1) return false;
  if (!fn(j[at[0].mol], at.map(a => a.idx))) return false;
  await K.setMolecule(JSON.stringify(j));
  return true;
}

/** brackets on the atoms `ids` (one molecule) with the delta; with `key` KEY_M, the multicentre attachment on them */
export const addPartial = (/** @type {any} */ K, /** @type {number[]} */ ids, /** @type {string} */ delta, key = KEY) => editKet(K, ids, (mol, idx) => {
  (mol.sgroups = mol.sgroups || []).push({ type: "DAT", atoms: idx, context: "Fragment", fieldName: key, fieldData: delta });
  return true;
});

/** removes the S-group `key` whose atoms are exactly `ids` */
export const removePartial = (/** @type {any} */ K, /** @type {number[]} */ ids, key = KEY) => editKet(K, ids, (mol, idx) => {
  const want = [...idx].sort((a, b) => a - b).join(","), n = (mol.sgroups || []).length;
  mol.sgroups = (mol.sgroups || []).filter((/** @type {any} */ s) => !(s.type === "DAT" && s.fieldName === key && [...s.atoms].sort((a, b) => a - b).join(",") === want));
  return mol.sgroups.length !== n;
});

/**
 * V3000 molfile of the piece `ids` with the multicentre attachments of `ms` written as the standard does: a «*» atom at the centre of the candidates and a
 * bond «*»–substituent with ENDPTS=(n a1 … an) ATTACH=ANY in place of the drawn bond. Coordinates of Ketcher (y up). Null for atoms without an element.
 */
export function multiMolfile(/** @type {any} */ K, /** @type {number[]} */ ids, /** @type {Multi[]} */ ms) {
  const st = K.editor.struct(), set = new Set(ids), idx = new Map(ids.map((id, n) => [id, n + 1])), atoms = ids.map(i => st.atoms.get(i));
  if (atoms.some((/** @type {any} */ a) => !/^[A-Z][a-z]?$/.test(a.label))) return null;
  const f = (/** @type {number} */ v) => v.toFixed(4), A = [], B = [], drop = new Set(ms.map(m => m.bond));
  atoms.forEach((/** @type {any} */ a, /** @type {number} */ k) => A.push(`M  V30 ${k + 1} ${a.label} ${f(a.pp.x)} ${f(-a.pp.y)} 0 0${a.charge ? " CHG=" + a.charge : ""}`));
  st.bonds.forEach((/** @type {any} */ b, /** @type {number} */ k) => { if (set.has(b.begin) && set.has(b.end) && !drop.has(k)) B.push(`${b.type >= 1 && b.type <= 4 ? b.type : 1} ${idx.get(b.begin)} ${idx.get(b.end)}`); });
  for (const m of ms.filter(q => set.has(q.sub))) {
    const pts = m.ends.map(i => st.atoms.get(i).pp), n = A.length + 1;
    A.push(`M  V30 ${n} * ${f(pts.reduce((s, p) => s + p.x, 0) / pts.length)} ${f(-pts.reduce((s, p) => s + p.y, 0) / pts.length)} 0 0`);
    B.push(`${st.bonds.get(m.bond).type || 1} ${n} ${idx.get(m.sub)} ENDPTS=(${m.ends.length} ${m.ends.map(i => idx.get(i)).join(" ")}) ATTACH=ANY`);
  }
  return ["", "  mzLab", "", "  0  0  0     0  0            999 V3000", "M  V30 BEGIN CTAB", `M  V30 COUNTS ${A.length} ${B.length} 0 0 0`, "M  V30 BEGIN ATOM", ...A,
    "M  V30 END ATOM", "M  V30 BEGIN BOND", ...B.map((b, k) => `M  V30 ${k + 1} ${b}`), "M  V30 END BOND", "M  V30 END CTAB", "M  END"].join("\n");
}

/** draws the brackets of every partial structure in `svg` (units = Ketcher units × `sc`) and hides Ketcher's own text of the delta */
export function drawPartials(/** @type {any} */ K, /** @type {SVGSVGElement} */ svg, /** @type {number} */ sc) {
  let g = svg.querySelector("#mz-parziali");
  if (!g) { g = svg.ownerDocument.createElementNS("http://www.w3.org/2000/svg", "g"); g.id = "mz-parziali"; g.setAttribute("pointer-events", "none"); }
  svg.appendChild(g);
  g.textContent = "";
  const st = K.editor.struct(), ps = partials(K), ms = multis(K), texts = new Set([...ps.map(p => p.delta), ...(ms.length ? [ANY] : [])]);
  const at = (/** @type {number} */ i) => { const a = st.atoms.get(i); return { x: a.pp.x * sc, y: a.pp.y * sc }; };
  for (const p of ps) drawBrackets(g, p.atoms.filter(i => st.atoms.has(i)).map(at), { text: p.delta, pad: sc * 0.45 });
  for (const m of ms) drawMulticenter(g, at(m.sub), m.ends.map(at), { r: sc * 0.12 });
  svg.querySelectorAll("text").forEach(t => { if (!t.closest("#mz-parziali, #" + (typeof NOMI !== "undefined" ? NOMI.KEY("labels") : "qqq-labels"))) t.style.visibility = texts.has((t.textContent || "").trim()) ? "hidden" : ""; });
}

/** the .ket text without the data S-groups of the partial structures: the exported image gets their brackets from `addExportPartials`, not Ketcher's DAT text */
export function ketWithoutPartials(/** @type {string} */ ket) {
  const j = JSON.parse(ket);
  for (const k of Object.keys(j)) if (j[k] && j[k].type === "molecule" && j[k].sgroups) j[k].sgroups = j[k].sgroups.filter((/** @type {any} */ s) => !(s.type === "DAT" && (s.fieldName === KEY || s.fieldName === KEY_M)));
  return JSON.stringify(j);
}

/**
 * Draws the brackets, the delta and the dashed attachment of the partial structures in an exported SVG (same drawing as `drawPartials`, 40 px per unit) and
 * grows the picture to hold them. `off` is the shift of the structures in the picture (`exportOffset` of draw.js). Returns the text unchanged with nothing to draw.
 */
export function addExportPartials(/** @type {any} */ K, /** @type {string} */ svgText, /** @type {{ ox: number, oy: number }} */ off, sc = 40) {
  const st = K.editor.struct(), ps = partials(K), ms = multis(K);
  if (!ps.length && !ms.length) return svgText;
  const doc = new DOMParser().parseFromString(svgText, "image/svg+xml"), root = doc.documentElement;
  const vb = (root.getAttribute("viewBox") || "").split(/[\s,]+/).map(Number);
  if (vb.length !== 4 || vb.some(isNaN)) return svgText;
  const g = doc.createElementNS("http://www.w3.org/2000/svg", "g"); g.setAttribute("id", "mz-parziali");
  const at = (/** @type {number} */ i) => { const a = st.atoms.get(i); return { x: a.pp.x * sc + off.ox, y: a.pp.y * sc + off.oy }; };
  let [x0, y0, x1, y1] = [vb[0], vb[1], vb[0] + vb[2], vb[1] + vb[3]];
  for (const p of ps) {
    const pts = p.atoms.filter(i => st.atoms.has(i)).map(at); if (!pts.length) continue;
    drawBrackets(g, pts, { text: p.delta, pad: sc * 0.45 });
    const b = bracketBox(pts, sc * 0.45), w = 13 * 0.6 * p.delta.length + 6;
    x0 = Math.min(x0, b.x0 - 4); y0 = Math.min(y0, b.y0 - 4); x1 = Math.max(x1, b.x1 + w); y1 = Math.max(y1, b.y1 + 4);
  }
  for (const m of ms) drawMulticenter(g, at(m.sub), m.ends.map(at), { r: sc * 0.12 });
  const bg = [...doc.querySelectorAll("rect")].find(r => !r.closest("defs"));
  root.setAttribute("viewBox", `${x0} ${y0} ${x1 - x0} ${y1 - y0}`);
  const pw = parseFloat(root.getAttribute("width") || ""), ph = parseFloat(root.getAttribute("height") || "");
  if (pw) root.setAttribute("width", String(pw * (x1 - x0) / vb[2])); if (ph) root.setAttribute("height", String(ph * (y1 - y0) / vb[3]));
  if (bg && bg.getAttribute("fill") && /^(rgb\(100%,\s*100%,\s*100%\)|#fff(fff)?|white)$/i.test(bg.getAttribute("fill") || "")) {
    bg.setAttribute("x", String(x0)); bg.setAttribute("y", String(y0)); bg.setAttribute("width", String(x1 - x0)); bg.setAttribute("height", String(y1 - y0));
  }
  root.appendChild(g);
  return new XMLSerializer().serializeToString(doc);
}

/**
 * The drawing for mzFinder: {smiles, delta, level, positionUnknown, molfile, info}. Level 3 with a delta or a multicentre attachment, otherwise null (a whole
 * structure: the level is not the drawing's to say); with a multicentre attachment `molfile` is the V3000 with ENDPTS of every atom drawn, else "".
 * @param {any} K
 * @param {(ids: number[]) => string | null} [molfile] V3000 writer (multiMolfile of all atoms when omitted)
 * @param {(ids: number[]) => string | null} [v2000] V2000 writer of the drawing, for `info` (formula, mass and m/z of [M+H]+ with the delta; null without a delta)
 */
export async function partialExport(K, molfile, v2000) {
  const delta = partials(K).map(p => p.delta).filter(Boolean).join(" "), ms = multis(K), all = [...K.editor.struct().atoms.keys()];
  const mf = ms.length ? (molfile ? molfile(all) : multiMolfile(K, all, ms)) || "" : "";
  // formula and m/z of the whole drawing with the delta (the engine, parz/1)
  const flat = delta && v2000 ? v2000(all) : null, r = flat ? await checkDelta(flat, delta) : null;
  const info = r && !r.error ? { formula: r.formula, mass: r.mass, mz: r.mz } : null;
  return { smiles: plainSmiles(await K.getSmiles()), delta, level: delta || ms.length ? 3 : null, positionUnknown: ms.length > 0, molfile: mf, info };
}

/** @typedef {{ pieces: () => { ids: number[] }[], pieceMolfile: (ids: number[]) => string | null, num: (v: number) => string, t: (k: string, p?: any) => string, copy: (s: string) => void, ask: (title: string, def: string) => Promise<string | null>, note: (s: string) => void }} Helpers */
/** @typedef {{ text: string, title?: string, run?: () => void }} Item */

/**
 * Rows of the context menu: on a partial structure its formula, mass and m/z with the delta (a click copies) and «Remove the brackets»; with atoms selected
 * in one piece and outside other brackets, «Brackets with delta…».
 * @returns {Promise<Item[]>}
 */
export async function partialItems(/** @type {any} */ K, /** @type {MouseEvent} */ ev, /** @type {Set<number>} */ sel, /** @type {Helpers} */ h) {
  const st = K.editor.struct(), ps = partials(K), hit = K.editor.findItem(ev, ["atoms", "bonds"]);
  const atom = hit ? (hit.map === "bonds" ? st.bonds.get(hit.id)?.begin : hit.id) : undefined;
  /** @type {Item[]} */ const items = [];
  const p = atom === undefined ? null : ps.find(q => q.atoms.includes(atom));
  if (p) {
    const piece = h.pieces().find(q => q.ids.includes(atom)), mf = piece && h.pieceMolfile(piece.ids);
    const all = ps.filter(q => piece && q.atoms.some(a => piece.ids.includes(a))).map(q => q.delta).join(" ");
    const r = mf ? await checkDelta(mf, all) : null;
    if (r && !r.error) { const line = h.t("draw.part.line", { delta: r.delta, formula: r.formula, mass: h.num(r.mass), mz: h.num(r.mz) }); items.push({ text: line, run: () => h.copy(line) }); }
    items.push({ text: h.t("draw.part.remove"), run: () => { removePartial(K, p.atoms); } });
  }
  const m = atom === undefined ? null : multis(K).find(q => q.atoms.includes(atom));
  if (m) items.push({ text: h.t("draw.multi.remove"), run: () => { removePartial(K, m.atoms, KEY_M); } });
  const ids = [...sel];
  if (!m && ids.length >= 3 && multiShape(st, ids) && !multis(K).some(q => q.atoms.some(a => ids.includes(a)))) items.push({ text: h.t("draw.multi.add"), title: h.t("draw.multi.add.title"), run: async () => {
    if (!(await addPartial(K, ids, ANY, KEY_M))) h.note(h.t("draw.part.onePiece"));
  } });
  if (ids.length && !ids.some(i => ps.some(q => q.atoms.includes(i)))) items.push({ text: h.t("draw.part.add"), title: h.t("draw.part.add.title"), run: async () => {
    const piece = h.pieces().find(q => q.ids.includes(ids[0]));
    if (!piece || !ids.every(i => piece.ids.includes(i))) { h.note(h.t("draw.part.onePiece")); return; }
    const d = await h.ask(h.t("draw.part.ask"), "+O"); if (!d) return;
    const mf = h.pieceMolfile(piece.ids), r = mf ? await checkDelta(mf, d) : null;
    if (r && r.error) { h.note(h.t(r.error === "error.grafo.delta.neg" ? "draw.part.neg" : "draw.part.bad", { delta: d })); return; }
    if (!(await addPartial(K, ids, r ? r.delta : d.trim()))) h.note(h.t("draw.part.onePiece"));
  } });
  return items;
}
