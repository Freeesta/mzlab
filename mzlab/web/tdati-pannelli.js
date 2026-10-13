// @ts-strict
// The panels of the Dati view (chromatograms, XIC, MRM, spectra, maps) give their data to TDATI (tdati.js): the table of what is drawn and the short description
// read by screen readers. Called by afterDraw (pannelli.js) after every drawing.
const T = (k, p) => I18N.t(k, p);
const MAXROWS = 300;
const nm = (v, n = 3) => TDATI.num(v, n);
const ptitle = p => String(p.title || "").replace(/<[^>]*>/g, "");

// at most MAXROWS indices of x inside [lo, hi], evenly spread
function pick(x, lo, hi) {
  const ix = []; for (let i = 0; i < x.length; i++) if (x[i] >= lo && x[i] <= hi) ix.push(i);
  if (ix.length <= MAXROWS) return ix;
  const out = [], st = ix.length / MAXROWS; for (let k = 0; k < MAXROWS; k++) out.push(ix[Math.floor(k * st)]);
  return out;
}
function lineData(p) {
  const a = p._a; if (!a || !a.sr) return null;
  const rows = [], dec = p.kind === "press" ? 1 : 0;
  a.sr.forEach(s => {
    pick(s.x, a.x0, a.x1).forEach(i => rows.push([s.name, nm(s.x[i], 4), nm(s.y[i], dec || 1)]));
    (p.ints || []).filter(it => it.key === s.key).forEach(it => {
      let b = -1; for (let i = 0; i < s.x.length; i++) if (s.x[i] >= it.a && s.x[i] <= it.b && (b < 0 || s.y[i] > s.y[b])) b = i;
      if (b >= 0) rows.push([`${s.name} · ${T("tdati.apex", { a: nm(it.a, 2), b: nm(it.b, 2) })}`, nm(s.x[b], 4), nm(s.y[b], 1)]);
    });
  });
  const unit = p.kind === "press" ? T("axis.pressure") : T("tdati.col.int");
  return { title: ptitle(p), head: [T("tdati.col.trace"), T("tdati.col.rt"), unit], rows, rowHeader: true, note: T("tdati.note.line", { a: nm(a.x0, 2), b: nm(a.x1, 2), n: MAXROWS }) };
}
function specData(p) {
  const a = p._a; if (!a || !a.data || !a.data.length) return null;
  const rows = [];
  a.data.forEach(x => {
    const d = x.d, top = Math.max(...d.y, 1e-9), thr = (p.thr ?? 5) / 100 * top, sel = [];
    d.mz.forEach((m, i) => { if (m >= a.x0 && m <= a.x1 && d.y[i] >= thr) sel.push(i); });
    sel.sort((u, v) => d.y[v] - d.y[u]).splice(MAXROWS);
    sel.sort((u, v) => d.mz[u] - d.mz[v]).forEach(i => rows.push([x.f ? x.f.label : "", nm(d.mz[i], a.dec || 4), nm(d.y[i], 1), nm(100 * d.y[i] / top, 1)]));
  });
  return { title: ptitle(p), head: [T("tdati.col.file"), "m/z", T("tdati.col.int"), T("tdati.col.rel")], rows, rowHeader: true, note: T("tdati.note.spec", { thr: p.thr ?? 5, a: nm(a.x0, 2), b: nm(a.x1, 2) }) };
}
function mapData(p) {
  const a = p._a; if (!a || !a.mapv) return null;
  const N = 60, cells = [];
  for (let i = 0; i < N; i++) for (let j = 0; j < N; j++) {
    const rt = a.x0 + (i + 0.5) / N * (a.x1 - a.x0), mz = a.y0 + (j + 0.5) / N * (a.y1 - a.y0), v = a.mapv(rt, mz);
    if (v != null && v > 0) cells.push([rt, mz, v]);
  }
  cells.sort((u, v) => v[2] - u[2]).splice(100);
  return { title: ptitle(p), head: [T("tdati.col.rt"), "m/z", T("tdati.col.int")], rows: cells.map(c => [nm(c[0], 3), nm(c[1], 2), nm(c[2], 1)]), note: T("tdati.note.map", { n: cells.length }) };
}
const data = p => (p.type === "spec" ? specData(p) : p.type === "map" ? mapData(p) : lineData(p));

function text(p) {
  const a = p._a; if (!a) return T("a11y.empty");
  if (p.type === "spec") {
    const n = a.data ? a.data[0].d.mz.filter((m, i) => m >= a.x0 && m <= a.x1 && a.data[0].d.y[i] >= (p.thr ?? 5) / 100 * Math.max(...a.data[0].d.y, 1e-9)).length : 0;
    return T("a11y.spec", { title: ptitle(p), a: nm(a.x0, 2), b: nm(a.x1, 2), n });
  }
  if (p.type === "map") return T("a11y.map", { title: ptitle(p), a: nm(a.x0, 2), b: nm(a.x1, 2), c: nm(a.y0, 1), d: nm(a.y1, 1) });
  return T("a11y.line", { title: ptitle(p), n: (a.sr || []).length, a: nm(a.x0, 2), b: nm(a.x1, 2), y: nm(a.ymax, 0), p: (p.ints || []).length });
}
function update(p) {
  if (!p.cv || p._exp) return;
  TDATI.attach(p.cv, () => data(p));
  TDATI.describe(p.cv, text(p));
  p.cv.setAttribute("aria-label", ptitle(p) || T("a11y.chart"));
}
// SVG charts that already sit next to a table of the same values (QA/QC, profile, PCA loadings): the same dialog shows that table
TDATI.watch("svg.qq-chart", (el, t) => TDATI.fromDom(el.closest(".qq-ion")?.querySelector("table"), t));
TDATI.watch("svg.prof-chart", (el, t) => TDATI.fromDom(el.closest("#prof-res")?.querySelector("table"), t));
TDATI.watch("svg#pca-loadings", (el, t) => TDATI.fromDom(document.querySelector("#pca-tab"), t));
TDATI.watch("svg#pca-scores", (el, t) => (window.PCA && PCA.scoresTable ? { title: t, ...PCA.scoresTable() } : null));
TDATI.watch("svg#kmd-plot", (el, t) => (window.KMD && KMD.plotTable ? { title: t, ...KMD.plotTable() } : null));
window.TDATI_P = { update, data, text };
