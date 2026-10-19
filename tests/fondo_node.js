// Node check of FONDO (mzlab/web/fondo.js): the properties the background correction promises. Exits 1 on the first failure.
const fs = require("fs"), assert = require("assert");
const FONDO = new Function(fs.readFileSync(process.argv[2], "utf8").replace("const FONDO", "var FONDO") + ";return FONDO")();
const FONDOSP = new Function("FONDO", fs.readFileSync(process.argv[3], "utf8").replace("const FONDOSP", "var FONDOSP") + ";return FONDOSP")(FONDO);
let seed = 12345; const rnd = () => { seed |= 0; seed = (seed + 0x6D2B79F5) | 0; let t = Math.imul(seed ^ (seed >>> 15), 1 | seed); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
const gauss = () => Math.sqrt(-2 * Math.log(rnd() + 1e-12)) * Math.cos(2 * Math.PI * rnd());
const N = 3000, x = Array.from({ length: N }, (_, i) => i * 0.01), sd = 1000;
const noise = () => x.map(() => 5000 + sd * gauss());
const mean = a => a.reduce((p, q) => p + q, 0) / a.length;
const peak = (h, c, fwhm) => x.map(t => h * Math.exp(-4 * Math.log(2) * Math.pow((t - c) / fwhm, 2)));
const ib = { mode: "mean", a: [0.2, 2], b: null };
// 1. constant and line backgrounds are linear: background(y - f·b) = background(y) - f·background(b)
{
  const y = noise().map((v, i) => v + 20 * x[i] * 100), b = noise(), f = 0.7, d = y.map((v, i) => v - f * b[i]);
  for (const o of [{ mode: "mean", a: [0.2, 2], b: null }, { mode: "median", a: [0.2, 2], b: null }, { mode: "line", a: [0.2, 2], b: [25, 28] }]) {
    const lhs = FONDO.apply(x, d, { mode: "tract", ib: o }).y, ry = FONDO.apply(x, y, { mode: "tract", ib: o }).y, rb = FONDO.apply(x, b, { mode: "tract", ib: o }).y;
    if (o.mode !== "median") lhs.forEach((v, i) => assert(Math.abs(v - (ry[i] - f * rb[i])) < 1e-6, "linearity " + o.mode));
  }
}
// 2. nothing is clipped: a trace of pure noise has negative values after the background
{ const r = FONDO.apply(x, noise(), { mode: "tract", ib }); assert(Math.min(...r.y) < -2 * sd, "negatives kept"); }
// 3. the automatic background is centred: the mean of pure noise is 0 (|mean| < 0.2 sigma)
{ const r = FONDO.apply(x, noise(), { mode: "auto", w: 1 }); assert(Math.abs(mean(r.y)) < 0.2 * sd, "centred mean " + mean(r.y)); }
// 4. a peak of FWHM 0.3 min keeps >= 95 % of its area with the 1 min window
{
  const p = peak(40000, 15, 0.3), y = p.map(v => v + 5000), r = FONDO.apply(x, y, { mode: "auto", w: 1 }), a = p.reduce((s, v) => s + v, 0), b = r.y.reduce((s, v, i) => s + (x[i] > 13 && x[i] < 17 ? v : 0), 0);
  assert(b / a > 0.95 && b / a < 1.05, "area " + b / a);
}
// 5. the migration of an old notebook
{
  assert.deepStrictEqual(FONDO.migra({ snip: true, snipw: 2 }), { mode: "auto", w: 2, ib: null });
  assert.deepStrictEqual(FONDO.migra({ ibk: { mode: "line", a: [0, 1], b: [5, 6], clip: true } }), { mode: "tract", w: 1, ib: { mode: "line", a: [0, 1], b: [5, 6] } });
  assert.strictEqual(FONDO.clipOld({ ibk: { mode: "mean", a: [0, 1], clip: true } }), true);
  assert.strictEqual(FONDO.clipOld({ fondo: { mode: "none" }, ibk: { clip: true } }), false);
  assert.deepStrictEqual(FONDO.migra({}), { mode: "none", w: 1, ib: null });
  assert.deepStrictEqual(FONDO.migra({ fondo: { mode: "auto", w: 0.5 } }), { mode: "auto", w: 0.5, ib: null });
  assert.strictEqual(FONDO.apply(x, noise(), { mode: "none" }), null);
}
// 6. the initial zone: SNIP does not use it, the front does not lift the baseline, the zone can be set
{
  const fr = peak(80000, 0.8, 0.25), y = noise().map((v, i) => v + fr[i]);
  const zd = FONDO.front(x, y), bl = FONDO.auto(x, y, 1, zd);
  assert(Math.abs(bl[0] - 5000) < sd && Math.abs(bl[100] - 5000) < sd, "baseline in the zone " + bl[0]);
  assert.strictEqual(FONDO.zoneEnd(x, { mode: "auto" }), 0.5);
  assert.strictEqual(FONDO.zoneEnd(x, { mode: "auto", zd: 1.2 }), 1.2);
  assert.strictEqual(FONDO.zoneEnd(x, { mode: "none" }), -Infinity);
  assert.deepStrictEqual(FONDO.migra({ fondo: { mode: "auto", w: 1, zd: 0.8 } }), { mode: "auto", w: 1, ib: null, zd: 0.8 });
  assert(FONDO.apply(x, y, { mode: "auto", w: 1 }).bl.max > 0, "baseline returned");
}
// 7. detection of the solvent front (synthetic, apex at 0.8 min): the end is just after the peak; a flat trace has none
{
  const fr = peak(80000, 0.8, 0.25), y = noise().map((v, i) => v + fr[i]), e = FONDO.front(x, y);
  assert(e != null && e > 0.8 && e < 1.6, "front end " + e);
  assert.strictEqual(FONDO.front(x, noise()), null);
}
// 8. automatic background windows for the spectrum: waste (zeros) + solvent front + stable state + gradient with a peak; the best window is quiet, after the front, and holds no peak
{
  const fr = peak(5e6, 2.0, 0.3), pk = peak(2e5, 6, 0.2), y = noise().map((v, i) => x[i] < 1.5 ? 0 : v + fr[i] + pk[i] + (x[i] > 10 ? (x[i] - 10) * 4000 : 0)), r = FONDOSP.regioni(x, y);
  assert(r && r.lead > 2 && r.lead < 3.5, "lead " + (r && r.lead));
  assert(r.cands.length === 3 && r.cands.every(c => c[0] >= r.lead && c[1] > c[0]), JSON.stringify(r));
  assert(r.cands[0][1] < 10 && !(r.cands[0][0] <= 6 && r.cands[0][1] >= 6), "best " + r.cands[0]);
  const ov = (a, b) => a[0] < b[1] && b[0] < a[1]; assert(!ov(r.cands[0], r.cands[1]) && !ov(r.cands[1], r.cands[2]) && !ov(r.cands[0], r.cands[2]), "overlap");
  assert(FONDOSP.waste(x, y) >= 1.4 && FONDOSP.waste(x, noise()) === 0, "waste");
  assert.strictEqual(FONDOSP.regioni([0, 1, 2], [1, 1, 1]), null);
}
console.log("ok");
