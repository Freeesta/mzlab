// @ts-check
// Flow (administrator mode): table.drift in this browser. The correction of the drift of the areas along the order of injection is `flow_drift` of the `flowtab` .wasm (module loaded here only, when this
// step runs): `mzlab_feat::flow_drift::drift_text`, the function `flow run` calls, so the deriva/1 text is the same bytes. The table is the alignment (or gap filling) of the same plan, kept in OPFS; `samples`
// and `design` are files of the plan (their text in S.samplesJson). The text is kept like the other results, under the key of the job. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const STEPS = ["table.drift"];
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  const enc = (/** @type {string} */ s) => new TextEncoder().encode(s);
  const base = (/** @type {any} */ inst) => String(inst.step).split("@")[0];
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key, params: j.params || {} }) : e; };

  /** the text of a file of the plan on port `port` (null when the port is not wired, or the browser does not hold the file) @param {any} x @param {any} inst @param {string} port */
  const fileText = (x, inst, port) => {
    const ref = String(F.refs(inst, port)[0] || "");
    if (!ref.startsWith("file:")) return null;
    const t = x.S.samplesJson && x.S.samplesJson[ref.slice(5)];
    return typeof t === "string" ? t : null;
  };

  /** table.drift @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), samples = fileText(x, inst, "samples"), design = fileText(x, inst, "design");
    const up0 = x.inst(String(F.refs(inst, "table")[0] || "").split(".")[0]), isTable = !!up0 && ["features.align", "features.gapfill"].includes(base(up0));
    const al = isTable ? x.alignOf(up0) : null, alKey = al && x.keys.get(x.jk({ instance: al.id, item: null }));
    if (!alKey || samples === null || design === null || !await J().has(p, J().DIR + alKey + ".mzfeat")) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha(spec.key + "\n" + await J().sha(samples) + "\n" + await J().sha(design) + "\n" + alKey), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });
    return J().detached(x, spec, inst, async () => {
      let text;
      try { text = await p.ident("flowtab", "flow_drift", { params, table: await p.fs.read(J().DIR + alKey + ".mzfeat"), samples: enc(samples), design: enc(design) }); } catch (e) { throw asError(e); }
      const r = { name: inst.id + ".json", text };
      await F.webMsn.keep(r.name, r.text); await J().remember(p, key, r);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  F.webDrift = { run, STEPS };
})();
