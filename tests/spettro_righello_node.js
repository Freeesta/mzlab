// Node check of RIGHELLO.layout (mzlab/web/spettro-righello.js): the ruler never crosses the labels of the peaks. Exits 1 on the first failure.
const fs = require("fs"), assert = require("assert");
const R = new Function(fs.readFileSync(process.argv[2], "utf8").replace("const RIGHELLO", "var RIGHELLO") + ";return RIGHELLO")();
const X = m => 40 + (m - 100) * 5, top = 20, base = 300, lh = 13, W = 560;
// labels of the peaks 152.2 and 194.2 (and 181.0) sitting where the first rows of the ruler would be
const lab = (m, y, w = 34) => ({ x: X(m) - w / 2 - 3, y, w: w + 6, h: 15 });
const obs = [lab(152.2, 40), lab(194.2, 38), lab(181.0, 60)];
const wt = 70, ref = { x: X(194.2), w: 20 };
const L = R.layout({ obs, x0: 40, x1: W - 10, top, base, lh, ref, items: [{ xa: X(152.2), xb: X(194.2), w: wt }, { xa: X(181.0), xb: X(194.2), w: wt }] });
for (const r of L.rects) for (const o of obs) assert(!R.hit(r, o), ["rect crosses a peak label", r, o]);
// the two rows and the «rif.» text do not overlap each other
for (let i = 0; i < L.rects.length; i++) for (let j = i + 1; j < L.rects.length; j++) assert(L.rects[i].g === L.rects[j].g || !R.hit(L.rects[i], L.rects[j]), ["ruler rects overlap", L.rects[i], L.rects[j]]);
// the verticals stay out of the labels (a vertical that crosses a label is cut around it)
L.items.forEach((it, i) => { for (const [seg, x] of [[it.va, i ? X(181.0) : X(152.2)], [it.vb, X(194.2)]]) for (const [a, b] of seg) for (const o of obs) if (x >= o.x && x <= o.x + o.w) assert(b <= o.y || a >= o.y + o.h, ["vertical crosses a label", x, a, b, o]); });
assert(L.ref && L.items.length === 2 && L.items.every(it => it.y > top && it.y < base));
// nothing in the way: the default rows
const E = R.layout({ obs: [], x0: 40, x1: W - 10, top, base, lh, ref: null, items: [{ xa: 100, xb: 200, w: 60 }, { xa: 120, xb: 220, w: 60 }] });
assert.strictEqual(E.items[0].y, top + 26); assert(E.items[1].y > E.items[0].y);
// the text stays inside the box
const N = R.layout({ obs: [], x0: 40, x1: 200, top, base, lh, ref: null, items: [{ xa: 150, xb: 195, w: 90 }] });
assert(N.items[0].bx >= 42 && N.items[0].bx + 90 <= 198, N.items[0].bx);
console.log("ok");
