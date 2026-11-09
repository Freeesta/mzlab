// @ts-check
// Flow (administrator mode): features.group in this browser. The grouping of the rows of a per-ion table into compounds (isotopes, ion species, ionnet/1) is `flow_group` of the `flowtab`
// .wasm (loaded here only, when this step runs): `mzlab_feat::ionnet::group_text`, the function `flow run` calls, so the text is the same bytes. The table is the alignment (or gap filling)
// of the same plan, kept in OPFS, or the table a `table.filter` view of it stands for (`F.liveJobs.tableOf`); the text is kept like the other results, under the key of the job. The engine
// works within the safe budget of the page; a refusal (phase `group`) that fits in the high budget waits for the user's yes (`permission.memory`, asked on Run as for the alignment)
// and runs again with it, any other is reported as the alignment's (`error.featMemory`). Its warnings go to the record. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (globalThis);
  const STEPS = ["features.group"];
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  /** the safe working memory of a step, as the pool gives it (`budgetOf` of feat-worker.js): the level it was lowered to, else the engine's choice */
  const safeMb = () => { const r = G.MZLAB_RUST; return r ? Number(r.budgetMb || (r.choice && r.choice.mb)) || 0 : 0; };
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => {
    let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ }
    if (!j || !j.error_key) return e;
    const key = j.error_key === "error.memoryBudget" ? "error.featMemory" : j.error_key;
    return Object.assign(new Error(key), { key, params: j.params || {} });
  };

  /** the words of the warnings of the step on its card (`ev.note` of flow-live.js), none when it has none @param {any} x @param {any} inst @param {string[] | undefined} w */
  const note = (x, inst, w) => { if (w && w.length) x.notes.set(inst.id, w.map(k => F.message({ key: k, params: {} })).join(" ")); else x.notes.delete(inst.id); };

  /** features.group @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool();
    const tab = await J().tableOf(x, p, inst, "table");
    if (!tab) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha(spec.key + "\n" + tab.key), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); note(x, inst, hit.warnings); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });
    const jk = x.jk(spec), raise = x.allowed.has(jk), high = kit().highMb();
    return J().detached(x, spec, inst, async () => {
      let made;
      try { made = await p.ident("flowtab", "flow_group", { params, table: await tab.read(), budgetMb: raise ? high : safeMb() }, true); } catch (e) {
        const err = asError(e), q = err.params || {}, need = Number(q.stima_mb);
        if (err.key !== "error.featMemory" || raise || !(need <= high && high > Number(q.budget_mb))) throw err;
        // only a job that is not running can be held: it ends refused, is held by `permission.memory` and goes back in line (Run asks, flow-live.js `answerAsks`)
        x.memAsk.set(jk, { spec, est: { mb: need }, err, fase: "group" });
        await x.call("finished", { job: j, result: { error_key: "error.featMemory", params: J().strings(q) } });
        await x.call("block", { job: j, by: "permission.memory" });
        await x.call("retry", { job: j });
        return;
      }
      const r = { name: inst.id + ".json", text: made.text }, warnings = made.warnings || [];
      await F.webMsn.keep(r.name, r.text); await J().remember(p, key, { ...r, warnings }); note(x, inst, warnings);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: j, result: warnings.length ? { ms: performance.now() - t0, warnings } : { ms: performance.now() - t0 } });
    });
  };

  F.webGroup = { run, STEPS };
})();
