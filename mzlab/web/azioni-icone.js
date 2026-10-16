// @ts-check
// @ts-strict
"use strict";
// Icons of the actions of the registry azioni/1 (azioni.js): one table for the right-click menu and the palette. 16x16, stroke = currentColor (follows the theme and the grey of a
// disabled entry), decorative (aria-hidden: the name of the action is the text next to it). An action gets its icon from the key `icona` or, when it has none, from POR_ID.
// The map actions (mappa.js) bring their own `svg`. Classic script, loaded before azioni.js.
const AZIONI_ICONE = (() => {
  /** @type {Record<string, string>} */
  const DISEGNI = {
    copy: '<rect x="5.5" y="5.5" width="8" height="8" rx="1.3"/><path d="M10.5 5.5V3.8a1.3 1.3 0 0 0-1.3-1.3H3.8a1.3 1.3 0 0 0-1.3 1.3v5.4a1.3 1.3 0 0 0 1.3 1.3h1.7"/>',
    xic: '<path d="M2 13.5h12"/><path d="M2.5 12.5c2.5 0 3-8 5.5-8s3 8 5.5 8"/>',
    ms2: '<path d="M8 2.5v3.5M8 6L4 10.5M8 6l4 4.5M4 10.5v3M12 10.5v3"/>',
    dda: '<path d="M8 2.5v3.5M8 6L3.5 11M8 6v6M8 6l4.5 5"/>',
    target: '<circle cx="8" cy="8" r="4.2"/><path d="M8 1.8v3M8 11.2v3M1.8 8h3M11.2 8h3"/>',
    overlay: '<path d="M1.5 12.5c1.7-8 3.5-8 5.2 0M5.5 12.5c1.7-8 3.5-8 5.2 0"/><path d="M2 13.5h12"/>',
    measure: '<rect x="2.5" y="5" width="11" height="6" rx="1"/><path d="M5 5v2.5M7.5 5v2M10 5v2.5M12 5v2"/>',
    note: '<path d="M2.5 3h11v7h-6.2L4.3 13v-3H2.5z"/>',
    iso: '<path d="M3 13V8M6 13V4M9 13V7M12 13V10"/>',
    isoRes: '<path d="M2.5 13.5h11"/><path d="M5 13V8M8 13V4M11 13V7"/>',
    isoOff: '<path d="M3 13V8M6 13V4M9 13V7M12 13V10"/><path d="M2.5 2.5l11 11"/>',
    fit: '<path d="M2.5 6V2.5H6M10 2.5h3.5V6M13.5 10v3.5H10M6 13.5H2.5V10"/>',
    reset: '<path d="M3 8a5 5 0 1 0 1.6-3.7"/><path d="M3 2.5v3h3"/>',
    lock: '<rect x="3.5" y="7.5" width="9" height="6" rx="1.2"/><path d="M5.5 7.5V5.5a2.5 2.5 0 0 1 5 0v2"/>',
    link: '<path d="M6.5 9.5l3-3"/><path d="M5 8L3.8 9.2a2.1 2.1 0 0 0 3 3L8 11"/><path d="M11 8l1.2-1.2a2.1 2.1 0 0 0-3-3L8 5"/>',
    avg: '<path d="M3 5V3h10v2"/><path d="M4.5 13.5V8.5M8 13.5V6.5M11.5 13.5V9.5"/>',
    atRt: '<path d="M8 2v12"/><path d="M3 13.5v-4M5.5 13.5v-7M10.5 13.5v-6M13 13.5v-3.5"/>',
    plus: '<path d="M8 3.5v9M3.5 8h9"/>',
    formula: '<path d="M2 12.5l3-8 3 8M3.2 9.5h3.6"/><path d="M10 5h4M10 8h4M10 11h4"/>',
    clear: '<path d="M4 4l8 8M12 4l-8 8"/>',
    flag: '<path d="M4 14V2.5"/><path d="M4 3h8l-2 2.5L12 8H4"/>',
    filter: '<path d="M2.5 3h11L9.5 8.3v4.7l-3-1.6V8.3z"/>',
    sliders: '<path d="M2.5 5h11M2.5 11h11"/><path d="M6 3v4M10 9v4"/>',
    eye: '<path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z"/><circle cx="8" cy="8" r="1.8"/>',
    book: '<path d="M3 2.5h8a2 2 0 0 1 2 2v9H5a2 2 0 0 1-2-2z"/><path d="M3 11.5a2 2 0 0 1 2-2h8"/>',
    search: '<circle cx="7" cy="7" r="4.2"/><path d="M10.2 10.2L14 14"/>',
    tree: '<path d="M8 2v4M8 6H4v3M8 6h4v3M4 9v3M12 9v3"/>',
    baseline: '<path d="M2 13.5h12"/><path d="M3 13c1.5-1 2-8 5-8s3.5 7 5 8"/>',
    area: '<path d="M2 13.5h12"/><path d="M3.5 13.5c1.5-1 2-8.5 4.5-8.5s3 7.5 4.5 8.5z"/>',
    gauge: '<path d="M3 11.5a5 5 0 1 1 10 0"/><path d="M8 11.5l2.5-3.5"/>',
    ramp: '<path d="M2.5 12.5h3.5l4-7h3.5"/>',
    save: '<path d="M8 2.5v8M4.5 7.5L8 11l3.5-3.5M3 13.5h10"/>',
    list: '<path d="M3 4h10M3 8h10M3 12h10"/>',
    listCheck: '<path d="M2.5 4h11M2.5 8h6M2.5 12h4"/><path d="M9.5 11.5l2 2 3-3.5"/>',
    tag: '<path d="M2.5 2.5h6l5 5-6 6-5-5z"/><circle cx="5.6" cy="5.6" r=".7"/>',
    edit: '<path d="M2.5 13.5l1-3.5 7-7 2.5 2.5-7 7z"/><path d="M9 4.5l2.5 2.5"/>',
    trash: '<path d="M3 4.5h10M6 4.5V3h4v1.5M4.5 4.5l.7 9h5.6l.7-9M7 7v4M9 7v4"/>',
  };
  // the icon of the actions that do not name one (`icona` of the action wins)
  /** @type {Record<string, string>} */
  const POR_ID = {
    "spec.copy": "copy", "spec.extractMs2": "ms2", "spec.xic": "xic", "spec.peak": "target", "spec.overlay": "overlay", "spec.measure": "measure", "spec.annotate": "note",
    "spec.iso": "iso", "spec.isoRes": "isoRes", "spec.isoOff": "isoOff", "spec.full": "fit", "spec.freeze": "lock", "spec.relink": "link", "spec.generate": "formula", "spec.fmClear": "clear",
    "spec.marker": "flag", "spec.noCont": "filter", "spec.disp": "sliders", "spec.ms2": "ms2", "spec.dda": "dda", "spec.ddaShow": "eye", "spec.lib": "book", "spec.exportMgf": "save",
    "spec.exportMsp": "save", "spec.exportMassbank": "save", "spec.exportSirius": "save", "spec.contLists": "list", "spec.known": "listCheck", "spec.addLab": "tag",
    "chrom.avgSel": "avg", "chrom.specAtRt": "atRt", "chrom.xic": "xic", "xic.addIon": "plus", "chrom.annotate": "note", "chrom.disp": "sliders", "chrom.ddaShow": "eye",
    "chrom.identAll": "search", "chrom.tree": "tree", "chrom.ibk": "baseline", "chrom.int": "area", "chrom.press": "gauge", "chrom.grad": "ramp",
    "view.resetZoom": "reset", "ann.edit": "edit", "ann.delete": "trash", "metodo.xic": "xic", "metodo.ms2": "ms2",
  };
  /** @param {string} k the SVG of the icon `k`, decorative; "" when it does not exist */
  const svg = k => DISEGNI[k]
    ? `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${DISEGNI[k]}</svg>` : "";
  return { DISEGNI, POR_ID, svg };
})();
if (typeof module !== "undefined") module.exports = AZIONI_ICONE;
