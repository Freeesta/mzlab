// @ts-check
// Flow (administrator mode): table.plausible in this browser. The plausible/1 text is `flow_plausible` of the `flowtab` .wasm (loaded here only, when this step runs):
// `mzlab_feat::plausible::plausible_text`, the function `flow run` calls, so the bytes are the same. The table is the alignment (or gap filling) of the same plan kept in OPFS,
// or the table a `table.filter` view of it stands for (`F.liveJobs.tableOf`). The text is kept under the key of the job. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const STEPS = ["table.plausible"];
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key, params: j.params || {} }) : e; };

  /** table.plausible @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), tab = await J().tableOf(x, p, inst, "table");
    if (!tab) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha(spec.key + "\n" + tab.key), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });
    return J().detached(x, spec, inst, async () => {
      let text;
      try { text = await p.ident("flowtab", "flow_plausible", { params, table: await tab.read() }); } catch (e) { throw asError(e); }
      const r = { name: inst.id + ".json", text };
      await F.webMsn.keep(r.name, r.text); await J().remember(p, key, r);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  F.webPlausible = { run, STEPS };
})();
