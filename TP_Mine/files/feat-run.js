// Runs the pipeline of mzFinder on a series of .mzidx files in OPFS with a pool of feat-worker.js (see docs/agenti/mzfeat-wasm.md):
//   detect  = L1-L3 of every file, in parallel (one job per file)
//   align   = L4-L6 of the series (alignment, gap filling) in one worker
//   proofs  = L7 (hypotheses of the parent in every file) and L8-L9 (graph of the proofs) in one worker, only with a parent
// Resolves with { stored, table, evid, tried, found, registry, timings }; `timings` has one row per phase { phase, ms, files?, perFileMs? }.
// The .mzfeatf and the .mzfeat are written to OPFS (`mzlab/feat/`) by the archive worker when `save` is true.
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
  let stored, table;
  try {
    let row;
    [stored, row] = await phase("detect", () => pool.detectAll(files, recipe));
    row.files = files.length; row.perFileMs = perFile.slice(); row.peakMB = mem;
    let arow;
    [table, arow] = await phase("align", () => pool.align(stored, files.map(f => f.path), recipe));
    arow.peakMB = peaks.align;
    const out = { stored, table, evid: null, tried: 0, found: 0, registry: null, timings };
    if (parent) {
      const [r, prow] = await phase("proofs", () => pool.proofs({ table, stored, paths: files.map(f => f.path), recipe, ...parent }));
      Object.assign(out, { evid: r.table, tried: r.tried, found: r.found, registry: r.registry }); prow.peakMB = peaks.proofs; prow.hypothesesMs = r.msHypotheses;
    }
    if (save) await phase("save", async () => {
      for (let i = 0; i < files.length; i++) await pool.fs.write("mzlab/feat/" + files[i].name + ".mzfeatf", stored[i].slice());
      await pool.fs.write("mzlab/feat/series.mzfeat", (out.evid || table).slice());
    });
    out.totalMs = Math.round(performance.now() - t00);
    return out;
  } finally { pool.onProgress = prev; }
}
