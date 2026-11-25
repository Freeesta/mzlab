// @ts-check
// Flow (administrator mode): features.union in this browser. The feature tables of the series (one alignment of the same plan per series, kept in OPFS, or the table a `table.filter` view of it
// stands for: `F.liveJobs.tableOf`) are joined into one target list (CSV) and its union/1 document by `flow_union` of the `flowtab` .wasm (loaded here only, when this step runs):
// `mzlab_feat::union::union_text`, the function `flow run` calls, so the bytes are the same. Both are kept like the two outputs of quant.targets: `<id>.csv` and, under `<id>.union`, `<id>.union.json`;
// the job cache holds one {name, text} per key, so the document is a second entry, `<key>-union`, and a hit needs both. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const STEPS = ["features.union"];
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key, params: j.params || {} }) : e; };

  /** features.union @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), n = F.refs(inst, "table").length, tabs = [];
    for (let k = 0; k < n; k++) { const t = await J().tableOf(x, p, inst, "table", k); if (!t) { await x.call("block", { job: j, by: "platform.computer" }); return {}; } tabs.push(t); }
    if (!tabs.length) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha(spec.key + "\n" + tabs.map(t => t.key).join("\n")), hit = await J().recall(p, key), hitDoc = hit && await J().recall(p, key + "-union");
    const keepBoth = async (/** @type {any} */ r) => {
      await F.webMsn.keep(r.name, r.text); await F.webMsn.keep(r.docName, r.doc);
      x.files.set(inst.id, { id: inst.id, name: r.name, text: r.text }); x.files.set(inst.id + ".union", { id: inst.id, name: r.docName, text: r.doc });
    };
    if (hit && hitDoc) { await keepBoth({ ...hit, docName: hitDoc.name, doc: hitDoc.text }); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });
    return J().detached(x, spec, inst, async () => {
      const bytes = await Promise.all(tabs.map(t => t.read())), all = new Uint8Array(bytes.reduce((a, b) => a + b.length, 0));
      bytes.reduce((at, b) => { all.set(b, at); return at + b.length; }, 0);
      let made;
      try { made = await p.ident("flowtab", "flow_union", { params, names: JSON.stringify(tabs.map(t => t.name)), lens: JSON.stringify(bytes.map(b => b.length)), tables: all }); } catch (e) { throw asError(e); }
      const r = { name: inst.id + ".csv", text: made.csv, docName: inst.id + ".union.json", doc: made.doc };
      await keepBoth(r);
      await J().remember(p, key, { name: r.name, text: r.text }); await J().remember(p, key + "-union", { name: r.docName, text: r.doc });
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  F.webUnion = { run, STEPS };
})();
