// @ts-check
// Flow (administrator mode): qc.run_order and report.methods in this browser. Both texts come from the `flowtab` .wasm (module loaded here only, when one of these steps runs): `flow_runorder` is
// `mzlab_feat::runorder::runorder_text` and `flow_methods` is `mzlab_feat::methods::methods_text`, the functions `flow run` calls, so the runorder/1 JSON and the Markdown are the same bytes. The sample
// table (and the batches) are files of the plan (their text in S.samplesJson); the run record of report.methods is a file of the plan too, else the corsa/1 this browser kept for the last run of the
// plan, else the step is blocked by `input.run`. The text is kept like the other results, under the key of the job. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const STEPS = ["qc.run_order", "report.methods"];
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

  /** the step's parameters merged with its defaults, as the text the .wasm reads @param {any} x @param {any} inst */
  const paramsOf = (x, inst) => JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });

  /** claims the job, runs `call` in the flowtab module and keeps the text under `name` @param {any} x @param {any} spec @param {any} inst @param {string} key @param {string} name @param {() => Promise<string>} call */
  const make = async (x, spec, inst, key, name, call) => {
    const j = J().job(spec), p = await kit().getPool(), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now();
    return J().detached(x, spec, inst, async () => {
      let text;
      try { text = await call(); } catch (e) { throw asError(e); }
      const r = { name, text };
      await F.webMsn.keep(r.name, r.text); await J().remember(p, key, r);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  /** qc.run_order @param {any} x @param {any} spec @param {any} inst */
  const runOrder = async (x, spec, inst) => {
    const samples = fileText(x, inst, "samples"), wired = F.refs(inst, "design").length > 0, design = fileText(x, inst, "design");
    if (samples === null || (wired && design === null)) { await x.call("block", { job: J().job(spec), by: "platform.computer" }); return {}; }
    const key = await J().sha(spec.key + "\n" + await J().sha(samples) + "\n" + await J().sha(design || ""));
    return make(x, spec, inst, key, inst.id + ".json", async () => (await kit().getPool()).ident("flowtab", "flow_runorder", { params: paramsOf(x, inst), samples: enc(samples), design: enc(design || "") }));
  };

  /** report.methods @param {any} x @param {any} spec @param {any} inst */
  const methods = async (x, spec, inst) => {
    const given = fileText(x, inst, "run"), last = F.webProv && F.webProv.last && F.webProv.last.corsa, run = given !== null ? given : typeof last === "string" ? last : null;
    if (run === null) { await x.call("block", { job: J().job(spec), by: "input.run" }); return {}; }
    const key = await J().sha(spec.key + "\n" + await J().sha(run));
    return make(x, spec, inst, key, inst.id + ".md", async () => (await kit().getPool()).ident("flowtab", "flow_methods", { params: paramsOf(x, inst), run: enc(run) }));
  };

  F.webQcReport = { runOrder, methods, STEPS };
})();
