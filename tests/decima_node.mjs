// Checks mzlab/web/decima.js against the point-by-point loop it replaced (grafici.js drawLines), on synthetic traces; prints one JSON object. See tests/test_decima.py.
import { amax, amin, isSorted, m4, windowIdx } from "../mzlab/web/decima.js";

let seed = 12345;
const rnd = () => { seed = (seed * 1664525 + 1013904223) >>> 0; return seed / 4294967296; };

// the old loop: flat [px, py, ...] of first, lowest, highest, last per pixel column, over the points with x0 <= x <= x1
function reference(x, y, x0, x1, X, V) {
  const out = []; let curX = -999, minY, maxY, firstY, lastY, minFirst = true;
  const flush = cx => { if (curX === -999) return; out.push(cx, firstY); if (minFirst) out.push(cx, minY, cx, maxY); else out.push(cx, maxY, cx, minY); out.push(cx, lastY); };
  x.forEach((r, i) => {
    if (r < x0 || r > x1) return;
    const px = Math.round(X(r)), py = V(y[i]);
    if (px !== curX) { flush(curX); curX = px; minY = maxY = firstY = lastY = py; minFirst = true; }
    else { if (py < minY) { minY = py; if (maxY === firstY) minFirst = true; } if (py > maxY) { maxY = py; if (minY === firstY) minFirst = false; } lastY = py; }
  });
  flush(curX);
  return out;
}
const same = (a, b) => a.length === b.length && a.every((v, i) => Object.is(v, b[i]) || (Number.isNaN(v) && Number.isNaN(b[i])));

// traces: [name, x, y]
function traces() {
  const T = [];
  const grid = (n, t0, t1) => Array.from({ length: n }, (_, i) => t0 + (t1 - t0) * i / (n - 1));
  for (const n of [50, 3000, 50000, 200000]) {
    const x = grid(n, 0.3, 25), y = x.map((t, i) => 200 + 80 * rnd() + [4, 9, 13, 18, 22].reduce((s, c) => s + 2e5 * Math.exp(-0.5 * ((t - c) / 0.05) ** 2), 0));
    T.push([`gauss${n}`, x, y]);
  }
  const x1 = grid(40000, 0, 30); T.push(["one-point-peaks", x1, x1.map((_, i) => (i % 997 === 0 ? 1e6 : 10 * rnd()))]);
  T.push(["three-point-peaks", x1, x1.map((_, i) => ([1, 2, 3].includes(i % 500) ? [5e4, 9e5, 4e4][(i % 500) - 1] : 5 * rnd()))]);
  T.push(["zeros", x1, x1.map(() => 0)]);
  T.push(["nan", x1, x1.map((_, i) => (i % 13 === 0 ? NaN : 100 * rnd()))]);
  T.push(["nan-first", x1, x1.map((_, i) => (i < 3 ? NaN : 100 * rnd()))]);
  T.push(["negative", x1, x1.map(() => 50 - 100 * rnd())]);
  const xu = Array.from({ length: 30000 }, () => 30 * rnd()).sort((a, b) => a - b); T.push(["non-uniform-x", xu, xu.map(() => 1000 * rnd())]);
  const xd = grid(20000, 0, 10).map((t, i) => (i % 7 ? t : t - 1e-9)).sort((a, b) => a - b); T.push(["duplicates", xd.map(v => Math.round(v * 20) / 20), xd.map(() => 10 * rnd())]);
  T.push(["float64array", Float64Array.from(x1), Float64Array.from(x1, () => 100 * rnd())]);
  return T;
}

const out = { traces: 0, windows: 0, mismatches: [], lost_apex: [] };
const log = v => (v <= 1 ? 0 : Math.log10(v));
for (const [name, x, y] of traces()) {
  const lo = x[0], hi = x[x.length - 1];
  const wins = [[lo, hi], [lo + (hi - lo) * 0.3, lo + (hi - lo) * 0.31], [lo + (hi - lo) * 0.45, lo + (hi - lo) * 0.7], [lo - 5, lo + (hi - lo) * 0.002], [hi - (hi - lo) * 0.0005, hi + 5], [lo + 1e-3, lo + 2e-3], [hi + 1, hi + 2]];
  for (const [x0, x1] of wins) for (const W of [300, 1200, 2400]) for (const kind of ["lin", "log"]) {
    out.windows++;
    const X = r => 66 + (r - x0) / (x1 - x0) * (W - 80), ymax = 1e6;
    const V = kind === "lin" ? v => 300 - (v - 0) / ymax * 250 : v => 300 - log(Math.max(v, 1)) / 6 * 250;
    const [i0, i1] = windowIdx(x, x0, x1);
    const got = m4(x, y, i0, i1, X, V), want = reference(Array.from(x), Array.from(y), x0, x1, X, V);
    if (!same(got, want)) out.mismatches.push(`${name} [${x0.toFixed(3)},${x1.toFixed(3)}] W=${W} ${kind}`);
    // no apex lost: the highest and the lowest value of every column are vertices of that column
    const cols = new Map();
    for (let i = i0; i < i1; i++) { if (Number.isNaN(y[i])) continue; const c = Math.round(X(x[i])); const p = V(y[i]); const e = cols.get(c); if (!e) cols.set(c, [p, p]); else { if (p < e[0]) e[0] = p; if (p > e[1]) e[1] = p; } }
    const have = new Map(); for (let k = 0; k < got.length; k += 2) { const e = have.get(got[k]) || new Set(); e.add(got[k + 1]); have.set(got[k], e); }
    for (const [c, [pmin, pmax]] of cols) if (!(have.get(c)?.has(pmin) && have.get(c)?.has(pmax)) && !name.startsWith("nan")) out.lost_apex.push(`${name} col ${c}`);
  }
  out.traces++;
}

// windowIdx against a linear search on random inputs
let bad = 0;
for (let t = 0; t < 100000; t++) {
  const n = 1 + Math.floor(rnd() * 40), x = Array.from({ length: n }, () => Math.round(rnd() * 20)).sort((a, b) => a - b);
  const a = Math.round(rnd() * 24) - 2, b = a + Math.round(rnd() * 12) - 2;
  const [i0, i1] = windowIdx(x, a, b), idx = x.map((v, i) => (v >= a && v <= b ? i : -1)).filter(i => i >= 0);
  const wantLo = idx.length ? idx[0] : i0, wantHi = idx.length ? idx[idx.length - 1] + 1 : i0;
  if (i0 !== wantLo || i1 !== wantHi || (idx.length === 0 && i1 > i0 && b >= a)) bad++;
}
out.window_idx_bad = bad;

// ten to the six values: no RangeError (Math.max(...a) throws above ~10^5), NaN ignored, ranges
const big = Float64Array.from({ length: 1000000 }, (_, i) => Math.sin(i) * 1e3); big[777777] = 5e9; big[123456] = -7e9; big[5] = NaN;
out.big = { max: amax(big), min: amin(big), range: amax(big, 0, 100), spread_throws: (() => { try { Math.max(...big); return false; } catch (e) { return e instanceof RangeError; } })() };
out.sorted = [isSorted([1, 2, 2, 3]), isSorted([1, 3, 2])];
console.log(JSON.stringify(out));
