// @ts-strict
"use strict";
// Drawing of the Kendrick mass defect section (kmd.js): the plot Kendrick mass against KMD, the table of its values for the «Dati» dialog, the tables of the
// candidate series and of the ions. Pure functions of the result of KMDCALC.analyse and of the selection (series, row): no state.
const KMDVIEW = (() => {
  const MAX_ROWS = 200;
  const t = (k, p) => I18N.t(k, p);
  const eh = s => EH(s == null ? "" : s);
  const fmt = (v, d = 3) => (v == null || !isFinite(v) ? "–" : v.toFixed(d));
  // ---------------------------------------------------------------- the plot: Kendrick mass (x) against KMD (y); a series has its colour AND its shape
  const W = 320, H = 250, L = 52, R = 10, T = 10, B = 34;
  const SHAPES = ["circle", "square", "triangle", "diamond"];
  const colorOf = n => PAL[(n - 1) % PAL.length] || "var(--accent)";
  function mark(shape, x, y, r, attrs) {
    if (shape === "square") return `<rect x="${(x - r).toFixed(1)}" y="${(y - r).toFixed(1)}" width="${2 * r}" height="${2 * r}" ${attrs}/>`;
    if (shape === "triangle") return `<path d="M${x.toFixed(1)} ${(y - r * 1.2).toFixed(1)}L${(x + r * 1.1).toFixed(1)} ${(y + r * 0.9).toFixed(1)}L${(x - r * 1.1).toFixed(1)} ${(y + r * 0.9).toFixed(1)}Z" ${attrs}/>`;
    if (shape === "diamond") return `<path d="M${x.toFixed(1)} ${(y - r * 1.3).toFixed(1)}L${(x + r * 1.3).toFixed(1)} ${y.toFixed(1)}L${x.toFixed(1)} ${(y + r * 1.3).toFixed(1)}L${(x - r * 1.3).toFixed(1)} ${y.toFixed(1)}Z" ${attrs}/>`;
    return `<circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="${r}" ${attrs}/>`;
  }
  function plotSvg(res, selSer, selRow) {
    const rows = res.rows;
    if (!rows.length) return "";
    const xs = rows.map(r => r.km), ys = rows.map(r => r.kmd);
    let x0 = Math.min(...xs), x1 = Math.max(...xs), y0 = Math.min(...ys), y1 = Math.max(...ys);
    const px = (x1 - x0 || 1) * 0.06, py = (y1 - y0 || 0.01) * 0.12;
    x0 -= px; x1 += px; y0 -= py; y1 += py;
    const X = v => L + (v - x0) / (x1 - x0) * (W - L - R), Y = v => T + (1 - (v - y0) / (y1 - y0)) * (H - T - B);
    let s = `<svg class="pca-plot" id="kmd-plot" viewBox="0 0 ${W} ${H}" role="img" aria-label="${eh(t("kmd.plot.aria", { unit: res.unit.id, n: rows.length, s: res.series.length }))}">`
      + `<line x1="${L}" y1="${H - B}" x2="${W - R}" y2="${H - B}" stroke="var(--line)"/><line x1="${L}" y1="${T}" x2="${L}" y2="${H - B}" stroke="var(--line)"/>`
      + `<text x="${(L + W - R) / 2}" y="${H - 6}" text-anchor="middle" font-size="9" fill="var(--ink)">${eh(t("kmd.axis.km", { unit: res.unit.id }))}</text>`
      + `<text x="9" y="${(T + H - B) / 2}" text-anchor="middle" font-size="9" fill="var(--ink)" transform="rotate(-90 9 ${(T + H - B) / 2})">${eh(t("kmd.axis.kmd"))}</text>`
      + `<text x="${L}" y="${H - B + 11}" font-size="8" fill="var(--muted)">${eh(fmt(x0, 1))}</text><text x="${W - R}" y="${H - B + 11}" text-anchor="end" font-size="8" fill="var(--muted)">${eh(fmt(x1, 1))}</text>`
      + `<text x="${L - 3}" y="${H - B}" text-anchor="end" font-size="8" fill="var(--muted)">${eh(fmt(y0, 4))}</text><text x="${L - 3}" y="${T + 6}" text-anchor="end" font-size="8" fill="var(--muted)">${eh(fmt(y1, 4))}</text>`;
    rows.forEach(r => {
      const x = X(r.km), y = Y(r.kmd), on = selRow === r.i, dim = selSer != null && r.series !== selSer;
      const shape = r.series ? SHAPES[(r.series - 1) % SHAPES.length] : "circle", col = r.series ? colorOf(r.series) : "var(--muted)";
      const tip = `<title>${eh(t("kmd.point.tip", { mz: fmt(r.mz, 4), rt: fmt(r.rt, 2), kmd: fmt(r.kmd, 4), s: r.series || "–" }))}</title>`;
      s += `<g data-i="${r.i}" tabindex="0" role="button" aria-label="${eh(t("kmd.row.aria", { mz: fmt(r.mz, 4), rt: fmt(r.rt, 2) }))}" opacity="${dim ? 0.3 : 1}">`
        // 24 px target around a small mark
        + `<circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="12" fill="transparent"/>`
        + mark(shape, x, y, on ? 5.6 : 3.8, `fill="${col}" fill-opacity="${r.series ? 1 : 0.55}" stroke="${on ? "var(--ink)" : "var(--ink)"}" stroke-width="${on ? 1.6 : 0.5}"`) + tip + `</g>`;
    });
    return s + "</svg>";
  }
  // the values of the plot, for the «Dati» table (tdati.js)
  function plotTable(res) {
    if (!res) return null;
    return { head: ["ID", "m/z", "RT", t("kmd.col.km"), "KMD", t("kmd.col.series")], rowHeader: true,
      rows: res.rows.map(r => [r.id, fmt(r.mz, 4), fmt(r.rt, 2), fmt(r.km, 4), fmt(r.kmd, 4), r.series || ""]) };
  }

  // ---------------------------------------------------------------- tables
  const stepsText = s => { const c = {}; s.steps.forEach(x => { c[x] = (c[x] || 0) + 1; }); return Object.keys(c).map(k => `${k}×${c[k]}`).join(", ") || "–"; };
  function seriesTable(res, selSer) {
    if (!res.series.length) return `<p class="sm" role="status">${eh(t("kmd.noseries", { min: res.minMembers, tol: res.tolMDa }))}</p>`;
    return `<div class="qq-scroll"><table class="qq-tab" id="kmd-series"><thead><tr><th scope="col">${t("kmd.col.series")}</th><th scope="col">${t("kmd.col.members")}</th><th scope="col">KMD</th><th scope="col">${t("kmd.col.span")}</th>`
      + `<th scope="col">${t("kmd.col.steps")}</th><th scope="col">${t("kmd.col.rt")}</th></tr></thead><tbody>`
      + res.series.map(s => `<tr data-s="${s.n}" tabindex="0" role="button" aria-label="${eh(t("kmd.series.aria", { n: s.n, m: s.members.length }))}"${selSer === s.n ? ' aria-current="true" class="sel"' : ""}>`
        + `<td class="num">${s.n}</td><td class="num">${s.members.length}</td><td class="num">${fmt(s.kmd, 4)}</td><td class="num">${fmt(s.spanMDa, 2)}</td><td>${eh(stepsText(s))}</td><td class="num">${fmt(s.rho, 2)}</td></tr>`).join("")
      + `</tbody></table></div>`;
  }
  function visibleRows(res, selSer) {
    const list = selSer != null ? res.series[selSer - 1].members.map(i => res.rows.find(r => r.i === i)) : res.rows.slice().sort((a, b) => (a.series ? 0 : 1) - (b.series ? 0 : 1) || a.kmd - b.kmd || a.km - b.km);
    return list.filter(Boolean);
  }
  function featTable(res, selSer, selRow) {
    const list = visibleRows(res, selSer), shown = list.slice(0, MAX_ROWS);
    return `<div class="qq-scroll"><table class="qq-tab" id="kmd-tab"><thead><tr><th scope="col">ID</th><th scope="col">m/z</th><th scope="col">RT</th><th scope="col">${t("kmd.col.km")}</th><th scope="col">KMD</th><th scope="col">${t("kmd.col.series")}</th></tr></thead><tbody>`
      + shown.map(r => `<tr data-i="${r.i}" tabindex="0" role="button" aria-label="${eh(t("kmd.row.aria", { mz: fmt(r.mz, 4), rt: fmt(r.rt, 2) }))}"${selRow === r.i ? ' aria-current="true" class="sel"' : ""}>`
        + `<td class="num">${eh(r.id)}</td><td class="num">${fmt(r.mz, 4)}</td><td class="num">${fmt(r.rt, 2)}</td><td class="num">${fmt(r.km, 4)}</td><td class="num">${fmt(r.kmd, 4)}</td><td class="num">${r.series || "–"}</td></tr>`).join("")
      + `</tbody></table></div>` + (list.length > shown.length ? `<p class="sm">${eh(t("kmd.more", { n: shown.length, m: list.length }))}</p>` : "");
  }

  return { plotSvg, plotTable, seriesTable, featTable, stepsText, fmt };
})();
window.KMDVIEW = KMDVIEW;
