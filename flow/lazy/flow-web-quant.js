// @ts-check
// Flow (administrator mode): quant.calibrate in this browser. Applying the imported calibration curves to the areas is `flow_quant` of the `flowcal` .wasm (`mzlab_ops::run::table::quant::quant_text`,
// the function `flow run` calls), so the concentrations CSV is the same bytes. The table is the alignment (or gap filling) of the same plan kept in OPFS, or the table a `table.filter` view of it
// stands for (`F.liveJobs.tableOf`); `curves` and `samples` are files of the plan (their text in S.samplesJson). The warnings go to the card of the step. The text is kept under the key of the job. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const STEPS = ["quant.calibrate"];
  /** the words of the warnings of the step on its card (`ev.note` of flow-live.js), none when it has none @param {any} x @param {any} inst @param {string[] | undefined} w */
  const note = (x, inst, w) => { if (w && w.length) x.notes.set(inst.id, w.map(k => F.message({ key: k, params: {} })).join(" ")); else x.notes.delete(inst.id); };
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  const enc = (/** @type {string} */ s) => new TextEncoder().encode(s);
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key, params: j.params || {} }) : e; };

  /** the text of a file of the plan on port `port` (null when the port is not wired, or the browser does not hold the file) @param {any} x @param {any} inst @param {string} port */
  const fileText = (x, inst, port) => {
    const ref = String(F.refs(inst, port)[0] || "");
    if (!ref.startsWith("file:")) return null;
    const t = x.S.samplesJson && x.S.samplesJson[ref.slice(5)];
    return typeof t === "string" ? t : null;
  };

  /** quant.calibrate @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), samples = fileText(x, inst, "samples"), curves = fileText(x, inst, "curves");
    const tab = samples === null || curves === null ? null : await J().tableOf(x, p, inst, "table");
    if (!tab) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha(spec.key + "\n" + await J().sha(samples) + "\n" + await J().sha(curves) + "\n" + tab.key), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); note(x, inst, hit.warnings); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });
    return J().detached(x, spec, inst, async () => {
      let made;
      try { made = JSON.parse(await p.ident("flowcal", "flow_quant", { params, table: await tab.read(), curves: enc(curves), samples: enc(samples) })); } catch (e) { throw asError(e); }
      const r = { name: inst.id + ".csv", text: made.text }, warnings = made.warnings || [];
      await F.webMsn.keep(r.name, r.text); await J().remember(p, key, { ...r, warnings }); note(x, inst, warnings);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: j, result: warnings.length ? { ms: performance.now() - t0, warnings } : { ms: performance.now() - t0 } });
    });
  };

  F.webQuant = { run, STEPS };
})();
