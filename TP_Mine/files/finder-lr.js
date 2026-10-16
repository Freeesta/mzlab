// @ts-check
// TPMINE-PRIVATE  The low-resolution Dig of mzFinder in Rust (crates/mzlab-finder-lr-wasm, wasm-feat/finder-lr/): imported by tpmine-worker.js for a series of
// unit-resolution full-scan files, each with its .mzidx in OPFS, a parent (with a formula or only its m/z) and the default or the user's list of transformations. The pool of feat-worker.js runs
// detection, alignment and the hypotheses of the parent (recipe "" = profile `lr`), then finder_lr_dig gives the JSON of api.run; detail(id) reads the XICs
// from the .mzidx. Every other case (a parent without formula) answers false and the worker goes on to Pyodide.

/** @type {any} */ let mod = null;
/** @type {any} */ let dig = null;                 // FinderLrDig of the last run
/** @type {string[]} */ let cols = [];             // names of its columns (their .mzidx for detail)

/**
 * What the worker lends: its URLs, its readers of OPFS, its pool and its detection cache.
 * @typedef {{ featUrl: (p: string) => string, base: () => string, manifest: () => Promise<any>, sha: (b: ArrayBuffer) => Promise<string>,
 *   pool: () => Promise<any>, idxFiles: (n: string[]) => Promise<File[] | null>, openIdx: (n: string[]) => Promise<any>,
 *   finder: () => Promise<any>, detect: (p: any, files: {name: string, path: string}[], idx: File[]) => Promise<Uint8Array[]>, held: Map<string, any>,
 *   natives: Set<string>, check: () => void, ask: (what: string, name: string) => Promise<any>, idxBase: (n: string) => string }} Ctx
 */

/** the finder_lr module of wasm-feat/manifest.json, checked against its SHA-256 @param {Ctx} c */
async function load(c) {
  if (mod) return mod;
  const m = (await c.manifest()).finder_lr;
  if (!m) return null;
  const bytes = await (await fetch(c.featUrl(m.wasm))).arrayBuffer();
  if (m.sha256 && await c.sha(bytes) !== m.sha256) throw new Error("the engine of Tabui does not match its manifest: reload the page");
  const x = await import(c.featUrl(m.js));
  await x.default({ module_or_path: bytes });
  return (mod = x);
}

/** @param {string} text @param {number | null} frac @param {string} phase @param {any} [payload] */
const progress = (text, frac, phase, payload) => postMessage({ type: "progress", text, frac, phase, payload: payload || null });
/** a step of the run: the label and the fraction come from the engine (`finder_lr_progress`, phase weights in Rust), `done` of `total` files @param {any} x @param {string} phase @param {number} [done] @param {number} [total] */
const step = (x, phase, done = 0, total = 0) => {
  const s = JSON.parse(x.finder_lr_progress(phase, done, total));
  if (s) postMessage({ type: "progress", text: s.label, frac: s.frac, phase, done: s.done, total: s.total, payload: null });
};

/** the plan of a run, null when Rust does not serve it @param {Ctx} c @param {string[]} a */
async function plan(c, a) {
  const parsed = JSON.parse(a[0]), files = Array.isArray(parsed) ? parsed : parsed.files || [];        // a list, or {files, campioni} with the conditions of the page
  if (!files.length || files.some((/** @type {any} */ f) => !["full", "ms2", "mrm"].includes(f.kind))) return null;
  const x = await load(c);
  if (!x) return null;
  const p = JSON.parse(x.finder_lr_plan(a[0], a[1]));
  if (!p.rust) return null;
  const idx = await c.idxFiles(p.lc);
  if (p.mrm.some((/** @type {string} */ n) => !c.held.has(n))) return null;          // the mzML of an MRM file is read from the bytes the page sent (a sample of a maker's file: its traces, from the page)
  return idx && (!p.ms2.length || await c.idxFiles(p.ms2)) ? { p, idx, x } : null;
}

/**
 * synchronous reader of the mzML of the MRM files (read(k, offset, length) as finder_lr_mrm wants it) over the Blobs or ArrayBuffers the page sent; a sample of a maker's file
 * has no mzML: its size is -1 and `packed[k]` holds its SRM traces (`srm::impacchetta`, made by the engine of the page)
 * @param {Ctx} c @param {string[]} names
 */
async function mzmlReader(c, names) {
  const held = c.held, src = names.map(n => held.get(n)), rd = typeof FileReaderSync === "function" ? new FileReaderSync() : null;
  const packed = await Promise.all(names.map(async n => {
    if (!c.natives.has(n)) return null;
    const b = await c.ask("srm", n);
    if (!b) throw new Error(`cannot read the traces of ${n}`);
    return new Uint8Array(b);
  }));
  const size = (/** @type {any} */ b, /** @type {number} */ k) => packed[k] ? -1 : b.size ?? b.byteLength;
  const read = (/** @type {number} */ k, /** @type {number} */ off, /** @type {number} */ len) => {
    const b = src[k];
    if (b instanceof Blob) {
      if (!rd) throw new Error("error.io");
      return new Uint8Array(rd.readAsArrayBuffer(b.slice(off, off + len)));
    }
    return new Uint8Array(b, off, len);
  };
  return { read, sizes: Float64Array.from(src, size), ends: new Float64Array(src.length), packed };
}

/**
 * api.run of a low-resolution series: the JSON string, or false when Rust does not serve the series.
 * @param {Ctx} c @param {string[]} a  [files, parent, settings, transformations]
 */
export async function run(c, a) {
  const got = await plan(c, a);
  if (!got) return false;
  const { p, idx, x } = got, t0 = performance.now(), pool = await c.pool();
  const files = p.lc.map((/** @type {string} */ n) => ({ name: n, path: "mzlab/idx/" + c.idxBase(n) + ".mzidx" })), paths = files.map((/** @type {any} */ f) => f.path);
  const timing = /** @type {Record<string, number>} */ ({}), lap = (/** @type {string} */ k, /** @type {number} */ t) => { timing[k] = Math.round((performance.now() - t) / 10) / 100; };
  let t = performance.now();
  step(x, "calibration", 0, files.length);
  const prev = pool.onProgress;
  pool.onProgress = (/** @type {any} */ i) => { if (i.phase === "detect") step(x, "calibration", i.done, i.total); };      // file k of N, as the pool finishes them
  let stored;
  try { stored = await c.detect(pool, files, idx); } finally { pool.onProgress = prev; }
  lap("feature", t); t = performance.now(); c.check();
  step(x, "candidates");
  let registry = "";             // a parent known only by its m/z has no transformations to apply: the Dig keeps its row and the unexpected ions
  if (p.formula) {
    let rules = new Uint8Array(await (await fetch(c.base() + "mzfinder/regole_tp.json")).arrayBuffer());
    const tr = (a[3] || "").trim();
    if (tr) rules = (await c.finder()).finder_transformations(rules, tr);      // the user's list of transformations: its rows become rules of the hypotheses
    registry = (await pool.hypotheses({ stored, paths, rules, parent: p.formula, parentRt: NaN, recipe: "" })).registry;
  }
  lap("candidates", t); t = performance.now();
  c.check(); step(x, "xic");
  const table = await pool.align(stored, paths, ""); lap("align", t); t = performance.now(); c.check();
  const ix = await c.openIdx(p.lc);     // the XICs: the chromatographic check of the peaks, then the in-source verdicts of the lighter ions
  try { dig = x.finder_lr_dig(a[0], a[1], a[2] || "{}", table, registry, ix.read, ix.sizes); dig.isf(ix.read, ix.sizes); } finally { ix.close(); }
  cols = [...p.lc, ...p.ms2];
  let mrm = null;
  if (p.mrm.length) {
    const r = await mzmlReader(c, p.mrm), rt = JSON.parse(a[2] || "{}").rt_tol_min;
    mrm = JSON.parse(x.finder_lr_mrm(a[0], r.read, r.sizes, r.ends, r.packed.map(b => b || null), rt > 0 ? rt : 0.25));
  }
  progress("", null, "xic", JSON.parse(dig.live()));
  c.check(); step(x, "unexpected");
  step(x, "ms2");
  const mx = await c.openIdx(cols);      // the columns then the MS2 files: the matches of the MS2 experiments to the rows
  try { dig.ms2_link(mx.read, mx.sizes, JSON.stringify(JSON.parse(a[2] || "{}").ms2_overrides || [])); } finally { mx.close(); }
  c.check(); step(x, "tables");
  lap("dig", t); lap("total", t0);
  const sum = dig.summary(JSON.stringify(timing));
  if (!mrm) return sum;
  const d = JSON.parse(sum);
  d.mrm = mrm;
  return JSON.stringify(d);
}

/** detail, frag_tree and frag_tree_compare of the last run (the XICs of detail from the .mzidx of its columns), and ms2_relink: the MS2 matches with new corrections (a JSON array), as ms2_links/1 @param {Ctx} c @param {string} fn @param {any[]} a */
export async function ofDig(c, fn, a) {
  if (!dig) throw new Error("no experiment in memory: run the analysis first");
  if (!["detail", "frag_tree", "frag_tree_compare", "ms2_relink"].includes(fn)) throw new Error("no high-resolution experiment in memory");
  const ix = await c.openIdx(cols);        // the columns then the MS2 files: the tree and the XICs read their .mzidx
  try {
    if (fn === "frag_tree") return dig.frag_tree(ix.read, ix.sizes);
    if (fn === "ms2_relink") return dig.relink(ix.read, ix.sizes, typeof a[0] === "string" ? a[0] : JSON.stringify(a[0] || []));
    return fn === "detail" ? dig.detail(+a[0], ix.read, ix.sizes) : dig.frag_tree_compare(+a[0], ix.read, ix.sizes);
  } finally { ix.close(); }
}
