// @ts-check
// Flow (administrator mode): the «This browser» runner of the MSn steps (spectra.msn_graph, msn.library_export, msn.table_export). The infusion is the file the plan names that is open in the Data tab;
// the graph, the library and the tables come from the routes of the engine of the page (`api/msn_grafo`, `api/msn_grafo/export`: the .wasm msngrafo, the same functions as `mzlab-cli msn-graph`
// and `msn-lib export`), so the bytes are the ones of the command. Reports the same events as flow-web-run.js, keeps each result in OPFS (mzlab/flow/) and resolves with the texts. Docs: docs/agenti/flow-web.md.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const STEPS = ["spectra.msn_graph", "msn.library_export", "msn.table_export"], DIR = "mzlab/flow/";
  const base = (/** @type {any} */ inst) => String(inst.step).split("@")[0];
  const fail = (/** @type {string} */ key, params = {}) => Object.assign(new Error(key), { key, params });
  /** @param {any} o @param {any} ev */
  const emit = (o, ev) => { if (o.onEvent) o.onEvent(ev); document.dispatchEvent(new CustomEvent("flow:web-step", { detail: ev })); };

  /** a plan made of the MSn steps only: no .mzidx, so the runner of the series is not the one to ask @param {any} plan */
  const handles = plan => plan.steps.length > 0 && plan.steps.every((/** @type {any} */ i) => STEPS.includes(base(i)));

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

  /** keeps `text` in OPFS as mzlab/flow/<name> @param {string} name @param {string} text */
  const keep = async (name, text) => {
    try {
      let d = await navigator.storage.getDirectory();
      for (const s of DIR.split("/").filter(Boolean)) d = await d.getDirectoryHandle(s, { create: true });
      const w = await (await d.getFileHandle(name, { create: true })).createWritable();
      await w.write(text); await w.close();
    } catch (_) { /* the result is still given to the page */ }
  };

  /** runs the MSn steps of S in order; resolves with { steps, files: [{ id, name, text }] } @param {any} S @param {{ onEvent?: (e: any) => void }} [opts] */
  const run = async (S, opts = {}) => {
    const o = opts, plan = S.plan, names = F.runPlanFromData(S).names;
    const open = typeof E !== "undefined" && Array.isArray(E.files) ? E.files : [];
    const k = names.length ? open.findIndex((/** @type {any} */ f) => f && !f.gone && String(f.file).split("#")[0] === names[0]) : -1;
    if (k < 0) throw fail("err.flow.msn.open", { input: names[0] || "" });
    const steps = /** @type {string[]} */ ([]), files = /** @type {{ id: string, name: string, text: string }[]} */ ([]);
    for (const inst of plan.steps) emit(o, { passo: inst.id, stato: "waiting", via: "browser", parte: 0, di: 0, mb: null, modo: null });
    /** @type {URLSearchParams | null} */ let graph = null;
    for (const inst of plan.steps) {
      const id = base(inst), p = inst.params || {}, ev = { passo: inst.id, via: "browser", parte: 1, di: 1, mb: null, modo: null };
      emit(o, { ...ev, stato: "running" });
      try {
        let name, text;
        if (id === "spectra.msn_graph") {
          graph = graphQuery(inst, k);
          // the command writes the graph followed by a newline
          name = inst.id + ".json"; text = (await ask("api/msn_grafo?" + graph)) + "\n";
        } else {
          // the graph of the first spectra.msn_graph of the plan (a plan without one asks for the graph with the defaults)
          const first = plan.steps.find((/** @type {any} */ i) => base(i) === "spectra.msn_graph"), q = new URLSearchParams(graph || graphQuery(first || { params: {} }, k));
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
        await keep(name, text); files.push({ id: inst.id, name, text });
        emit(o, { ...ev, stato: "done" });
      } catch (e) { emit(o, { ...ev, stato: "error" }); throw e; }
      steps.push(inst.id);
    }
    return { steps, table: null, files, llm: S.llmRef || null };
  };

  F.webMsn = { handles, run };
})();
