// @ts-strict
// Page side of api/msn_grafo and api/msnlib/add (rust-bridge.js): asks the worker of the engine for the graph or the library (op "msngrafo", rust-msngrafo.js) and words the answer.
// Null = the route is unavailable (browser.js answers err.route.unavailable); an error of the module is a 400 with the usual JSON.

/** Each `progress` message of the worker (phase load|convert|index|graph, frac of the whole bar) is told to the page: the tab shows the bar and watches for a stall. @param {{ phase: string, frac: number, done?: number, total?: number }} p */
const avanza = p => { try { window.dispatchEvent(new CustomEvent("mzlab-msngrafo", { detail: { phase: p.phase, frac: p.frac, done: p.done, total: p.total } })); } catch (_) { /* no window */ } };

/**
 * @param {{ slots: any[], rpc: (msg: any, onp?: (p: any) => void) => Promise<any>, stats: { rust: number } }} c
 * @param {number} k @param {string} search the query string of the request @param {boolean} [asExport] the spectra as text (api/msn_grafo/export)
 * @param {{ library: string, molecule: string } | null} [lib] the entry to add to the library (api/msnlib/add) or the library to search (`library_search`)
 * @param {boolean} [librarySearch] search the library instead of adding to it (api/msnlib/search)
 * @returns {Promise<{ status: number, text: string, type?: string, disposition?: string } | null>}
 */
export async function grafoRoute({ slots, rpc, stats }, k, search, asExport = false, lib = null, librarySearch = false) {
  const s = slots[k];
  if (!s) return null;
  await s.opening;
  if (s.bad || !s.idx || !s.blob) return null;
  const r = await rpc({ op: "msngrafo", slot: k, query: search, blob: s.blob, export: asExport, library: lib ? lib.library : undefined, library_search: librarySearch, molecule: lib ? lib.molecule : undefined }, avanza);
  if (r.dead) return null;
  if (r.error) {
    /** @type {any} */ let e = null;
    try { e = JSON.parse(r.error); } catch (_) { /* not an error of the module */ }
    if (!e || !e.error_key) e = { error_key: "err.load.failed", params: { fase: "decode", raw: String(r.error).slice(0, 600) } };      // never silence: an error of the worker that is no keyed error of the module still reaches the tab with its text
    return { status: 400, text: JSON.stringify({ error: e.error_key, error_key: e.error_key, params: e.params || {} }) };
  }
  stats.rust++;
  if (!asExport) return { status: 200, text: r.info };
  const f = new URLSearchParams(search).get("format");
  const ext = f === "csv" || f === "mgf" || f === "msp" ? f : "txt";
  return { status: 200, text: r.info, type: f === "csv" ? "text/csv" : "text/plain", disposition: `attachment; filename="msn-graph.${ext}"` };
}

/**
 * POST api/msnlib/add: the body is the flat JSON `{library, molecule_id, k, ...the parameters of the graph}`; the answer `{library, duplicate}`.
 * @param {{ slots: any[], rpc: (msg: any) => Promise<any>, stats: { rust: number } }} c @param {any} init the init of the fetch
 */
export async function libreriaRoute(c, init) {
  /** @type {any} */ let b = null;
  try { b = JSON.parse(String(init && init.body)); } catch (_) { /* answered below */ }
  if (!b || typeof b !== "object") return { status: 400, text: JSON.stringify({ error: "err.msnlib.bad", error_key: "err.msnlib.bad", params: {} }) };
  const { library, molecule_id: molecule, ...rest } = b;
  const q = new URLSearchParams();
  for (const [n, v] of Object.entries(rest)) q.set(n, typeof v === "object" ? JSON.stringify(v) : String(v));
  return grafoRoute(c, Number(rest.k), "?" + q.toString(), false, { library: String(library || ""), molecule: String(molecule || "") });
}

/**
 * POST api/msnlib/search: the body is the flat JSON `{library, k, ...the parameters of the graph, precursor_ppm, tol_mz, min_evidence, top, delta_max}`; the answer is `msn-search/1`.
 * @param {{ slots: any[], rpc: (msg: any) => Promise<any>, stats: { rust: number } }} c @param {any} init the init of the fetch
 */
export async function cercaRoute(c, init) {
  /** @type {any} */ let b = null;
  try { b = JSON.parse(String(init && init.body)); } catch (_) { /* answered below */ }
  if (!b || typeof b !== "object") return { status: 400, text: JSON.stringify({ error: "err.msnlib.bad", error_key: "err.msnlib.bad", params: {} }) };
  const { library, ...rest } = b;
  const q = new URLSearchParams();
  for (const [n, v] of Object.entries(rest)) q.set(n, typeof v === "object" ? JSON.stringify(v) : String(v));
  return grafoRoute(c, Number(rest.k), "?" + q.toString(), false, { library: String(library || ""), molecule: "" }, true);
}
