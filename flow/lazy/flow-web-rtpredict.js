// @ts-check
// Flow (administrator mode): rt.predict in this browser. The text is `flow_rtpredict` of the `attributio` .wasm (loaded here only, when this step runs): `mzlab_feat::flow_rtpredict::rtpredict_text`, the function
// `flow run` calls, so the candidates/1 text is the same bytes. The candidates are the result of a step of this plan (or a file of the plan); the standards (CSV smiles,rt) and the optional observed RTs (CSV query,rt)
// are files of the plan (their text in S.samplesJson): a .mzfeat file is not read here, the step then stays with the computer. The text is kept under the key of the job. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const STEPS = ["rt.predict"];
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  const enc = (/** @type {string} */ s) => new TextEncoder().encode(s);
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key, params: j.params || {} }) : e; };

  /** rt.predict @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), text = (/** @type {string} */ port) => F.webIdent.textOf(x, p, inst, port);
    const wired = F.refs(inst, "queries").length > 0, [cands, standards] = await Promise.all([text("candidates"), text("standards")]), queries = wired ? await text("queries") : null;
    const bad = (/** @type {any} */ f) => !f || f.text.startsWith("MZFEAT");
    if (bad(cands) || bad(standards) || (wired && bad(queries))) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const sha = (/** @type {any} */ f) => (f ? J().sha(f.text) : "");
    const key = await J().sha([spec.key, await sha(cands), await sha(standards), await sha(queries)].join("\n")), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });
    const alg = (F.effective(inst, F.stepOf(x.S.reg, inst)).algorithm || {}).id || "";
    return J().detached(x, spec, inst, async () => {
      let out;
      try { out = await p.ident("attributio_ms2", "flow_rtpredict", { params, algorithm: alg, candidates: enc(/** @type {any} */ (cands).text), standards: enc(/** @type {any} */ (standards).text), queries: enc(queries ? queries.text : "") }); } catch (e) { throw asError(e); }
      const r = { name: inst.id + ".json", text: out };
      await F.webMsn.keep(r.name, r.text); await J().remember(p, key, r);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  F.webRtPredict = { run, STEPS };
})();
