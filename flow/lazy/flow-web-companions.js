// @ts-check
// Flow (administrator mode): features.companions in this browser. The companion edges of a unit-mass table (companions/1: isotope steps, adducts and in-source losses
// with their RT offset, correlation and ratio) are `flow_companions` of the `flowtab` .wasm (loaded here only, when this step runs): `mzlab_feat::companions::companions_text`,
// the function `flow run` calls, so the text is the same bytes. The table is the alignment (or gap filling) of the same plan kept in OPFS, or the table a `table.filter` view of it
// stands for (`F.liveJobs.tableOf`); the text is kept under the key of the job and its warnings go to the record and to the step card. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const STEPS = ["features.companions"];
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key, params: j.params || {} }) : e; };
  /** the words of the warnings of the step on its card (`ev.note` of flow-live.js), none when it has none @param {any} x @param {any} inst @param {string[] | undefined} w */
  const note = (x, inst, w) => { if (w && w.length) x.notes.set(inst.id, w.map(k => F.message({ key: k, params: {} })).join(" ")); else x.notes.delete(inst.id); };

  /** features.companions @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), tab = await J().tableOf(x, p, inst, "table");
    if (!tab) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha(spec.key + "\n" + tab.key), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); note(x, inst, hit.warnings); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });
    return J().detached(x, spec, inst, async () => {
      let made;
      try { made = await p.ident("flowtab", "flow_companions", { params, table: await tab.read() }, true); } catch (e) { throw asError(e); }
      const r = { name: inst.id + ".json", text: made.text }, warnings = made.warnings || [];
      await J().remember(p, key, { ...r, warnings }); note(x, inst, warnings);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: j, result: warnings.length ? { ms: performance.now() - t0, warnings } : { ms: performance.now() - t0 } });
    });
  };

  F.webCompanions = { run, STEPS };
})();
