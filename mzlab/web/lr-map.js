// @ts-check
// @ts-strict
// The map routes (`api/map`, `api/mapsnap`, `api/mappunti`, `api/mappunti3d`) of the browser version: the Rust engine answers from the
// indexes of the files of the session (unit or high resolution, centroids or profile points). null = not served: a file without index,
// or a parameter the reference would reject.

/** @param {string} path  @param {URLSearchParams} q  @param {{rb: any, rpc: (m: any) => Promise<any>, items: any[] | null}} env */
async function mapRoute(path, q, { rb, rpc, items }) {
  if (!items || !(await rb.ready(items.map((_, k) => k)))) return null;
  const metas = items.map((it, k) => ({ slot: k, full: it.kind !== "mrm" && it.type !== "blank" }));
  const r = await rpc({ op: "map", path, items: JSON.stringify(metas), query: q.toString() });
  return r.info ? { status: 200, text: r.info } : null;
}

/** `api/subxic` (the combined XIC of the sub-formulas of an ion) of a high-resolution survey, from its index. null = not served. @param {URLSearchParams} q  @param {{rb: any, rpc: (m: any) => Promise<any>}} env */
async function subxicRoute(q, { rb, rpc }) {
  const k = Number(q.get("k"));
  if (!Number.isInteger(k) || !(await rb.ready([k]))) return null;
  const r = await rpc({ op: "subxic", slot: k, k, query: q.toString() });
  return r.info ? { status: 200, text: r.info } : null;
}

/** `api/qaqc` (QA/QC measures of one ion over the series) from the indexes of the files. null = not served. @param {URLSearchParams} q  @param {{rb: any, rpc: (m: any) => Promise<any>}} env */
async function qaqcRoute(q, { rb, rpc }) {
  const ks = (q.get("k") || "").split(",").filter(Boolean).map(Number);
  if (!ks.length || !ks.every(k => Number.isInteger(k) && k >= 0) || !(await rb.ready(ks))) return null;
  const r = (await rpc({ op: "qaqc", query: q.toString() })).info;
  if (!r) return null;
  return r[0] === "E" ? { status: 400, text: r.slice(1) } : { status: 200, text: r };
}

/** The routes of the Rust engine that need an index or no file at all: undefined = not one of them, null = not served, else { status, text }.
 * @param {string} path  @param {URLSearchParams} q  @param {{rb: any, rpc: (m: any) => Promise<any>, items: any[] | null}} env */
export async function engineRoute(path, q, env) {
  if (path === "/api/composition") {                            // the formulas that fit an m/z: no file, any session
    const r = (await env.rpc({ op: "lr_composition", query: "?" + q.toString() })).info;
    return r ? { status: 200, text: r } : null;
  }
  if (path === "/api/perf") return { status: 200, text: '{"files":[]}' };                  // the ?perf meter lists how Python read each file: Python reads none here
  if (path === "/api/contaminants") return { status: 200, text: await (await fetch(new URL("contaminanti.json", import.meta.url))).text() };   // a built-in list: data, not a calculation
  if (path === "/api/subxic") return await subxicRoute(q, env);
  if (path === "/api/qaqc") return await qaqcRoute(q, env);
  if (/^\/api\/map(snap|punti|punti3d|tiles)?$/.test(path)) return await mapRoute(path, q, env);
  return undefined;
}

/** The POST routes of the statistics (`api/stat/profile|blank|reference|pca`): the Rust engine from the table the page sends. null = not served.
 * @param {string} path  @param {string} body  @param {{rpc: (m: any) => Promise<any>}} env */
export async function statRoute(path, body, { rpc }) {
  if (path.startsWith("/api/campioni/")) return { status: 200, text: (await rpc({ op: "lr_campioni", cop: path.slice(14), body })).info };   // roles and times of the files (campioni/2): proponi, valida, ...
  if (!/^\/api\/stat\/(profile|blank|reference|pca)$/.test(path)) return undefined;
  const r = (await rpc({ op: "lr_stat", path: path.slice(1), body })).info;
  if (!r) return null;
  return r[0] === "E" ? { status: 400, text: r.slice(1) } : { status: 200, text: r };
}
