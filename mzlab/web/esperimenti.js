// @ts-strict
"use strict";
// Registry of the experiment types of the Dati view (format esperimenti/1): the only place that names Full Scan, MS2 (Product Ion) and MRM.
// Classic script, loaded after stato.js. A new experiment type is one entry here plus its texts in modi.js (didattica = key of QMODI.M).
const ESP = (() => {
  const voci = [
    { id: "full", ordine: 1, etichetta: "Full Scan", didattica: "full", spettri: true },
    { id: "ms2", ordine: 2, etichetta: "MS2 (Product Ion)", didattica: "prod", spettri: true },
    { id: "mrm", ordine: 3, etichetta: "MRM", didattica: "mrm", spettri: false },
  ];
  const voce = id => voci.find(v => v.id === id);
  /** @param {any} p */
  const di = p => p.type === "mrm" ? "mrm" : p.tab;             // the experiment of a panel: the MRM panel is its own, the other ones follow their tab
  return {
    formato: "esperimenti/1", voci, voce,
    ids: voci.map(v => v.id),
    coppie: voci.map(v => [v.id, v.etichetta]),            // [id, label] in order: what the tab bar and the overview list
    etichetta: id => (voce(id) || { etichetta: id }).etichetta,
    di,
    ms2: "ms2",                                              // the id of the product-ion experiment, for the code that only has to name it
    /** @param {any} p */
    spettri: p => { const v = voce(di(p)); return v ? v.spettri : true; },   // a trace with scans behind (not the MRM) can open a spectrum
    didattica: id => (voce(id) || {}).didattica,           // key of the teaching texts in modi.js
  };
})();
