// @ts-check
// Flow (administrator mode): export.annotations in this browser. The CSV is `flow_export_annot` of the `flowtab` .wasm (module loaded here only, when this step runs): `mzlab_feat::flow_export_annot::export_text_packed`,
// the text `flow run` writes through the same core, so the bytes are the same. The table is the alignment (or gap filling) of the same plan, kept in OPFS; the candidates, screen, formulas and annot files are results of
// this plan made in this page (or files of the plan), packed with their names so that `source_file` is the one `flow run` writes. The text is kept like the other results, under the key of the job. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const STEPS = ["export.annotations"];
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  const enc = (/** @type {string} */ s) => new TextEncoder().encode(s);
  const base = (/** @type {any} */ inst) => String(inst.step).split("@")[0];
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key, params: j.params || {} }) : e; };
  const PORTS = ["candidates", "screen", "formulas", "annot"];

  /** export.annotations @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool();
    const up0 = x.inst(String(F.refs(inst, "table")[0] || "").split(".")[0]), isTable = !!up0 && ["features.align", "features.gapfill"].includes(base(up0));
    const al = isTable ? x.alignOf(up0) : null, alKey = al && x.keys.get(x.jk({ instance: al.id, item: null }));
    /** @type {{ port: string, name: string, text: string }[]} */ const files = [];
    let missing = false;
    for (const port of PORTS) {
      for (let k = 0; k < F.refs(inst, port).length; k++) {
        const t = await F.webIdent.textOf(x, p, inst, port, k);
        if (t) files.push({ port, name: t.name, text: t.text }); else missing = true;
      }
    }
    if (!alKey || missing || !await J().has(p, J().DIR + alKey + ".mzfeat")) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const parts = [];
    for (const f of files) parts.push(f.port + "\n" + f.name + "\n" + await J().sha(f.text));
    const key = await J().sha(spec.key + "\n" + parts.join("\n") + "\n" + alKey), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });
    return J().detached(x, spec, inst, async () => {
      const bytes = files.map(f => enc(f.text)), blob = new Uint8Array(bytes.reduce((n, b) => n + b.length, 0));
      let at = 0; for (const b of bytes) { blob.set(b, at); at += b.length; }
      const manifest = JSON.stringify(files.map((f, i) => ({ port: f.port, name: f.name, len: bytes[i].length })));
      let text;
      try { text = await p.ident("flowtab", "flow_export_annot", { params, table: await p.fs.read(J().DIR + alKey + ".mzfeat"), manifest, blob }); } catch (e) { throw asError(e); }
      const r = { name: inst.id + ".csv", text };
      await F.webMsn.keep(r.name, r.text); await J().remember(p, key, r);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  F.webExportAnnot = { run, STEPS };
})();
