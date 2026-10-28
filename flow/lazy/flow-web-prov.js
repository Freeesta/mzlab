// @ts-check
// Flow (administrator mode): the record of a run of «This browser». When the live run has made every step of the base chain (index, detection, alignment, gap filling) the facts the record asks of
// the files in OPFS (size and SHA-256, the source of each index, the estimates and the memory of a job: worker-flow-web.js) go to the engine (api/flow/sched/record_prov, record_corsa), which writes
// `prov/1` and `corsa/1` with the same Rust as `flow run`. The two texts are kept in OPFS (mzlab/flow/prov.json, corsa.json) and in `F.webProv.last`. Used by flow-live.js at the end of a run.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const CHAIN = ["index.mzidx", "features.detect", "features.align", "features.gapfill"];
  const OUT = "mzlab/flow/";
  const base = (/** @type {any} */ inst) => String(inst.step).split("@")[0];

  /** one request to the facts worker @param {Worker} w @param {any} wasm @param {any} m */
  const ask = (w, wasm, m) => new Promise((resolve, reject) => {
    const id = Math.random();
    const done = (/** @type {MessageEvent} */ ev) => {
      if (ev.data.id !== id) return;
      w.removeEventListener("message", done);
      if (ev.data.error) { reject(Object.assign(new Error(ev.data.error.text), ev.data.error)); return; }
      try { resolve(JSON.parse(ev.data.json)); } catch (e) { reject(e); }
    };
    w.addEventListener("message", done);
    w.postMessage({ id, wasm, ...m });
  });

  /** a POST to the scheduler routes; the answer is the text of the record @param {string} op @param {any} body */
  const post = async (op, body) => {
    const t = await (await fetch("api/flow/sched/" + op, { method: "POST", body: JSON.stringify(body) })).text();
    let v = null; try { v = JSON.parse(t); } catch (_) { /* not JSON */ }
    if (v && !Array.isArray(v) && v.error_key) throw Object.assign(new Error(v.error_key), { key: v.error_key, params: v.params || {} });
    return t;
  };

  /** the record of the run of the live session `x`, or null when the plan has a step outside the base chain, a job is not done or an input has no hash.
   * Resolves with { prov, corsa } (the texts) @param {any} x @param {any[]} jobs the jobs the scheduler reports (`jobs`) */
  const record = async (x, jobs) => {
    const plan = x.plan;
    if (!plan.steps.every((/** @type {any} */ i) => CHAIN.includes(base(i))) || !x.items.length || !x.registry) return null;
    if (!jobs.length || jobs.some(j => j.state !== "done" && j.state !== "reused")) return null;
    const k = F.runners.web.kit, p = await k.getPool(), D = F.liveJobs.DIR, reg = x.S.reg;
    let alone = "";
    const w = new Worker(new URL("mzfinder/lazy/worker-flow-web.js", G.QTOOLS.ctx.base), { type: "module" });
    try {
      const wasm = p.featWasm, text0 = JSON.stringify(F.exportPlan(plan)), doc = JSON.parse(text0);
      const sources = new Map();
      for (const it of x.items) {
        if (!it.idx) return null;
        const s = await ask(w, wasm, { op: "source", paths: [it.idx.path] });
        if (!s.sha256) return null;
        sources.set(it.input, s);
      }
      for (const f of doc.inputs || []) { const s = sources.get(f.name); if (!s) return null; f.sha256 = s.sha256; f.bytes = s.bytes; }
      const planText = JSON.stringify(doc), first = x.source(x.items[0]).path;
      const fileOut = async (/** @type {any} */ inst, /** @type {string} */ path, /** @type {string} */ ext) => {
        const out = Object.entries(F.stepOf(reg, inst).out)[0], s = await ask(w, wasm, { op: "sha", paths: [path] });
        return { port: out[0], type: /** @type {any} */ (out[1]).type, sha256: s.sha256, bytes: s.bytes, ext };
      };
      const reports = [];
      for (const j of jobs) {
        const inst = x.inst(j.instance), id = base(inst), key = x.keys.get(x.jk(j)), it = j.item ? x.item(j.item) : null, cached = j.state === "reused";
        let r;
        if (id === "index.mzidx") {
          if (!it) return null;
          r = { outputs: [await fileOut(inst, it.idx.path, "mzidx")], estimates: [], mem: null };
        } else if (id === "features.detect") {
          if (!it || !key) return null;
          r = { outputs: [await fileOut(inst, D + key + ".mzfeatf", "mzfeatf")], ...(await ask(w, wasm, { op: "meta", paths: [first, x.source(it).path], plan: planText, registry: JSON.stringify(x.registry), instance: j.instance, budgetMb: Number(G.MZLAB_RUST && G.MZLAB_RUST.budgetMb) || 0 })) };
        } else {
          const al = x.alignOf(inst), alKey = x.keys.get(x.jk({ instance: al.id, item: null })), up = x.upstream(al);
          if (!alKey) return null;
          const meta = await ask(w, wasm, { op: "meta", paths: [first, first], stored: up.map((/** @type {any} */ u) => D + u.key + ".mzfeatf"), plan: planText, registry: JSON.stringify(x.registry), instance: j.instance, budgetMb: Number(G.MZLAB_RUST && G.MZLAB_RUST.budgetMb) || 0 });
          let path = D + alKey + ".mzfeat";
          if (id === "features.align" && x.plan.steps.some((/** @type {any} */ i) => base(i) === "features.gapfill")) {
            // the file kept under the key of the alignment is the filled table: the table of the alignment alone is made again without the gap filling
            const recipe = { ...JSON.parse(k.recipeOf(plan, al)), fill: false };
            const stored = await Promise.all(up.map((/** @type {any} */ u) => p.fs.read(D + u.key + ".mzfeatf")));
            const table = await p.align(stored, up.map((/** @type {any} */ u) => u.path), JSON.stringify(recipe), x.jk(j), !!meta.mem && meta.mem.via === "slabs");
            alone = path = OUT + "alone.mzfeat";
            await p.fs.write(path, table.slice());
          }
          r = { outputs: [await fileOut(inst, path, "mzfeat")], ...meta };
        }
        reports.push({ instance: j.instance, ...(j.item ? { item: j.item } : {}), ms: 0, cached, ...r });
      }
      const prov = await post("record_prov", { plan: planText, registry: x.registry, jobs: reports });
      const corsa = await post("record_corsa", { prov, uscite: [] });
      await p.fs.write(OUT + "prov.json", new TextEncoder().encode(prov));
      await p.fs.write(OUT + "corsa.json", new TextEncoder().encode(corsa));
      F.webProv.last = { prov, corsa };
      return F.webProv.last;
    } finally { w.terminate(); if (alone) await p.fs.remove(alone).catch(() => {}); }
  };

  /** @type {{ last: { prov: string, corsa: string } | null, record: (x: any, jobs: any[]) => Promise<{ prov: string, corsa: string } | null> }} */
  F.webProv = { last: null, record };
})();
