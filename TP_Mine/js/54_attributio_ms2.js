// @ts-check
// TPMINE-PRIVATE  MS2 engine of Attributio (admin mode, experimental, off by default): `ATTRIBUTIO_MS2.mount(host, ctx)` draws a panel under the Attributio entry with four tools
// (library search, in-silico ranking of candidates, hydrogen-shift attribution, retention-time prediction) and the co-isolated features of one feature. The engine is the .wasm
// `attributio_ms2` (or the native route `feat/attributio/ms2`); everything is evidence for the chemist: no level changes, nothing is written into the campaign. English only, no lang/ keys.
(() => {
  "use strict";
  /** @typedef {{ pool: () => Promise<any>, files: () => {path: string, file: File}[], root: () => string, say: (text: string, err?: boolean) => void }} Ctx */

  /** @param {string} name @param {BlobPart} data @param {string} type */
  function save(name, data, type) {
    const a = document.createElement("a"); a.href = URL.createObjectURL(new Blob([data], { type })); a.download = name; a.click(); setTimeout(() => URL.revokeObjectURL(a.href), 2000);
  }

  /** a table of text cells, built with textContent only @param {HTMLElement} box @param {string} caption @param {string[]} head @param {(string | number)[][]} rows */
  function table(box, caption, head, rows) {
    box.textContent = "";
    const t = document.createElement("table"), c = t.createCaption(); c.textContent = caption;
    const tr = t.createTHead().insertRow();
    for (const h of head) { const th = document.createElement("th"); th.scope = "col"; th.textContent = h; tr.appendChild(th); }
    const body = t.createTBody();
    for (const r of rows) { const row = body.insertRow(); for (const v of r) row.insertCell().textContent = String(v); }
    box.appendChild(t);
  }

  /** @param {number} x @param {number} nd */
  const fx = (x, nd) => (Number.isFinite(x) ? x.toFixed(nd) : "");

  /** the files of the campaign plus the extra files of the request under `__ms2/` @param {Ctx} ctx @param {File[]} extra */
  const withExtra = (ctx, extra) => [...ctx.files(), ...extra.map(f => ({ path: "__ms2/" + f.name, file: f }))];

  /** @param {HTMLElement} host @param {Ctx} ctx */
  function mount(host, ctx) {
    host.innerHTML = `<details class="at-ms2"><summary>MS2 engine (experimental)</summary>
      <p class="mut">Evidence only: no level changes and nothing is written into the campaign.</p>
      <div class="row"><label>Library files <input type="file" id="ms2-lib" multiple accept=".json,.jsonl,.msp" aria-label="MS2 library files"></label>
        <button type="button" class="at-x" id="ms2-run" aria-label="Run MS2 engine" disabled>Run MS2 engine</button>
        <button type="button" class="at-x" id="ms2-pos" aria-label="Download engine_pos.csv" disabled>engine_pos.csv</button>
        <button type="button" class="at-x" id="ms2-neg" aria-label="Download engine_neg.csv" disabled>engine_neg.csv</button></div>
      <div class="row"><label>Polarity <select id="ms2-pol" aria-label="Polarity"><option>POS</option><option>NEG</option></select></label>
        <label>Feature ID <input type="number" id="ms2-id" min="0" step="1" aria-label="Feature ID"></label>
        <label>Formula <input type="text" id="ms2-formula" size="14" aria-label="Candidate formula"></label>
        <label>Candidate list <input type="file" id="ms2-cand" accept=".csv,.tsv,.txt" aria-label="Candidate list"></label></div>
      <div class="row"><button type="button" class="at-x" id="ms2-insilico" aria-label="Rank candidates in silico" disabled>Rank candidates</button>
        <button type="button" class="at-x" id="ms2-shift" aria-label="Attribute fragments with hydrogen shifts" disabled>Fragments with H shifts</button>
        <button type="button" class="at-x" id="ms2-chim" aria-label="Find co-isolated features" disabled>Co-isolated features</button></div>
      <div class="row"><label>RT training (smiles,rt) <input type="file" id="ms2-rt" accept=".csv,.txt" aria-label="Retention-time training table"></label>
        <label>SMILES to predict <input type="text" id="ms2-smiles" size="24" aria-label="SMILES to predict"></label>
        <button type="button" class="at-x" id="ms2-predict" aria-label="Predict retention time" disabled>Predict RT</button></div>
      <div id="ms2-out" class="tp" role="region" aria-label="MS2 engine result" tabindex="0"></div></details>`;
    const q = (/** @type {string} */ s) => /** @type {any} */ (host.querySelector(s));
    const out = /** @type {HTMLElement} */ (q("#ms2-out"));
    /** @type {{ pos: string, neg: string } | null} */
    let csv = null;
    const sync = () => {
      const ready = ctx.files().length > 0;
      for (const id of ["#ms2-run", "#ms2-insilico", "#ms2-shift", "#ms2-chim", "#ms2-predict"]) q(id).disabled = !ready;
    };
    host.addEventListener("focusin", sync);
    host.querySelector("summary")?.addEventListener("click", sync);
    const feature = () => ({ pol: q("#ms2-pol").value, id: Number(q("#ms2-id").value) });
    /** @param {File[]} extra @param {any} request @param {string} busy */
    async function ask(extra, request, busy) {
      ctx.say(busy);
      try {
        const pool = await ctx.pool(), r = await pool.attributioMs2(withExtra(ctx, extra), ctx.root(), request);
        ctx.say("Done.");
        return r;
      } catch (e) {
        const x = /** @type {any} */ (e);
        ctx.say("MS2 engine: " + ((x && x.params && x.params.detail) || (x && x.message) || String(e)), true);
        return null;
      }
    }
    q("#ms2-run").onclick = async () => {
      const libs = [...(q("#ms2-lib").files || [])];
      if (!libs.length) { ctx.say("Choose at least one library file (.json, .jsonl or .msp).", true); return; }
      const r = await ask(libs, { op: "engine", libraries: libs.map(f => "__ms2/" + f.name) }, "Searching " + libs.length + " library file(s)…");
      if (!r) return;
      csv = { pos: r.pos, neg: r.neg };
      q("#ms2-pos").disabled = !r.pos_hits; q("#ms2-neg").disabled = !r.neg_hits;
      table(out, "MS2 engine run", ["Library spectra", "POS hits", "NEG hits", "Warnings"], [[r.library, r.pos_hits, r.neg_hits, (r.warnings || []).join("; ")]]);
    };
    q("#ms2-pos").onclick = () => { if (csv) save("engine_pos.csv", csv.pos, "text/csv;charset=utf-8"); };
    q("#ms2-neg").onclick = () => { if (csv) save("engine_neg.csv", csv.neg, "text/csv;charset=utf-8"); };
    q("#ms2-insilico").onclick = async () => {
      const f = q("#ms2-cand").files && q("#ms2-cand").files[0];
      if (!f) { ctx.say("Choose a candidate list (CSV with SMILES).", true); return; }
      const r = await ask([f], { op: "insilico", ...feature(), candidates: "__ms2/" + f.name }, "Ranking candidates…");
      if (r) table(out, "In-silico ranking of " + r.candidates + " candidates (hypotheses, not identities)", ["Name", "Formula", "Score", "Mass error (ppm)", "Explained peaks"],
        r.ranked.map((/** @type {any} */ c) => [c.name || c.id, c.formula, fx(c.score, 3), fx(c.mass_ppm, 2), c.explained.length]));
    };
    q("#ms2-shift").onclick = async () => {
      const r = await ask([], { op: "rearrange", ...feature(), formula: q("#ms2-formula").value.trim() }, "Attributing fragments…");
      if (!r) return;
      if (!r.attribution) { table(out, "Fragment attribution", ["Result"], [["The formula does not fit the precursor of this feature."]]); return; }
      const a = r.attribution;
      table(out, `Fragments of ${a.ion_formula} ${a.adduct}: ${a.n_explained} explained, ${a.n_rearranged} with H shifts, score ${fx(a.score, 3)}`, ["m/z", "H shift", "Formula", "ppm", "Loss"],
        a.peaks.map((/** @type {any} */ p) => [fx(p.mz, 4), p.shift === null ? "" : p.shift, p.formula || "", p.shift === null ? "" : fx(p.ppm, 1), p.loss || ""]));
    };
    q("#ms2-chim").onclick = async () => {
      const r = await ask([], { op: "chimeric", ...feature() }, "Looking for co-isolated features…");
      if (r) table(out, "Co-isolated features (window " + r.isolation_window + " Da)", ["ID", "m/z", "RT", "Δ m/z", "Δ RT"],
        r.intruders.map((/** @type {any} */ x) => [x.id, fx(x.mz, 4), fx(x.rt, 2), fx(x.delta_mz, 4), fx(x.delta_rt, 2)]));
    };
    q("#ms2-predict").onclick = async () => {
      const f = q("#ms2-rt").files && q("#ms2-rt").files[0], smiles = q("#ms2-smiles").value.trim();
      if (!f || !smiles) { ctx.say("Choose the training table and type a SMILES.", true); return; }
      const train = (await f.text()).split(/\r?\n/).map(l => l.split(/[,;\t]/)).map(c => [(c[0] || "").trim(), Number(c[1])]).filter(c => c[0] && Number.isFinite(c[1]));
      const r = await ask([], { op: "rt", train, predict: [smiles] }, "Training the retention-time model…");
      if (r) table(out, r.model ? "Predicted retention time (model trained on " + r.trained + " structures)" : "Retention-time prediction", ["SMILES", "RT (min)"],
        r.model ? r.predictions.map((/** @type {any} */ p) => [p.smiles, p.rt === null ? "not read" : fx(p.rt, 2)]) : [["At least 10 training structures are needed", ""]]);
    };
  }

  /** @type {any} */ (window).ATTRIBUTIO_MS2 = { mount };
})();
