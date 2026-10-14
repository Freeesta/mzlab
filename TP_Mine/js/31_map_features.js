// @ts-strict
// TPMINE-PRIVATE  Features of the last high-resolution result for the RT x m/z map of the page (window.MAPFEAT, read by mzlab/web/mappa-estrai.js): markers, and
// «How it was found» for the chosen one. Read-only: it only re-reads the result the page already holds (rows, files, settings, inclusion list); the search is not run again.
// window.MAPFEAT.features(file) -> [{ id, mz, rt, ppm, rt0?, rt1?, filters: [{ key, label, thr, value?, from?, cmp, unit?, note? }] }] or null (no result, or the file is not in it).
(() => {
  "use strict";
  const G = typeof window !== "undefined" ? window : globalThis;
  const PPM = 5;                                          // tolerance of the alignment of the traces (hr/features.py, align() and trace_area())
  const base = n => (String(n || "").split(/[\\/]/).pop() || "").replace(/\.(mzml|raw|d|wiff)$/i, "").toLowerCase();
  const num = v => typeof v === "number" && Number.isFinite(v);

  /** The features of a result `s` (mode "hr") that were seen in the file `f` ({file, label}): one per row with a positive area in that file. */
  function featuresFrom(s, f) {
    if (!s || s.mode !== "hr" || !Array.isArray(s.rows) || !Array.isArray(s.files) || !f) return null;
    const i = s.files.findIndex(x => base(x.name) === base(f.file) || (f.label && base(x.label) === base(f.label)));
    if (i < 0) return null;
    const st = s.settings || {}, types = s.files.map(x => x.type), incl = s.inclusion || [];
    const treated = [], blank = []; types.forEach((t, k) => { if (t === "blank") blank.push(k); else if (t === "sample") treated.push(k); });
    return s.rows.filter(r => r.series && r.series[i] > 0 && num(r.mz) && num(r.rt)).map(r => {
      const edges = incl.find(x => Math.abs(x.mz - r.mz) < 1e-4 && Math.abs(x.rt - r.rt) < 0.011);          // only the best candidates have the edges of their integration in the result
      const nT = treated.filter(k => r.series[k] > 0).length, aT = Math.max(0, ...treated.map(k => r.series[k] || 0)), aB = Math.max(0, ...blank.map(k => r.series[k] || 0));
      const filters = [];
      if (num(st.min_height)) filters.push({ key: "min_height", label: "Peak height in this file", thr: st.min_height, from: "height", cmp: ">=", unit: "counts" });
      if (num(st.min_treated)) filters.push({ key: "min_treated", label: "Treated files with the peak", value: nT, thr: st.min_treated, cmp: ">=" });
      if (num(st.fold)) filters.push({ key: "fold", label: "Fold over the blanks", value: blank.length && aB > 0 ? aT / aB : null, thr: st.fold, cmp: ">=", note: blank.length ? "no signal in the blanks" : "no blank file" });
      if (num(st.rt_min)) filters.push({ key: "rt_min", label: "RT after the void", value: r.rt, thr: st.rt_min, cmp: ">=", unit: "min" });
      return { id: r.id, mz: r.mz, rt: r.rt, ppm: PPM, rt0: edges ? edges.rt_from : null, rt1: edges ? edges.rt_to : null, filters };
    });
  }

  let last = null, memo = null;
  function wrap() {                                      // the result of a run goes through TPHR.render: remember it there
    const h = G.TPHR; if (!h || h._mapfeat) return false;
    const render = h.render; h.render = function (root, s, ...rest) { last = s; memo = null; return render.call(this, root, s, ...rest); }; h._mapfeat = true; return true;
  }
  wrap();
  G.MAPFEAT = {
    features(f) { const key = f && (f.file || f.label); if (memo && memo.s === last && memo.key === key) return memo.v; const v = featuresFrom(last, f); memo = { s: last, key, v }; return v; },
    _from: featuresFrom, _set(s) { last = s; memo = null; }       // for the tests
  };
})();
