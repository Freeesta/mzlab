// @ts-check
// Formula, mass and arrow change of the structures of the drawing, from the Rust engine (`labels` of the grafo .wasm, `etich/1`; the app asks `feat/grafo`):
// isotopes ([13C], D), every element, charge. The page sends the atoms as Ketcher holds them and which structures each arrow joins; the answers are kept by
// request, so the labels are drawn at once from a known answer and redrawn when a new one arrives (nothing is shown until then).
import { carica } from "./disegno-frammenti.js";

/** a Ketcher label that is an element symbol (not R#, A, Q, M, X, L, *, an atom list) */
export const isElement = (/** @type {string} */ l) => /^[A-Z][a-z]?$/.test(l) && !/^[AQMXLR]$/.test(l);

/** the atom as the engine reads it */
export const atomOf = (/** @type {any} */ a) => ({ el: a.label, iso: a.isotope || 0, h: a.implicitH || 0, q: a.charge || 0 });

/** @type {Map<string, any>} request text -> answer (null while it is asked) */
const cache = new Map();

/** the answer for a request when known; otherwise null, and `redraw` is called once it arrives */
export function labelsOf(/** @type {any} */ req, /** @type {() => void} */ redraw) {
  const key = JSON.stringify(req);
  if (cache.has(key)) return cache.get(key);
  cache.set(key, null);
  const old = cache.keys().next().value; if (cache.size > 64 && old !== undefined) cache.delete(old);
  carica().then(m => m.labels(key)).then(t => { cache.set(key, JSON.parse(t)); redraw(); }).catch(() => cache.delete(key));
  return null;
}

/** the same, awaited (property card) */
export async function labelsNow(/** @type {any} */ req) {
  const key = JSON.stringify(req), known = cache.get(key);
  if (known) return known;
  try { const r = JSON.parse(await (await carica()).labels(key)); cache.set(key, r); return r; } catch (_) { return null; }
}

/** pieces of a formula for a label: `[13C]` as a superscript mass number before the symbol, counts as subscripts */
export function formulaParts(/** @type {string} */ f) {
  const out = /** @type {[string, string][]} */ ([]);
  for (const x of f.matchAll(/(?:\[(\d+)([A-Z][a-z]?)\]|([A-Z][a-z]?))(\d*)/g)) {
    if (x[1]) out.push([x[1], "sup"]);
    out.push([x[2] || x[3], ""]);
    if (x[4]) out.push([x[4], "sub"]);
  }
  return out;
}
