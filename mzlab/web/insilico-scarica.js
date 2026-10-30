// @ts-check
// Download of the in-silico candidate package (docs/agenti/insilico.md) into the Cache Storage `mzlab-libs`: the PubChemLite structures that `spectra.insilico` ranks, in mass-range
// pieces. The catalogue `insilico/catalogo.json` (`insilico/1`) and its zlib pieces come from the site itself (CSP unchanged) and are requested only on a click. Every piece is
// checked (size and SHA-256), kept as it came, and inflated alone, only when a query's mass window needs it (the inflated rows are checked too). The helpers of the
// regulatory-list package (`normative-scarica.js`) do the checking. After the first download everything is read from the cache, offline. Loaded on demand, not at start-up.

import { CACHE, fetchPiece, parseCatalog as parseBase, sha256 } from "./normative-scarica.js";

export const FORMAT = "insilico/1";
/** The folder of the package next to this module (`static/insilico/`). */
export const BASE = new URL("insilico/", import.meta.url).href;

/** @typedef {{ file: string, size: number, sha256: string, mass_min: number, mass_max: number, rows: number, size_raw: number, sha256_raw: string }} Piece */
/** @typedef {{ format: string, size: number, size_compressed: number, sha256: string, rows: number, licence: string, attribution: string, source: { name: string, version: string, doi: string }, pieces: Piece[] }} Catalog */
/** @typedef {{ base?: string, fetchFn?: typeof fetch, caches?: CacheStorage, signal?: AbortSignal, onProgress?: (p: { piece: number, pieces: number, bytes: number, total: number }) => void }} Options */

const err = (/** @type {string} */ key, /** @type {Record<string, unknown>} */ params = {}) => Object.assign(new Error(key), { key, params });
const where = (/** @type {Options} */ o) => ({ base: o.base ?? BASE, fetchFn: o.fetchFn ?? ((u, i) => fetch(u, i)), cs: o.caches ?? caches });

/** The catalogue text checked: the common checks of the packages, then what the windowed reading needs (mass range, raw size and SHA-256 of every piece). */
export function parseCatalog(/** @type {string} */ text) {
  /** @type {Catalog} */ const c = /** @type {any} */ (parseBase(text, FORMAT));
  for (const p of c.pieces) {
    if (!(p.mass_min <= p.mass_max) || !(p.size_raw > 0) || !/^[0-9a-f]{64}$/.test(p.sha256_raw || "")) throw err("normative.dl.err.catalog", { detail: String(p.file) });
  }
  return c;
}

/** «N MB» of the compressed package, as the button shows it. */
export const megabytes = (/** @type {Catalog} */ c) => Math.max(1, Math.round(c.size_compressed / 1048576));

/** The catalogue the site serves, with its text; nothing else is requested. */
export async function fetchCatalog(/** @type {Options} */ o = {}) {
  const { base, fetchFn } = where(o);
  let text;
  try {
    const res = await fetchFn(base + "catalogo.json", { cache: "no-store", signal: o.signal });
    if (!res.ok) throw err("normative.dl.err.http", { status: res.status });
    text = await res.text();
  } catch (e_) {
    const e = /** @type {any} */ (e_);
    if (e && e.key) throw e;
    throw o.signal && o.signal.aborted ? err("normative.dl.err.cancelled") : err("normative.dl.err.offline", { detail: String(e && e.message || e) });
  }
  return { catalog: parseCatalog(text), text };
}

/** The package kept in the cache, `{ catalog, text }`, or null. Reads the cache only: no request. */
export async function installed(/** @type {Options} */ o = {}) {
  const { base, cs } = where(o);
  try {
    const r = await (await cs.open(CACHE)).match(base + "catalogo.json");
    if (!r) return null;
    const text = await r.text();
    return { catalog: parseCatalog(text), text };
  } catch (_) { return null; }
}

/**
 * Downloads every piece of the package into the cache (the ones already there and right are not asked again); the catalogue is written last, so a download that stops leaves
 * the previous package whole. Returns `{ catalog, text }`.
 */
export async function install(/** @type {Options & { fetched?: { catalog: Catalog, text: string } }} */ o = {}) {
  const { base, fetchFn, cs } = where(o);
  const cache = await cs.open(CACHE);
  const { catalog: cat, text } = o.fetched || await fetchCatalog(o);
  const before = await installed(o);
  let done = 0;
  for (let i = 0; i < cat.pieces.length; i++) {
    if (o.signal && o.signal.aborted) throw err("normative.dl.err.cancelled");
    const pc = cat.pieces[i];
    const hit = await cache.match(base + pc.file);
    let ok = false;
    if (hit) { const x = new Uint8Array(await hit.arrayBuffer()); ok = x.length === pc.size && (await sha256(x)) === pc.sha256; }
    if (!ok) await cache.put(base + pc.file, new Response(await fetchPiece(fetchFn, base + pc.file, pc, o.signal)));
    done += pc.size;
    if (o.onProgress) o.onProgress({ piece: i + 1, pieces: cat.pieces.length, bytes: done, total: cat.size_compressed });
  }
  await cache.put(base + "catalogo.json", new Response(text));
  if (before) for (const p of before.catalog.pieces) if (!cat.pieces.some(q => q.file === p.file)) await cache.delete(base + p.file);
  return { catalog: cat, text };
}

/**
 * The reader of the pieces of an installed package: `read(file)` gives the inflated CSV of one piece, checked against the catalogue (compressed and inflated SHA-256). A piece that
 * is not in the cache is `insilico.dl.err.missing` (the browser's storage was cleared): the package is to be downloaded again.
 */
export function reader(/** @type {Catalog} */ cat, /** @type {Options} */ o = {}) {
  const { base, cs } = where(o);
  return async (/** @type {string} */ file) => {
    const pc = cat.pieces.find(p => p.file === file);
    if (!pc) throw err("normative.dl.err.catalog", { detail: file });
    const hit = await (await cs.open(CACHE)).match(base + file);
    if (!hit) throw err("insilico.dl.err.missing", { file });
    const z = new Uint8Array(await hit.arrayBuffer());
    if (z.length !== pc.size || (await sha256(z)) !== pc.sha256) throw err("normative.dl.err.hash", { what: file });
    const raw = new Uint8Array(await new Response(new Blob([z]).stream().pipeThrough(new DecompressionStream("deflate"))).arrayBuffer());
    if (raw.length !== pc.size_raw || (await sha256(raw)) !== pc.sha256_raw) throw err("normative.dl.err.hash", { what: file });
    return raw;
  };
}

/** Texts the page shows for the errors of this module (the others are the `normative.dl.err.*` keys). */
export const KEYS = ["insilico.dl.err.missing"];
