// @ts-check
// Flow (administrator mode): the view of a `.mzfeat` feature table chosen from disk. It reads the file in the page (PIPEV.readFeat), opens the table session of the Pipeline (PIPEQC, the .wasm of the table)
// and mounts its virtual table with the expanded row (PIPET). Read only: no step is run; the traces of a row need the .mzidx of the series in the browser storage and say so when they are missing.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const ready = () => !!(G.PIPEV && G.PIPEQC && G.PIPET && G.ATTRIBUTIO_CORE);

  /** the names of the files of a table (stems of its inputs) and the polarity written in its meta (1 when none) @param {any} meta */
  F.featMeta = meta => {
    const inputs = Array.isArray(meta && meta.inputs) ? meta.inputs : [];
    return { inputs, names: inputs.map((/** @type {any} */ i) => String((i && i.name) || "").replace(/\.(mzidx|mzml)$/i, "")), pol: meta && meta.polarity < 0 ? -1 : 1 };
  };

  /** the family of each row of a table, read once per row from `riga/1` as the rows come into view: `get(id)` is the size or null while unknown, `members(id)` the ions; `redraw` is called (debounced) when more are known
   * @param {any} session @param {() => void} redraw */
  F.familyLoader = (session, redraw) => {
    /** @type {Map<number, any>} */ const known = new Map(), asked = new Set();
    let timer = 0, running = 0;
    const queue = /** @type {number[]} */ ([]);
    const pump = () => {
      while (running < 4 && queue.length) {
        const id = /** @type {number} */ (queue.shift()); running++;
        session.row(id).then((/** @type {any} */ r) => { known.set(id, r && r.family ? r.family : null); }, () => { known.set(id, null); })
          .then(() => { running--; clearTimeout(timer); timer = window.setTimeout(redraw, 120); pump(); });
      }
    };
    const want = (/** @type {number} */ id) => { if (!asked.has(id)) { asked.add(id); queue.push(id); pump(); } };
    return {
      /** @param {number} id @returns {number | null} */
      size: id => { want(id); const f = known.get(id); return f ? f.size : null; },
      /** @param {number} id */
      load: id => (known.has(id) ? Promise.resolve(known.get(id)) : session.row(id).then((/** @type {any} */ r) => { known.set(id, r && r.family ? r.family : null); return known.get(id); })),
    };
  };

  /** the column «Family» (ions in the row's family) and the tab «Family» (the ions with m/z, RT and role, the reference ion first) for PIPET.mount @param {any} fam the loader of F.familyLoader @param {{ names: string[], pol: number } | undefined} proofs the files and polarity of the table: with them the tab has the button «Show proofs» */
  F.familyHooks = (fam, proofs) => ({
    extraCols: [{ key: "family", label: "Family", num: true, sort: (/** @type {any} */ r) => fam.size(r.id) ?? -1, cell: (/** @type {any} */ r) => { const n = fam.size(r.id); return n == null ? "" : String(n); }, text: (/** @type {any} */ r) => { const n = fam.size(r.id); return n == null ? "" : String(n); } }],
    extraTabs: [{ key: "family", label: "Family", fill: (/** @type {HTMLElement} */ panel, /** @type {any} */ r) => {
      const { el } = F;
      panel.append(el("p", { class: "fl-note", text: "Reading the family…" }));
      fam.load(r.id).then((/** @type {any} */ f) => {
        panel.textContent = "";
        if (!f) { panel.append(el("p", { class: "fl-note", text: "This table has one row per ion, so a row has no family." })); return; }
        panel.append(el("p", { text: `${f.size} ${f.size === 1 ? "ion" : "ions"} in this family; the reference ion is first.` }),
          el("div", { class: "flx-tablewrap", tabindex: 0, role: "region", "aria-label": "Ions of the family" }, el("table", { class: "flx-table" }, el("caption", { text: `Family of row ${r.id}` }),
            el("thead", {}, el("tr", {}, ...["m/z", "RT (min)", "Role"].map(h => el("th", { scope: "col", text: h })))),
            el("tbody", {}, ...f.members.map((/** @type {any} */ m) => el("tr", {}, el("td", { text: Number(m.mz).toFixed(4) }), el("td", { text: Number(m.rt).toFixed(2) }), el("td", { text: String(m.role) })))))));
        if (proofs && F.familyProofs) F.familyProofs(panel, { ...proofs, ions: f.members });
      }, (/** @type {any} */ e) => { panel.textContent = ""; panel.append(el("p", { class: "fl-note", role: "alert", text: "The family could not be read: " + (e && e.message || e) })); });
    } }],
  });

  F.registerView({ id: "feattable", title: "Feature table", types: ["table"], accepts: (/** @type {any} */ item) => !!item.file && /\.mzfeat$/i.test(item.label), open: async (/** @type {HTMLElement} */ host, /** @type {any} */ item) => {
    const { el } = F, say = el("p", { class: "fl-note", role: "status" }), box = el("div", { class: "tp flw-feat" });
    host.append(say, box);
    if (!ready()) { say.textContent = "The feature table needs the Tabui scripts of this page."; return; }
    try {
      say.textContent = "Reading the table…";
      const bytes = new Uint8Array(await item.file.arrayBuffer()), data = G.PIPEV.readFeat(bytes), { inputs, names, pol } = F.featMeta(data.meta);
      // the roles of the files come from the sample list of the plan; without it the table opens as before and the line says why
      const { cs, note } = F.samplesFor(F.current, names);
      const session = await G.PIPEQC.open({ base: G.QTOOLS.ctx.base, bytes, names, inputs, campioni: cs, pol, pool: G.ATTRIBUTIO_CORE.pool });
      say.textContent = `${data.rows.toLocaleString("en")} rows in ${data.files} files.${note ? " " + note : ""}`;
      /** @type {any} */ let table = null;
      const fam = F.familyLoader(session, () => { if (table) table.redraw(); }), hooks = F.familyHooks(fam, { names, pol });
      table = G.PIPET.mount(box, { ...hooks, session, names, gate: { ok: true, table: true }, say: (/** @type {any} */ e) => { say.textContent = String((e && (e.message || e.key)) || e); }, cs, ppm: () => 5, pol: () => pol, formula: "", levels: null, identsOf: () => [] });
      // with blanks in the list the filter «Not in blank» is on from the start
      const chip = /** @type {HTMLButtonElement | null} */ (box.querySelector('[data-fc="blank"]'));
      if (cs && chip && !chip.disabled && chip.getAttribute("aria-pressed") !== "true") chip.click();
      box.append(F.legend({ levels: true })); F.watchRows(box, "feattable");
    } catch (e) { say.textContent = "Feature table: " + ((/** @type {any} */ (e)).message || e); }
  } });
})();
