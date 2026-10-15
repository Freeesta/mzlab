// @ts-check
// @ts-strict
"use strict";
// Sight of the RT x m/z map, the parts that do not touch the page (classic script, loaded before mappa.js; node loads it with require).
// The same code serves the Python engine (Pyodide) and the Rust one: it only decides WHAT is asked and WHEN, never how the engine answers.
//   nearest(rts, rt)      index of the scan closest to rt in a sorted RT axis (ties: the earlier scan, like np.argmin in Item.nearest_scan)
//   latest(fn, schedule)  coalescing: ask(v) any number of times, fn runs for the newest v only, one at a time, at most once per frame
//   lru(maxBytes)         cache with a memory budget; the least recently used entries leave first
//   diffPeaks / diffTrace  A − B of two spectra (peak by peak within a tolerance) and of two traces (B interpolated on the RT of A)
//   trace(v, nmz, nrt, j) / column(...)  the row and the column of the map matrix at a bin: the preview drawn while the sight moves
(() => {
  function nearest(rts, rt) {
    const n = rts.length; if (!n) return -1;
    let lo = 0, hi = n - 1;
    while (hi - lo > 1) { const mid = (lo + hi) >> 1; if (rts[mid] <= rt) lo = mid; else hi = mid; }
    return Math.abs(rts[hi] - rt) < Math.abs(rts[lo] - rt) ? hi : lo;
  }

  function latest(fn, schedule) {
    const later = schedule || (typeof requestAnimationFrame === "function" ? requestAnimationFrame : f => setTimeout(f, 16));
    let has = false, val, busy = false, queued = false, gen = 0;
    const pump = async () => {
      if (busy || !has) return;
      busy = true; const v = val, g = ++gen; has = false;
      try { await fn(v, () => has); } catch (e) { /* the next ask retries */ }
      busy = false; if (has) pump();
    };
    const ask = v => { val = v; has = true; if (!queued && !busy) { queued = true; later(() => { queued = false; pump(); }); } };
    ask.pending = () => has || busy; ask.generation = () => gen;
    return ask;
  }

  function lru(maxBytes) {
    const m = new Map(); let bytes = 0;
    const api = {
      get(k) { if (!m.has(k)) return undefined; const e = m.get(k); m.delete(k); m.set(k, e); return e.v; },
      set(k, v, size) {
        if (m.has(k)) { bytes -= m.get(k).size; m.delete(k); }
        m.set(k, { v, size }); bytes += size;
        for (const [key, e] of m) { if (bytes <= api.max || m.size <= 1) break; bytes -= e.size; m.delete(key); }
      },
      drop(prefix) { for (const [k, e] of m) if (k.startsWith(prefix)) { bytes -= e.size; m.delete(k); } },
      dropIf(pred) { for (const [k, e] of m) if (pred(k)) { bytes -= e.size; m.delete(k); } },
      clear() { m.clear(); bytes = 0; },
      get bytes() { return bytes; }, get size() { return m.size; }, max: maxBytes,
    };
    return api;
  }

  // the budget: 64 MB, 32 MB on an iPad (and any touch device with little memory)
  const budget = () => {
    const nav = typeof navigator !== "undefined" ? navigator : {};
    const small = /iPad|iPhone/.test(nav.userAgent || "") || (nav.maxTouchPoints > 1 && /Mac/.test(nav.platform || "")) || (nav.deviceMemory && nav.deviceMemory <= 4);
    return (small ? 32 : 64) * 1024 * 1024;
  };

  // row j of a matrix stored as v[i * nmz + j] (i = RT bin): {x: centres of the RT bins, y}
  function trace(v, nrt, nmz, j, rt0, rt1) {
    const x = new Array(nrt), y = new Array(nrt), w = (rt1 - rt0) / nrt;
    for (let i = 0; i < nrt; i++) { x[i] = rt0 + (i + 0.5) * w; y[i] = v[i * nmz + j]; }
    return { x, y };
  }
  // column i: {x: centres of the m/z bins, y}
  function column(v, nmz, i, mz0, dmz) {
    const x = new Array(nmz), y = new Array(nmz);
    for (let j = 0; j < nmz; j++) { x[j] = mz0 + (j + 0.5) * dmz; y[j] = v[i * nmz + j]; }
    return { x, y };
  }

  // A − B of two centroid spectra (sorted m/z): every A peak minus the most intense B peak within tol(mz) Da (a B peak is used once);
  // the B peaks left over come out negative. Result sorted by m/z; positive = more in A, negative = more in B.
  function diffPeaks(amz, ay, bmz, by, tol) {
    const used = new Uint8Array(bmz.length), out = [];
    let lo = 0;
    for (let i = 0; i < amz.length; i++) {
      const t = tol(amz[i]); while (lo < bmz.length && bmz[lo] < amz[i] - t) lo++;
      let best = -1; for (let j = lo; j < bmz.length && bmz[j] <= amz[i] + t; j++) if (!used[j] && (best < 0 || by[j] > by[best])) best = j;
      if (best >= 0) used[best] = 1;
      out.push([amz[i], ay[i] - (best >= 0 ? by[best] : 0)]);
    }
    for (let j = 0; j < bmz.length; j++) if (!used[j]) out.push([bmz[j], -by[j]]);
    out.sort((u, v) => u[0] - v[0]);
    return { mz: out.map(o => o[0]), y: out.map(o => o[1]) };
  }
  // A − B of two traces: B read on the RT of A by linear interpolation (0 outside the RT range of B)
  function diffTrace(rta, ya, rtb, yb) {
    if (!rtb.length) return { rt: Array.from(rta), y: Array.from(ya) };
    const y = new Array(rta.length); let j = 0;
    for (let i = 0; i < rta.length; i++) {
      const t = rta[i]; while (j < rtb.length - 2 && rtb[j + 1] < t) j++;
      let b = 0;
      if (rtb.length === 1) b = rtb[0] === t ? yb[0] : 0;
      else if (t >= rtb[0] && t <= rtb[rtb.length - 1]) { const d = rtb[j + 1] - rtb[j]; b = d > 0 ? yb[j] + (yb[j + 1] - yb[j]) * Math.min(1, Math.max(0, (t - rtb[j]) / d)) : yb[j]; }
      y[i] = ya[i] - b;
    }
    return { rt: Array.from(rta), y };
  }

  const MIRINO = { nearest, latest, lru, budget, trace, column, diffPeaks, diffTrace };
  if (typeof window !== "undefined") window.MIRINO = MIRINO;
  if (typeof module !== "undefined") module.exports = MIRINO;
})();
