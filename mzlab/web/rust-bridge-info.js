// @ts-check
// The routes of the bridge that read the head of a file or one answer per file from the indexes: `api/ms2tr` and `api/fileinfo`.
// Returns { status, text, ms } or null (Pyodide answers).

/** @param {string} path  @param {URL} u  @param {{slots: any[], rpc: (m: any) => Promise<any>, base: (n: string) => string, stats: any, t0: number}} env */
export async function infoRoute(path, u, { slots, rpc, base, stats, t0 }) {
  const q = u.searchParams;
  const ks = (q.get("k") || "").split(",").filter(Boolean).map(Number);
  if (!ks.length || !ks.every(k => Number.isInteger(k) && slots[k])) return null;
  await Promise.all(ks.map(k => slots[k].opening));
  if (!ks.every(k => slots[k] && slots[k].idx && !slots[k].bad)) return null;
  const done = text => { stats.rust++; return { status: 200, text, ms: performance.now() - t0 }; };
  if (path === "/api/fileinfo") {                                       // instrument and software from the head of the mzML, counts and m/z range from the index
    const s = slots[ks[0]];
    if (ks.length !== 1 || !s.idx.scans || !s.blob) return null;
    const r = await rpc({ op: "fileinfo", slot: ks[0], name: base(s.name).split("#")[0], blob: s.blob });
    return r.dead || r.error || !r.info ? null : done(r.info);
  }
  const parts = [];                                                      // ms2tr: the precursor > fragment pairs of the MS2 scans of each file
  for (const k of ks) {
    const r = await rpc({ op: "route", slot: k, path, query: u.search });
    if (r.dead || r.error || !r.info) return null;
    parts.push(`{"k":${k},"transitions":${r.info}}`);
  }
  return done(`{"files":[${parts.join(",")}]}`);
}
