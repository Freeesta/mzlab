// Node check of LAYOUT.geometria (mzlab/web/layout.js): prints one JSON object with the cases.
const fs = require("fs");
const L = new Function(fs.readFileSync(process.argv[2], "utf8").replace("const LAYOUT", "var LAYOUT") + ";return LAYOUT")();
const two = [{ voci: [{ id: 1, h: 300 }, { id: 2, h: 200 }] }], out = {};
for (const W of [930, 1280, 1920, 800]) out[W] = L.geometria(two, W, 6, 440);
out.quote = L.geometria([{ voci: [{ id: 1, h: 100, quota: 0.9 }, { id: 2, h: 100, quota: 0.1 }] }], 1280, 6, 440);
out.tre = L.geometria([{ voci: [{ id: 1, h: 100 }, { id: 2, h: 100 }, { id: 3, h: 100 }] }], 1920, 6, 440);
out.tre930 = L.geometria([{ voci: [{ id: 1, h: 100 }, { id: 2, h: 100 }, { id: 3, h: 100 }] }], 930, 6, 440);
out.pila = L.geometria([{ voci: [{ id: 1, h: 100 }] }, { voci: [{ id: 2, h: 80 }], gapDopo: 0 }, { voci: [{ id: 3, h: 50 }] }], 1000, 6, 440);
console.log(JSON.stringify(out));
