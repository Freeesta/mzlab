// Runs the pipeline of mzFinder on a series of .mzidx files in OPFS with a pool of feat-worker.js (see docs/agenti/mzfeat-wasm.md):
//   detect  = L1-L3 of every file, in parallel (one job per file)
//   align   = L4-L6 of the series (alignment, gap filling) in one worker
//   hypotheses = L7 in a worker of its own, only with a parent; proofs = L8-L9 (graph of the proofs) in another one
// Resolves with { stored, table, evid, tried, found, registry, timings, degraded }; `timings` has one row per phase { phase, ms, files?, perFileMs? }.
// The .mzfeatf and the .mzfeat are written to OPFS (`mzlab/feat/`) by the archive worker when `save` is true, each as soon as its phase ends, so a phase that the
// memory refuses (`error.featMemory`) leaves the earlier ones saved: the hypotheses or the proofs refused give the table without proofs and the warning
// `tpmine.feat.degraded` (`degraded` = the phase), the detection or the alignment refused stop the run with the error (and `params.files`).
export async function runFeatPipeline(pool, { files, recipe, parent, save = false, onPhase = () => {} }) {
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
  const putFiles = async () => { for (let i = 0; i < files.length; i++) await pool.fs.write("mzlab/feat/" + files[i].name + ".mzfeatf", stored[i].slice()); };
  const putTable = bytes => pool.fs.write("mzlab/feat/series.mzfeat", bytes.slice());
  let stored, table;
  try {
    let row;
    [stored, row] = await phase("detect", () => pool.detectAll(files, recipe));
    row.files = files.length; row.perFileMs = perFile.slice(); row.peakMB = mem;
    if (save) await phase("save", putFiles);
    let arow;
    try { [table, arow] = await phase("align", () => pool.align(stored, files.map(f => f.path), recipe)); }
    catch (e) { if (refused(e)) e.params = { ...e.params, files: files.length }; throw e; }
    arow.peakMB = peaks.align;
    if (save) await phase("save", () => putTable(table));
    const out = { stored, table, evid: null, tried: 0, found: 0, registry: null, timings, degraded: null };
    if (parent) {
      const args = { table, stored, paths: files.map(f => f.path), recipe, ...parent };
      try {
        const [h, hrow] = await phase("hypotheses", () => pool.hypotheses(args));
        hrow.peakMB = peaks.hypotheses;
        const [r, prow] = await phase("proofs", () => pool.evidence({ ...args, state: h.state }));
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
  } finally { pool.onProgress = prev; }
}
