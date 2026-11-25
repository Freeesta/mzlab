// @ts-check
// Flow (administrator mode): features.union in this browser. The feature tables of the series (one alignment of the same plan per series, kept in OPFS, or the table a `table.filter` view of it
// stands for: `F.liveJobs.tableOf`) are written as CSV by `flow_series_csv` of the `flowtab` .wasm (`mzlab_feat::union::series_csv`) and joined by `union_tables` of the `rotte` .wasm (the rust worker op `lr_union`:
// `series_union::run`, the function `flow run` calls), so the bytes are the same and flowtab does not carry the join. Both are kept like the two outputs of quant.targets: `<id>.csv` and, under `<id>.union`,
// `<id>.union.json`; the job cache holds one {name, text} per key, so the document is a second entry, `<key>-union`, and a hit needs both. English only.
(() => {
  "use strict";
  const G = /** @type {any} */ (globalThis), F = G.FLOW;
  const STEPS = ["features.union"];
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key, params: j.params || {} }) : e; };

  /** features.union @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), n = F.refs(inst, "table").length, tabs = [], rb = await G.MZLAB_RUST_READY;
    if (!rb || rb.disabled || !rb.rpc) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    for (let k = 0; k < n; k++) { const t = await J().tableOf(x, p, inst, "table", k); if (!t) { await x.call("block", { job: j, by: "platform.computer" }); return {}; } tabs.push(t); }
    if (!tabs.length) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha(spec.key + "\n" + tabs.map(t => t.key).join("\n")), hit = await J().recall(p, key), hitDoc = hit && await J().recall(p, key + "-union");
    const keepBoth = async (/** @type {any} */ r) => {
      await F.webMsn.keep(r.name, r.text); await F.webMsn.keep(r.docName, r.doc);
      x.files.set(inst.id, { id: inst.id, name: r.name, text: r.text }); x.files.set(inst.id + ".union", { id: inst.id, name: r.docName, text: r.doc });
    };
    if (hit && hitDoc) { await keepBoth({ ...hit, docName: hitDoc.name, doc: hitDoc.text }); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = { ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) };
    return J().detached(x, spec, inst, async () => {
      let made;
      try {
        const texts = await Promise.all(tabs.map(async t => p.ident("flowtab", "flow_series_csv", { table: await t.read() })));
        const got = await rb.rpc({ op: "lr_union", body: JSON.stringify({ tables: tabs.map((t, k) => ({ name: t.name, text: texts[k] })), params }) });
        if (got.dead || got.error) throw got.error || "error.featFailed";
        made = { doc: got.info, csv: JSON.parse(got.info).csv };
      } catch (e) { throw asError(e); }
      const r = { name: inst.id + ".csv", text: made.csv, docName: inst.id + ".union.json", doc: made.doc };
      await keepBoth(r);
      await J().remember(p, key, { name: r.name, text: r.text }); await J().remember(p, key + "-union", { name: r.docName, text: r.doc });
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  F.webUnion = { run, STEPS };
})();
