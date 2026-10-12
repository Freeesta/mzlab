// @ts-check
// The map routes (`api/map`, `api/mapsnap`, `api/mappunti`, `api/mappunti3d`) of the flow without Pyodide: the Rust engine answers from the
// indexes of the files of the session (unit-resolution centroids). null = Python's: high resolution, a profile file, a file without index,
// or a parameter that Python must reject with its own error.

/** @param {string} path  @param {URLSearchParams} q  @param {{rb: any, rpc: (m: any) => Promise<any>, items: any[] | null, hr: boolean}} env */
async function mapRoute(path, q, { rb, rpc, items, hr }) {
  if (!items || hr || !(await rb.ready(items.map((_, k) => k)))) return null;
  const metas = items.map((it, k) => ({ slot: k, full: it.kind !== "mrm" && it.type !== "blank" }));
  const r = await rpc({ op: "map", path, items: JSON.stringify(metas), query: q.toString() });
  return r.info ? { status: 200, text: r.info } : null;
}

/** `api/subxic` (the combined XIC of the sub-formulas of an ion) of a high-resolution survey, from its index. null = Python's. @param {URLSearchParams} q  @param {{rb: any, rpc: (m: any) => Promise<any>}} env */
async function subxicRoute(q, { rb, rpc }) {
  const k = Number(q.get("k"));
  if (!Number.isInteger(k) || !(await rb.ready([k]))) return null;
  const r = await rpc({ op: "subxic", slot: k, k, query: q.toString() });
  return r.info ? { status: 200, text: r.info } : null;
}

/** The routes of the Rust engine that need an index or no file at all: undefined = not one of them, null = Python's, else { status, text }.
 * @param {string} path  @param {URLSearchParams} q  @param {{rb: any, rpc: (m: any) => Promise<any>, items: any[] | null, hr: boolean}} env */
export async function engineRoute(path, q, env) {
  if (path === "/api/composition") {                            // the formulas that fit an m/z: no file, any session
    const r = (await env.rpc({ op: "lr_composition", query: "?" + q.toString() })).info;
    return r ? { status: 200, text: r } : null;
  }
  if (path === "/api/subxic") return await subxicRoute(q, env);
  if (/^\/api\/map(snap|punti|punti3d)?$/.test(path)) return await mapRoute(path, q, env);
  return undefined;
}
