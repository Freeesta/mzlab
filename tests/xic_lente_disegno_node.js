// Node check of XICDRAW.ticks / columns (mzlab/web/xic-lente-disegno.js): round, equidistant, equally-decimal ticks. Exits 1 on the first failure.
const fs = require("fs"), assert = require("assert");
const D = new Function(fs.readFileSync(process.argv[2], "utf8").replace("const XICPIECES", "var XICPIECES").replace("const XICDRAW", "var XICDRAW") + ";return XICDRAW")();
const show = (a, b) => { const t = D.ticks(a, b); return t.vals.map(v => v.toFixed(t.dec)); };
assert.deepStrictEqual(show(0.5, 20), ["5", "10", "15", "20"]);
assert.deepStrictEqual(show(0, 20), ["0", "5", "10", "15", "20"]);
assert.deepStrictEqual(show(0, 18), ["0", "5", "10", "15"]);
assert.deepStrictEqual(show(0.5, 10.3), ["2", "4", "6", "8", "10"]);
assert.deepStrictEqual(show(12, 14), ["12.0", "12.5", "13.0", "13.5", "14.0"]);   // zoom on 2 min: one decimal on every tick
assert.deepStrictEqual(show(14.2, 14.5), ["14.2", "14.3", "14.4", "14.5"]);
for (const [a, b] of [[0, 20], [0.5, 10.3], [3, 47], [0, 120], [13.7, 14.1], [0, 1], [5, 5.04]]) {
  const t = D.ticks(a, b), d = t.vals.map((v, i) => i ? v - t.vals[i - 1] : 0).slice(1);
  assert(t.vals.length >= 4 && t.vals.length <= 10, [a, b, t.vals]);
  assert(d.every(x => Math.abs(x - d[0]) < 1e-9), ["equidistant", a, b, t.vals]);
  assert(t.vals.every(v => v >= a - 1e-9 && v <= b + 1e-9), ["inside", a, b, t.vals]);
}
// a 0.05 min peak on 20 min at 160 px is one column wide and keeps its top
const n = 4000, rt = Array.from({ length: n }, (_, i) => 20 * i / (n - 1)), y = rt.map(r => Math.exp(-(((r - 14.3) / 0.015) ** 2) / 2) * 1000 + 1);
const idx = D.columns({ rt, y }, 160), top = Math.max(...idx.map(i => y[i]));
assert(top > 990, top); assert(idx.length <= 4 * 160, idx.length);
// the shared pieces used by the map XIC: axis numbers stay inside [lo, hi]; trace fills with the given colour and keeps the tall column
const calls = [], g = { globalAlpha: 1, measureText: s => ({ width: 6 * s.length }), beginPath() {}, moveTo() {}, lineTo() {}, stroke() {}, closePath() {}, setLineDash() {}, fill() { calls.push(["fill", this.fillStyle, this.globalAlpha]); }, fillText(s, x) { calls.push(["text", s, x]); } };
const X = v => 6 + v / 20 * 150, tx = D.axis(g, X, 0, 20, { base: 80, labelY: 94, lo: 1, hi: 159, color: "#123", font: "9px sans-serif" });
assert.strictEqual(tx.length, 5); assert(calls.filter(c => c[0] === "text").every(c => c[2] >= 1 && c[2] <= 159));
D.trace(g, { rt, y }, X, v => 80 - v / 1000 * 60, 80, 150, "#e69f00");
assert.deepStrictEqual(calls.pop(), ["fill", "#e69f00", 0.3]);
console.log("ok");
