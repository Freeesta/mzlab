// @ts-check
// TPMINE-PRIVATE  The low-resolution Dig of mzFinder in Rust (crates/mzlab-finder-lr-wasm, wasm-feat/finder-lr/): imported by tpmine-worker.js for a series of
// unit-resolution full-scan files, each with its .mzidx in OPFS, a parent with a formula and the default transformations. The pool of feat-worker.js runs
// detection, alignment and the hypotheses of the parent (recipe "" = profile `lr`), then finder_lr_dig gives the JSON of api.run; detail(id) reads the XICs
// from the .mzidx. Every other case (a parent without formula) answers false and the worker goes on to Pyodide.

/** @type {any} */ let mod = null;
/** @type {any} */ let dig = null;                 // FinderLrDig of the last run
/** @type {string[]} */ let cols = [];             // names of its columns (their .mzidx for detail)

/**
 * What the worker lends: its URLs, its readers of OPFS, its pool and its detection cache.
 * @typedef {{ featUrl: (p: string) => string, base: () => string, manifest: () => Promise<any>, sha: (b: ArrayBuffer) => Promise<string>,
 *   pool: () => Promise<any>, idxFiles: (n: string[]) => Promise<File[] | null>, openIdx: (n: string[]) => Promise<any>,
 *   detect: (p: any, files: {name: string, path: string}[], idx: File[]) => Promise<Uint8Array[]>, held: Map<string, any> }} Ctx
 */

/** the finder_lr module of wasm-feat/manifest.json, checked against its SHA-256 @param {Ctx} c */
async function load(c) {
  if (mod) return mod;
  const m = (await c.manifest()).finder_lr;
  if (!m) return null;
  const bytes = await (await fetch(c.featUrl(m.wasm))).arrayBuffer();
  if (m.sha256 && await c.sha(bytes) !== m.sha256) throw new Error("the engine of mzFinder does not match its manifest: reload the page");
  const x = await import(c.featUrl(m.js));
  await x.default({ module_or_path: bytes });
  return (mod = x);
}

/** @param {string} text @param {number | null} frac @param {string} phase @param {any} [payload] */
const progress = (text, frac, phase, payload) => postMessage({ type: "progress", text, frac, phase, payload: payload || null });

/** the plan of a run, null when Rust does not serve it @param {Ctx} c @param {string[]} a */
async function plan(c, a) {
  const files = JSON.parse(a[0]);
  if (!files.length || files.some((/** @type {any} */ f) => !["full", "ms2", "mrm"].includes(f.kind))) return null;
  const x = await load(c);
  if (!x) return null;
  const p = JSON.parse(x.finder_lr_plan(a[0], a[1]));
  if (!p.rust) return null;
  const idx = await c.idxFiles(p.lc);
  if (p.mrm.some((/** @type {string} */ n) => !c.held.has(n))) return null;          // the mzML of an MRM file is read from the bytes the page sent
  return idx && (!p.ms2.length || await c.idxFiles(p.ms2)) ? { p, idx, x } : null;
}

/**
 * synchronous reader of the mzML of the MRM files (read(k, offset, length) as finder_lr_mrm wants it) over the Blobs or ArrayBuffers the page sent
 * @param {Map<string, any>} held @param {string[]} names
 */
function mzmlReader(held, names) {
  const src = names.map(n => held.get(n)), rd = typeof FileReaderSync === "function" ? new FileReaderSync() : null;
  const size = (/** @type {any} */ b) => b.size ?? b.byteLength;
  const read = (/** @type {number} */ k, /** @type {number} */ off, /** @type {number} */ len) => {
    const b = src[k];
    if (b instanceof Blob) {
      if (!rd) throw new Error("error.io");
      return new Uint8Array(rd.readAsArrayBuffer(b.slice(off, off + len)));
    }
    return new Uint8Array(b, off, len);
  };
  return { read, sizes: Float64Array.from(src, size), ends: new Float64Array(src.length) };
}

/**
 * api.run of a low-resolution series: the JSON string, or false when the run belongs to Pyodide.
 * @param {Ctx} c @param {string[]} a  [files, parent, settings, transformations]
 */
export async function run(c, a) {
  const got = await plan(c, a);
  if (!got) return false;
  const { p, idx, x } = got, t0 = performance.now(), pool = await c.pool();
  const files = p.lc.map((/** @type {string} */ n) => ({ name: n, path: "mzlab/idx/" + n + ".mzidx" })), paths = files.map((/** @type {any} */ f) => f.path);
  const timing = /** @type {Record<string, number>} */ ({}), lap = (/** @type {string} */ k, /** @type {number} */ t) => { timing[k] = Math.round((performance.now() - t) / 10) / 100; };
  let t = performance.now();
  progress(`Detecting the peaks of ${files.length} files...`, 0.02, "calibration");
  const stored = await c.detect(pool, files, idx); lap("feature", t); t = performance.now();
  progress("Generating candidates...", 0.4, "candidates");
  const rules = new Uint8Array(await (await fetch(c.base() + "mzfinder/regole_tp.json")).arrayBuffer());
  const h = await pool.hypotheses({ stored, paths, rules, parent: p.formula, parentRt: NaN, recipe: "" }); lap("candidates", t); t = performance.now();
  progress("Aligning the peaks of the series...", 0.6, "xic");
  const table = await pool.align(stored, paths, ""); lap("align", t); t = performance.now();
  dig = x.finder_lr_dig(a[0], a[1], a[2] || "{}", table, h.registry); cols = [...p.lc, ...p.ms2];
  let mrm = null;
  if (p.mrm.length) {
    const r = mzmlReader(c.held, p.mrm), rt = JSON.parse(a[2] || "{}").rt_tol_min;
    mrm = JSON.parse(x.finder_lr_mrm(a[0], r.read, r.sizes, r.ends, rt > 0 ? rt : 0.25));
  }
  progress("", null, "xic", JSON.parse(dig.live()));
  progress("Searching for unexpected ions...", 0.9, "unexpected");
  progress("Looking for MS2 scans...", 0.95, "ms2");
  progress("Computing the tables...", 0.97, "tables");
  lap("dig", t); lap("total", t0);
  const sum = dig.summary(JSON.stringify(timing));
  if (!mrm) return sum;
  const d = JSON.parse(sum);
  d.mrm = mrm;
  return JSON.stringify(d);
}

/** detail, frag_tree and frag_tree_compare of the last run (the XICs of detail from the .mzidx of its columns) @param {Ctx} c @param {string} fn @param {any[]} a */
export async function ofDig(c, fn, a) {
  if (!dig) throw new Error("no experiment in memory: run the analysis first");
  if (fn === "frag_tree") return dig.frag_tree();
  if (fn === "frag_tree_compare") return dig.frag_tree_compare(+a[0]);
  if (fn !== "detail") throw new Error("no high-resolution experiment in memory");
  const ix = await c.openIdx(cols);
  try { return dig.detail(+a[0], ix.read, ix.sizes); } finally { ix.close(); }
}
