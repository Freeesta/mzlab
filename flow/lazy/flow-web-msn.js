// @ts-check
// Flow (administrator mode): the functions of the MSn steps (spectra.msn_graph, msn.library_export, msn.table_export, and spectra.match with the `tree` algorithm). The infusion is the file the plan names that is open in the Data tab;
// the graph, the library and the tables come from the routes of the engine of the page (`api/msn_grafo`, `api/msn_grafo/export`: the .wasm msngrafo, the same functions as `mzlab-cli msn-graph`
// and `msn-lib export`; the library search posts the library of the user, the one the Fragmentation graph card keeps, to `api/msnlib/cerca`), so the bytes are the ones of the command. Reports the same events as flow-web-run.js, keeps each result in OPFS (mzlab/flow/) and resolves with the texts. Docs: docs/agenti/flow-web.md.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const STEPS = ["spectra.msn_graph", "msn.library_export", "msn.table_export", "spectra.match"], DIR = "mzlab/flow/";
  // the library the Fragmentation graph card keeps in OPFS (msnlib/1), and the defaults of spectra.match that the command applies
  const LIB_FILE = "msnlib.json", MATCH = { precursor_ppm: 10, tol_da: 0.02, top_n: 5, min_prove: 3 };
  const base = (/** @type {any} */ inst) => String(inst.step).split("@")[0];
  const fail = (/** @type {string} */ key, params = {}) => Object.assign(new Error(key), { key, params });
  /** @param {any} o @param {any} ev */
  const emit = (o, ev) => { if (o.onEvent) o.onEvent(ev); document.dispatchEvent(new CustomEvent("flow:web-step", { detail: ev })); };

  /** the query of the graph of `inst` for the file number `k`: the parameters of the step under the names of the route @param {any} inst @param {number} k */
  const graphQuery = (inst, k) => {
    const p = inst.params || {}, q = new URLSearchParams({ k: String(k) });
    const text = (/** @type {string} */ n) => (typeof p[n] === "string" ? p[n].trim() : "");
    let ion = text("formula");
    if (!ion && text("molecule")) throw fail("err.flow.msn.molecule");
    if (ion && text("adduct") && !/[+\-[]/.test(ion)) ion += " " + text("adduct");
    if (ion) q.set("formula", ion);
    if (ion && text("smiles")) q.set("smiles", text("smiles"));
    if (text("attivazione")) q.set("attivazione", text("attivazione"));
    for (const n of ["ppm_formula", "mz_basso", "ppm_basso", "prove_min"]) if (typeof p[n] === "number") q.set(n, String(p[n]));
    if (p.isotopi === true) q.set("isotopi", "1");
    if (text("scelte")) q.set("scelte", JSON.stringify(text("scelte").split(",").map(s => { const [m, f] = s.split("="); return { mz: Number(m), formula: (f || "").trim() }; })));
    if (p.interpreta === true) { q.set("interpreta", "1"); if (typeof p.ppm_perdita === "number") q.set("ppm_perdita", String(p.ppm_perdita)); }
    if (text("strutture_nodo")) { q.set("strutture", text("strutture_nodo")); if (typeof p.ppm_struttura === "number") q.set("ppm_struttura", String(p.ppm_struttura)); }
    return q;
  };

  /** the text an engine route answers; an error answer becomes {key, params}, a refusal of the memory the message to use the desktop app @param {string} url */
  const ask = async url => {
    let t;
    try { t = await (await fetch(url)).text(); } catch (_) { throw fail("error.engineFallback"); }
    if (/^\s*\{\s*"error/.test(t)) {
      let j = /** @type {any} */ ({});
      try { j = JSON.parse(t); } catch (_) { /* not JSON: shown as it is */ }
      const key = j.error_key || "err.flow.run";
      throw /memory/i.test(key) ? fail("err.flow.msn.desktop") : fail(key, j.params || {});
    }
    return t;
  };

  /** the library of the user (msnlib/1), "" when there is none yet */
  const library = async () => {
    try { return await (await (await (await navigator.storage.getDirectory()).getFileHandle(LIB_FILE)).getFile()).text(); } catch (_) { return ""; }
  };

  /** the candidates (msncerca/1) of the library for the graph of `q`, with the options of the step @param {URLSearchParams} q @param {any} p */
  const search = async (q, p) => {
    const lib = await library();
    if (!lib) throw fail("err.flow.msn.library");
    const body = /** @type {Record<string, any>} */ ({ libreria: lib });
    for (const [n, v] of q) if (!["interpreta", "ppm_perdita", "strutture", "ppm_struttura"].includes(n)) body[n] = n === "k" ? Number(v) : v;
    const o = (/** @type {string} */ n) => (typeof p[n] === "number" ? p[n] : /** @type {Record<string, number>} */ (MATCH)[n]);
    Object.assign(body, { ppm_prec: o("precursor_ppm"), tol_mz: o("tol_da"), top: o("top_n"), min_prove: o("min_prove") });
    let t;
    try { t = await (await fetch("api/msnlib/cerca", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) })).text(); } catch (_) { throw fail("error.engineFallback"); }
    if (/^\s*\{\s*"error/.test(t)) { const j = JSON.parse(t), key = j.error_key || "err.flow.run"; throw /memory/i.test(key) ? fail("err.flow.msn.desktop") : fail(key, j.params || {}); }
    return t;
  };

  /** keeps `text` in OPFS as mzlab/flow/<name> @param {string} name @param {string} text */
  const keep = async (name, text) => {
    try {
      let d = await navigator.storage.getDirectory();
      for (const s of DIR.split("/").filter(Boolean)) d = await d.getDirectoryHandle(s, { create: true });
      const w = await (await d.getFileHandle(name, { create: true })).createWritable();
      await w.write(text); await w.close();
    } catch (_) { /* the result is still given to the page */ }
  };

  /** the infusion of S: the position in Data of the file the plan names first @param {any} S */
  const infusion = S => {
    const names = F.runPlanFromData(S).names, open = typeof E !== "undefined" && Array.isArray(E.files) ? E.files : [];
    const k = names.length ? open.findIndex((/** @type {any} */ f) => f && !f.gone && String(f.file).split("#")[0] === names[0]) : -1;
    if (k < 0) throw fail("err.flow.msn.open", { input: names[0] || "" });
    return k;
  };

  /** one MSn step: its text, kept in OPFS as `name`; `st` = { plan, k, graph } carries the query of the first graph of the plan to the steps after it @param {any} inst @param {{ plan: any, k: number, graph: URLSearchParams | null }} st */
  const one = async (inst, st) => {
    const id = base(inst), p = inst.params || {}, k = st.k, plan = st.plan;
    let name, text;
    if (id === "spectra.msn_graph") {
      st.graph = graphQuery(inst, k);
      // the command writes the graph followed by a newline
      name = inst.id + ".json"; text = (await ask("api/msn_grafo?" + st.graph)) + "\n";
      const j = JSON.parse(text);
      if (!Array.isArray(j.nodi) || !j.nodi.length) throw fail("err.flow.msn.empty");
    } else if (id === "spectra.match") {
      const first = plan.steps.find((/** @type {any} */ i) => base(i) === "spectra.msn_graph");
      name = inst.id + ".json"; text = await search(st.graph || graphQuery(first || { params: {} }, k), p);
    } else {
      // the graph of the first spectra.msn_graph of the plan (a plan without one asks for the graph with the defaults)
      const first = plan.steps.find((/** @type {any} */ i) => base(i) === "spectra.msn_graph"), q = new URLSearchParams(st.graph || graphQuery(first || { params: {} }, k));
      q.delete("interpreta"); q.delete("ppm_perdita"); q.delete("strutture"); q.delete("ppm_struttura");
      if (p.clean === true) q.set("pulito", "1");
      if (id === "msn.library_export") {
        q.set("libreria", "1"); q.set("formato", p.format === "massbank" ? "massbank" : "msp");
        if (p.levels === "ms2") q.set("livelli", "ms2");
        if (p.annotate === false) q.set("annota", "0");
        if (typeof p.id === "string" && p.id) q.set("id", p.id);
        name = inst.id + (p.format === "massbank" ? ".txt" : ".msp");
      } else { q.set("vista", p.view || "modifications"); name = inst.id + ".csv"; }
      text = await ask("api/msn_grafo/export?" + q);
    }
    await keep(name, text);
    return { name, text };
  };

  /** runs the MSn steps of S in order, outside the scheduler: the reference the tests compare the live session with; resolves with { steps, files: [{ id, name, text }] } @param {any} S @param {{ onEvent?: (e: any) => void }} [opts] */
  const run = async (S, opts = {}) => {
    const o = opts, plan = S.plan, st = { plan, k: infusion(S), graph: /** @type {URLSearchParams | null} */ (null) };
    const steps = /** @type {string[]} */ ([]), files = /** @type {{ id: string, name: string, text: string }[]} */ ([]);
    for (const inst of plan.steps) emit(o, { passo: inst.id, stato: "waiting", via: "browser", parte: 0, di: 0, mb: null, modo: null });
    for (const inst of plan.steps) {
      const ev = { passo: inst.id, via: "browser", parte: 1, di: 1, mb: null, modo: null };
      emit(o, { ...ev, stato: "running" });
      try {
        const r = await one(inst, st);
        files.push({ id: inst.id, ...r });
        emit(o, { ...ev, stato: "done" });
      } catch (e) { emit(o, { ...ev, stato: "error" }); throw e; }
      steps.push(inst.id);
    }
    return { steps, table: null, files, llm: S.llmRef || null };
  };

  F.webMsn = { run, one, infusion, STEPS };
})();
