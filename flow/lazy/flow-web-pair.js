// @ts-check
// Flow (administrator mode): features.pair in this browser. The feature table of each polarity (the alignment of the same plan kept in OPFS, or the table a `table.filter` view of it
// stands for: `F.liveJobs.tableOf`) goes to `flow_pair` of the `flowtab` .wasm (`mzlab_feat::pair::pair_text`, the function `flow run` calls), so the bytes are the same. Both outputs are kept
// like the two of features.union: `<id>.csv` (the pairs) and, under `<id>.pair`, `<id>.pair.json` (pair/1); the job cache holds one {name, text} per key, so the document is a second
// entry, `<key>-pair`, and a hit needs both. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const STEPS = ["features.pair"];
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key, params: j.params || {} }) : e; };

  /** features.pair @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), pos = await J().tableOf(x, p, inst, "pos"), neg = await J().tableOf(x, p, inst, "neg");
    if (!pos || !neg) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha(spec.key + "\n" + pos.key + "\n" + neg.key), hit = await J().recall(p, key), hitDoc = hit && await J().recall(p, key + "-pair");
    const keepBoth = async (/** @type {any} */ r) => {
      await F.webMsn.keep(r.name, r.text); await F.webMsn.keep(r.docName, r.doc);
      x.files.set(inst.id, { id: inst.id, name: r.name, text: r.text }); x.files.set(inst.id + ".pair", { id: inst.id, name: r.docName, text: r.doc });
    };
    if (hit && hitDoc) { await keepBoth({ ...hit, docName: hitDoc.name, doc: hitDoc.text }); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });
    return J().detached(x, spec, inst, async () => {
      let doc;
      try { doc = await p.ident("flowtab", "flow_pair", { params, posName: pos.name, negName: neg.name, pos: await pos.read(), neg: await neg.read() }); } catch (e) { throw asError(e); }
      const r = { name: inst.id + ".csv", text: JSON.parse(doc).csv, docName: inst.id + ".pair.json", doc };
      await keepBoth(r);
      await J().remember(p, key, { name: r.name, text: r.text }); await J().remember(p, key + "-pair", { name: r.docName, text: r.doc });
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  F.webPair = { run, STEPS };
})();
