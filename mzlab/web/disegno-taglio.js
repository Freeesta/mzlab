// @ts-check
// The cut tool of the Drawing: a cut line drawn across bonds, or the selected bonds, breaks them the way the student chooses: homolytic (default, a
// radical mark on each end), heterolytic (+ on one piece, − on the other) or the «+H» shorthand. The Rust engine (`cut` of the grafo .wasm, `cut/1`)
// says which atoms change and how far each piece moves; the page writes that into the .ket (one undo step) and the labels under each piece follow.
import { carica } from "./disegno-frammenti.js";
import { ketIndex } from "./disegno-parziali.js";
import { dots } from "./disegno-etichette.js";

/** @typedef {{ formula: string, text: string, charge: number, mass: number, mz: number | null, unpaired: number, odd: boolean, kind: string }} Label */
/** @typedef {{ atoms: number[], shift: [number, number], label: Label }} Piece */
/** @typedef {{ mode: string, atoms: { i: number, charge: number, radical: number, h: number, pin: boolean }[], pieces: Piece[] }} Cut */
/** @typedef {{ pieces: () => { ids: number[] }[], pieceMolfile: (ids: number[]) => string | null, num: (v: number) => string, t: (k: string, p?: any) => string }} Helpers */

/** the cut of a request, or `{error}` with the key the engine gave (`error.grafo.cut.*`), or null when the engine cannot answer */
export async function cutOf(/** @type {Record<string, any>} */ req) {
  try { return /** @type {Cut} */ (JSON.parse(await (await carica()).cut(JSON.stringify(req)))); }
  catch (e) { try { return { error: /** @type {string} */ (JSON.parse(String(e)).error_key) }; } catch (_) { return null; } }
}

/** the request for Ketcher's bonds `bids`: the molfile of every piece they touch (Ketcher's atom order) and the bonds as positions in it */
export function requestFor(/** @type {any} */ K, /** @type {number[]} */ bids, /** @type {string} */ mode, /** @type {Helpers} */ h) {
  const st = K.editor.struct(), bonds = bids.map(id => st.bonds.get(id)).filter(Boolean);
  const ids = h.pieces().filter(p => bonds.some(b => p.ids.includes(b.begin))).flatMap(p => p.ids), mf = ids.length ? h.pieceMolfile(ids) : null;
  if (!mf) return null;
  return { ids, req: { molfile: mf, bonds: bonds.map(b => [ids.indexOf(b.begin), ids.indexOf(b.end)]), mode } };
}

/** writes a cut into the drawing: bonds removed, charges and radical marks set (and the H count where Ketcher's valence rule would differ), pieces moved apart */
export async function applyCut(/** @type {any} */ K, /** @type {number[]} */ ids, /** @type {number[][]} */ pairs, /** @type {Cut} */ cut) {
  const j = JSON.parse(await K.getKet()), at = ketIndex(j, K.editor.struct(), ids);
  if (!at) return false;
  for (const c of cut.atoms) {
    const a = j[at[c.i].mol].atoms[at[c.i].idx];
    if (c.charge) a.charge = c.charge; else delete a.charge;
    if (c.radical) a.radical = c.radical; else delete a.radical;
    if (c.pin) a.implicitHCount = c.h;
  }
  for (const [x, y] of pairs) {
    const m = j[at[x].mol], u = at[x].idx, v = at[y].idx;
    m.bonds = m.bonds.filter((/** @type {any} */ b) => !((b.atoms[0] === u && b.atoms[1] === v) || (b.atoms[0] === v && b.atoms[1] === u)));
  }
  for (const p of cut.pieces) for (const i of p.atoms) { const l = j[at[i].mol].atoms[at[i].idx].location; l[0] += p.shift[0]; l[1] += p.shift[1]; }   // .ket and molfile share x and y (up)
  await K.setMolecule(JSON.stringify(j));
  return true;
}

/** the bonds a segment from `p` to `q` (Ketcher coordinates) crosses */
export function bondsCrossed(/** @type {any} */ st, /** @type {{x: number, y: number}} */ p, /** @type {{x: number, y: number}} */ q) {
  const side = (/** @type {any} */ a, /** @type {any} */ b, /** @type {any} */ c) => Math.sign((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x));
  const out = /** @type {number[]} */ ([]);
  st.bonds.forEach((/** @type {any} */ b, /** @type {number} */ id) => {
    const a = st.atoms.get(b.begin).pp, c = st.atoms.get(b.end).pp;
    if (side(p, q, a) * side(p, q, c) < 0 && side(a, c, p) * side(a, c, q) < 0) out.push(id);
  });
  return out;
}

const KINDS = /** @type {Record<string, string>} */ ({ neutral: "draw.cut.kind.neutral", radical: "draw.cut.kind.radical", cation: "draw.cut.kind.cation",
  anion: "draw.cut.kind.anion", radicalCation: "draw.cut.kind.radicalCation", radicalAnion: "draw.cut.kind.radicalAnion" });

/** one line per piece for the status line and the screen reader: «C6H4F• radical · exact mass 95.0297» */
export function pieceLine(/** @type {Label} */ l, /** @type {Helpers} */ h) {
  const dot = dots(l), q = l.charge ? (Math.abs(l.charge) > 1 ? Math.abs(l.charge) : "") + (l.charge > 0 ? "+" : "−") : "";
  const kind = h.t(KINDS[l.kind] || KINDS.neutral), mass = l.mz != null ? `m/z ${h.num(l.mz)}` : `${h.t("draw.labels.exactMass")} ${h.num(l.mass)}`;
  return `${l.formula}${q}${dot} ${kind} · ${mass}`;
}

/**
 * Cuts Ketcher's bonds `bids` in `mode`. Heterolytic asks `pick` which of the two pieces takes the + (rows for both ways, from the engine); a cut that
 * does not give two pieces cannot be heterolytic. Returns the status text, or the text of an error.
 * @param {(rows: { text: string, run: () => void }[]) => void} pick
 */
export async function cutBonds(/** @type {any} */ K, /** @type {number[]} */ bids, /** @type {string} */ mode, /** @type {Helpers} */ h, pick) {
  const r = requestFor(K, bids, mode === "heterolytic" ? "homolytic" : mode, h); if (!r) return h.t("draw.cut.none");
  const first = await cutOf(r.req);
  if (!first) return h.t("draw.frag.off");
  if ("error" in first) return h.t(/** @type {string} */ (first.error));
  const done = async (/** @type {Cut} */ c) => (await applyCut(K, r.ids, r.req.bonds, c)) ? h.t("draw.cut.done", { pieces: c.pieces.map(p => pieceLine(p.label, h)).join("; ") }) : h.t("draw.cut.none");
  if (mode !== "heterolytic") return done(/** @type {Cut} */ (first));
  if (first.pieces.length !== 2) return h.t("draw.cut.hetero.two");
  const ways = await Promise.all(first.pieces.map(p => cutOf({ ...r.req, mode, cation: r.req.bonds.map(([a, b]) => p.atoms.includes(a) ? a : b) })));
  if (ways.some(w => !w || "error" in w)) { const w = ways.find(x => x && "error" in x); return h.t(w && "error" in w ? w.error : "draw.frag.off"); }
  return new Promise(res => pick(/** @type {Cut[]} */ (ways).map(w => {
    const cat = w.pieces.find(p => p.label.charge > 0), an = w.pieces.find(p => p.label.charge < 0);
    return { text: h.t("draw.cut.hetero.row", { cation: cat ? pieceLine(cat.label, h) : "", anion: an ? pieceLine(an.label, h) : "" }), run: () => { done(w).then(res); } };
  })));
}

/**
 * The cut line: while the tool is on, a layer over the editor frame takes the pointer (mouse, finger or pen); a drag draws a dashed line and the
 * bonds it crosses are cut on release. Esc or the button turn it off.
 * @param {{ frame: HTMLIFrameElement, button: HTMLButtonElement, K: () => any, cut: (bids: number[], at: { x: number, y: number }) => void }} o
 */
export function mountCutLine(o) {
  const host = /** @type {HTMLElement} */ (o.frame.parentElement), layer = document.createElement("div");
  layer.id = "cut-layer"; layer.hidden = true; layer.setAttribute("aria-hidden", "true");
  layer.innerHTML = '<svg width="100%" height="100%"><line stroke-dasharray="6 4" stroke-width="2.5" stroke-linecap="round" visibility="hidden"/></svg>';
  host.appendChild(layer);
  const line = /** @type {SVGLineElement} */ (layer.querySelector("line"));
  /** @type {{ x: number, y: number } | null} */ let from = null;
  const set = (/** @type {boolean} */ on) => {
    layer.hidden = !on; o.button.setAttribute("aria-pressed", String(on)); from = null; line.setAttribute("visibility", "hidden");
    if (on) { const f = o.frame.getBoundingClientRect(), hb = host.getBoundingClientRect(); Object.assign(layer.style, { left: f.left - hb.left + "px", top: f.top - hb.top + "px", width: f.width + "px", height: f.height + "px" }); }
  };
  const local = (/** @type {PointerEvent} */ e) => { const f = layer.getBoundingClientRect(); return { x: e.clientX - f.left, y: e.clientY - f.top }; };
  o.button.addEventListener("click", () => set(layer.hidden));
  layer.addEventListener("pointerdown", e => { from = local(e); layer.setPointerCapture(e.pointerId); ["x1", "x2"].forEach(k => line.setAttribute(k, String(from && from.x))); ["y1", "y2"].forEach(k => line.setAttribute(k, String(from && from.y))); line.setAttribute("visibility", "visible"); e.preventDefault(); });
  layer.addEventListener("pointermove", e => { if (!from) return; const p = local(e); line.setAttribute("x2", String(p.x)); line.setAttribute("y2", String(p.y)); });
  layer.addEventListener("pointerup", e => {
    if (!from) return;
    const p = local(e), K = o.K(), R = K.editor.render, a = R.page2obj({ clientX: from.x, clientY: from.y }), b = R.page2obj({ clientX: p.x, clientY: p.y });
    from = null; line.setAttribute("visibility", "hidden");
    const bids = bondsCrossed(K.editor.struct(), a, b);
    if (bids.length) o.cut(bids, { x: e.clientX, y: e.clientY });
  });
  layer.addEventListener("click", e => e.stopPropagation());   // the click after a drag would close the menu the cut may open (heterolytic)
  document.addEventListener("keydown", e => { if (e.key === "Escape" && !layer.hidden) set(false); });
  return { off: () => set(false), on: () => set(true) };
}
