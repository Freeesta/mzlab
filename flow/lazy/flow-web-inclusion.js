// @ts-check
// Flow (administrator mode): export.inclusion in this browser. The inclusion list is `flow_inclusion` of the `interp` .wasm (module loaded here only, when this step runs):
// `mzlab_feat::export_inclusione_tp::inclusion_csv_of`, the function `flow run` calls, so the CSV is the same bytes. The products are the `.tp.mzfeat` of the tp.link of the plan (OPFS) or a JSON
// list of the plan (its text in S.samplesJson); an unwritten `adducts` takes [M-H]- for products linked in negative mode. The CSV is kept like the other results, under the key of the job. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const STEPS = ["export.inclusion"];
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  const enc = (/** @type {string} */ s) => new TextEncoder().encode(s);
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key, params: j.params || {} }) : e; };

  /** the products on port `tp`: the bytes of the `.tp.mzfeat` of a tp.link of the plan, or of a JSON file of the plan (null when the browser does not hold them) @param {any} x @param {any} p @param {any} inst */
  const products = async (x, p, inst) => {
    const ref = String(F.refs(inst, "tp")[0] || "");
    if (ref.startsWith("file:")) { const t = x.S.samplesJson && x.S.samplesJson[ref.slice(5)]; return typeof t === "string" ? { bytes: enc(t), salt: t } : null; }
    const up = x.inst(ref.split(".")[0]), key = up && String(up.step).split("@")[0] === "tp.link" && x.keys.get(x.jk({ instance: up.id, item: null })), path = J().DIR + key + ".tp.mzfeat";
    return key && await J().has(p, path) ? { bytes: await p.fs.read(path), salt: String(key) } : null;
  };

  /** export.inclusion @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), tp = await products(x, p, inst);
    if (!tp) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha(spec.key + "\n" + await J().sha(tp.salt)), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });
    return J().detached(x, spec, inst, async () => {
      let text;
      try { text = await p.ident("interp", "flow_inclusion", { params, products: tp.bytes }); } catch (e) { throw asError(e); }
      const r = { name: inst.id + ".csv", text };
      await F.webMsn.keep(r.name, r.text); await J().remember(p, key, r);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  F.webInclusion = { run, STEPS };
})();
