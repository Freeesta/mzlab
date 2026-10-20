// @ts-check
// Flow (administrator mode): a campaign folder opened from the «Confidence level» node. The folder is read by the engine of the Attributio panel (ATTRIBUTIO_CORE, the .wasm
// `attributio` or the native route), and the table of levels it gives opens under the canvas with the table of the panel (ATTRIBUTIO_TABLE), its CSV and XLSX exports and the Banco 2b of a row (ATTRIBUTIO_BANCO).
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const ready = () => !!(G.ATTRIBUTIO_CORE && G.ATTRIBUTIO_TABLE);

  /** the files of a picked folder as {path relative to the folder, File}, and its name @param {File[]} list */
  F.campaignFiles = list => {
    const root = (list[0].webkitRelativePath || "").split("/")[0] || "campaign";
    return { root, files: list.map(f => ({ path: (f.webkitRelativePath || f.name).split("/").slice(1).join("/") || f.name, file: f })) };
  };

  /** the cascade on a folder: the table ({columns, rows}) and its summary (fingerprint, settings, counts) @param {{path: string, file: File}[]} files @param {string} root */
  F.campaignRun = async (files, root) => {
    const r = await (await G.ATTRIBUTIO_CORE.pool()).attributioAnnotate(files, root, "json");
    return { data: JSON.parse(new TextDecoder().decode(r.bytes)), summary: r.summary };
  };

  /** the table or the export of the same folder again, in the format asked @param {any} c the campaign of an item @param {"csv" | "xlsx"} kind */
  const exportTable = async (c, kind) => {
    const r = await (await G.ATTRIBUTIO_CORE.pool()).attributioAnnotate(c.files, c.root, kind), stem = c.root + "_attributio";
    if (kind === "csv") G.ATTRIBUTIO_CORE.download(stem + ".csv", r.bytes, "text/csv;charset=utf-8");
    else {
      const sheet = JSON.parse(new TextDecoder().decode(r.bytes)).hojas[0];
      await G.dlx(stem + ".xlsx", [{ name: sheet.nombre, head: sheet.filas[0], rows: sheet.filas.slice(1) }], false);
    }
  };

  F.registerView({ id: "campaign", title: "Levels of a campaign folder", types: ["annot"], accepts: (/** @type {any} */ item) => !!item.campaign, open: async (/** @type {HTMLElement} */ host, /** @type {any} */ item) => {
    const { el } = F, c = item.campaign, say = el("p", { class: "fl-note", role: "status" }), box = el("div", { class: "tp" });
    const out = (/** @type {"csv" | "xlsx"} */ kind) => () => exportTable(c, kind).then(() => { say.textContent = ""; }, (/** @type {any} */ e) => { say.textContent = "Export failed: " + (e && e.message || e); });
    const rv = el("div", { class: "flw-review" }), exp = el("span", { class: "flw-exp" });
    // the Review screen replaces the table; leaving it redraws the table when a decision was saved (the cascade runs again once)
    const startReview = () => {
      box.hidden = true; bench.textContent = "";
      F.reviewMount(rv, c, {
        onSaved: (/** @type {string} */ text, /** @type {string} */ path) => { const at = c.files.findIndex((/** @type {any} */ f) => f.path === path), file = { path, file: new File([text], path.split("/").pop() || "feature_decisions.csv", { type: "text/csv" }) }; if (at >= 0) c.files[at] = file; else c.files.push(file); },
        expert: (/** @type {any[]} */ list, /** @type {number} */ i) => review(list, i),
        close: async (/** @type {boolean} */ changed) => { rv.textContent = ""; box.hidden = false; if (changed) { Object.assign(c, await F.campaignRun(c.files, c.root)); draw(); } },
      });
    };
    host.append(el("div", { class: "fl-meta" }, F.btn("ok", "Review the features that wait for a decision, one at a time", startReview, "Review"), F.btn("export", "Export the table as CSV", out("csv"), "CSV"), F.btn("export", "Export the table as XLSX", out("xlsx"), "XLSX"), exp), say, rv, box);
    // the six engine exports of Attributio (evidence, Schymanski folder, publication table, quantification, gradient, SIRIUS) on the same campaign files
    G.ATTRIBUTIO_EXPORT.mount(exp, { pool: G.ATTRIBUTIO_CORE.pool, files: () => c.files, root: () => c.root, say: (/** @type {string} */ t, /** @type {boolean} */ err) => { say.textContent = t; if (err) say.setAttribute("role", "alert"); else say.setAttribute("role", "status"); } });
    // the MS2 panel (library search, in-silico ranking, hydrogen shifts, retention-time prediction, co-isolated features) is evidence only: it writes nothing into the campaign
    const ms2 = el("div", { class: "flw-ms2" });
    G.ATTRIBUTIO_MS2.mount(ms2, { pool: G.ATTRIBUTIO_CORE.pool, files: () => c.files, root: () => c.root, say: (/** @type {string} */ t, /** @type {boolean} */ err) => { say.textContent = t; say.setAttribute("role", err ? "alert" : "status"); } });
    host.append(ms2);
    for (const b of exp.querySelectorAll("#at-export")) /** @type {HTMLButtonElement} */ (b).disabled = false;
    if (c.summary && c.summary.warnings && c.summary.warnings.length) say.textContent = c.summary.warnings.join(" · ");
    const bench = el("div", { class: "flw-bench" });
    // a saved decision replaces the decisions file in memory, the cascade runs again and the table is drawn from the new levels
    const review = async (/** @type {any[]} */ list, /** @type {number} */ index) => {
      try {
        await G.ATTRIBUTIO_BANCO.open({ pool: await G.ATTRIBUTIO_CORE.pool(), files: c.files, root: c.root, list, index, host: bench, onSaved: async (/** @type {string} */ text, /** @type {string} */ path) => {
          const file = new File([text], path.split("/").pop() || "feature_decisions.csv", { type: "text/csv" }), at = c.files.findIndex((/** @type {any} */ f) => f.path === path);
          if (at >= 0) c.files[at] = { path, file }; else c.files.push({ path, file });
          Object.assign(c, await F.campaignRun(c.files, c.root)); draw();
        } });
      } catch (e) { say.textContent = "Bench 2b: " + ((/** @type {any} */ (e)).message || e); }
    };
    const draw = () => { box.textContent = ""; G.ATTRIBUTIO_TABLE.mount(box, c.data, { onReview: review }); };
    draw(); host.append(bench);
    if (G.WORKFLOW && G.WORKFLOW.logOp) G.WORKFLOW.logOp("attributio", { attributio: { fingerprint: c.summary.fingerprint, settings: c.summary.settings, counts: c.summary.by_level } });
  } });

  // the block of the «Confidence level» node: choose a folder, the table opens under the canvas
  F.extraResults.push((S, node) => {
    if (!ready() || !(node.ids || []).some((/** @type {string} */ id) => { const i = F.instanceOf(S.plan, id); return i && i.step.split("@")[0] === "annot.level"; })) return null;
    const { el } = F, id = node.ids[0], say = el("p", { class: "fl-note", role: "status" });
    const pick = /** @type {HTMLInputElement} */ (el("input", { type: "file", hidden: true, webkitdirectory: true, multiple: true, "aria-label": "Choose a campaign folder", onchange: async (/** @type {any} */ e) => {
      const list = /** @type {File[]} */ ([...e.target.files]).slice(0, G.ATTRIBUTIO_CORE.MAX_FILES);
      if (!list.length) return;
      const { root, files } = F.campaignFiles(list);
      say.textContent = `Reading ${root} (${files.length} files)…`;
      try {
        const run = await F.campaignRun(files, root), item = { type: "annot", label: `${root} (campaign folder)`, campaign: { root, files, ...run }, text: async () => "" };
        S.results = S.results || {}; S.results[id] = [...(S.results[id] || []).filter((/** @type {any} */ r) => r.label !== item.label), item];
        say.textContent = ""; F.openResult(item); if (S.redraw) S.redraw();
      } catch (err) { const x = /** @type {any} */ (err); say.textContent = "Attributio: " + ((x && x.params && x.params.detail) || (x && x.message) || String(err)); }
    } }));
    return el("div", { class: "fl-results" }, pick, F.btn("import", "Open a campaign folder", () => pick.click(), "Open a campaign folder…"), say);
  });
})();
