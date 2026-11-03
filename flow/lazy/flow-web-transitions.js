// @ts-check
// Flow (administrator mode): chrom.transitions in this browser. The mzML files of the plan open in Data are packed in one call to `flow_transitions` of the `flowtab` .wasm
// (`mzlab_feat::transitions::transitions_packed`, over the same text `flow run` writes), so the CSV is the same bytes. A source that is not an mzML open in Data (a raw file to
// convert first) stays with the computer. The text is kept like the other results, under the key of the job. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const STEPS = ["chrom.transitions"];
  /** the most mzML bytes the page reads into memory for one run of the step; more stays with the computer */
  const MAX_MZML = 128 * 1024 * 1024;
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key, params: j.params || {} }) : e; };

  /** chrom.transitions @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), key = await J().sha(spec.key), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    const names = F.refs(inst, "source").map(String), blobOf = G.MZLAB_RUST && G.MZLAB_RUST.blobOf;
    const blobs = blobOf && names.length && names.every((/** @type {string} */ r) => r.startsWith("file:")) ? names.map((/** @type {string} */ r) => blobOf(r.slice(5))) : [];
    if (!blobs.length || blobs.some((/** @type {any} */ b) => !b) || blobs.reduce((/** @type {number} */ n, /** @type {any} */ b) => n + b.size, 0) > MAX_MZML) {
      await x.call("block", { job: j, by: "platform.computer" }); return {};
    }
    await x.call("started", { job: j });
    const t0 = performance.now(), all = { ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) };
    const table = typeof all.transitions === "string" ? all.transitions : "", params = JSON.stringify(all);
    return J().detached(x, spec, inst, async () => {
      const parts = await Promise.all(blobs.map(async (/** @type {Blob} */ b) => new Uint8Array(await b.arrayBuffer())));
      const manifest = JSON.stringify(parts.map((b, k) => ({ name: names[k].slice(5), len: b.length })));
      const blob = new Uint8Array(parts.reduce((n, b) => n + b.length, 0));
      parts.reduce((at, b) => { blob.set(b, at); return at + b.length; }, 0);
      let text;
      try { text = await p.ident("flowtab", "flow_transitions", { params, table, manifest, blob }); } catch (e) { throw asError(e); }
      const r = { name: inst.id + ".csv", text };
      await F.webMsn.keep(r.name, r.text); await J().remember(p, key, r);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  F.webTransitions = { run, STEPS };
})();
