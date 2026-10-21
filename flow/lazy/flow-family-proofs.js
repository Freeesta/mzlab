// @ts-check
// Flow (administrator mode): the button «Show proofs» of the tab «Family» of the Feature table. The arcs of a family and their proofs are not stored in the .mzfeat, so the engine
// (`family_record` of the .wasm, or the route feat/family of the app) reads one .mzidx again and gives the `famiglia/1` record. A function for computers: on a phone the tab says so.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const MZ_TOL = 0.01, RT_TOL = 0.25;

  /** the sentence for an error of the engine (the pool's errors carry `key` and `params`) @param {any} e */
  F.proofsError = e => {
    const key = e && e.key, p = (e && e.params) || {};
    if (key === "error.badRecipe") return "The parameters of the pipeline are not valid, so the proofs cannot be read.";
    if (key === "error.featMemory" || key === "error.memoryBudget") return `Not enough memory to read the proofs of this file (about ${p.stima_mb ?? "?"} MB needed of ${p.budget_mb ?? "?"} MB). Use the app on a computer with more memory.`;
    if (key === "error.badIndex" || key === "error.io" || key === "err.engine.index") return "The index (.mzidx) of this file could not be read; open the file in Tabui first.";
    return "The proofs could not be read: " + (e && (e.message || e.key) || e);
  };

  /** the families of a record that share an ion with the ions of the row's family @param {any} rec @param {any[]} ions {mz, rt} of the row's family */
  F.proofsFamilies = (rec, ions) => ((rec && rec.families) || []).filter((/** @type {any} */ fam) => fam.members.some((/** @type {any} */ m) => ions.some(i => Math.abs(i.mz - m.mz) <= MZ_TOL && Math.abs(i.rt - m.rt) <= RT_TOL)));

  /** the arcs of one family as a table, every proof with its value against its threshold @param {any} fam */
  const arcTable = fam => {
    const { el } = F, mz = new Map(fam.members.map((/** @type {any} */ m) => [m.row, m.mz]));
    const proofs = (/** @type {any} */ a) => a.proofs.map((/** @type {any} */ p) => `${p.kind} ${p.value} (threshold ${p.threshold})`).join("; ");
    const rows = fam.arcs.map((/** @type {any} */ a) => el("tr", {}, ...[Number(mz.get(a.lo)).toFixed(4), Number(mz.get(a.up)).toFixed(4), a.delta_mz, a.delta_rt, a.kind, a.admitted_by, a.r, proofs(a)].map(t => el("td", { text: String(t) }))));
    return el("div", { class: "flx-tablewrap", tabindex: 0, role: "region", "aria-label": `Arcs of family ${fam.family}` }, el("table", { class: "flx-table" }, el("caption", { text: `Family ${fam.family}: ${fam.arcs.length} ${fam.arcs.length === 1 ? "arc" : "arcs"}` }),
      el("thead", {}, el("tr", {}, ...["Lower m/z", "Upper m/z", "Δ m/z", "Δ RT (min)", "Kind", "Admitted by", "r", "Proofs"].map(h => el("th", { scope: "col", text: h })))),
      el("tbody", {}, ...rows)));
  };

  /** the block «Show proofs» for the tab «Family» @param {HTMLElement} panel @param {{ names: string[], pol: number, ions: any[] }} o the files of the table, its polarity and the ions of the row's family */
  F.familyProofs = (panel, o) => {
    const { el } = F, box = el("div", { class: "fl-proofs" });
    panel.append(box);
    if (matchMedia("(max-width: 700px) and (pointer: coarse)").matches) { box.append(el("p", { class: "fl-note", text: "The proofs of the arcs are a function designed for computers; open this page on a computer to see them." })); return; }
    const pick = /** @type {HTMLSelectElement} */ (el("select", { "aria-label": "File to read the proofs from" }, ...o.names.map(n => el("option", { value: n, text: n })))),
      go = /** @type {HTMLButtonElement} */ (el("button", { class: "fl-b", type: "button", text: "Show proofs" })), say = el("p", { class: "fl-note", role: "status" }), out = el("div", {});
    go.addEventListener("click", async () => {
      const name = pick.value; go.disabled = true; out.textContent = ""; say.textContent = `Reading the proofs from ${name}…`;
      try {
        const pool = await G.ATTRIBUTIO_CORE.pool(), rec = await pool.family({ name, path: "mzlab/idx/" + name + ".mzidx" }, JSON.stringify({ polarity: o.pol < 0 ? -1 : 1, mode: "family" })), fams = F.proofsFamilies(rec, o.ions);
        say.textContent = fams.length ? `${fams.length} ${fams.length === 1 ? "family" : "families"} of ${name} with these ions.` : `${name} has no family with these ions.`;
        out.append(...fams.map(arcTable));
      } catch (e) { say.textContent = ""; out.append(el("p", { class: "fl-note", role: "alert", text: F.proofsError(e) })); } finally { go.disabled = false; }
    });
    box.append(el("div", { class: "fl-row" }, pick, go), say, out);
  };
})();
