// @ts-check
// Download-menu entries «MassBank record» and «SIRIUS 6 .ms» of an HR MS2 spectrum. Only the entries live at start-up; fair.js (the
// record builders and the dialog) is fetched with carica() when one is chosen, to keep the start-up list light (tests/test_peso_avvio.py).
if (typeof AZIONI !== "undefined") {
  /** @param {any} c */
  const ok = c => c.p.level === 2;
  // same gate as FAIR.available (an HR MS2 spectrum that WORKFLOW can read), without loading fair.js
  /** @param {any} c */
  const can = c => { try { const x = window.WORKFLOW && WORKFLOW.extractSpecData(c.p); return !!x && !x.isLr; } catch (_) { return false; } };
  /** @param {any} c @param {"massbank" | "sirius"} f */
  const run = (c, f) => carica("m:fair.js").then(() => FAIR[f](c.p));
  AZIONI.registra({ id: "spec.exportMassbank", gruppo: "lib", ordine: 3, viste: ["spec"], menu: false, scarico: true, visibile: ok, disponibile: can, nome: () => I18N.t("ctx.spec.exportMassbank"), esegui: /** @param {any} c */ c => run(c, "massbank") });
  AZIONI.registra({ id: "spec.exportSirius", gruppo: "lib", ordine: 3, viste: ["spec"], menu: false, scarico: true, visibile: ok, disponibile: can, nome: () => I18N.t("ctx.spec.exportSirius"), esegui: /** @param {any} c */ c => run(c, "sirius") });
}
