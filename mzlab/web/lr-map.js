// @ts-check
// The map routes (`api/map`, `api/mapsnap`, `api/mappunti`, `api/mappunti3d`) of the flow without Pyodide: the Rust engine answers from the
// indexes of the files of the session (unit-resolution centroids). null = Python's: high resolution, a profile file, a file without index,
// or a parameter that Python must reject with its own error.

/** @param {string} path  @param {URLSearchParams} q  @param {{rb: any, rpc: (m: any) => Promise<any>, items: any[] | null, hr: boolean}} env */
export async function mapRoute(path, q, { rb, rpc, items, hr }) {
  if (!items || hr || !(await rb.ready(items.map((_, k) => k)))) return null;
  const metas = items.map((it, k) => ({ slot: k, full: it.kind !== "mrm" && it.type !== "blank" }));
  const r = await rpc({ op: "map", path, items: JSON.stringify(metas), query: q.toString() });
  return r.info ? { status: 200, text: r.info } : null;
}
