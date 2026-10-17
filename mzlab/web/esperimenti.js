// @ts-strict
"use strict";
// Registry of the experiment types of the Dati view (format esperimenti/1): the only place that names Full Scan, MS2 (Product Ion) and MRM.
// Classic script, loaded after stato.js. A new experiment type is one entry here plus its texts in modi.js (didattica = key of QMODI.M).
const ESP = (() => {
  const voci = [
    { id: "full", ordine: 1, etichetta: "Full Scan", didattica: "full", spettri: true, vista: "tic-spettro" },
    { id: "ms2", ordine: 2, etichetta: "MS2 (Product Ion)", didattica: "prod", spettri: true },
    { id: "mrm", ordine: 3, etichetta: "MRM", didattica: "mrm", spettri: false },
    // direct infusion with MSn on fixed precursors (the tree of infusione.js)
    { id: "infusion", ordine: 4, etichetta: I18N.t("inf.name"), didattica: "msn", spettri: true, vista: "tic-spettro", nodi: true },
  ];
  const voce = id => voci.find(v => v.id === id);
  /** @param {any} p */
  // the experiment of a panel: the MRM panel is its own, the other ones follow their tab
  const di = p => p.type === "mrm" ? "mrm" : p.tab;
  return {
    formato: "esperimenti/1", voci, voce,
    ids: voci.map(v => v.id),
    // [id, label] in order: what the tab bar and the overview list
    coppie: voci.map(v => [v.id, v.etichetta]),
    etichetta: id => (voce(id) || { etichetta: id }).etichetta,
    di,
    // the id of the product-ion experiment, for the code that only has to name it
    ms2: "ms2",
    /** @param {any} p */
    // a trace with scans behind (not the MRM) can open a spectrum
    spettri: p => { const v = voce(di(p)); return v ? v.spettri : true; },
    didattica: id => (voce(id) || {}).didattica,           // key of the teaching texts in modi.js
  };
})();
