"use strict";
// Kendrick mass defect (KMD) of the feature table (high resolution only; «Statistica» panel), for the PFAS and any other series of ions that differ by a whole
// number of repeating units (CF2, CF2O, C2F4, CH2). The arithmetic is in kmd-calc.js (KMDCALC). A VIEW and a list of CANDIDATE series with their evidence
// (members, steps, retention trend): the program never says what an ion is. Click on a point or a row: XIC of the ion and its profile in time.
const KMD = (() => {
  const KEY = "qqq.kmd", MAX_ROWS = 200;
  const t = (k, p) => I18N.t(k, p);
  const eh = s => EH(s == null ? "" : s);
  const st = Object.assign({ unit: "CF2", tol: 3, min: 3 }, lsObj(KEY));
  const save = () => { try { localStorage.setItem(KEY, JSON.stringify(st)); } catch (_) { /* no storage */ } };
  const fmt = (v, d = 3) => (v == null || !isFinite(v) ? "–" : v.toFixed(d));
  let res = null, selSer = null, selRow = null, curBox = null;

  function compute() {
    const tb = TABELLA.table;
    if (!tb) { res = null; return; }
    res = KMDCALC.analyse(tb.features, { unit: st.unit, tolMDa: st.tol, minMembers: st.min });
    selSer = null; selRow = null;
    if (window.WORKFLOW && WORKFLOW.logOp) WORKFLOW.logOp("stat_kmd", { unit: st.unit, tol: st.tol, min: st.min, n: res.rows.length, series: res.series.length });
  }

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
  function plotSvg() {
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
        + `<circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="12" fill="transparent"/>`          // 24 px target around a small mark
        + mark(shape, x, y, on ? 5.6 : 3.8, `fill="${col}" fill-opacity="${r.series ? 1 : 0.55}" stroke="${on ? "var(--ink)" : "var(--ink)"}" stroke-width="${on ? 1.6 : 0.5}"`) + tip + `</g>`;
    });
    return s + "</svg>";
  }
  // the values of the plot, for the «Dati» table (tdati.js)
  function plotTable() {
    if (!res) return null;
    return { head: ["ID", "m/z", "RT", t("kmd.col.km"), "KMD", t("kmd.col.series")], rowHeader: true,
      rows: res.rows.map(r => [r.id, fmt(r.mz, 4), fmt(r.rt, 2), fmt(r.km, 4), fmt(r.kmd, 4), r.series || ""]) };
  }

  // ---------------------------------------------------------------- tables
  const stepsText = s => { const c = {}; s.steps.forEach(x => { c[x] = (c[x] || 0) + 1; }); return Object.keys(c).map(k => `${k}×${c[k]}`).join(", ") || "–"; };
  function seriesTable() {
    if (!res.series.length) return `<p class="sm" role="status">${eh(t("kmd.noseries", { min: res.minMembers, tol: res.tolMDa }))}</p>`;
    return `<div class="qq-scroll"><table class="qq-tab" id="kmd-series"><thead><tr><th scope="col">${t("kmd.col.series")}</th><th scope="col">${t("kmd.col.members")}</th><th scope="col">KMD</th><th scope="col">${t("kmd.col.span")}</th>`
      + `<th scope="col">${t("kmd.col.steps")}</th><th scope="col">${t("kmd.col.rt")}</th></tr></thead><tbody>`
      + res.series.map(s => `<tr data-s="${s.n}" tabindex="0" role="button" aria-label="${eh(t("kmd.series.aria", { n: s.n, m: s.members.length }))}"${selSer === s.n ? ' aria-current="true" class="sel"' : ""}>`
        + `<td class="num">${s.n}</td><td class="num">${s.members.length}</td><td class="num">${fmt(s.kmd, 4)}</td><td class="num">${fmt(s.spanMDa, 2)}</td><td>${eh(stepsText(s))}</td><td class="num">${fmt(s.rho, 2)}</td></tr>`).join("")
      + `</tbody></table></div>`;
  }
  function visibleRows() {
    const list = selSer != null ? res.series[selSer - 1].members.map(i => res.rows.find(r => r.i === i)) : res.rows.slice().sort((a, b) => (a.series ? 0 : 1) - (b.series ? 0 : 1) || a.kmd - b.kmd || a.km - b.km);
    return list.filter(Boolean);
  }
  function featTable() {
    const list = visibleRows(), shown = list.slice(0, MAX_ROWS);
    return `<div class="qq-scroll"><table class="qq-tab" id="kmd-tab"><thead><tr><th scope="col">ID</th><th scope="col">m/z</th><th scope="col">RT</th><th scope="col">${t("kmd.col.km")}</th><th scope="col">KMD</th><th scope="col">${t("kmd.col.series")}</th></tr></thead><tbody>`
      + shown.map(r => `<tr data-i="${r.i}" tabindex="0" role="button" aria-label="${eh(t("kmd.row.aria", { mz: fmt(r.mz, 4), rt: fmt(r.rt, 2) }))}"${selRow === r.i ? ' aria-current="true" class="sel"' : ""}>`
        + `<td class="num">${eh(r.id)}</td><td class="num">${fmt(r.mz, 4)}</td><td class="num">${fmt(r.rt, 2)}</td><td class="num">${fmt(r.km, 4)}</td><td class="num">${fmt(r.kmd, 4)}</td><td class="num">${r.series || "–"}</td></tr>`).join("")
      + `</tbody></table></div>` + (list.length > shown.length ? `<p class="sm">${eh(t("kmd.more", { n: shown.length, m: list.length }))}</p>` : "");
  }

  function paintRes(box) {
    const el = box.querySelector("#kmd-res");
    if (!el) return;
    if (!res) { el.innerHTML = ""; return; }
    const inSer = res.rows.filter(r => r.series).length;
    el.innerHTML = `<p class="sm" role="status">${eh(t("kmd.sum", { n: res.rows.length, s: res.series.length, k: inSer, unit: res.unit.id }))}</p>${plotSvg()}<p class="sm">${t("kmd.plot.note")}</p>`
      + `<h4 class="sm" style="margin:8px 0 2px">${t("kmd.series.title")}</h4>${seriesTable()}<p class="sm">${t("kmd.evidence")}</p>`
      + `<h4 class="sm" style="margin:8px 0 2px">${t("kmd.feat.title")}${selSer != null ? ` <button type="button" id="kmd-all" class="sec">${t("kmd.all")}</button>` : ""}</h4>${featTable()}`
      + `<div class="qq-form"><button type="button" id="kmd-xlsx">Excel</button></div>`;
  }
  function paint(box) {
    const tb = TABELLA.table;
    box.innerHTML = `<p class="sm">${t("kmd.intro")}</p>`
      + `<form id="kmd-form" class="qq-form"><label>${t("kmd.unit")} <select id="kmd-unit">${KMDCALC.UNITS.map(u => `<option value="${u.id}"${u.id === st.unit ? " selected" : ""}>${u.id} (${u.mass.toFixed(4)})</option>`).join("")}</select></label>`
      + `<label>${t("kmd.tol")} <input id="kmd-tol" value="${st.tol}" inputmode="decimal" size="4"></label><label>${t("kmd.min")} <input id="kmd-min" value="${st.min}" inputmode="numeric" size="3"></label>`
      + `<button type="submit" id="kmd-go">${t("kmd.go")}</button></form>${tb ? "" : `<p class="sm">${t("kmd.needtable")}</p>`}<div id="kmd-msg" class="sm bad" role="alert"></div><div id="kmd-res"></div>`;
    if (tb && !res) compute();
    paintRes(box);
  }

  function openIonXic(mz) {
    if (!window.HR || typeof addPanel !== "function") return;
    const ht = HR.ionTrace(mz, { obs: false });
    if (!ht) return;
    const ex = E.panels.find(p => p.kmdIon);
    if (ex) { ex.traces = [ht]; if (typeof ctl === "function") ctl(ex); draw(ex); reveal(ex); }
    else { const np = addPanel("xic", { traces: [ht] }); np.kmdIon = true; reveal(np); }
  }
  function selectRow(i, box) {
    selRow = i;
    const r = res.rows.find(x => x.i === i); if (!r) return;
    paintRes(box);
    openIonXic(r.mz);
    if (window.PROFILO && r.rt != null) PROFILO.show(r.mz, r.rt);
  }

  document.addEventListener("tabella", () => { res = null; selSer = null; selRow = null; if (curBox && curBox.isConnected) paint(curBox); });
  function mount(box) {
    curBox = box;
    paint(box);
    box.onsubmit = e => {
      e.preventDefault();
      const tol = numIt(box.querySelector("#kmd-tol").value), mn = parseInt(box.querySelector("#kmd-min").value, 10);
      st.unit = box.querySelector("#kmd-unit").value; st.tol = tol > 0 ? tol : 3; st.min = mn >= 2 ? mn : 3; save();
      try { compute(); box.querySelector("#kmd-msg").textContent = ""; } catch (err) { box.querySelector("#kmd-msg").textContent = String(err && err.message || err); res = null; }
      paint(box);
    };
    box.onclick = e => {
      if (e.target.id === "kmd-xlsx" && res) { xlsx(); return; }
      if (e.target.id === "kmd-all") { selSer = null; paintRes(box); return; }
      const sr = e.target.closest("#kmd-series tbody tr");
      if (sr && res) { selSer = selSer === +sr.dataset.s ? null : +sr.dataset.s; paintRes(box); return; }
      const el = e.target.closest("#kmd-tab tbody tr, #kmd-plot g[data-i]");
      if (el && res) selectRow(+el.dataset.i, box);
    };
    box.onkeydown = e => { const el = e.target.closest && e.target.closest("#kmd-tab tbody tr, #kmd-series tbody tr, #kmd-plot g[data-i]"); if (el && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); el.dispatchEvent(new MouseEvent("click", { bubbles: true })); } };
  }
  function xlsx() {
    const u = res.unit;
    dlx("kmd.xlsx", [{ name: "Features", head: ["ID", "m/z", "RT", "KM", "NKM", "KMD", t("kmd.col.series")], rows: res.rows.map(r => [r.id, r.mz, r.rt, r.km, r.nkm, r.kmd, r.series || ""]), widths: [10, 12, 8, 12, 8, 10, 8] },
      { name: "Series", head: [t("kmd.col.series"), t("kmd.col.members"), "KMD", t("kmd.col.span"), t("kmd.col.steps"), t("kmd.col.rt")], rows: res.series.map(s => [s.n, s.members.length, s.kmd, s.spanMDa, stepsText(s), s.rho == null ? "" : s.rho]), widths: [8, 10, 10, 12, 16, 12] },
      { name: "Parameters", head: [t("kmd.param"), t("kmd.value")], rows: [[t("kmd.unit"), u.id], [t("kmd.unit.mass"), u.mass], [t("kmd.tol"), res.tolMDa], [t("kmd.min"), res.minMembers]], widths: [30, 14] }]);
  }
  if (typeof STAT !== "undefined") STAT.add({ id: "kmd", title: "kmd.title", hr: true, visible: () => true, mount });
  return { mount, compute, plotTable, selectRow, get result() { return res; }, get params() { return st; } };
})();
window.KMD = KMD;
