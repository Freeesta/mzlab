// @ts-check
// Flow (administrator mode): features.group in this browser. The grouping of the rows of a per-ion table into compounds (isotopes, ion species, ionnet/1) is `flow_group` of the `flowtab`
// .wasm (loaded here only, when this step runs): `mzlab_feat::ionnet::group_text`, the function `flow run` calls, so the text is the same bytes. The table is the alignment (or gap filling)
// of the same plan, kept in OPFS; the text is kept like the other results, under the key of the job. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const STEPS = ["features.group"];
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  const base = (/** @type {any} */ inst) => String(inst.step).split("@")[0];
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key, params: j.params || {} }) : e; };

  /** features.group @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool();
    const up0 = x.inst(String(F.refs(inst, "table")[0] || "").split(".")[0]), isTable = !!up0 && ["features.align", "features.gapfill"].includes(base(up0));
    const al = isTable ? x.alignOf(up0) : null, alKey = al && x.keys.get(x.jk({ instance: al.id, item: null }));
    if (!alKey || !await J().has(p, J().DIR + alKey + ".mzfeat")) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha(spec.key + "\n" + alKey), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });
    return J().detached(x, spec, inst, async () => {
      let text;
      try { text = await p.ident("flowtab", "flow_group", { params, table: await p.fs.read(J().DIR + alKey + ".mzfeat") }); } catch (e) { throw asError(e); }
      const r = { name: inst.id + ".json", text };
      await F.webMsn.keep(r.name, r.text); await J().remember(p, key, r);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  F.webGroup = { run, STEPS };
})();
