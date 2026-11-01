// @ts-strict
"use strict";
// Registry of the experiment types of the Dati view (format esperimenti/1): the only place that names Full Scan, MS2 (Product Ion) and MRM.
// Classic script, loaded after stato.js. A new experiment type is one entry here plus its texts in modi.js (didattica = key of QMODI.M).
const ESP = (() => {
  const voci = [
    { id: "full", ordine: 1, etichetta: "Full Scan", didattica: "full", spettri: true, vista: "tic-spettro" },
    { id: "ms2", ordine: 2, etichetta: "MS2 (Product Ion)", didattica: "prod", spettri: true, chiuso: true },
    { id: "mrm", ordine: 3, etichetta: "MRM", didattica: "mrm", spettri: false },
    // direct infusion with MSn on fixed precursors (the tree of infusione.js)
    { id: "infusion", ordine: 4, etichetta: I18N.t("inf.name"), didattica: "msn", spettri: true, vista: "tic-spettro", nodi: true },
  ];
  const voce = id => voci.find(v => v.id === id);
  // What an experiment type of api/canali (tipo.proposto) starts on: the kind of channel of the first trace (canali.js picks the channel of the file) and
  // whether the XIC lens is proposed. A default, not a permission: a type never switches a tool off.
  const predefiniti = {
    "lc-full": { canale: "tic", lente: true },
    "lc-ms2-mirato": { canale: "ms2", lente: true },
    "lc-dda": { canale: "ms1", lente: true },
    "lc-mrm": { canale: "srm", lente: true },
    "infusione-msn": { canale: "profondo", lente: false },
    "misto": { canale: "tic", lente: true },
    "solo-uv": { canale: "uv", lente: true },
  };
  /** @param {string | null | undefined} tipo */
  const predefinito = tipo => predefiniti[tipo || ""] || predefiniti["lc-full"];
  /** @param {any} p */
  // the channel a type starts on among the channels `v` of a file (`quant`: tells the SRM transition the student quantifies); null = the TIC
  /** @param {string} tipo @param {any[]} v @param {(x: any) => boolean} quant */
  const scegli = (tipo, v, quant) => {
    const d = predefinito(tipo).canale, liv = n => v.find(x => x.fam === "ms" && x.liv === n);
    return d === "ms1" ? liv(1) : d === "ms2" ? liv(2) : d === "profondo" ? v.filter(x => x.fam === "ms").sort((a, b) => b.liv - a.liv)[0]
      : d === "srm" ? v.find(x => x.fam === "srm" && quant(x)) || v.find(x => x.fam === "srm") : d === "uv" ? v.find(x => x.fam === "uv") : null;
  };
  // the experiment of a panel: the MRM panel is its own, the other ones follow their tab
  const di = p => p.type === "mrm" ? "mrm" : p.tab;
  return {
    formato: "esperimenti/1", voci, voce, predefiniti, predefinito, scegli,
    mrm: /** @param {any} p */ p => p.type === "mrm",
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
