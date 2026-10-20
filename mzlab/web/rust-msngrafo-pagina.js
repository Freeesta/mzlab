// @ts-check
// Page side of api/msn_grafo (rust-bridge.js): asks the worker of the engine for the graph (op "msngrafo", rust-msngrafo.js) and words the answer.
// Null = the route is unavailable (browser.js answers err.route.unavailable); an error of the module is a 400 with the usual JSON.

/**
 * @param {{ slots: any[], rpc: (msg: any) => Promise<any>, stats: { rust: number } }} c
 * @param {number} k @param {string} search the query string of the request @param {boolean} [esporta] the spectra as text (api/msn_grafo/export)
 * @returns {Promise<{ status: number, text: string } | null>}
 */
export async function grafoRoute({ slots, rpc, stats }, k, search, esporta = false) {
  const s = slots[k];
  if (!s) return null;
  await s.opening;
  if (s.bad || !s.idx || !s.blob) return null;
  const r = await rpc({ op: "msngrafo", slot: k, query: search, blob: s.blob, esporta });
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
  return { status: 200, text: r.info, type: f === "csv" ? "text/csv" : "text/plain" };
}
