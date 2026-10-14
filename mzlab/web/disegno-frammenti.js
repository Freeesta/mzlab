// @ts-check
// «Fragments» of the drawing: right click on a bond gives the two pieces it would leave (formula, neutral mass, m/z of [F+H]+), and a table lists every cut
// of the molecule. The numbers come from the Rust graph (crates/mzlab-feat-grafo-wasm, `fragments`), fed with the V2000 molfile of the piece so that the atom
// order is the one of Ketcher (a SMILES from Indigo is renumbered). Hydrogens: each side takes one H per bond order, as in mzFinder's cleavages.

/** @typedef {{ atoms: number[], formula: string, mass: number, mz: number }} Side */
/** @typedef {{ a: number, b: number, order: number, kind: string, sides: Side[] }} Cut */

/** @type {Promise<any> | null} */
let modulo = null;

/** hex SHA-256 of some bytes */
async function sha256(/** @type {ArrayBuffer} */ b) {
  return [...new Uint8Array(await crypto.subtle.digest("SHA-256", b))].map(x => x.toString(16).padStart(2, "0")).join("");
}

/** the app: the same two functions as the .wasm, asked of the native engine (`feat/grafo`); an error is thrown as the text `{error_key, params}`, as the .wasm throws it */
function nativo() {
  const base = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : "mzlab://localhost/";
  const ask = async (/** @type {Record<string, any>} */ body) => {
    const r = await fetch(base + "rpc", { method: "POST", body: JSON.stringify({ op: "feat/grafo", ...body }) });
    const t = await r.text();
    if (r.status !== 200) throw t;
    return t;
  };
  return {
    fragments: (/** @type {string} */ molfile, /** @type {number} */ a, /** @type {number} */ b) => ask({ kind: "fragments", molfile, a, b }),
    partial: (/** @type {string} */ molfile, /** @type {string} */ delta) => ask({ kind: "partial", molfile, delta }),
  };
}

/** the graph module, loaded on the first cut; the .wasm is checked against the SHA-256 of wasm-feat/manifest.json */
export function carica() {
  if (modulo) return modulo;
  if (/** @type {any} */ (window).MZLAB_DESKTOP) return (modulo = Promise.resolve(nativo()));
  modulo = (async () => {
    const base = new URL("./wasm-feat/", import.meta.url), man = await (await fetch(new URL("manifest.json", base))).json(), e = man.grafo;
    if (!e) throw new Error("grafo");
    const bytes = await (await fetch(new URL(e.wasm, base))).arrayBuffer();
    if ((await sha256(bytes)) !== e.sha256) throw new Error("sha");
    const mod = await import(new URL(e.js, base).href);
    await mod.default({ module_or_path: bytes });
    return mod;
  })();
  modulo.catch(() => { modulo = null; });
  return modulo;
}

/**
 * The cuts of a molfile: `a`/`b` = positions (0-based) of the bond's atoms in the file, or -1 for every cut outside the rings.
 * @returns {Promise<Cut[] | "ring" | null>} null when the engine cannot answer (offline, molfile refused)
 */
export async function cutsOf(/** @type {string} */ molfile, /** @type {number} */ a = -1, /** @type {number} */ b = -1) {
  try {
    const mod = await carica();
    return JSON.parse(await mod.fragments(molfile, a, b)).cuts;
  } catch (e) {
    return typeof e === "string" && e.includes("error.grafo.ring") ? "ring" : null;
  }
}

/** one line for a side, numbers already formatted: «C2H6 · 30.05 · m/z 31.05» */
export const sideLine = (/** @type {Side} */ s, /** @type {(v: number) => string} */ num, /** @type {(k: string, p?: any) => string} */ t) =>
  t("draw.frag.side", { formula: s.formula, mass: num(s.mass), mz: num(s.mz) });

/** options of `TDATI.table` for every cut: the bond by element and number, the two pieces with formula, mass and m/z */
export function cutsTable(/** @type {Cut[]} */ cuts, /** @type {string[]} */ labels, /** @type {(v: number) => string} */ num, /** @type {(k: string, p?: any) => string} */ t) {
  const name = (/** @type {number} */ i) => `${labels[i]}${i + 1}`;
  return {
    title: t("draw.frag.title"), rowHeader: true,
    head: [t("draw.frag.col.bond"), t("draw.frag.col.formula", { n: 1 }), t("draw.frag.col.mz", { n: 1 }), t("draw.frag.col.formula", { n: 2 }), t("draw.frag.col.mz", { n: 2 })],
    rows: cuts.map(c => [`${name(c.a)}–${name(c.b)}`, `${c.sides[0].formula} (${num(c.sides[0].mass)})`, num(c.sides[0].mz), `${c.sides[1].formula} (${num(c.sides[1].mass)})`, num(c.sides[1].mz)]),
    note: t("draw.frag.note"),
  };
}

/** @typedef {{ pieces: () => { ids: number[] }[], pieceMolfile: (ids: number[]) => string | null, num: (v: number) => string, t: (k: string, p?: any) => string, copy: (s: string) => void, table: (o: any) => void }} Helpers */
/** @typedef {{ text: string, title?: string, run?: () => void }} Item */

/**
 * Rows for the context menu at the event `ev` of the editor frame: the two pieces of the bond under the pointer (read-only rows, a click copies the line) and
 * «All cuts…» for the molecule under it. Empty when the pointer is on nothing, or on a piece without a plain structure (R groups, abbreviations).
 * @returns {Promise<Item[]>}
 */
export async function fragmentItems(/** @type {any} */ K, /** @type {MouseEvent} */ ev, /** @type {Helpers} */ h) {
  const hit = K.editor.findItem(ev, ["bonds", "atoms"]); if (!hit) return [];
  const st = K.editor.struct(), b = hit.map === "bonds" ? st.bonds.get(hit.id) : null, atom = b ? b.begin : hit.id;
  const piece = h.pieces().find(p => p.ids.includes(atom)), mf = piece && h.pieceMolfile(piece.ids); if (!piece || !mf) return [];
  /** @type {Item[]} */ const items = [];
  if (b) {
    const r = await cutsOf(mf, piece.ids.indexOf(b.begin), piece.ids.indexOf(b.end));
    if (r === "ring") items.push({ text: h.t("draw.frag.ring") });
    else if (r && r[0]) r[0].sides.forEach(s => { const line = sideLine(s, h.num, h.t); items.push({ text: line, run: () => h.copy(line) }); });
    else items.push({ text: h.t("draw.frag.off") });
  }
  items.push({ text: h.t("draw.frag.all"), title: h.t("draw.frag.all.title"), run: async () => {
    const cuts = await cutsOf(mf);
    if (Array.isArray(cuts)) h.table(cutsTable(cuts, piece.ids.map(i => st.atoms.get(i).label), h.num, h.t));
  } });
  return items;
}
