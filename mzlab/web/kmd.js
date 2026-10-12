"use strict";
// Kendrick mass defect (KMD) of the feature table (high resolution only; «Statistica» panel), for the PFAS and any other series of ions that differ by a whole
// number of repeating units (CF2, CF2O, C2F4, CH2). The arithmetic is in kmd-calc.js (KMDCALC). A VIEW and a list of CANDIDATE series with their evidence
// (members, steps, retention trend): the program never says what an ion is. Click on a point or a row: XIC of the ion and its profile in time.
const KMD = (() => {
  const KEY = "qqq.kmd";
  const t = (k, p) => I18N.t(k, p);
  const eh = s => EH(s == null ? "" : s);
  const st = Object.assign({ unit: "CF2", tol: 3, min: 3 }, lsObj(KEY));
  const save = () => { try { localStorage.setItem(KEY, JSON.stringify(st)); } catch (_) { /* no storage */ } };
  let res = null, selSer = null, selRow = null, curBox = null;
  function compute() {
    const tb = TABELLA.table;
    res = tb ? KMDCALC.analyse(tb.features, { unit: st.unit, tolMDa: st.tol, minMembers: st.min }) : null;
    selSer = null; selRow = null;
    if (res && window.WORKFLOW && WORKFLOW.logOp) WORKFLOW.logOp("stat_kmd", { unit: st.unit, tol: st.tol, min: st.min, n: res.rows.length, series: res.series.length });
  }
  function paintRes(box) {
    const el = box.querySelector("#kmd-res");
    if (!el) return;
    if (!res) { el.innerHTML = ""; return; }
    const inSer = res.rows.filter(r => r.series).length;
    el.innerHTML = `<p class="sm" role="status">${eh(t("kmd.sum", { n: res.rows.length, s: res.series.length, k: inSer, unit: res.unit.id }))}</p>${KMDVIEW.plotSvg(res, selSer, selRow)}<p class="sm">${t("kmd.plot.note")}</p>`
      + `<h4 class="sm" style="margin:8px 0 2px">${t("kmd.series.title")}</h4>${KMDVIEW.seriesTable(res, selSer)}<p class="sm">${t("kmd.evidence")}</p>`
      + `<h4 class="sm" style="margin:8px 0 2px">${t("kmd.feat.title")}${selSer != null ? ` <button type="button" id="kmd-all" class="sec">${t("kmd.all")}</button>` : ""}</h4>${KMDVIEW.featTable(res, selSer, selRow)}`
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
      { name: "Series", head: [t("kmd.col.series"), t("kmd.col.members"), "KMD", t("kmd.col.span"), t("kmd.col.steps"), t("kmd.col.rt")], rows: res.series.map(s => [s.n, s.members.length, s.kmd, s.spanMDa, KMDVIEW.stepsText(s), s.rho == null ? "" : s.rho]), widths: [8, 10, 10, 12, 16, 12] },
      { name: "Parameters", head: [t("kmd.param"), t("kmd.value")], rows: [[t("kmd.unit"), u.id], [t("kmd.unit.mass"), u.mass], [t("kmd.tol"), res.tolMDa], [t("kmd.min"), res.minMembers]], widths: [30, 14] }]);
  }
  if (typeof STAT !== "undefined") STAT.add({ id: "kmd", title: "kmd.title", hr: true, visible: () => true, mount });
  return { mount, compute, plotTable: () => KMDVIEW.plotTable(res), selectRow, get result() { return res; }, get params() { return st; } };
})();
window.KMD = KMD;
