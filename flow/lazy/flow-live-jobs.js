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

  // results of the MSn steps kept in OPFS (mzlab/flow/msn/<key>.json = {name, text}), the newest MSN_KEEP of them; index.json lists the keys, oldest first
  const MSN_DIR = "mzlab/flow/msn/", MSN_KEEP = 32;
  /** SHA-256 hex of a text @param {string} s */
  const sha = async s => [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(s)))].map(b => b.toString(16).padStart(2, "0")).join("");
  /** the key of an MSn job: its plan_key (step, parameters, program version, the open infusion, the jobs upstream) and, for the search, the library of the user, which the plan does not hold @param {any} spec @param {any} inst */
  const msnKey = async (spec, inst) => sha(spec.key + (base(inst) === "spectra.match" ? "\n" + await F.webMsn.library() : ""));
  /** the {name, text} kept under `key`, or null @param {any} p @param {string} key */
  const recall = async (p, key) => {
    try { return await has(p, MSN_DIR + key + ".json") ? JSON.parse(new TextDecoder().decode(await p.fs.read(MSN_DIR + key + ".json"))) : null; } catch (_) { return null; }
  };
  /** keeps a result and forgets the oldest ones past MSN_KEEP @param {any} p @param {string} key @param {{ name: string, text: string }} r */
  const remember = async (p, key, r) => {
    try {
      await p.fs.write(MSN_DIR + key + ".json", new TextEncoder().encode(JSON.stringify({ name: r.name, text: r.text })));
      let list = [];
      try { list = JSON.parse(new TextDecoder().decode(await p.fs.read(MSN_DIR + "index.json"))); } catch (_) { /* no index yet */ }
      list = [...list.filter((/** @type {string} */ k) => k !== key), key];
      for (const old of list.splice(0, Math.max(0, list.length - MSN_KEEP))) await p.fs.remove(MSN_DIR + old + ".json").catch(() => {});
      await p.fs.write(MSN_DIR + "index.json", new TextEncoder().encode(JSON.stringify(list)));
    } catch (_) { /* the cache is a convenience: the result is still given */ }
  };

  /** the MSn steps (flow-web-msn.js): one job for the step, on the infusion open in Data; the text is kept in OPFS by the runner of the MSn steps and given back with the other results (`x.files`).
   * A result already kept under the key of the job (same plan_key, same library for the search) is taken from OPFS without asking the engine: the job is `cached`.
   * The routes of the engine are the ones of the runner of an MSn plan, so the bytes are the same; spectra.match with another algorithm stays with the computer */
  const msn = async (/** @type {any} */ x, /** @type {any} */ spec, /** @type {any} */ inst) => {
    const j = job(spec), M = F.webMsn;
    if (!M || (base(inst) === "spectra.match" && inst.algorithm !== "tree")) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const p = await kit().getPool(), key = await msnKey(spec, inst);
    // a closed infusion is an error to show, never a result to take from the cache
    let open = true;
    try { M.infusion(x.S); } catch (_) { open = false; }
    const hit = open ? await recall(p, key) : null;
    if (hit) {
      x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text });
      await M.keep(hit.name, hit.text);
      F.webMsnAfterRun(x.S, [x.files.get(inst.id)]);
      await x.call("cached", { job: j });
      return {};
    }
    await x.call("started", { job: j });
    const t0 = performance.now();
    return detached(x, spec, inst, async () => {
      const r = await M.one(inst, { plan: x.plan, k: M.infusion(x.S), graph: null, phase: (/** @type {number} */ frac) => { x.call("progress", { job: j, frac }).catch(() => {}); } });
      await remember(p, key, r);
      x.files.set(inst.id, { id: inst.id, ...r });
      F.webMsnAfterRun(x.S, [x.files.get(inst.id)]);
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };
  const base = (/** @type {any} */ inst) => String(inst.step).split("@")[0];

  /** tp.generate (flow-web-tp.js): the registry of the hypotheses of the parent the plan holds; kept like the MSn results, under the key of the job (the plan_key has the SHA-256 of the parent) */
  const tpGenerate = async (/** @type {any} */ x, /** @type {any} */ spec, /** @type {any} */ inst) => {
    const j = job(spec), p = await kit().getPool(), key = await sha(spec.key), hit = await recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now();
    return detached(x, spec, inst, async () => {
      const r = await F.webTp.generate(x.S, inst);
      await F.webMsn.keep(r.name, r.text); await remember(p, key, r);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  /** tp.filter@2, tp.position, tp.network, tp.review (flow-web-tp.js `post`): a text made by the pool from the `.tp.mzfeat` of the tp.link they follow, kept like the MSn results under the key of the job */
  const tpPost = async (/** @type {any} */ x, /** @type {any} */ spec, /** @type {any} */ inst) => {
    const j = job(spec), p = await kit().getPool(), key = await sha(spec.key), hit = await recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    const t0 = performance.now();
    /** @type {any} */ let r = null;
    try { r = F.webTp ? await F.webTp.post({ x, p, dir: DIR }, inst) : null; } catch (e) { await x.call("started", { job: j }); await fail(x, spec, inst, e); return {}; }
    if (!r) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    await x.call("started", { job: j });
    return detached(x, spec, inst, async () => {
      await F.webMsn.keep(r.name, r.text); await remember(p, key, r);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  /** tp.link@2 (flow-web-tp.js): the hypotheses searched in the series, the proofs on the table of the alignment and the interpretation of the rows, in two calls to the pool (the same Rust as `flow run`); the `.mzfeat`
   * is kept in OPFS under the plan_key of the job. A file of hypotheses or a table that is not this plan's stays with the computer @param {any} x @param {any} spec @param {any} inst */
  const tpLink = async (x, spec, inst) => {
    const j = job(spec), k = kit(), p = await k.getPool(), path = DIR + spec.key + ".tp.mzfeat";
    if (await has(p, path)) { await x.call("cached", { job: j }); return {}; }
    const al = x.alignOf(x.inst(String(F.refs(inst, "table")[0] || "").split(".")[0]) || inst), alKey = x.keys.get(x.jk({ instance: al.id, item: null })), up = x.upstream(al);
    const inp = F.webTp ? await F.webTp.linkInputs(x.S, inst, x.inst).catch(/** @param {any} e */ e => { x.errors.set(inst.id, e); return e; }) : null;
    if (inp instanceof Error) { await x.call("started", { job: j }); await fail(x, spec, inst, inp); return {}; }
    if (!inp || !alKey || !up.length) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now();
    return detached(x, spec, inst, async () => {
      const read = async (/** @type {string} */ f) => p.fs.read(f), paths = up.map(u => u.path);
      const table = await read(DIR + alKey + ".mzfeat"), peaks = await Promise.all(up.map(u => read(DIR + u.key + ".mzfeatf")));
      const given = (inst.params || {}).polarity, step = F.stepOf(x.S.reg, inst);
      const ev = await p.linkEvidence({ table, rules: inp.rules, parent: inp.text, structure: inp.structure, hypotheses: inp.hypotheses, samples: inp.samples, recipe: k.recipeOf(x.plan), polarity: given == null ? NaN : Number(given), budgetMb: budgetMb(), peaks, paths });
      const out = await p.linkInterp({ table: ev.table, parent: ev.parent, registry: ev.registry, polarity: ev.polarity, params: JSON.stringify({ ...F.defaults(inst, step), ...(inst.params || {}) }), infusion: ev.infusion, paths });
      await p.fs.write(path, out.slice());
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  F.liveJobs = { msnKey, "tp.generate": tpGenerate, "tp.link": tpLink, "tp.filter": tpPost, "tp.position": tpPost, "tp.network": tpPost, "tp.review": tpPost, "spectra.msn_graph": msn, "msn.library_export": msn, "msn.table_export": msn, "spectra.match": msn, has, "spectra.centroid": centroid, "index.mzidx": index, "features.detect": detect, "features.align": align, "features.gapfill": align, DIR, fresh, strings };
})();
