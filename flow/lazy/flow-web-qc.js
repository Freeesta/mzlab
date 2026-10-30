// @ts-check
// Flow (administrator mode): qc.mass_accuracy in this browser. The check of the mass error of every file (and, with a model, the feature table with its m/z corrected) is `mass_accuracy` of the `table` .wasm,
// one call to the pool that reads the .mzidx of the files from OPFS: the Rust function is the one `flow run` calls, so the massacc/1 text and the corrected .mzfeat are the same bytes. The lab list of background ions is the
// file the site serves (contaminanti.json), the sample list and the CSV of reference ions are files of the plan (their text in S.samplesJson). The result is kept like the other texts, under the key of the job; the corrected table
// is kept beside it in OPFS under the SHA-256 of the massacc/1 text (the view offers it). Raw data and indexes are never written. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const STEPS = ["qc.mass_accuracy"];
  const J = () => F.liveJobs, kit = () => F.runners.web.kit;
  const enc = (/** @type {string} */ s) => new TextEncoder().encode(s);
  const hex = (/** @type {ArrayBuffer} */ b) => [...new Uint8Array(b)].map(x => x.toString(16).padStart(2, "0")).join("");
  const base = (/** @type {any} */ inst) => String(inst.step).split("@")[0];
  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key, params: j.params || {} }) : e; };

  /** where the corrected table of a massacc/1 text is kept @param {string} digest SHA-256 of the text */
  const tablePath = digest => J().DIR + "massacc-" + digest.slice(0, 32) + ".mzfeat";

  /** the lab list of background ions, as the site serves it (the same file `flow run` embeds) @type {Promise<Uint8Array> | null} */
  let listP = null;
  const contaminants = () => listP || (listP = fetch(new URL("static/contaminanti.json", document.baseURI)).then(r => (r.ok ? r.arrayBuffer() : Promise.reject(new Error("contaminanti.json")))).then(b => new Uint8Array(b)).catch(e => { listP = null; throw e; }));

  /** the text of a file of the plan on port `port` (null when the port is not wired, or the browser does not hold the file) @param {any} x @param {any} inst @param {string} port */
  const fileText = (x, inst, port) => {
    const ref = String(F.refs(inst, port)[0] || "");
    if (!ref.startsWith("file:")) return null;
    const t = x.S.samplesJson && x.S.samplesJson[ref.slice(5)];
    return typeof t === "string" ? t : null;
  };

  /** the files of the `raw` port: the indexes of the series, in the order of the items, with the name each is reported under @param {any} x @param {any} inst */
  const rawFiles = (x, inst) => {
    const ids = F.refs(inst, "raw").map((/** @type {any} */ r) => String(r)).filter((/** @type {string} */ r) => !r.startsWith("file:")).map((/** @type {string} */ r) => r.split(".")[0]);
    if (!ids.length || ids.some((/** @type {string} */ id) => { const up = x.inst(id); return !up || base(up) !== "index.mzidx"; })) return [];
    return x.items.map((/** @type {any} */ it) => ({ name: it.input, src: x.source(it) })).filter((/** @type {any} */ f) => f.src);
  };

  /** qc.mass_accuracy @param {any} x @param {any} spec @param {any} inst */
  const run = async (x, spec, inst) => {
    const j = J().job(spec), p = await kit().getPool(), files = rawFiles(x, inst);
    const wired = F.refs(inst, "table").length > 0, samples = fileText(x, inst, "samples"), csv = fileText(x, inst, "references");
    const up0 = wired ? x.inst(String(F.refs(inst, "table")[0]).split(".")[0]) : null, isTable = !!up0 && ["features.align", "features.gapfill"].includes(base(up0));
    const al = isTable ? x.alignOf(up0) : null, alKey = al && x.keys.get(x.jk({ instance: al.id, item: null }));
    const needs = (F.refs(inst, "samples").length > 0 && samples === null) || (F.refs(inst, "references").length > 0 && csv === null) || (wired && (!isTable || !alKey));
    if (!files.length || files.length !== x.items.length || needs) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    const key = await J().sha(spec.key + "\n" + await J().sha(samples || "") + "\n" + await J().sha(csv || "") + "\n" + (alKey || "")), hit = await J().recall(p, key);
    const model = ({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) }).model;
    if (hit && (model === "none" || !wired || await J().has(p, tablePath(await J().sha(hit.text))))) {
      x.files.set(inst.id, { id: inst.id, name: hit.name, text: hit.text }); await F.webMsn.keep(hit.name, hit.text); await x.call("cached", { job: j }); return {};
    }
    if (wired && !await J().has(p, J().DIR + alKey + ".mzfeat")) { await x.call("block", { job: j, by: "platform.computer" }); return {}; }
    await x.call("started", { job: j });
    const t0 = performance.now();
    return J().detached(x, spec, inst, async () => {
      const params = JSON.stringify({ ...F.defaults(inst, F.stepOf(x.S.reg, inst), true), ...(inst.params || {}) });
      let out;
      try {
        out = await p.ident("table", "mass_accuracy", { paths: files.map((/** @type {any} */ f) => f.src.path), names: JSON.stringify(files.map((/** @type {any} */ f) => [f.name, ""])), samples: samples === null ? null : enc(samples), csv: csv === null ? null : enc(csv),
          contaminants: await contaminants(), table: wired ? await p.fs.read(J().DIR + alKey + ".mzfeat") : null, params }, true);
      } catch (e) { throw asError(e); }
      const r = { name: inst.id + ".json", text: out.text };
      if (out.bytes && out.bytes.length) await p.fs.write(tablePath(await J().sha(r.text)), out.bytes.slice());
      await F.webMsn.keep(r.name, r.text); await J().remember(p, key, r);
      x.files.set(inst.id, { id: inst.id, ...r });
      await x.call("finished", { job: j, result: { ms: performance.now() - t0 } });
    });
  };

  // the CSV of reference ions: a file the user gives the plan (its text is the content of the input, S.samplesJson, with its SHA-256, like the other text inputs of a plan)
  F.extraResults.push((/** @type {any} */ S, /** @type {any} */ node) => {
    const insts = (node.ids || []).map((/** @type {string} */ id) => F.instanceOf(S.plan, id)).filter((/** @type {any} */ i) => i && base(i) === "qc.mass_accuracy");
    const refs = insts.flatMap((/** @type {any} */ i) => F.refs(i, "references")).filter((/** @type {string} */ r) => r.startsWith("file:"));
    if (!refs.length) return null;
    const { el } = F, say = el("p", { class: "fl-note", role: "status" });
    const pick = /** @type {HTMLInputElement} */ (el("input", { type: "file", hidden: true, accept: ".csv,.tsv,.txt", "aria-label": "Choose the CSV of reference ions", onchange: async (/** @type {any} */ e) => {
      const f = e.target.files && e.target.files[0]; if (!f) return;
      const text = await f.text(), input = S.plan.inputs.find((/** @type {any} */ i) => "file:" + i.name === refs[0]);
      if (!input) return;
      S.samplesJson = S.samplesJson || {}; S.samplesJson[input.name] = text;
      input.sha256 = hex(await crypto.subtle.digest("SHA-256", enc(text)));
      say.textContent = `${f.name}: ${text.length} characters kept as ${input.name}.`;
      if (S.redraw) S.redraw();
    } }));
    return el("div", { class: "fl-results" }, pick, F.btn("import", "Choose the reference ions", () => pick.click(), "Reference ions…"), say);
  });

  F.webQc = { run, STEPS, tablePath, hex };
  void G;
})();
