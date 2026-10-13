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
  return {
    formato: "esperimenti/1", voci, voce,
    ids: voci.map(v => v.id),
    coppie: voci.map(v => [v.id, v.etichetta]),            // [id, label] in order: what the tab bar and the overview list
    etichetta: id => (voce(id) || { etichetta: id }).etichetta,
    didattica: id => (voce(id) || {}).didattica,           // key of the teaching texts in modi.js
  };
})();
