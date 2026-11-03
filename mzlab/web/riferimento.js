// @ts-strict
"use strict";
// «Difference from the reference» (high resolution only; «Statistica» panel). This stub only registers the sections of the panel; the code that computes and
// draws the table is riferimento-corpo.js, fetched when the section is first shown (it sets window.RIFERIMENTO: mount, run, inspect, tsv, params, result).
if (typeof STAT !== "undefined") STAT.add({ id: "reference", title: "ref.title", hr: true, visible: () => true, mount: box => { carica("riferimento-corpo.js").then(() => window.RIFERIMENTO.mount(box)).catch(console.error); } });
// the PCA section keeps its place; pca.js is fetched when the section is first shown (it registers the same id, which STAT ignores)
if (typeof STAT !== "undefined") STAT.add({ id: "pca", title: "pca.title", hr: true, visible: () => true, mount: box => { carica("pca.js").then(() => PCA.mount(box)).catch(console.error); } });
// KMD and PFAS likewise: their scripts (and kmd-vista.js) are fetched when the section is first shown, in the same place of the panel
if (typeof STAT !== "undefined") {
  STAT.add({ id: "kmd", title: "kmd.title", hr: true, visible: () => true, mount: box => { carica("kmd-vista.js").then(() => carica("kmd.js")).then(() => KMD.mount(box)).catch(console.error); } });
  STAT.add({ id: "pfas", title: "pfas.title", hr: true, visible: () => true, mount: box => { carica("m:pfas.js").then(() => /** @type {any} */ (window).PFAS.mount(box)).catch(console.error); } });
}
