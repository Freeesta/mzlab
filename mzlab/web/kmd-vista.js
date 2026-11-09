// @ts-strict
"use strict";
// Drawing of the Kendrick mass defect section (kmd.js): the plot Kendrick mass against KMD, the table of its values for the «Dati» dialog, the tables of the
// candidate series and of the ions. Pure functions of the result of KMDCALC.analyse and of the selection (series, row): no state.
const KMDVIEW = (() => {
  const MAX_ROWS = 200;
  const t = (k, p) => I18N.t(k, p);
  const eh = s => EH(s == null ? "" : s);
  const fmt = (v, d = 3) => I18N.dash(v, d);
  // ---------------------------------------------------------------- the plot: Kendrick mass (x) against KMD (y); a series has its colour AND its shape
  const { W, H, L, R, T, B, SHAPES } = KMDCLOUD;          // the frame of the plot, shared with the dense drawing
  const colorOf = n => PAL[(n - 1) % PAL.length] || "var(--accent)";
  function mark(shape, x, y, r, attrs) {
    if (shape === "square") return `<rect x="${(x - r).toFixed(1)}" y="${(y - r).toFixed(1)}" width="${2 * r}" height="${2 * r}" ${attrs}/>`;
    if (shape === "triangle") return `<path d="M${x.toFixed(1)} ${(y - r * 1.2).toFixed(1)}L${(x + r * 1.1).toFixed(1)} ${(y + r * 0.9).toFixed(1)}L${(x - r * 1.1).toFixed(1)} ${(y + r * 0.9).toFixed(1)}Z" ${attrs}/>`;
    if (shape === "diamond") return `<path d="M${x.toFixed(1)} ${(y - r * 1.3).toFixed(1)}L${(x + r * 1.3).toFixed(1)} ${y.toFixed(1)}L${x.toFixed(1)} ${(y + r * 1.3).toFixed(1)}L${(x - r * 1.3).toFixed(1)} ${y.toFixed(1)}Z" ${attrs}/>`;
    return `<circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="${r}" ${attrs}/>`;
  }
  // above KMDCLOUD.CLOUD ions the points are paths (kmd-nuvola.js); the selected ion stays a button
  const tipOf = r => t("kmd.point.tip", { mz: fmt(r.mz, 4), rt: fmt(r.rt, 2), kmd: fmt(r.kmd, 4), s: r.series || "–" });
  function pointG(r, x, y, on, dim) {
    const shape = r.series ? SHAPES[(r.series - 1) % SHAPES.length] : "circle", col = r.series ? colorOf(r.series) : "var(--muted)";
    return `<g data-i="${r.i}" tabindex="0" role="button" aria-label="${eh(t("kmd.row.aria", { mz: fmt(r.mz, 4), rt: fmt(r.rt, 2) }))}" opacity="${dim ? 0.3 : 1}">`
      // 24 px target around a small mark
      + `<circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="12" fill="transparent"/>`
      + mark(shape, x, y, on ? 5.6 : 3.8, `fill="${col}" fill-opacity="${r.series ? 1 : 0.55}" stroke="var(--ink)" stroke-width="${on ? 1.6 : 0.5}"`) + `<title>${eh(tipOf(r))}</title></g>`;
  }
  function plotSvg(res, selSer, selRow) {
    const rows = res.rows;
    if (!rows.length) return "";
    const g = KMDCLOUD.geom(res), { x0, x1, y0, y1 } = g, cloud = KMDCLOUD.isCloud(res);
    let s = `<svg class="pca-plot" id="kmd-plot" viewBox="0 0 ${W} ${H}" role="img" aria-label="${eh(t("kmd.plot.aria", { unit: res.unit.id, n: rows.length, s: res.series.length }))}"${cloud ? ' data-cloud="1"' : ""}>`
      + `<line x1="${L}" y1="${H - B}" x2="${W - R}" y2="${H - B}" stroke="var(--line)"/><line x1="${L}" y1="${T}" x2="${L}" y2="${H - B}" stroke="var(--line)"/>`
      + `<text x="${(L + W - R) / 2}" y="${H - 6}" text-anchor="middle" font-size="9" fill="var(--ink)">${eh(t("kmd.axis.km", { unit: res.unit.id }))}</text>`
      + `<text x="9" y="${(T + H - B) / 2}" text-anchor="middle" font-size="9" fill="var(--ink)" transform="rotate(-90 9 ${(T + H - B) / 2})">${eh(t("kmd.axis.kmd"))}</text>`
      + `<text x="${L}" y="${H - B + 11}" font-size="8" fill="var(--muted)">${eh(fmt(x0, 1))}</text><text x="${W - R}" y="${H - B + 11}" text-anchor="end" font-size="8" fill="var(--muted)">${eh(fmt(x1, 1))}</text>`
      + `<text x="${L - 3}" y="${H - B}" text-anchor="end" font-size="8" fill="var(--muted)">${eh(fmt(y0, 4))}</text><text x="${L - 3}" y="${T + 6}" text-anchor="end" font-size="8" fill="var(--muted)">${eh(fmt(y1, 4))}</text>`;
    if (!cloud) {
      rows.forEach((r, k) => { s += pointG(r, g.px[k], g.py[k], selRow === r.i, selSer != null && r.series !== selSer); });
      return s + "</svg>";
    }
    KMDCLOUD.paths(res).forEach(c => {
      s += `<path class="kmd-cloud" data-s="${c.series || 0}" d="${c.d}" fill="${c.series ? colorOf(c.series) : "var(--muted)"}" fill-opacity="${c.series ? 1 : 0.55}" stroke="var(--ink)" stroke-width="0.5" opacity="${selSer != null && c.series !== selSer ? 0.3 : 1}"/>`;
    });
    // the pointer's ion: a transparent target with its tooltip, moved by KMDCLOUD.bind
    s += `<g id="kmd-hov" visibility="hidden"><circle r="12" fill="transparent"/><title></title></g>`;
    const k = selRow == null ? -1 : rows.findIndex(r => r.i === selRow);
    if (k >= 0) s += pointG(rows[k], g.px[k], g.py[k], true, selSer != null && rows[k].series !== selSer);
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

  return { plotSvg, pointG, tipOf, plotTable, seriesTable, featTable, stepsText, fmt };
})();
window.KMDVIEW = KMDVIEW;
