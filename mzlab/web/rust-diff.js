// @ts-strict
// `ref=` (the difference from a reference file) on api/chrom, api/xic and api/mrm for rust-bridge.js: the traces come from the indexes of the
// files (the MRM transitions from their mzML), the subtraction is Rust's. Returns { status, text } or null when Python must answer.
export async function diffRoute({ slots, rpc }, path, u) {
  const q = u.searchParams, ref = Number(q.get("ref"));
  const ks = (q.get("k") || "").split(",").filter(Boolean).map(Number), all = [...ks, ref];
  if (!Number.isInteger(ref) || !all.every(k => Number.isInteger(k) && slots[k])) return null;
  await Promise.all(all.map(k => slots[k].opening));
  if (!all.every(k => slots[k] && slots[k].idx && !slots[k].bad)) return null;
  let r;
  if (path === "/api/mrm") {
    if (!all.every(k => slots[k].blob)) return null;
    const one = async k => { const t = await rpc({ op: "mrm", slot: k, blob: slots[k].blob }); return t.dead || t.error || !t.info ? null : t.info; };
    const parts = [];
    for (const k of ks) { const t = await one(k); if (t === null) return null; parts.push(`{"k":${k},"transitions":${t}}`); }
    const rt = await one(ref);
    if (rt === null) return null;
    r = await rpc({ op: "mrmdiff", files: `[${parts.join(",")}]`, ref: rt, query: u.search });
  } else r = await rpc({ op: "diff", path, query: u.search });
  if (r.dead || r.error || !r.info) return null;
  return r.info[0] === "E" ? { status: 400, text: r.info.slice(1) } : { status: 200, text: r.info };
}
