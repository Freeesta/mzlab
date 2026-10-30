// @ts-check
// Download of the regulatory-list package (docs/agenti/normative.md) into the Cache Storage `mzlab-libs`. The catalogue `normative/catalogo.json` (`normative/1`) and its
// zlib pieces come from the site itself (CSP unchanged) and are requested only on a click. Every piece is checked (size and SHA-256), the inflated bytes are checked against
// the catalogue, and the raw `.mzsus` goes to the engine. After the first download everything is served from the cache, offline. Loaded on demand, not at start-up.

export const CACHE = "mzlab-libs";
export const FORMAT = "normative/1";
const TRIES = 2;

/** @typedef {{ file: string, size: number, sha256: string }} Piece */
/** @typedef {{ format: string, piece: number, size: number, size_compressed: number, sha256: string, rows: number, lists: Array<Record<string, any>>, attributions: string[], pieces: Piece[] }} Catalog */
/** @typedef {{ base?: string, fetchFn?: typeof fetch, caches?: CacheStorage, signal?: AbortSignal, onProgress?: (p: { piece: number, pieces: number, bytes: number, total: number }) => void }} Options */

const err = (/** @type {string} */ key, /** @type {Record<string, unknown>} */ params = {}) => Object.assign(new Error(key), { key, params });
const hex = (/** @type {ArrayBuffer} */ b) => [...new Uint8Array(b)].map(x => x.toString(16).padStart(2, "0")).join("");
const sha256 = async (/** @type {Uint8Array<ArrayBuffer>} */ u8) => hex(await crypto.subtle.digest("SHA-256", u8));

/** The catalogue text checked the way the Rust packer writes it: format, plain piece names, sizes that add up. */
export function parseCatalog(/** @type {string} */ text) {
  /** @type {Catalog} */ let c;
  try { c = JSON.parse(text); } catch (_) { throw err("normative.dl.err.catalog", { detail: "json" }); }
  if (!c || c.format !== FORMAT || !Array.isArray(c.pieces) || !/^[0-9a-f]{64}$/.test(c.sha256 || "")) throw err("normative.dl.err.catalog", { detail: String(c && c.format) });
  for (const p of c.pieces) if (!/^\d{3}\.bin$/.test(p.file) || !/^[0-9a-f]{64}$/.test(p.sha256) || !(p.size > 0)) throw err("normative.dl.err.catalog", { detail: String(p.file) });
  if (c.pieces.reduce((s, p) => s + p.size, 0) !== c.size_compressed) throw err("normative.dl.err.catalog", { detail: "size" });
  return c;
}

const where = (/** @type {Options} */ o) => ({ base: o.base ?? "normative/", fetchFn: o.fetchFn ?? ((u, i) => fetch(u, i)), cs: o.caches ?? caches });

/** One piece, fetched and checked; fetched once more when it is wrong. An aborted request is a cancel, a refused connection is «offline». */
async function fetchPiece(/** @type {typeof fetch} */ fetchFn, /** @type {string} */ url, /** @type {Piece} */ pc, /** @type {AbortSignal | undefined} */ signal) {
  for (let t = 1; ; t++) {
    let b;
    try {
      const res = await fetchFn(url, { cache: "no-store", signal });
      if (!res.ok) throw err("normative.dl.err.http", { status: res.status });
      b = new Uint8Array(await res.arrayBuffer());
    } catch (e_) {
      const e = /** @type {any} */ (e_);
      if (e && e.key) throw e;
      throw signal && signal.aborted ? err("normative.dl.err.cancelled") : err("normative.dl.err.offline", { detail: String(e && e.message || e) });
    }
    if (b.length === pc.size && (await sha256(b)) === pc.sha256) return b;
    if (t >= TRIES) throw err("normative.dl.err.hash", { what: pc.file });
  }
}

/** The catalogue stored in the cache, or null. */
async function cachedCatalog(/** @type {Cache} */ cache, /** @type {string} */ base) {
  const r = await cache.match(base + "catalogo.json");
  if (!r) return null;
  try { return parseCatalog(await r.text()); } catch (_) { return null; }
}

/** The raw bytes of the pieces of `cat` inflated and checked as a whole. */
async function inflate(/** @type {Catalog} */ cat, /** @type {Uint8Array<ArrayBuffer>[]} */ pieces) {
  const stream = new Blob(pieces).stream().pipeThrough(new DecompressionStream("deflate"));
  const raw = new Uint8Array(await new Response(stream).arrayBuffer());
  if (raw.length !== cat.size || (await sha256(raw)) !== cat.sha256) throw err("normative.dl.err.hash", { what: "normative.mzsus" });
  return raw;
}

/**
 * The raw `.mzsus` of the package: from the cache when its catalogue is the one the site serves (or the site is not reachable), downloaded piece by piece otherwise.
 * Returns `{ raw, catalog, cached }`; the cache is written only after everything checked out.
 */
export async function ensure(/** @type {Options} */ o = {}) {
  const { base, fetchFn, cs } = where(o);
  const cache = await cs.open(CACHE);
  const have = await cachedCatalog(cache, base);
  /** @type {Catalog | null} */ let live = null, liveText = "";
  try {
    const res = await fetchFn(base + "catalogo.json", { cache: "no-store", signal: o.signal });
    if (!res.ok) throw err("normative.dl.err.http", { status: res.status });
    liveText = await res.text();
    live = parseCatalog(liveText);
  } catch (e_) {
    const e = /** @type {any} */ (e_);
    if (o.signal && o.signal.aborted) throw err("normative.dl.err.cancelled");
    if (!have) throw e && e.key && e.key !== "normative.dl.err.http" ? e : err("normative.dl.err.offline", { detail: String(e && e.message || e) });
  }
  const cat = live || /** @type {Catalog} */ (have);
  /** @type {Uint8Array<ArrayBuffer>[]} */ const parts = [];
  let done = 0, fromCache = true;
  for (let i = 0; i < cat.pieces.length; i++) {
    if (o.signal && o.signal.aborted) throw err("normative.dl.err.cancelled");
    const pc = cat.pieces[i];
    let b = null;
    const hit = await cache.match(base + pc.file);
    if (hit) { const x = new Uint8Array(await hit.arrayBuffer()); if (x.length === pc.size && (await sha256(x)) === pc.sha256) b = x; }
    if (!b) { b = await fetchPiece(fetchFn, base + pc.file, pc, o.signal); fromCache = false; }
    parts.push(b);
    done += pc.size;
    if (o.onProgress) o.onProgress({ piece: i + 1, pieces: cat.pieces.length, bytes: done, total: cat.size_compressed });
  }
  const raw = await inflate(cat, parts);
  if (!fromCache || !have || have.sha256 !== cat.sha256) {
    for (let i = 0; i < parts.length; i++) await cache.put(base + cat.pieces[i].file, new Response(parts[i]));
    await cache.put(base + "catalogo.json", new Response(liveText || JSON.stringify(cat)));
    if (have) for (const p of have.pieces) if (!cat.pieces.some(q => q.file === p.file)) await cache.delete(base + p.file);
  }
  return { raw, catalog: cat, cached: fromCache };
}

/**
 * «Check for updates»: compares the installed catalogue with the one the site serves. `state`: `none` (nothing installed), `current`, `update` or `offline`.
 * Nothing is downloaded.
 */
export async function check(/** @type {Options} */ o = {}) {
  const { base, fetchFn, cs } = where(o);
  const installed = await cachedCatalog(await cs.open(CACHE), base);
  try {
    const res = await fetchFn(base + "catalogo.json", { cache: "no-store", signal: o.signal });
    if (!res.ok) throw err("normative.dl.err.http", { status: res.status });
    const available = parseCatalog(await res.text());
    const state = !installed ? "none" : installed.sha256 === available.sha256 ? "current" : "update";
    return { state, installed, available };
  } catch (_) {
    return { state: "offline", installed, available: null };
  }
}

/** The texts of the package: one key per outcome (the page shows them with `t()`). */
/** Texts the page shows; `io` and `missing` come from the desktop route (`normative/*`). */
export const KEYS = ["normative.dl.err.io", "normative.dl.err.missing", "normative.dl.check.current", "normative.dl.check.update", "normative.dl.check.offline", "normative.dl.check.none", "normative.dl.progress"];
