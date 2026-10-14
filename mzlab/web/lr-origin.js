// @ts-check
// @ts-strict
// `api/origin` of the browser version: "where does this ion come from?", the evidence for the ion X against the candidate precursor P over
// the files of the session, computed by the Rust engine from the indexes (unit-resolution centroids). null = not served: high resolution,
// a part of a mixed file, a file without index.

/** @param {URLSearchParams} q  @param {{rb: any, rpc: (m: any) => Promise<any>, json: (o: any, status?: number) => any, items: any[] | null}} env */
export async function originRoute(q, { rb, rpc, json, items }) {
  if (!items || !(await rb.ready(items.map((_, k) => k)))) return null;
  const metas = items.map((it, k) => ({ slot: k, label: it.label, time: it.time ?? null, full: it.kind !== "mrm" && it.type !== "blank" }));
  const query = new URLSearchParams([...q].filter(([n]) => ["mz", "parent", "rt0", "rt1", "formula"].includes(n))).toString();
  const k = q.get("k") === null || q.get("k") === "" ? -1 : Number(q.get("k"));
  try {
    const r = await rpc({ op: "origin", items: JSON.stringify(metas), k, query });
    return r.info ? { status: 200, text: r.info } : null;
  } catch (e) {
    let j = null;
    try { j = JSON.parse(/** @type {any} */ (e).raw); } catch (_) { /* not an engine error */ }
    if (j && j.error_key) return json({ error: j.error_key === "err.origin.needFullScan" ? "full scan files are needed" : j.error_key, error_key: j.error_key, params: j.params || {} }, 400);
    throw e;
  }
}
