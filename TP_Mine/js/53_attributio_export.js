// @ts-check
// TPMINE-PRIVATE  Exports of Attributio (admin mode): the menu «Export» of the Attributio entry. `ATTRIBUTIO_EXPORT.mount(host, ctx)` draws a button that opens six items (evidence, Schymanski folder,
// publication table, quantification of Level 1, gradient, SIRIUS input); the engine (the .wasm `attributio`, or `attributio_rev` for the evidence, or the native route `feat/attributio/export`) answers the files and this page
// saves them: a spreadsheet as XLSX, the evidence and the publication table as a .zip (the plots with the CSV of their data / the XLSX with its CSV). Texts in English, without keys of lang/.
(() => {
  "use strict";
  /** @typedef {{ pool: () => Promise<any>, files: () => {path: string, file: File}[], root: () => string, say: (text: string, err?: boolean) => void }} Ctx */

  /** the exports: engine kind, label, what the user gets */
  const ITEMS = [
    { kind: "evidence", label: "Evidence (mirror plots + CSV)", hint: "A .zip with one mirror plot (SVG) per decided 2a/2b feature, the CSV of the data of each plot and the evidence table." },
    { kind: "schymanski", label: "Schymanski folder", hint: "An XLSX with the sheets of the identification folder." },
    { kind: "publication", label: "Publication table", hint: "A .zip with the XLSX (Identities, README, Excluded) and the CSV of the identities." },
    { kind: "quantify", label: "Quantification (Level 1)", hint: "An XLSX with the Level 1 compounds quantified on the calibration of the campaign." },
    { kind: "gradient", label: "Gradient", hint: "An XLSX with the identities and the features at the two sites." },
    { kind: "sirius", label: "SIRIUS input (.ms + MGF)", hint: "A .zip with the MS1 and MS2 spectra of the features in the SIRIUS .ms format and as MGF." },
  ];

  /** @param {string} name @param {BlobPart} data @param {string} type */
  function save(name, data, type) {
    const a = document.createElement("a"); a.href = URL.createObjectURL(new Blob([data], { type })); a.download = name; a.click(); setTimeout(() => URL.revokeObjectURL(a.href), 2000);
  }

  /** the rows `{hojas}` of the engine as the sheets of `xlsxBuild` @param {{ hojas: { nombre: string, filas: any[][] }[] }} rows */
  const sheets = rows => rows.hojas.map(h => ({ name: h.nombre, head: h.filas[0], rows: h.filas.slice(1) }));

  /** save the files of an answer of the engine; returns the names saved @param {string} kind @param {string} root @param {{ files: any[] }} answer */
  async function deliver(kind, root, answer) {
    const W = /** @type {any} */ (window), enc = new TextEncoder(), files = answer.files;
    const bytes = /** @type {[string, Uint8Array][]} */ (files.map(f => [f.name, f.rows ? W.xlsxBuild(sheets(f.rows)) : enc.encode(f.text)]));
    const single = files.length === 1 || (kind !== "evidence" && kind !== "publication" && kind !== "sirius");
    if (single) {
      const f = files[0];
      if (f.rows) await W.dlx(f.name, sheets(f.rows), false); else save(f.name, f.text, "text/plain;charset=utf-8");
      return [f.name];
    }
    const stem = (kind === "evidence" ? "evidence_" : kind === "sirius" ? "sirius_" : "publication_") + (root || "campaign");
    save(stem + ".zip", W.zipStore(bytes.map(([n, b]) => [stem + "/" + n, b])), "application/zip");
    return [stem + ".zip"];
  }

  /** @param {HTMLElement} host @param {Ctx} ctx */
  function mount(host, ctx) {
    host.innerHTML = `<span class="at-exp"><button type="button" class="at-x" id="at-export" aria-haspopup="menu" aria-expanded="false" aria-label="Export" disabled>Export</button>
      <span role="menu" id="at-export-menu" aria-label="Export" hidden>${ITEMS.map(i => `<button type="button" class="at-x" role="menuitem" data-kind="${i.kind}" title="${i.hint}" aria-label="${i.label}">${i.label}</button>`).join("")}</span></span>`;
    const btn = /** @type {HTMLButtonElement} */ (host.querySelector("#at-export")), menu = /** @type {HTMLElement} */ (host.querySelector("#at-export-menu"));
    const close = () => { menu.hidden = true; btn.setAttribute("aria-expanded", "false"); };
    btn.onclick = () => { menu.hidden = !menu.hidden; btn.setAttribute("aria-expanded", String(!menu.hidden)); };
    host.addEventListener("keydown", ev => { if (/** @type {KeyboardEvent} */ (ev).key === "Escape") { close(); btn.focus(); } });
    menu.onclick = async ev => {
      const b = /** @type {HTMLElement} */ (ev.target).closest("button[data-kind]");
      if (!b) return;
      close();
      const kind = /** @type {string} */ (b.getAttribute("data-kind")), item = ITEMS.find(i => i.kind === kind);
      ctx.say("Exporting: " + (item ? item.label : kind) + "...");
      try {
        const pool = await ctx.pool(), r = await pool.attributioExport(ctx.files(), ctx.root(), kind, { generated: new Date().toISOString().slice(0, 10) });
        const names = await deliver(kind, ctx.root(), r);
        ctx.say("Exported " + names.join(", ") + (r.warnings && r.warnings.length ? " (" + r.warnings.join("; ") + ")" : ""));
      } catch (e) { ctx.say("Export failed: " + (/** @type {any} */ (e).message || e), true); }
    };
  }

  /** @type {any} */ (window).ATTRIBUTIO_EXPORT = { mount, ITEMS, deliver, sheets };
})();
