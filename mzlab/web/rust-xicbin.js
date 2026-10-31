// @ts-strict
// api/xicbin for rust-bridge.js: the XIC as doubles. Layout: 4 bytes = length of the JSON header {mz, tol, traces:[{k, n}]}, the header, zeros up to a multiple of 8, then per trace rt (n doubles) and y (n doubles).

/** @param {number} mz @param {number} tol @param {{k: number, rt: ArrayLike<number>, y: ArrayLike<number>}[]} list */
export const packXic = (mz, tol, list) => {
  const head = new TextEncoder().encode(JSON.stringify({ mz, tol, traces: list.map(t => ({ k: t.k, n: t.rt.length })) }));
  const at = (4 + head.length + 7) & ~7, buf = new ArrayBuffer(at + 16 * list.reduce((a, t) => a + t.rt.length, 0));
  new DataView(buf).setUint32(0, head.length, true);
  new Uint8Array(buf, 4, head.length).set(head);
  const all = new Float64Array(buf, at);
  let w = 0;
  for (const t of list) { all.set(t.rt, w); all.set(t.y, w + t.rt.length); w += 2 * t.rt.length; }
  return buf;
};

/** The files of the query with an index, straight from the worker's buffers; the others (no index) through the JSON of `api/xic`; null = the route is unavailable. */
export async function indexXicBin(/** @type {any} */ b, /** @type {URL} */ u, /** @type {number} */ level, /** @type {number} */ t0) {
  const { slots, rpc, recycle, stats } = b, q = u.searchParams;
  const ks = (q.get("k") || "").split(",").filter(Boolean).map(Number), mz = Number(q.get("mz") ?? NaN), tol = q.get("tol") === null || q.get("tol") === "" ? 0.35 : Number(q.get("tol"));
  if (!ks.length || !isFinite(mz) || !isFinite(tol) || !ks.every(k => Number.isInteger(k) && slots[k])) return null;
  await Promise.all(ks.map(k => slots[k].opening));
  if (!ks.every(k => slots[k] && slots[k].idx && !slots[k].bad)) {
    const r = await b.handle("GET", "/api/xic" + u.search);
    return r && r.text ? { status: 200, buf: packXic(mz, tol, JSON.parse(r.text).traces), ms: performance.now() - t0 } : r;
  }
  // the worker answers in order, so the requests go out together and the answers come back in the order of `ks`
  const rs = await Promise.all(ks.map(k => rpc({ op: "idxxicbin", slot: k, level, mz, tol })));
  let buf;
  try {
    if (rs.some(r => r.dead || r.error)) return null;
    const list = rs.map((r, i) => {
      const a = r.n ? new Float64Array(r.buf, 0, r.len) : new Float64Array(0);
      return { k: ks[i], rt: a.subarray(0, r.n), y: a.subarray(r.n, 2 * r.n) };
    });
    buf = packXic(mz, tol, list);
  } finally { for (const r of rs) if (r.buf) recycle(r.buf); }   // the worker's buffers go back once copied into the answer, also when one answer failed
  stats.rust++;
  return { status: 200, buf, ms: performance.now() - t0 };
}
