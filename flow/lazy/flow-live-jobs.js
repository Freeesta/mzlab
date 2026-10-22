// @ts-check
// Flow (administrator mode): what runs a job of the live Flow in this browser (flow-live.js asks for the jobs, the scheduler of the engine says which). One executor per step: index.mzidx
// observes the index Data builds (never makes a second one), features.detect and features.align go to the pool of feat-worker.js, features.gapfill reuses the table of the alignment when it
// was made with the same recipe. Every result is kept in OPFS under the plan_key of its job (mzlab/flow/live/<plan_key>.*); the scheduler only keeps states and keys. English only.
// An executor reports to the scheduler what it claims (started | cached | block) before it returns, so the next `next` does not offer the job again; the rest runs on its own (`done`).
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const DIR = "mzlab/flow/live/", LATEST = "mzlab/flow/series.mzfeat", LOCK = "mzlab-mem-grande", BIG_MB = 512;
  const kit = () => F.runners.web.kit;
  /** @param {any} o @returns {Record<string, string>} */
  const strings = o => Object.fromEntries(Object.entries(o || {}).map(([k, v]) => [k, String(v)]));
  const job = (/** @type {any} */ spec) => ({ instance: spec.instance, item: spec.item ?? null });
  /** a file of OPFS is there (a folder that is gone is not an error) @param {any} p @param {string} path */
  const has = async (p, path) => { try { return await p.fs.exists(path); } catch (_) { return false; } };
  const budgetMb = () => Number(G.MZLAB_RUST && G.MZLAB_RUST.budgetMb) || 0;

  /** a failed or cancelled job reported to the scheduler: a cancelled one is not an error of the step @param {any} x @param {any} spec @param {any} inst @param {any} e */
  const fail = async (x, spec, inst, e) => {
    const cancelled = e && e.key === "tpmine.feat.cancelled";
    if (!cancelled) x.errors.set(inst.id, e);
    await x.call("finished", { job: job(spec), result: { error_key: cancelled ? "err.flow.cancelled" : (e && e.key) || "error.featWorker", params: strings(e && e.params) } });
  };

  /** @param {any} x @param {any} spec @param {any} inst @param {() => Promise<void>} body the work after the claim */
  const detached = (x, spec, inst, body) => ({ done: body().catch((/** @type {any} */ e) => fail(x, spec, inst, e)) });

  /** spectra.centroid: a file of centroids is reused as it is («already centroided»); a profile file has the index of its centroids that Data builds (`<name>.cen.mzidx`, the existing route of the engine, nothing is
   * computed here): reused when it is there, else asked for. The browser builds it for the files it reads as a maker's file; an mzML in profile stays with the computer (blocked by platform.computer) */
  const centroid = async (/** @type {any} */ x, /** @type {any} */ spec, /** @type {any} */ inst) => {
    const it = x.item(spec.item), j = job(spec);
    if (!it || !it.idx) { await x.call("block", { job: j, by: "input.missing" }); x.waitingInput.add(x.jk(spec)); return {}; }
    if (!x.profile(it) || it.cen) { await x.call("cached", { job: j }); return {}; }
    const rb = await G.MZLAB_RUST_READY;
    if (!rb || !rb.nativeCentroidIndex || !rb.isNative || !rb.isNative(it.input)) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now();
    return detached(x, spec, inst, async () => {
      await rb.nativeCentroidIndex(it.input);
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
      F.live.change();
    });
  };

  /** index.mzidx: the index of the file is the one Data keeps in OPFS (reused), or the one Data is building now (progress from the event mzlab-index), else the job waits for the file */
  const index = async (/** @type {any} */ x, /** @type {any} */ spec) => {
    const it = x.item(spec.item), j = job(spec);
    if (it && it.idx) { await x.call("cached", { job: j }); return {}; }
    if (it && x.loading.has(it.stem)) { await x.call("started", { job: j }); x.observing.set(x.jk(spec), spec); return {}; }
    await x.call("block", { job: j, by: "input.missing" }); x.waitingInput.add(x.jk(spec));
    return {};
  };

  /** features.detect: one job per file, kept as <plan_key>.mzfeatf; a file already there is reused */
  const detect = async (/** @type {any} */ x, /** @type {any} */ spec, /** @type {any} */ inst) => {
    const k = kit(), p = await k.getPool(), it = x.item(spec.item), path = DIR + spec.key + ".mzfeatf", j = job(spec);
    if (it && x.profileHeld(it)) { await x.call("block", { job: j, by: "input.profile" }); x.waitingProfile.add(x.jk(spec)); return {}; }
    const src = x.source(it);
    if (!src) { await x.call("block", { job: j, by: "input.missing" }); x.waitingInput.add(x.jk(spec)); return {}; }
    if (await has(p, path)) { await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now();
    return detached(x, spec, inst, async () => {
      const out = (await p.detectAll([{ name: src.name, path: src.path, id: x.jk(spec) }], k.recipeOf(x.plan, inst)))[0];
      await p.fs.write(path, out.slice());
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  /** true when the table of the alignment `key` was made with this recipe @param {any} p @param {string} key @param {string} recipe */
  const fresh = async (p, key, recipe) => {
    try { return (await has(p, DIR + key + ".mzfeat")) && new TextDecoder().decode(await p.fs.read(DIR + key + ".recipe")) === recipe; } catch (_) { return false; }
  };

  /** the refusal of the budget with the numbers, as the pipeline reports it @param {any} est @param {number} budget */
  const refusal = (est, budget) => Object.assign(new Error("error.featMemory"), { key: "error.featMemory", params: { fase: "align", stima_mb: est.mb, budget_mb: budget, libero_mb: budget } });

  /** features.align, and features.gapfill when the table of the alignment was made with another recipe (the pool aligns and fills the gaps in one call): the estimate first; past the budget
   * the job is blocked by `permission.memory` until the user agrees (flow-live.js asks on Run), past the high level it is refused with its numbers @param {any} x @param {any} spec @param {any} inst */
  const align = async (x, spec, inst) => {
    const k = kit(), p = await k.getPool(), j = job(spec), recipe = k.recipeOf(x.plan);
    const al = x.alignOf(inst), alKey = al === inst ? spec.key : x.keys.get(x.jk({ instance: al.id, item: null }));
    if (!alKey) { await fail(x, spec, inst, Object.assign(new Error("err.flow.lineage"), { key: "err.flow.lineage", params: { step: inst.step } })); return {}; }
    if (await fresh(p, alKey, recipe)) { await x.call("cached", { job: j }); return {}; }
    const up = x.upstream(al);
    let stored;
    try { stored = await Promise.all(up.map(async u => p.fs.read(DIR + u.key + ".mzfeatf"))); } catch (_) { stored = []; }
    if (!stored.length || stored.length !== up.length) { await x.call("started", { job: j }); await fail(x, spec, inst, Object.assign(new Error("err.flow.lineage"), { key: "err.flow.lineage", params: { step: inst.step } })); return {}; }
    const est = await k.stima(stored, recipe), jk = x.jk(spec), high = k.highMb(), budget = budgetMb();
    x.info.set(inst.id, { mb: est.mb, modo: est.via });
    const raise = est.via === "refused" && x.allowed.has(jk);
    if (est.via === "refused" && !raise) {
      const err = refusal(est, budget);
      if (est.mb <= high && high > budget) { x.memAsk.set(jk, { spec, est, err }); await x.call("block", { job: j, by: "permission.memory" }); return {}; }
      x.errors.set(inst.id, err);
      await x.call("started", { job: j });
      await x.call("finished", { job: j, result: { error_key: "error.featMemory", params: strings(err.params) } });
      return {};
    }
    await x.call("started", { job: j });
    const t0 = performance.now();
    return detached(x, spec, inst, async () => {
      const raised = raise ? p.setBudget(high) : undefined;
      try {
        const run = () => p.align(stored, up.map(u => u.path), recipe, jk, est.mb > BIG_MB);
        const table = est.mb > BIG_MB && navigator.locks ? await navigator.locks.request(LOCK, run) : await run();
        await p.fs.write(DIR + alKey + ".mzfeat", table.slice());
        await p.fs.write(LATEST, table.slice());
        await p.fs.write(DIR + alKey + ".recipe", new TextEncoder().encode(recipe));
        await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
      } finally { if (raised !== undefined) p.setBudget(raised); }
    });
  };

  F.liveJobs = { has, "spectra.centroid": centroid, "index.mzidx": index, "features.detect": detect, "features.align": align, "features.gapfill": align, DIR, fresh, strings };
})();
