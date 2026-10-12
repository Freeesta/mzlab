"use strict";
// Kendrick mass defect (KMD) of a list of ions: pure arithmetic, no page. Used by kmd.js (Statistica panel, high resolution) and tested under node.
// Kendrick mass KM = m/z x (nominal mass of the repeating unit / its exact mass); KMD = round(KM) - KM. The ions of a homologous series (they differ by a
// whole number of repeating units, e.g. CF2 in the PFAS: their KM differ by the nominal mass of the unit) fall on the SAME KMD: the series are CANDIDATES with their evidence (members, steps, retention trend);
// the program never says what an ion is.
(function (root) {
  // exact monoisotopic masses (C = 12 by definition)
  const M = { C: 12, H: 1.00782503223, F: 18.99840316273, O: 15.99491461957 };
  const unit = (id, parts) => { const mass = Object.entries(parts).reduce((a, [e, n]) => a + M[e] * n, 0); return { id, mass, nominal: Math.round(mass) }; };
  const UNITS = [unit("CF2", { C: 1, F: 2 }), unit("CF2O", { C: 1, F: 2, O: 1 }), unit("C2F4", { C: 2, F: 4 }), unit("CH2", { C: 1, H: 2 })];
  const unitById = id => UNITS.find(u => u.id === id) || UNITS[0];

  // Kendrick mass, nominal Kendrick mass and KMD of one m/z for a repeating unit
  function kendrick(mz, u) {
    const km = mz * u.nominal / u.mass, nkm = Math.round(km);
    return { km, nkm, kmd: nkm - km };
  }

  // Spearman rank correlation of two equal-length lists (average ranks for ties); null under 3 points or without variation
  function ranks(v) {
    const idx = v.map((x, i) => i).sort((a, b) => v[a] - v[b]), r = new Array(v.length);
    for (let i = 0; i < idx.length;) {
      let j = i; while (j + 1 < idx.length && v[idx[j + 1]] === v[idx[i]]) j++;
      for (let k = i; k <= j; k++) r[idx[k]] = (i + j) / 2 + 1;
      i = j + 1;
    }
    return r;
  }
  function spearman(x, y) {
    if (x.length < 3) return null;
    const a = ranks(x), b = ranks(y), n = a.length, ma = a.reduce((s, v) => s + v, 0) / n, mb = b.reduce((s, v) => s + v, 0) / n;
    let sab = 0, saa = 0, sbb = 0;
    for (let i = 0; i < n; i++) { sab += (a[i] - ma) * (b[i] - mb); saa += (a[i] - ma) ** 2; sbb += (b[i] - mb) ** 2; }
    return saa > 0 && sbb > 0 ? sab / Math.sqrt(saa * sbb) : null;
  }

  // features: [{id, mz, rt}]. opts: unit (id of UNITS), tolMDa (width of the KMD window of a series, in mDa), minMembers (distinct nominal Kendrick masses).
  // -> { unit, rows: [{i, id, mz, rt, km, nkm, kmd, series: number|null}], series: [{n, members: [row indexes by nkm], kmd, spanMDa, steps, consecutive, rho}] }
  function analyse(features, opts) {
    const o = Object.assign({ unit: "CF2", tolMDa: 3, minMembers: 3 }, opts || {}), u = unitById(o.unit), tol = Math.max(0, +o.tolMDa || 0) / 1000;
    const rows = features.map((f, i) => Object.assign({ i, id: f.id, mz: +f.mz, rt: f.rt == null ? null : +f.rt, series: null }, kendrick(+f.mz, u))).filter(r => isFinite(r.mz) && r.mz > 0);
    const order = rows.slice().sort((a, b) => a.kmd - b.kmd), series = [];
    // KMD windows: a window starts at the lowest free KMD and takes every ion within 2 x tolerance of it (the window is +-tol around its centre)
    for (let s = 0; s < order.length;) {
      let e = s; while (e + 1 < order.length && order[e + 1].kmd - order[s].kmd <= 2 * tol) e++;
      const grp = order.slice(s, e + 1), nk = new Set(grp.map(r => r.nkm));
      if (nk.size >= o.minMembers) {
        const mem = grp.slice().sort((a, b) => a.nkm - b.nkm || a.kmd - b.kmd), kmds = grp.map(r => r.kmd);
        const steps = []; for (let k = 1; k < mem.length; k++) if (mem[k].nkm !== mem[k - 1].nkm) steps.push(Math.round((mem[k].nkm - mem[k - 1].nkm) / u.nominal));
        const withRt = mem.filter(r => r.rt != null), rho = spearman(withRt.map(r => r.nkm), withRt.map(r => r.rt));
        const ser = { n: series.length + 1, members: mem.map(r => r.i), kmd: kmds.reduce((a, v) => a + v, 0) / kmds.length, spanMDa: (Math.max(...kmds) - Math.min(...kmds)) * 1000,
          steps, consecutive: steps.length ? steps.filter(x => x === 1).length / steps.length : 0, distinct: nk.size, rho };
        series.push(ser); mem.forEach(r => { r.series = ser.n; });
        s = e + 1;
      } else s++;
    }
    return { unit: u, tolMDa: o.tolMDa, minMembers: o.minMembers, rows, series };
  }

  const api = { UNITS, unitById, kendrick, spearman, analyse };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else root.KMDCALC = api;
})(typeof window !== "undefined" ? window : globalThis);
