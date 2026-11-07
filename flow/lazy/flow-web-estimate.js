// @ts-check
// Flow (administrator mode): features.estimate in this browser, one job per file. The text is `flow_estimate` of the `hr` .wasm on the index the detection reads
// (`run_estimate::estimate_of`, the function `flow run` calls), so each estimates/1 is the same bytes, named `<instance>-<root>.json` as `flow run` names it
// (`root` from the scheduler). Kept like the other texts, under the key of the job. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const STEPS = ["features.estimate"];
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;

  /** features.estimate on the file of `spec` @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), it = x.item(spec.item);
    if (it && x.profileHeld(it)) { await x.call("block", { job: j, by: "input.profile" }); x.waitingProfile.add(x.jk(spec)); return {}; }
    const src = x.source(it);
    if (!src) { await x.call("block", { job: j, by: "input.missing" }); x.waitingInput.add(x.jk(spec)); return {}; }
    const at = { id: inst.id, item: spec.item }, key = await J().sha(spec.key), hit = await J().recall(p, key);
    if (hit) { x.files.set(x.jk(spec), { ...at, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now();
    return J().detached(x, spec, inst, async () => {
      const text = await p.ident("feat", "flow_estimate", { path: src.path, name: it.input || src.name, params: JSON.stringify(inst.params || {}) });
      const r = { name: inst.id + "-" + spec.root + ".json", text };
      await F.webMsn.keep(r.name, r.text); await J().remember(p, key, r);
      x.files.set(x.jk(spec), { ...at, ...r });
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  F.webEstimate = { run, STEPS };
})();
