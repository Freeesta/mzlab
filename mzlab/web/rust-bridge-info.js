// @ts-check
// @ts-strict
// The routes of the bridge that read the head of a file or one answer per file from the indexes: `api/ms2tr`, `api/fileinfo` and the `api/chrom` of an MRM file.
// Returns { status, text, ms } or null (the route is unavailable: browser.js answers err.route.unavailable).

/** @param {string} path  @param {URL} u  @param {{slots: any[], rpc: (m: any) => Promise<any>, base: (n: string) => string, stats: any, t0: number}} env */
export async function infoRoute(path, u, { slots, rpc, base, stats, t0 }) {
  const q = u.searchParams;
  const ks = (q.get("k") || "").split(",").filter(Boolean).map(Number);
  if (!ks.length || !ks.every(k => Number.isInteger(k) && slots[k])) return null;
  await Promise.all(ks.map(k => slots[k].opening));
  if (!ks.every(k => slots[k] && slots[k].idx && !slots[k].bad)) return null;
  const done = text => { stats.rust++; return { status: 200, text, ms: performance.now() - t0 }; };
  // instrument and software from the head of the mzML, counts and m/z range from the index
  if (path === "/api/fileinfo") {
    const s = slots[ks[0]];
    if (ks.length !== 1 || !s.idx.scans || !s.blob) return null;
    const r = await rpc({ op: "fileinfo", slot: ks[0], name: base(s.name).split("#")[0], blob: s.blob });
    return r.dead || r.error || !r.info ? null : done(r.info);
  }
  // an MRM file: the chromatogram of the mzML of that kind (the m/z window, precursor and filter do not apply)
  if (path === "/api/chrom") {
    const s = slots[ks[0]];
    if (ks.length !== 1 || s.idx.scans || !s.blob) return null;
    const r = await rpc({ op: "mrmchrom", slot: ks[0], kind: q.get("kind") || "tic", blob: s.blob });
    return r.dead || r.error || !r.info ? null : done(r.info);
  }
  // ms2tr: the precursor > fragment pairs of the MS2 scans of each file
  const parts = [];
  for (const k of ks) {
    const r = await rpc({ op: "route", slot: k, path, query: u.search });
    if (r.dead || r.error || !r.info) return null;
    parts.push(`{"k":${k},"transitions":${r.info}}`);
  }
  return done(`{"files":[${parts.join(",")}]}`);
}
