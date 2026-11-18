// @ts-check
// @ts-strict
// The routes of the bridge that read the head of a file or one answer per file from the indexes: `api/ms2tr`, `api/fileinfo`, `api/msn_grafo` (the graph) and `api/msn_grafo/export` (its spectra as text), `api/msnlib/add` and `api/msnlib/search` (POST) and the `api/chrom` of an MRM file.
// `api/manint` (the table of the integrations drawn by hand: the api/mrm transitions of the files measured again by the engine, manint.rs) and
// `api/manvalley` (where one of them splits at its valleys).
// Returns { status, text, ms } or null (the route is unavailable: browser.js answers err.route.unavailable).
import { cercaRoute, grafoRoute, libreriaRoute } from "./rust-msngrafo-pagina.js";

/** @param {string} path  @param {URL} u  @param {{slots: any[], rpc: (m: any) => Promise<any>, base: (n: string) => string, stats: any, t0: number, init?: any}} env */
export async function infoRoute(path, u, { slots, rpc, base, stats, t0, init }) {
  if (path === "/api/manint" || path === "/api/manvalley" || path === "/api/manfollow" || path === "/api/manauto") return manintRoute(u, { slots, rpc, stats, t0 }, path.slice(8));
  if (path === "/api/mantraces") return tracesRoute({ rpc, stats, t0 }, init);   // POST {files, windows, sums}: the traces the page supplies (chromatogram, XIC, DAD)
  if (path === "/api/msnlib/add") {                               // add an entry to the msnlib/1 library of the user (POST)
    const r = await libreriaRoute({ slots, rpc, stats }, init);
    return r && { ...r, ms: performance.now() - t0 };
  }
  if (path === "/api/msnlib/search") {                                  // search the msnlib/1 library of the user for the graph of a file (POST)
    const r = await cercaRoute({ slots, rpc, stats }, init);
    return r && { ...r, ms: performance.now() - t0 };
  }
  if (path === "/api/msn_grafo" || path === "/api/msn_grafo/export") {                                     // the fragmentation graph (the .wasm msngrafo, rust-msngrafo-pagina.js)
    const r = await grafoRoute({ slots, rpc, stats }, Number(u.searchParams.get("k")), u.search, path.endsWith("/export"));
    return r && { ...r, ms: performance.now() - t0 };
  }
  const q = u.searchParams;
  const ks = (q.get("k") || "").split(",").filter(Boolean).map(Number);
  if (!ks.length || !ks.every(k => Number.isInteger(k) && slots[k])) return null;
  /** @type {any} */ let gs = []; try { gs = JSON.parse(q.get("g") || "[]"); } catch (_) { return null; }
  if (!Array.isArray(gs)) return null;
  await Promise.all(ks.map(k => slots[k].opening));
  if (!ks.every(k => slots[k] && slots[k].idx && !slots[k].bad)) return null;
  const done = text => { stats.rust++; return { status: 200, text, ms: performance.now() - t0 }; };
  if (path === "/api/masscal") {                                       // the mass error of the background ions of one file (massacc/1, the .wasm proofs, rust-masscal.js)
    if (ks.length !== 1) return null;
    const num = (n, d) => { const v = Number(q.get(n)); return q.get(n) && Number.isFinite(v) && v >= 0 ? v : d; };
    const params = JSON.stringify({ search_ppm: num("ppm", 20), min_intensity: num("min_intensity", 1e4), min_refs: 3, model: "linear_mz_rt" });
    const r = await rpc({ op: "masscal", slot: ks[0], name: slots[ks[0]].name, params });
    if (r.dead) return null;
    if (r.error) {
      let e = null;
      try { e = JSON.parse(r.error); } catch (_) { /* not an error of the module */ }
      if (!e || !e.error_key) e = { error_key: "err.load.failed", params: { fase: "decode", raw: String(r.error).slice(0, 600) } };
      return { status: 400, text: JSON.stringify({ error: e.error_key, error_key: e.error_key, params: e.params || {} }), ms: performance.now() - t0 };
    }
    return done(r.info);
  }
  // instrument and software from the head of the mzML, counts and m/z range from the index
  if (path === "/api/fileinfo") {
    const s = slots[ks[0]];
    if (ks.length !== 1 || !s.idx.scans || !s.blob) return null;
    const r = await rpc({ op: "fileinfo", slot: ks[0], name: base(s.name).split("#")[0], blob: s.blob });
    return r.dead || r.error || !r.info ? null : done(r.info);
  }
  // an MRM file: the chromatogram of the mzML of that kind (the m/z window, precursor and filter do not apply); the DAD (`pda`, with or without a band in nm)
  // is not read from the mzML's chromatograms: null, so lr-method.js answers it (mrm_chrom would give the TWC for any band)
  if (path === "/api/chrom") {
    const s = slots[ks[0]];
    if (ks.length !== 1 || s.idx.scans || !s.blob || q.get("kind") === "pda") return null;
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

/** api/mantraces (POST `{files, windows, sums}`, `files` = `[{"name", "conc"?, "conc_unit"?, "transitions": [{"rt", "y", "panel", "label", "unit"}]}]`): the same table as api/manint
 * (manint::table_text), for traces that are not the file's transitions; "E" + JSON = a 400.
 * @param {{rpc: (m: any) => Promise<any>, stats: any, t0: number}} env  @param {any} init the init of the fetch */
async function tracesRoute({ rpc, stats, t0 }, init) {
  /** @type {any} */ let b = null; try { b = JSON.parse(String(init && init.body)); } catch (_) { return null; }
  if (!b || typeof b.files !== "string" || typeof b.windows !== "string") return null;
  const r = await rpc({ op: "manint", files: b.files, windows: b.windows, sums: !!b.sums });
  if (r.dead || r.error || !r.info) return null;
  stats.rust++;
  const bad = r.info[0] === "E";
  return { status: bad ? 400 : 200, text: bad ? r.info.slice(1) : r.info, type: bad ? "application/json" : "text/csv", ms: performance.now() - t0 };
}

/** api/manint?k=..&w=JSON windows (manint::table_text, `file` = the place in k)[&sums=1][&tm=JSON minutes of the files, null = none]: the `mrm` transitions of every slot, then the op `manint` of the
 * worker; "E" + JSON (a window that names no channel, an unknown baseline) = a 400. api/manvalley?k=..&w=JSON window&depth=D: the op `valleys`
 * (manint::valleys_text), `{"splits":[MIN,...]}`. api/manfollow?k=..&w=JSON window: the op `follow` (manint::follow_text), where the window lands in the other files.
 * api/manauto?k=..&p=JSON params: the op `auto` (manint::auto_text), every peak of every file grouped by time.
 * @param {URL} u  @param {{slots: any[], rpc: (m: any) => Promise<any>, stats: any, t0: number}} env  @param {string} [what] "int" | "valley" | "follow" | "auto" */
async function manintRoute(u, { slots, rpc, stats, t0 }, what = "int") {
  const valleys = what === "valley", follow = what === "follow", auto = what === "auto";
  const q = u.searchParams, ks = (q.get("k") || "").split(",").filter(Boolean).map(Number), windows = q.get("w") || "[]";
  if (!ks.length || !ks.every(k => Number.isInteger(k) && slots[k])) return null;
  await Promise.all(ks.map(k => slots[k].opening));
  if (!ks.every(k => slots[k] && slots[k].idx && !slots[k].bad && slots[k].blob)) return null;
  const rs = await Promise.all(ks.map(k => rpc({ op: "mrm", slot: k, blob: slots[k].blob })));
  if (rs.some(r => r.dead || r.error || !r.info)) return null;
  // `tm`: the time point of each file in minutes (null = none), as the table of the files has it; without it the engine reads it from the name
  let tm = /** @type {any} */ (null); try { tm = JSON.parse(q.get("tm") || "null"); } catch (_) { /* the names decide */ }
  const timeOf = (/** @type {number} */ i) => Array.isArray(tm) && tm.length === ks.length ? `,"time":${typeof tm[i] === "number" && Number.isFinite(tm[i]) ? tm[i] : "null"}` : "";
  const files = `[${ks.map((k, i) => `{"name":${JSON.stringify(slots[k].name)}${timeOf(i)},"transitions":${rs[i].info}}`).join(",")}]`;
  const r = await rpc(valleys ? { op: "valleys", files, window: windows, depth: Number(q.get("depth") || "NaN") }
    : auto ? { op: "auto", files, params: q.get("p") || "{}" } : follow ? { op: "follow", files, window: windows } : { op: "manint", files, windows, sums: q.get("sums") === "1" });
  if (r.dead || r.error || !r.info) return null;
  stats.rust++;
  const bad = r.info[0] === "E";
  return { status: bad ? 400 : 200, text: bad ? r.info.slice(1) : r.info, type: bad || valleys || follow || auto ? "application/json" : "text/csv", ms: performance.now() - t0 };
}
