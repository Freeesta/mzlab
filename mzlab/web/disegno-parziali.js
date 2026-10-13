// @ts-check
// Partial structures of the drawing: a set of atoms inside brackets with a delta «+O -H2» (a change whose position is unknown: a hydroxylation somewhere on
// the ring). Stored in the drawing as a data S-group of Ketcher on the atoms (fieldName «MZLAB», fieldData = the delta, context «Fragment»), so it is saved in
// the .ket and travels with the molfile. Brackets and text are drawn by parentesi.js over the canvas; formula, mass and m/z with the delta come from the Rust
// graph (`partial`, parz/1). The export for mzFinder is {smiles, delta, level: 3} (level 3 of Schymanski: a candidate class, never one structure).
import { drawBrackets } from "./parentesi.js";
import { carica } from "./disegno-frammenti.js";

export const KEY = "MZLAB";

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

/** the delta checked by the engine; null when it is not readable (the text of the error is shown by the caller) */
async function checkDelta(/** @type {string} */ molfile, /** @type {string} */ delta) {
  try { return JSON.parse((await carica()).partial(molfile, delta)); } catch (e) { return typeof e === "string" ? { error: JSON.parse(e).error_key } : null; }
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

/** brackets on the atoms `ids` (one molecule) with the delta */
export const addPartial = (/** @type {any} */ K, /** @type {number[]} */ ids, /** @type {string} */ delta) => editKet(K, ids, (mol, idx) => {
  (mol.sgroups = mol.sgroups || []).push({ type: "DAT", atoms: idx, context: "Fragment", fieldName: KEY, fieldData: delta });
  return true;
});

/** removes the brackets whose atoms are exactly `ids` */
export const removePartial = (/** @type {any} */ K, /** @type {number[]} */ ids) => editKet(K, ids, (mol, idx) => {
  const want = [...idx].sort((a, b) => a - b).join(","), n = (mol.sgroups || []).length;
  mol.sgroups = (mol.sgroups || []).filter((/** @type {any} */ s) => !(s.type === "DAT" && s.fieldName === KEY && [...s.atoms].sort((a, b) => a - b).join(",") === want));
  return mol.sgroups.length !== n;
});

/** draws the brackets of every partial structure in `svg` (units = Ketcher units × `sc`) and hides Ketcher's own text of the delta */
export function drawPartials(/** @type {any} */ K, /** @type {SVGSVGElement} */ svg, /** @type {number} */ sc) {
  let g = svg.querySelector("#mz-parziali");
  if (!g) { g = svg.ownerDocument.createElementNS("http://www.w3.org/2000/svg", "g"); g.id = "mz-parziali"; g.setAttribute("pointer-events", "none"); }
  svg.appendChild(g);
  g.textContent = "";
  const st = K.editor.struct(), ps = partials(K), texts = new Set(ps.map(p => p.delta));
  for (const p of ps) drawBrackets(g, p.atoms.map(i => st.atoms.get(i)).filter(Boolean).map((/** @type {any} */ a) => ({ x: a.pp.x * sc, y: a.pp.y * sc })), { text: p.delta, pad: sc * 0.45 });
  svg.querySelectorAll("text").forEach(t => { if (!t.closest("#mz-parziali, #qqq-labels")) t.style.visibility = texts.has((t.textContent || "").trim()) ? "hidden" : ""; });
}

/** the drawing for mzFinder: {smiles, delta, level}; level 3 when there is a delta, otherwise null (a whole structure: the level is not the drawing's to say) */
export async function partialExport(/** @type {any} */ K) {
  const delta = partials(K).map(p => p.delta).filter(Boolean).join(" ");
  return { smiles: plainSmiles(await K.getSmiles()), delta, level: delta ? 3 : null };
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
  const ids = [...sel];
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
