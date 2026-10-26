// @ts-strict
// A phase the memory refuses (`error.featMemory`) whose estimate fits in the high budget of the engine of the page (`MZLAB_RUST.choice.high_mb`, or the level it was lowered to) asks
// the user once (`askMore({ fase, stima_mb, budget_mb, libero_mb, high_mb })` resolves true = use more memory) and runs again with that budget, for the rest of the run. Without an
// answer, or beyond the high budget, the refusal goes on with its numbers.
async function withMore(pool, askMore, raised, run) {
  try { return await run(); } catch (e) {
    const p = e && e.key === "error.featMemory" && e.params, rb = globalThis.MZLAB_RUST, high = rb ? rb.budgetMb || (rb.choice && rb.choice.high_mb) || 0 : 0, need = p ? Number(p.stima_mb) : NaN;
    if (!p || !askMore || !pool.setBudget || !(need <= high) || !(high > Number(p.budget_mb))) throw e;
    if (!await askMore({ fase: p.fase, stima_mb: need, budget_mb: Number(p.budget_mb), libero_mb: Number(p.libero_mb), high_mb: high })) throw e;
    raised.push(pool.setBudget(high));
    return run();
  }
}

// The cache of the detections: the .mzfeatf of a file is kept in OPFS as `mzlab/feat/cache/<sha256 of the .mzidx>.<fingerprint of the detection>.mzfeatf` (`file_key` of the .wasm), so
// a run detects only the files it has not seen with these numbers, whatever `save` says. An entry is used when `check_stored` agrees with its key and the name of its source is the file's;
// one of a newer format (`error.newerFormat`) is neither used, nor overwritten, nor deleted; after a run that wrote entries the oldest ones go until the folder is within CACHE_CAP.
const CACHE_DIR = "mzlab/feat/cache", CACHE_CAP = 512 * 1048576;
async function detectWithCache(pool, files, recipe, unt) {
  const which = unt ? "untargeted" : "feat", path = key => CACHE_DIR + "/" + key + ".mzfeatf", stored = new Array(files.length);
  const keys = await Promise.all(files.map(f => pool.op(which, { op: "file_key", path: f.path, name: f.name, recipe }).then(r => r.text, e => { if (e.fatal || e.key === "tpmine.feat.cancelled") throw e; return ""; })));   // an error of the recipe is the detection's to report
  await Promise.all(files.map(async (f, i) => {
    let b; try { b = keys[i] && await pool.fs.read(path(keys[i])); } catch (_) { return; }
    if (!b) return;
    try { if ((await pool.op(which, { op: "check_stored", key: keys[i], bytes: b })).text === f.name) stored[i] = b; }
    catch (e) { if (e.fatal) throw e; if (e.key === "error.newerFormat") keys[i] = ""; }
  }));
  const miss = files.map((_, i) => i).filter(i => !stored[i]);
  if (miss.length) {
    const got = await pool[unt ? "detectAllUnt" : "detectAll"](miss.map(i => files[i]), recipe);
    for (let k = 0; k < miss.length; k++) { stored[miss[k]] = got[k]; if (keys[miss[k]]) { try { await pool.fs.write(path(keys[miss[k]]), got[k].slice()); } catch (_) { /* no room: the run goes on */ } } }
    try { await pool.fs.trim(CACHE_DIR, CACHE_CAP); } catch (_) { /* checked at the next run */ }
  }
  return { stored, cached: files.length - miss.length };
}
/** empties the cache of the detections; resolves with the bytes freed */
export async function clearDetectCache(pool) { return (await pool.fs.trim(CACHE_DIR, 0)).freed; }

// Runs the pipeline of mzFinder on a series of .mzidx files in OPFS with a pool of feat-worker.js (see docs/agenti/mzfeat-wasm.md):
//   detect  = L1-L3 of every file, in parallel (one job per file); a file already detected with these numbers comes from the cache of OPFS (`row.cached` = how many)
//   align   = L4-L6 of the series (alignment, gap filling) in one worker
//   hypotheses = L7 in a worker of its own, only with a parent; proofs = L8-L9 (graph of the proofs) in another one
// Resolves with { stored, table, evid, tried, found, registry, timings, degraded }; `timings` has one row per phase { phase, ms, files?, cached?, perFileMs? }.
// The .mzfeatf and the .mzfeat are written to OPFS (`mzlab/feat/`) by the archive worker when `save` is true, each as soon as its phase ends, so a phase that the
// memory refuses (`error.featMemory`) leaves the earlier ones saved: the hypotheses or the proofs refused give the table without proofs and the warning
// `tpmine.feat.degraded` (`degraded` = the phase), the detection or the alignment refused stop the run with the error (and `params.files`).
export async function runFeatPipeline(pool, { files, recipe, parent, save = false, onPhase = _ => {}, askMore = globalThis.mzlabAskMore }) {
  const raised = [], more = run => withMore(pool, askMore, raised, run);      // the budget the pool had is given back at the end of the run
  const timings = [], t00 = performance.now();
  const phase = async (name, fn) => {
    onPhase({ phase: name, state: "start" });
    const t0 = performance.now(), r = await fn();
    const row = { phase: name, ms: Math.round(performance.now() - t0) };
    timings.push(row); onPhase({ phase: name, state: "done", ms: row.ms });
    return [r, row];
  };
  const perFile = [], peaks = {};
  let mem = 0;
  const prev = pool.onProgress;
  pool.onProgress = p => { mem = Math.max(mem, p.memMB || 0); if (p.phase === "detect") perFile.push(p.ms);
    if (p.phase !== "detect") peaks[p.phase] = p.memMB; if (prev) prev(p); };
  const refused = e => e && e.key === "error.featMemory";
  // the engine «untargeted» (mzmine-like, Attributio): its own detect and align calls, no hypotheses or proofs
  const unt = (() => { try { return JSON.parse(recipe).engine === "untargeted"; } catch (_) { return false; } })();
  const putFiles = async () => { for (let i = 0; i < files.length; i++) await pool.fs.write("mzlab/feat/" + files[i].name + ".mzfeatf", stored[i].slice()); };
  const putTable = bytes => pool.fs.write("mzlab/feat/series.mzfeat", bytes.slice());
  let stored, table, cached = 0;
  try {
    let row;
    [stored, row] = await phase("detect", async () => {
      if (!(pool.op && pool.fs && pool.fs.trim)) return unt ? pool.detectAllUnt(files, recipe) : pool.detectAll(files, recipe);
      const d = await detectWithCache(pool, files, recipe, unt); cached = d.cached; return d.stored;
    });
    row.files = files.length; row.cached = cached; row.perFileMs = perFile.slice(); row.peakMB = mem;
    if (save) await phase("save", putFiles);
    let arow;
    try { [table, arow] = await phase("align", () => more(() => (unt ? pool.alignUnt : pool.align)(stored, files.map(f => f.path), recipe))); }
    catch (e) { if (refused(e)) e.params = { ...e.params, files: files.length }; throw e; }
    arow.peakMB = peaks.align;
    if (save) await phase("save", () => putTable(table));
    const out = { stored, table, evid: null, tried: 0, found: 0, registry: null, timings, degraded: null };
    if (parent && !unt) {
      const args = { table, stored, paths: files.map(f => f.path), recipe, ...parent };
      try {
        const [h, hrow] = await phase("hypotheses", () => pool.hypotheses(args));
        hrow.peakMB = peaks.hypotheses;
        const [r, prow] = await phase("proofs", () => more(() => pool.evidence({ ...args, state: h.state })));
        Object.assign(out, { evid: r.table, tried: h.tried, found: h.found, registry: h.registry }); prow.peakMB = peaks.proofs; prow.hypothesesMs = h.ms;
        if (save) await phase("save", () => putTable(out.evid));
      } catch (e) {
        if (!refused(e)) throw e;
        out.degraded = e.params.fase || "proofs";
        pool.notify("tpmine.feat.degraded", { phase: out.degraded, files: files.length, stima_mb: e.params.stima_mb, budget_mb: e.params.budget_mb });
      }
    }
    out.totalMs = Math.round(performance.now() - t00);
    return out;
  } finally { pool.onProgress = prev; if (raised.length) pool.setBudget(raised[0]); }
}
