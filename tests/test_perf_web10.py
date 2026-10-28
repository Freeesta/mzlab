"""PERF-WEB10: the array helpers of grafici.js, the allocation-free paths of liste.js and the sgFilter of picchi.js give the numbers they always gave.

The old code (Math.max(...a), find().some(), clamp in the inner loop, regex count of ; and ,) is copied below as the reference."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
WEB = ROOT / "mzlab" / "web"
NODE = shutil.which("node")
pytestmark = pytest.mark.skipif(not NODE, reason="node non disponibile")

PRELUDE = r"""
const fs = require('fs');
const web = process.argv[1];
// the helpers of numeri.js, evaluated alone
const gsrc = fs.readFileSync(web + '/numeri.js', 'utf8');
const H = new Function(gsrc + '; return { maxOf, minOf, maxAbs, lowerIdx, spanOf, b64F32 };')();
let seed = 12345; const rnd = () => (seed = (seed * 1664525 + 1013904223) >>> 0) / 4294967296;
"""


def run(body):
    p = subprocess.run([NODE, "-e", PRELUDE + body, str(WEB)], capture_output=True, text=True, timeout=120)
    assert p.returncode == 0, p.stderr
    return json.loads(p.stdout.strip().splitlines()[-1])


def test_helpers_big_arrays_and_nan():
    r = run(r"""
const big = new Array(200000).fill(0).map((_, i) => i * 0.5), out = {};
try { Math.max(...big); out.spreadThrows = false; } catch (e) { out.spreadThrows = e instanceof RangeError; }
out.max = H.maxOf(big); out.min = H.minOf(big, 7); out.empty = [H.maxOf([]), H.minOf([]), H.maxOf([], 1e-9), H.maxAbs([], 1e-9)];
out.nan = [H.maxOf([1, NaN, 3]), H.minOf([NaN, 1]), H.maxAbs([1, NaN])].map(v => v !== v);
out.typed = [H.maxOf(new Float32Array([1, 5, 2])), H.maxAbs(new Float64Array([-9, 3]), 1e-9)];
// spanOf holds every index of the window, for ascending and for shuffled arrays
let ok = true;
for (let t = 0; t < 300; t++) {
  const n = 1 + Math.floor(rnd() * 60), a = Array.from({ length: n }, () => Math.round(rnd() * 100) / 4);
  if (t % 2) a.sort((p, q) => p - q);
  const lo = rnd() * 25, hi = lo + rnd() * 10, [i0, i1] = H.spanOf(a, lo, hi);
  for (let i = 0; i < n; i++) if (a[i] >= lo && a[i] <= hi && !(i >= i0 && i < i1)) ok = false;
  if (t % 2 && !(i1 - i0 <= a.filter(v => v >= lo && v <= hi).length)) ok = false;   // ascending: nothing outside the window is visited
}
out.span = ok;
console.log(JSON.stringify(out));
""")
    assert r["spreadThrows"] is True
    assert r["max"] == 99999.5 and r["min"] == 0
    assert r["empty"] == [None, None, 1e-9, 1e-9]
    assert r["nan"] == [True, True, True]
    assert r["typed"] == [5, 9]
    assert r["span"] is True


def test_liste_series_hasmatch_csv_equal_to_the_old_code():
    r = run(r"""
globalThis.I18N = { lang: 'it', t: k => k };
const L = require(web + '/liste.js');
const items = [];
for (let i = 0; i < 4000; i++) {
  const s = i % 3 === 0 ? { id: 'ser' + (i % 7), n: Math.floor(rnd() * 12) } : null;
  items.push({ id: 'e' + i, name: 'n' + i + (s ? ' n=' + s.n : '') + ' (x)', mz: 100 + rnd() * 900, pol: [0, 1, -1][i % 3], adduct: '[M+H]+', series: s });
}
const idx = L.buildIndex([{ id: 'keller', name: 'K', items: items.slice(0, 2000) }, { id: 'usr', name: 'U', items: items.slice(2000) }]);
// old seriesRuns, byte for byte
function oldRuns(idx, pol, mzs, ppm, minRun = 3) {
  const found = new Map();
  mzs.forEach(m => L.find(idx, pol, m, ppm).forEach(it => {
    if (it.series && it.series.n != null) { const k = it.list + ':' + it.series.id; if (!found.has(k)) found.set(k, new Map()); found.get(k).set(it.series.n, it); }
  }));
  const out = new Map();
  found.forEach((byN, k) => {
    const ns = [...byN.keys()].sort((a, b) => a - b); let i = 0;
    while (i < ns.length) { let j = i; while (j + 1 < ns.length && ns[j + 1] === ns[j] + 1) j++;
      if (j - i + 1 >= minRun) { const nm = byN.get(ns[i]).name.replace(/ n=\d+$/, '').split(' (')[0]; const txt = `serie ${nm} (n = ${ns[i]}-${ns[j]})`; for (let q = i; q <= j; q++) out.set(k + '#' + ns[q], txt); }
      i = j + 1; }
  });
  return out;
}
let runsEq = true, hasEq = true, nonEmpty = 0;
for (let t = 0; t < 60; t++) {
  const mzs = Array.from({ length: 1000 }, () => (t % 2 ? items[Math.floor(rnd() * items.length)].mz + (rnd() - 0.5) * 1e-3 : 100 + rnd() * 900));
  const pol = ['positive', 'negative', null][t % 3], ppm = [2, 5, 50, 3000][t % 4];
  const a = [...oldRuns(idx, pol, mzs, ppm)].join('|'), b = [...L.seriesRuns(idx, pol, mzs, ppm)].join('|');
  if (a !== b) runsEq = false; if (b) nonEmpty++;
  for (const m of mzs.slice(0, 200)) if (L.find(idx, pol, m, ppm).some(x => x.list === 'keller') !== L.hasMatch(idx, pol, m, ppm, 'keller')) hasEq = false;
}
const out = { runsEq, hasEq, nonEmpty: nonEmpty > 5 };
out.head = [L.csvParse('mz;name\n150,5;A\n').length, L.csvParse('"a,b,c,d";mz;name\n"x";150.5;A\n').length, L.csvParse('mz,name\n150.5,A\n').length];
console.log(JSON.stringify(out));
""")
    assert r["runsEq"] and r["hasEq"] and r["nonEmpty"]
    assert r["head"] == [1, 1, 1]


def test_sgfilter_same_numbers_as_the_clamped_loop():
    r = run(r"""
const psrc = fs.readFileSync(web + '/picchi.js', 'utf8');
const a = psrc.indexOf('function sgFilter'), b = psrc.indexOf('// Parameters of the automatic integration');
const sg = new Function(psrc.slice(a, b) + '; return sgFilter;')();
function old(y, m, deriv) {
  const n = y.length, out = new Array(n), mm = m * (m + 1) * (2 * m + 1) / 3;
  const w = []; for (let j = -m; j <= m; j++) w.push(deriv ? j / mm : 3 * (3 * m * m + 3 * m - 1 - 5 * j * j) / ((2 * m - 1) * (2 * m + 1) * (2 * m + 3)));
  for (let i = 0; i < n; i++) { let v = 0; for (let j = -m; j <= m; j++) v += w[j + m] * y[Math.min(n - 1, Math.max(0, i + j))]; out[i] = v; }
  return out;
}
let same = true;
for (const n of [1, 5, 40, 500]) for (const m of [2, 3, 7, 30]) for (const d of [false, true]) {
  const y = Array.from({ length: n }, () => rnd() * 1000);
  const p = sg(y, m, d), q = old(y, m, d);
  for (let i = 0; i < n; i++) if (!Object.is(p[i], q[i])) same = false;
}
console.log(JSON.stringify({ same }));
""")
    assert r["same"] is True


def test_map_payload_decoding_and_corrupt_payload_error():
    r = run(r"""
global.atob = s => Buffer.from(s, 'base64').toString('latin1');
const f = new Float32Array([1.5, -2, 0, 1e9]), b64 = Buffer.from(f.buffer).toString('base64'), out = {};
out.same = Array.from(H.b64F32(b64, 4)).join() === Array.from(f).join() && H.b64F32(b64) instanceof Float32Array;
const msg = fn => { try { fn(); return null; } catch (e) { return e instanceof Error ? e.message : 'not an Error'; } };
out.badLen = msg(() => H.b64F32(Buffer.from([1, 2, 3, 4, 5]).toString('base64')));
out.badCount = msg(() => H.b64F32(b64, 5));
global.atob = () => { throw new DOMException('bad', 'InvalidCharacterError'); };
out.notB64 = msg(() => H.b64F32('%%%'));
console.log(JSON.stringify(out));
""")
    assert r["same"] is True
    assert "multiple of 4" in r["badLen"] and "expected 5" in r["badCount"] and "not base64" in r["notB64"]
