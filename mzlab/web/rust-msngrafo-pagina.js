// @ts-check
// Page side of api/msn_grafo and api/msnlib/aggiungi (rust-bridge.js): asks the worker of the engine for the graph or the library (op "msngrafo", rust-msngrafo.js) and words the answer.
// Null = the route is unavailable (browser.js answers err.route.unavailable); an error of the module is a 400 with the usual JSON.

/**
 * @param {{ slots: any[], rpc: (msg: any) => Promise<any>, stats: { rust: number } }} c
 * @param {number} k @param {string} search the query string of the request @param {boolean} [esporta] the spectra as text (api/msn_grafo/export)
 * @param {{ libreria: string, molecola: string } | null} [lib] the entry to add to the library (api/msnlib/aggiungi) or the library to search (`cerca`)
 * @param {boolean} [cerca] search the library instead of adding to it (api/msnlib/cerca)
 * @returns {Promise<{ status: number, text: string, type?: string, disposition?: string } | null>}
 */
export async function grafoRoute({ slots, rpc, stats }, k, search, esporta = false, lib = null, cerca = false) {
  const s = slots[k];
  if (!s) return null;
  await s.opening;
  if (s.bad || !s.idx || !s.blob) return null;
  const r = await rpc({ op: "msngrafo", slot: k, query: search, blob: s.blob, esporta, libreria: lib ? lib.libreria : undefined, cerca, molecola: lib ? lib.molecola : undefined });
  if (r.dead) return null;
  if (r.error) {
    /** @type {any} */ let e = null;
    try { e = JSON.parse(r.error); } catch (_) { /* not an error of the module */ }
    if (!e || !e.error_key) return null;
    return { status: 400, text: JSON.stringify({ error: e.error_key, error_key: e.error_key, params: e.params || {} }) };
  }
  stats.rust++;
  if (!esporta) return { status: 200, text: r.info };
  const f = new URLSearchParams(search).get("formato");
  const ext = f === "csv" || f === "mgf" || f === "msp" ? f : "txt";
  return { status: 200, text: r.info, type: f === "csv" ? "text/csv" : "text/plain", disposition: `attachment; filename="msn-graph.${ext}"` };
}

/**
 * POST api/msnlib/aggiungi: the body is the flat JSON `{libreria, molecola_id, k, ...the parameters of the graph}`; the answer `{libreria, doppione}`.
 * @param {{ slots: any[], rpc: (msg: any) => Promise<any>, stats: { rust: number } }} c @param {any} init the init of the fetch
 */
export async function libreriaRoute(c, init) {
  /** @type {any} */ let b = null;
  try { b = JSON.parse(String(init && init.body)); } catch (_) { /* answered below */ }
  if (!b || typeof b !== "object") return { status: 400, text: JSON.stringify({ error: "err.msnlib.bad", error_key: "err.msnlib.bad", params: {} }) };
  const { libreria, molecola_id: molecola, ...rest } = b;
  const q = new URLSearchParams();
  for (const [n, v] of Object.entries(rest)) q.set(n, typeof v === "object" ? JSON.stringify(v) : String(v));
  return grafoRoute(c, Number(rest.k), "?" + q.toString(), false, { libreria: String(libreria || ""), molecola: String(molecola || "") });
}

/**
 * POST api/msnlib/cerca: the body is the flat JSON `{libreria, k, ...the parameters of the graph, ppm_prec, tol_mz, min_prove, top, delta_max}`; the answer is `msncerca/1`.
 * @param {{ slots: any[], rpc: (msg: any) => Promise<any>, stats: { rust: number } }} c @param {any} init the init of the fetch
 */
export async function cercaRoute(c, init) {
  /** @type {any} */ let b = null;
  try { b = JSON.parse(String(init && init.body)); } catch (_) { /* answered below */ }
  if (!b || typeof b !== "object") return { status: 400, text: JSON.stringify({ error: "err.msnlib.bad", error_key: "err.msnlib.bad", params: {} }) };
  const { libreria, ...rest } = b;
  const q = new URLSearchParams();
  for (const [n, v] of Object.entries(rest)) q.set(n, typeof v === "object" ? JSON.stringify(v) : String(v));
  return grafoRoute(c, Number(rest.k), "?" + q.toString(), false, { libreria: String(libreria || ""), molecola: "" }, true);
}
