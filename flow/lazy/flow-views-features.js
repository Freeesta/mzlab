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

  F.registerView({ id: "feattable", title: "Feature table", types: ["table"], accepts: (/** @type {any} */ item) => !!item.file && /\.mzfeat$/i.test(item.label), open: async (/** @type {HTMLElement} */ host, /** @type {any} */ item) => {
    const { el } = F, say = el("p", { class: "fl-note", role: "status" }), box = el("div", { class: "tp flw-feat" });
    host.append(say, box);
    if (!ready()) { say.textContent = "The feature table needs the Tabui scripts of this page."; return; }
    try {
      say.textContent = "Reading the table…";
      const bytes = new Uint8Array(await item.file.arrayBuffer()), data = G.PIPEV.readFeat(bytes), { inputs, names, pol } = F.featMeta(data.meta);
      const session = await G.PIPEQC.open({ base: G.QTOOLS.ctx.base, bytes, names, inputs, campioni: null, pol, pool: G.ATTRIBUTIO_CORE.pool });
      say.textContent = `${data.rows.toLocaleString("en")} rows in ${data.files} files.`;
      G.PIPET.mount(box, { session, names, gate: { ok: true, table: true }, say: (/** @type {any} */ e) => { say.textContent = String((e && (e.message || e.key)) || e); }, cs: null, ppm: () => 5, pol: () => pol, formula: "", levels: null, identsOf: () => [] });
    } catch (e) { say.textContent = "Feature table: " + ((/** @type {any} */ (e)).message || e); }
  } });
})();
