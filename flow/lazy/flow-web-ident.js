// @ts-check
// Flow (administrator mode): the identification steps that run in this browser. spectra.ms2 reads the mzML open in Data (`flow_ms2` of the `hr` .wasm, one file per call, in the pool),
// spectra.match with the algorithms other than `tree` runs `flow_match` of the `lib` .wasm, table.match builds the MGF of the rows of the alignment with the `table` .wasm (`table_match_rows`, in the pool,
// the indexes read from OPFS) and then matches it the same way; formula.ms2 and
// annot.level run `flow_formula_ms2` and `flow_level` of the `attributio` .wasm on the texts of the steps upstream: the Rust functions are the ones `flow run` calls, so the text is the same byte for byte. The library is a file the user gave
// the plan (its text in S.samplesJson, like a parent or a sample list); nothing is bundled or fetched. Results are kept in OPFS under the key of the job (flow-live-jobs.js). English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const STEPS = ["spectra.ms2", "spectra.match", "table.match", "formula.ms1", "formula.ms2", "annot.level"];
  /** the largest mzML the page reads into memory for the step (bytes); a bigger file stays with the computer */
  const MAX_MZML = 256 * 1024 * 1024;
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  const base = (/** @type {any} */ inst) => String(inst.step).split("@")[0];
  const enc = (/** @type {string} */ s) => new TextEncoder().encode(s);
  const fail = (/** @type {string} */ key, /** @type {any} */ params = {}) => Object.assign(new Error(key), { key, params });
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? fail(j.error_key, j.params || {}) : e; };

  /** @type {Promise<any> | null} */ let libP = null;
  /** the `lib` .wasm (the one of the library page), loaded once */
  const lib = () => libP || (libP = (async () => {
    const c = await F.tpCtx("regole_tp_2.json"), mod = await import(/* webpackIgnore: true */ new URL("wasm-lib/mzlab_lib_wasm.js", c.base).href);
    await mod.default();
    return mod;
  })().catch(e => { libP = null; throw e; }));

  /** the resolved parameters of the instance as the text the .wasm takes @param {any} x @param {any} inst */
  const paramsOf = (x, inst) => JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });
  /** the id of the algorithm the instance runs ("" = the default of the matcher) @param {any} x @param {any} inst */
  const algorithmOf = (x, inst) => { const a = F.effective(inst, F.stepOf(x.S.reg, inst)).algorithm; return (a && a.id) || ""; };

  /** the text an input port holds: a file of the plan (its content in S.samplesJson) or the result of a step of this plan made in this page; null when the browser does not have it
   * @param {any} x @param {any} p the pool @param {any} inst @param {string} port @param {number} [k] which file of a port that takes several @returns {Promise<{ name: string, text: string } | null>} */
  const textOf = async (x, p, inst, port, k = 0) => {
    const ref = String(F.refs(inst, port)[k] || "");
    if (!ref) return null;
    if (ref.startsWith("file:")) { const n = ref.slice(5), t = x.S.samplesJson && x.S.samplesJson[n]; return typeof t === "string" ? { name: n, text: t } : null; }
    const id = ref.split(".")[0], made = x.files.get(id);
    if (made) return { name: made.name, text: made.text };
    const key = x.keys.get(x.jk({ instance: id, item: null })), hit = key ? await J().recall(p, await J().sha(key)) : null;
    return hit ? { name: hit.name, text: hit.text } : null;
  };

  /** spectra.ms2: the MGF of the MS2 scans of the mzML files of the plan that are open in Data, with the feature table (a CSV of export.table_csv) when the plan wires one
   * @param {any} x @param {any} spec @param {any} inst */
  const ms2 = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), key = await J().sha(spec.key), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    const names = F.refs(inst, "source").map(String), blobOf = G.MZLAB_RUST && G.MZLAB_RUST.blobOf;
    const blobs = blobOf && names.length && names.every((/** @type {string} */ r) => r.startsWith("file:")) ? names.map((/** @type {string} */ r) => blobOf(r.slice(5))) : [];
    const wired = F.refs(inst, "features").length > 0, feat = wired ? await textOf(x, p, inst, "features") : null;
    if (!blobs.length || blobs.some((/** @type {any} */ b) => !b) || blobs.reduce((/** @type {number} */ n, /** @type {any} */ b) => n + b.size, 0) > MAX_MZML || (wired && !feat)) {
      await x.call("block", { job: j, by: "platform.computer" }); return {};
    }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = paramsOf(x, inst), features = feat ? enc(feat.text) : null;
    return J().detached(x, spec, inst, async () => {
      let text = "";
      for (let k = 0; k < blobs.length; k++) {
        const mzml = new Uint8Array(await blobs[k].arrayBuffer());
        try { text += await p.ident("proofs", "flow_ms2", { params, name: names[k].slice(5), mzml, features }); } catch (e) { throw asError(e); }
        x.call("progress", { job: j, frac: (k + 1) / blobs.length }).catch(() => {});
      }
      const r = { name: inst.id + ".mgf", text };
      await F.webMsn.keep(r.name, r.text); await J().remember(p, key, r);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  /** the result of a match job kept like the other texts, under the key of the job and the library (the plan does not hold the library) @param {any} x @param {any} spec @param {any} inst @param {string} key
   * @param {number} t0 @param {string} text */
  const keep = async (x, spec, inst, key, t0, text) => {
    const p = await kit().getPool(), r = { name: inst.id + ".json", text };
    await F.webMsn.keep(r.name, r.text); await J().remember(p, key, r);
    x.files.set(inst.id, { id: inst.id, ...r });
    await x.call("finished", { job: J().job(spec), result: { ms: performance.now() - t0 } });
  };

  /** spectra.match with the algorithms other than `tree`: the query (the MGF of a spectra.ms2 of this plan, or a file of the plan) against the library file of the plan @param {any} x @param {any} spec @param {any} inst */
  const match = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), query = await textOf(x, p, inst, "query"), library = await textOf(x, p, inst, "library");
    if (!query || !library) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha(spec.key + "\n" + await J().sha(library.text)), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = paramsOf(x, inst), alg = algorithmOf(x, inst);
    return J().detached(x, spec, inst, async () => {
      const lb = await lib();
      let text;
      try { text = lb.flow_match("spectra.match", alg, params, query.name, enc(query.text), library.name, enc(library.text)); } catch (e) { throw asError(e); }
      await keep(x, spec, inst, key, t0, text);
    });
  };

  /** table.match: the MGF of the rows of the alignment that have an MS2 spectrum (the `table` .wasm reads the indexes of the files), then the same matcher as spectra.match. A table that is not this
   * plan's alignment, or without samples, indexes or library, stays with the computer @param {any} x @param {any} spec @param {any} inst */
  const tableMatch = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), library = await textOf(x, p, inst, "library");
    const up0 = x.inst(String(F.refs(inst, "table")[0] || "").split(".")[0]), isTable = !!up0 && ["features.align", "features.gapfill"].includes(base(up0));
    const al = isTable ? x.alignOf(up0) : null, alKey = al && x.keys.get(x.jk({ instance: al.id, item: null })), up = al ? x.upstream(al) : [];
    const sref = (F.refs(inst, "samples")[0] || "").replace(/^file:/, ""), samples = x.S.samplesJson && x.S.samplesJson[sref];
    if (!library || !alKey || !samples || !F.refs(inst, "index").length || !up.length || !await J().has(p, J().DIR + alKey + ".mzfeat")) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha(spec.key + "\n" + await J().sha(library.text)), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = paramsOf(x, inst), alg = algorithmOf(x, inst);
    return J().detached(x, spec, inst, async () => {
      const table = await p.fs.read(J().DIR + alKey + ".mzfeat"), hex = [...new Uint8Array(await crypto.subtle.digest("SHA-256", table))].map(b => b.toString(16).padStart(2, "0")).join("");
      let mgf, text;
      try {
        mgf = await p.ident("table", "table_match_rows", { table, samples, params, sha: hex, paths: up.map((/** @type {any} */ u) => u.path) });
        text = (await lib()).flow_match("table.match", alg, params, "rows.mgf", enc(mgf), library.name, enc(library.text));
      } catch (e) { throw asError(e); }
      await keep(x, spec, inst, key, t0, text);
    });
  };

  /** formula.ms2: the sub-formulas of the candidates that explain the peaks of the MGF of a spectra.ms2 of this plan @param {any} x @param {any} spec @param {any} inst */
  const ms2Formula = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), spectra = await textOf(x, p, inst, "spectra"), formulas = await textOf(x, p, inst, "formulas");
    if (!spectra || !formulas) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha(spec.key + "\n" + await J().sha(spectra.text) + "\n" + await J().sha(formulas.text)), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = paramsOf(x, inst), alg = algorithmOf(x, inst);
    return J().detached(x, spec, inst, async () => {
      let text;
      try { text = await p.ident("attributio_ms2", "flow_formula_ms2", { alg, params, name: spectra.name, mgf: enc(spectra.text), formulas: enc(formulas.text) }); } catch (e) { throw asError(e); }
      await keep(x, spec, inst, key, t0, text);
    });
  };

  /** formula.ms1: the candidate formulas of each feature of the CSV of a table export of this plan (or a file of the plan) from the MS1 scan at its apex, in the mzML where the feature is largest.
   * The `formula` .wasm is loaded here only; one mzML per call to `flow_ms1_rows`, then `flow_ms1_join` in the order of the table @param {any} x @param {any} spec @param {any} inst */
  const ms1Formula = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), feat = await textOf(x, p, inst, "features");
    const names = F.refs(inst, "index").map(String), blobOf = G.MZLAB_RUST && G.MZLAB_RUST.blobOf;
    const blobs = blobOf && names.length && names.every((/** @type {string} */ r) => r.startsWith("file:")) ? names.map((/** @type {string} */ r) => blobOf(r.slice(5))) : [];
    if (!feat || !blobs.length || blobs.some((/** @type {any} */ b) => !b) || blobs.reduce((/** @type {number} */ n, /** @type {any} */ b) => n + b.size, 0) > MAX_MZML) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha(spec.key + "\n" + await J().sha(feat.text)), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = paramsOf(x, inst), features = enc(feat.text), fileNames = JSON.stringify(names.map((/** @type {string} */ r) => r.slice(5)));
    return J().detached(x, spec, inst, async () => {
      /** @type {string[]} */ const rows = [];
      for (let k = 0; k < blobs.length; k++) {
        const mzml = new Uint8Array(await blobs[k].arrayBuffer());
        let part;
        try { part = await p.ident("formula", "flow_ms1_rows", { params, features, names: fileNames, index: k, mzml }); } catch (e) { throw asError(e); }
        for (const line of part.split("\n")) { if (!line) continue; const t = line.indexOf("\t"); rows[Number(line.slice(0, t))] = line.slice(t + 1); }
        x.call("progress", { job: j, frac: (k + 1) / blobs.length }).catch(() => {});
      }
      let text;
      try { text = await p.ident("formula", "flow_ms1_join", { rows: Array.from(rows, r => r || "").join("\n") }); } catch (e) { throw asError(e); }
      await keep(x, spec, inst, key, t0, text);
    });
  };

  // flow-live-jobs.js is loaded after this file: its table of executors takes formula.ms1 when it is set
  /** @type {any} */ let jobs;
  Object.defineProperty(F, "liveJobs", { configurable: true, get: () => jobs, set: v => { jobs = v && Object.assign(v, { "formula.ms1": (/** @type {any[]} */ ...a) => F.webIdent.ms1Formula(...a) }); } });

  /** annot.level: the level of each query from the hits of a spectra.match of this plan (and the screen/1 of a screen.suspects, the same port), the candidate formulas, and the optional standards file and MS2 spectra (the RT of each query)
   * @param {any} x @param {any} spec @param {any} inst */
  const level = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), formulas = await textOf(x, p, inst, "formulas");
    const wired = (/** @type {string} */ port) => F.refs(inst, port).length > 0;
    const files = await Promise.all(F.refs(inst, "candidates").map((/** @type {any} */ _, /** @type {number} */ k) => textOf(x, p, inst, "candidates", k)));
    const standards = wired("standards") ? await textOf(x, p, inst, "standards") : null, spectra = wired("spectra") ? await textOf(x, p, inst, "spectra") : null;
    if (!files.length || files.some((/** @type {any} */ f) => !f) || !formulas || (wired("standards") && !standards) || (wired("spectra") && !spectra)) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const isScreen = (/** @type {any} */ f) => f.text.startsWith('{"format":"screen/1"');
    const hits = files.find((/** @type {any} */ f) => !isScreen(f)), screen = files.find(isScreen), sha = (/** @type {any} */ f) => f ? J().sha(f.text) : "";
    if (!hits) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha([spec.key, await sha(hits), await sha(formulas), await sha(screen), await sha(standards), await sha(spectra)].join("\n")), hit = await J().recall(p, key);
    if (hit) { x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now(), params = paramsOf(x, inst), bytes = (/** @type {any} */ f) => enc(f ? f.text : "");
    return J().detached(x, spec, inst, async () => {
      let text;
      try { text = await p.ident("attributio_ms2", "flow_level", { params, hits: bytes(hits), screen: bytes(screen), formulas: bytes(formulas), standards: bytes(standards), spectra: bytes(spectra) }); } catch (e) { throw asError(e); }
      await keep(x, spec, inst, key, t0, text);
    });
  };

  // a library file for the match steps: its text is the content of the input (S.samplesJson) with its SHA-256, like the other text inputs of a plan
  F.extraResults.push((/** @type {any} */ S, /** @type {any} */ node) => {
    const insts = (node.ids || []).map((/** @type {string} */ id) => F.instanceOf(S.plan, id)).filter((/** @type {any} */ i) => i && ["spectra.match", "table.match"].includes(base(i)));
    const refs = insts.flatMap((/** @type {any} */ i) => F.refs(i, "library")).filter((/** @type {string} */ r) => r.startsWith("file:"));
    if (!refs.length) return null;
    const { el } = F, say = el("p", { class: "fl-note", role: "status" });
    const pick = /** @type {HTMLInputElement} */ (el("input", { type: "file", hidden: true, accept: ".msp,.mgf,.json,.txt", "aria-label": "Choose a spectral library file", onchange: async (/** @type {any} */ e) => {
      const f = e.target.files && e.target.files[0]; if (!f) return;
      const text = await f.text(), input = S.plan.inputs.find((/** @type {any} */ i) => "file:" + i.name === refs[0]);
      if (!input) return;
      S.samplesJson = S.samplesJson || {}; S.samplesJson[input.name] = text;
      input.sha256 = [...new Uint8Array(await crypto.subtle.digest("SHA-256", enc(text)))].map(b => b.toString(16).padStart(2, "0")).join("");
      say.textContent = `${f.name}: ${text.length} characters kept as ${input.name}.`;
      if (S.redraw) S.redraw();
    } }));
    return el("div", { class: "fl-results" }, pick, F.btn("import", "Choose the library file", () => pick.click(), "Library file…"), say);
  });

  F.webIdent = { ms2, match, tableMatch, ms1Formula, ms2Formula, level, textOf, STEPS };
})();
