// @ts-check
// Flow (administrator mode): structure.render, structure.reactivity, structure.moiety and msn.figure in this browser. The texts are `flow_render` of the `interp` .wasm, `flow_reactivity` of the
// `chem` .wasm, `flow_moiety` of the `molkit` .wasm and `flow_figure` of the `msnfigura` .wasm (each loaded here only, when its step runs): `interp::redraw::render_text`,
// `mzlab_chem::flow::reactivity_text`, `chem::moiety::moiety_text` and `msnfigura::figure_text`, the functions `flow run` calls, so drawings/1, reatt/1, the CSV and the SVG or HTML are the
// same bytes. A PNG figure stays with the computer (resvg is native only). English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  // step -> the .wasm, its function, the input ports it reads (in the order of the arguments) and the extension of the output
  /** @type {Record<string, [string, string, string[], string]>} */
  const OPS = { "structure.render": ["interp", "flow_render", ["sites", "parent"], "json"], "structure.reactivity": ["chem", "flow_reactivity", ["parent"], "json"], "msn.figure": ["msnfigura", "flow_figure", ["graph"], ""], "structure.moiety": ["molkit", "flow_moiety", ["candidates"], "csv"] };
  const STEPS = Object.keys(OPS);
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  const enc = (/** @type {string} */ s) => new TextEncoder().encode(s);
  const base = (/** @type {any} */ inst) => String(inst.step).split("@")[0];
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key, params: j.params || {} }) : e; };

  /** structure.render | structure.reactivity | msn.figure @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const [mod, fn, ports, ext0] = OPS[base(inst)], j = J().job(spec), p = await kit().getPool();
    const params = { ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) }, ext = ext0 || String(params.format || "svg");
    const texts = await Promise.all(ports.map(port => F.webIdent.textOf(x, p, inst, port)));
    if (ext === "png" || texts.includes(null)) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const shas = await Promise.all(texts.map(t => J().sha(/** @type {any} */ (t).text)));
    const key = await J().sha([spec.key, ...shas].join("\n")), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now();
    return J().detached(x, spec, inst, async () => {
      /** @type {Record<string, any>} */
      const m = { params: JSON.stringify(params) };
      ports.forEach((port, i) => { m[port] = enc(/** @type {any} */ (texts[i]).text); });
      let text;
      try { text = await p.ident(mod, fn, m); } catch (e) { throw asError(e); }
      const r = { name: inst.id + "." + ext, text };
      await F.webMsn.keep(r.name, r.text); await J().remember(p, key, r);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  F.webStruct = { run, STEPS };
})();
