// @ts-check
// Flow (administrator mode): quant.targets in this browser. The target list is integrated over the .mzidx of every file of the series by `flow_quant_targets` of the
// `quanttargets` .wasm (`mzlab_feat::quant_targets::run`, the function `flow run` and `mzlab-cli quant targets` call, one pass per file), one call to the pool that reads
// the indexes from OPFS, so the table and the log are the same bytes (`tests/test_quant_targets_wasm.py`). The module is fetched only when this step runs. The targets are a
// file of the plan (its text in S.samplesJson) or the text an earlier step kept; both outputs are kept (`<id>.csv`, and `<id>.log.json` under `<id>.log`). English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const STEPS = ["quant.targets"];
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  const enc = (/** @type {string} */ s) => new TextEncoder().encode(s);
  const base = (/** @type {any} */ inst) => String(inst.step).split("@")[0];
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key, params: j.params || {} }) : e; };
  /** the words of the warnings of the step on its card, none when it has none @param {any} x @param {any} inst @param {string[] | undefined} w */
  const note = (x, inst, w) => { if (w && w.length) x.notes.set(inst.id, w.map(k => F.message({ key: k, params: {} })).join(" ")); else x.notes.delete(inst.id); };

  /** the text on port `port`: a file of the plan, or the text an earlier step kept (null when neither is there) @param {any} x @param {any} inst @param {string} port */
  const portText = (x, inst, port) => {
    const ref = String(F.refs(inst, port)[0] || "");
    if (ref.startsWith("file:")) { const t = x.S.samplesJson && x.S.samplesJson[ref.slice(5)]; return typeof t === "string" ? t : null; }
    const up = ref ? x.files.get(ref) || x.files.get(ref.split(".")[0]) : null;
    return up && typeof up.text === "string" ? up.text : null;
  };

  /** the files of the `raw` port: the indexes of the series made by an index.mzidx step, in the order of the items, with the name each is reported under @param {any} x @param {any} inst */
  const rawFiles = (x, inst) => {
    const ids = F.refs(inst, "raw").map((/** @type {any} */ r) => String(r)).filter((/** @type {string} */ r) => !r.startsWith("file:")).map((/** @type {string} */ r) => r.split(".")[0]);
    if (!ids.length || ids.some((/** @type {string} */ id) => { const up = x.inst(id); return !up || base(up) !== "index.mzidx"; })) return [];
    return x.items.map((/** @type {any} */ it) => ({ name: it.input, src: x.source(it) })).filter((/** @type {any} */ f) => f.src);
  };

  /** quant.targets @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), files = rawFiles(x, inst), targets = portText(x, inst, "targets");
    if (!files.length || files.length !== x.items.length || targets === null) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    // the job cache holds one {name, text} per key: the log is a second entry, `<key>-log`, and a hit needs both
    const key = await J().sha(spec.key + "\n" + await J().sha(targets)), hit = await J().recall(p, key), hitLog = hit && await J().recall(p, key + "-log");
    const keepBoth = async (/** @type {any} */ r) => {
      await F.webMsn.keep(r.name, r.text); await F.webMsn.keep(r.logName, r.log);
      x.files.set(inst.id, { id: inst.id, name: r.name, text: r.text }); x.files.set(inst.id + ".log", { id: inst.id, name: r.logName, text: r.log });
    };
    if (hit && hitLog) { await keepBoth({ ...hit, logName: hitLog.name, log: hitLog.text }); note(x, inst, hit.warnings); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });
    return J().detached(x, spec, inst, async () => {
      let out;
      try {
        out = await p.ident("quanttargets", "flow_quant_targets", { paths: files.map((/** @type {any} */ f) => f.src.path), names: JSON.stringify(files.map((/** @type {any} */ f) => f.name)), targets: enc(targets), params }, true);
      } catch (e) { throw asError(e); }
      const warnings = out.warnings || [], r = { name: inst.id + ".csv", text: out.text, logName: inst.id + ".log.json", log: out.log };
      await keepBoth(r);
      await J().remember(p, key, { name: r.name, text: r.text, warnings }); await J().remember(p, key + "-log", { name: r.logName, text: r.log });
      note(x, inst, warnings);
      await x.call("finished", { job: j, result: warnings.length ? { ms: performance.now() - t0, warnings } : { ms: performance.now() - t0 } });
    });
  };

  F.webQuantTargets = { run, STEPS };
})();
