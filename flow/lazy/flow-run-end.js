// @ts-check
// Flow (administrator mode): what the page does when a run ends and the pieces the result views share. The finished table opens by itself (a .mzfeat as the Feature table with the sample roles of the plan,
// a CSV as the preview), and one legend of the states and the levels (the programme's own levels are marked «auto», outlined) is drawn under the views that show them.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);

  /** the file stem used to match a sample list to the files of a table @param {string} name */
  const stem = name => String(name || "").replace(/^.*[\\/]/, "").replace(/\.(mzidx|mzml|raw|wiff|d)$/i, "");

  /** the roles of the files of a table from the sample list of the plan (`campioni/2` kept in `S.samples[input name]`), in the order of `names`; null when the plan has no list or it leaves a file out
   * @param {any} S @param {string[]} names @returns {{cs: any[] | null, note: string}} */
  F.samplesFor = (S, names) => {
    const inp = S && S.plan && S.plan.inputs.find((/** @type {any} */ i) => i.type === "samples");
    if (!inp) return { cs: null, note: "No sample list in the plan, so the table has no roles for the files." };
    const doc = S.samples && S.samples[inp.name], list = doc && Array.isArray(doc.campioni) ? doc.campioni : null;
    if (!list) return { cs: null, note: `The sample list ${inp.name} has no content yet, so the table has no roles for the files.` };
    const by = new Map(list.map((/** @type {any} */ c) => [stem(c.file), c])), cs = names.map(n => by.get(stem(n)));
    const missing = names.filter((n, i) => !cs[i]);
    return missing.length ? { cs: null, note: `The sample list leaves out ${missing.length} of ${names.length} files (${missing.slice(0, 3).join(", ")}${missing.length > 3 ? ", …" : ""}).` } : { cs, note: "" };
  };

  /** the legend of the result views: `states` = the peak states of a table cell (the symbols of the table), `levels` = the levels of identification with «auto» for those the programme set @param {{states?: boolean, levels?: boolean}} parts */
  F.legend = parts => {
    const { el } = F, box = el("div", { class: "flx-leg", role: "group", "aria-label": "Legend" });
    if (parts.states && G.TPTAB && G.TPTAB.legend) box.insertAdjacentHTML("beforeend", G.TPTAB.legend(["detected", "filled", "targeted", "absent"]));
    if (parts.levels) {
      const tag = (/** @type {string} */ k) => (G.LIVELLI ? G.LIVELLI.tag(k) : "L" + k), cls = (/** @type {string} */ k) => (G.LIVELLI ? G.LIVELLI.cls(k) : "lv-" + k);
      const ls = ["1", "2a", "2b", "3a", "3b", "4"].map(k => el("span", { class: "lvb " + cls(k), text: tag(k) }));
      box.append(el("span", { class: "flx-leg-lv" }, ...ls, el("span", { class: "lvb flx-auto", text: "auto" }), el("span", { text: "= set by the programme, counted apart; 2b is only ever the chemist's decision" })));
    }
    return box;
  };

  /** the item to show when a run ends: the first `.mzfeat` made, else the first CSV table; null when nothing was made @param {any} S @returns {any} */
  F.endItem = S => {
    const all = /** @type {any[]} */ ([]);
    for (const list of Object.values(S.results || {})) all.push(...(/** @type {any[]} */ (list)));
    return all.find(r => r.type === "table" && /\.mzfeat$/i.test(r.label) && (r.file || r.load)) || all.find(r => /\.(csv|tsv|txt)$/i.test(r.label)) || null;
  };

  const SERIES = "mzlab/flow/series.mzfeat";
  /** the table «This browser» kept in OPFS (the pool's reader, as the live jobs use it) as a result item of the same form as the desktop's (`load` reads it); null when the browser cannot or the file is not there @returns {Promise<any>} */
  F.webTableItem = async () => {
    const R = F.runners && F.runners.web;
    if (!R || !R.available()) return null;
    try {
      const pool = await R.kit.getPool();
      if (!(await pool.fs.exists(SERIES))) return null;
      return { type: "table", label: "series.mzfeat", load: async () => new File([await pool.fs.read(SERIES)], "series.mzfeat") };
    } catch (_) { return null; }
  };

  /** the step whose output is the table of a web run (the last of fill gaps or align) @param {any} S */
  F.webTableStep = S => {
    const base = (/** @type {any} */ i) => String(i.step).split("@")[0], steps = S.plan.steps;
    return steps.findLast((/** @type {any} */ i) => base(i) === "features.gapfill") || steps.findLast((/** @type {any} */ i) => base(i) === "features.align") || null;
  };

  /** a web run ended: the table kept in OPFS joins the Results of its step and opens as after a desktop run; without the file or the step the run note stays as it is @param {any} S @returns {Promise<boolean>} */
  F.webAfterRun = async S => {
    const inst = F.webTableStep(S), item = inst && await F.webTableItem();
    if (!item) return false;
    S.results = S.results || {};
    S.results[inst.id] = [...(S.results[inst.id] || []).filter((/** @type {any} */ r) => r.label !== item.label), item];
    if (S.redraw) S.redraw();
    await F.afterRun(S);
    // the run box redraws after the run: the focus goes back to the title of the view once it has
    setTimeout(() => { const h = document.querySelector(".flx-res h3"); if (h instanceof HTMLElement) h.focus({ preventScroll: true }); }, 0);
    return true;
  };

  /** a run ended: open the table by itself. A made file that is read on demand (`load`) is read first; a file that cannot be read leaves the buttons of the Results as they were @param {any} S */
  F.afterRun = async S => {
    const item = F.endItem(S);
    if (!item) return;
    if (!item.file && item.load) { try { item.file = await item.load(); } catch (_) { return; } }
    await F.openResult(item);
  };
})();
