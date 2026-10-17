// @ts-strict
"use strict";
// Exploratory PCA of the feature table (high resolution only; «Statistica» panel). A VIEW of the series, not a test: no p-value, nothing «significant».
// The work is in mzlab/statistica.py (/api/stat/pca): filters first (above the blank, above an intensity threshold, present in k samples), declared
// imputation (1/5 of the minimum of the feature), log2 + Pareto scaling by default, SVD, variance explained, stability of the loadings leaving one
// sample out at a time. The page draws: scores joined in time order (trajectory) with the colours of the time gradient, replicates with one colour
// and a 95 % ellipse, loadings (paler when unstable) tied to the table of the features: a click shows the XIC and the time profile of the ion.
// Off below 5 samples.
const PCA = (() => {
  const KEY = "qqq.pca", MIN_SAMPLES = 5;
  const t = (k, p) => I18N.t(k, p);
  const eh = s => EH(s == null ? "" : s);
  let st = { min: 1000, k: 2, transform: "log", scaling: "pareto" };
  // res: {r, feats, names, files, groups}; sel: index (in r.features) of the feature chosen
  let res = null, sel = null;
  Object.assign(st, lsObj(KEY));
  const save = () => { try { localStorage.setItem(KEY, JSON.stringify(st)); } catch (_) { /* no storage */ } };
  const fmt = (v, d = 3) => (v == null || !isFinite(v) ? "–" : v.toFixed(d));
  const pct = v => (v * 100).toFixed(1);
  const nSamples = () => TABELLA.roles().filter(r => r !== "blank" && r !== "dark").length;

  async function run() {
    const tb = TABELLA.table;
    if (!tb) throw new Error(t("pca.needtable"));
    const files = TABELLA.fileObjs(), roles = files.map(f => CAMPIONI.roleOf(f)), bp = BIANCO.params;
    const groups = files.map(f => CAMPIONI.repKey(f, CAMPIONI.roleOf(f)));
    const j = await fetch("api/stat/pca", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ matrix: TABELLA.matrix(), roles, groups,
      factor: bp.factor, stat: bp.stat, min_detect: bp.min, min_intensity: st.min, min_samples: st.k, transform: st.transform, scaling: st.scaling }) }).then(r => r.json());
    if (j.error) throw I18N.err(j);
    res = { r: j, feats: j.features.map(i => tb.features[i]), files, groups, roles };
    sel = null;
    if (window.WORKFLOW && WORKFLOW.logOp) WORKFLOW.logOp("stat_pca", { transform: st.transform, scaling: st.scaling, n: j.n_samples, features: j.n_features });
    return res;
  }

  // ---------------------------------------------------------------- colours of the groups: gradient of the time, categorical when there is no time
  function groupInfo() {
    const r = res.r, order = [], by = new Map();
    r.samples.forEach((si, row) => {
      const g = res.groups[si];
      if (!by.has(g)) { by.set(g, { g, rows: [], times: [] }); order.push(g); }
      const o = by.get(g); o.rows.push(row); if (res.files[si].time != null) o.times.push(res.files[si].time);
    });
    const list = order.map(g => by.get(g));
    list.forEach(o => { o.time = o.times.length ? o.times.reduce((a, b) => a + b, 0) / o.times.length : null; });
    const times = list.map(o => o.time).filter(x => x != null);
    let c = 0;
    list.forEach(o => { o.color = o.time != null ? seqColor(CAMPIONI.timeFrac(o.time, times) ?? 0.5) : PAL[c++ % PAL.length]; });
    return list;
  }

  // ---------------------------------------------------------------- drawing
  const W = 320, H = 250, L = 46, R = 10, T = 10, B = 34;
  function frame(xs, ys, xl, yl) {
    let x0 = Math.min(...xs), x1 = Math.max(...xs), y0 = Math.min(...ys), y1 = Math.max(...ys);
    const px = (x1 - x0 || 1) * 0.1, py = (y1 - y0 || 1) * 0.1;
    x0 -= px; x1 += px; y0 -= py; y1 += py;
    const X = v => L + (v - x0) / (x1 - x0) * (W - L - R), Y = v => T + (1 - (v - y0) / (y1 - y0)) * (H - T - B);
    let s = `<line x1="${L}" y1="${H - B}" x2="${W - R}" y2="${H - B}" stroke="var(--line)"/><line x1="${L}" y1="${T}" x2="${L}" y2="${H - B}" stroke="var(--line)"/>`;
    if (x0 < 0 && x1 > 0) s += `<line x1="${X(0)}" y1="${T}" x2="${X(0)}" y2="${H - B}" stroke="var(--line)" stroke-dasharray="3 3"/>`;
    if (y0 < 0 && y1 > 0) s += `<line x1="${L}" y1="${Y(0)}" x2="${W - R}" y2="${Y(0)}" stroke="var(--line)" stroke-dasharray="3 3"/>`;
    s += `<text x="${(L + W - R) / 2}" y="${H - 6}" text-anchor="middle" font-size="9" fill="var(--ink)">${eh(xl)}</text>`;
    s += `<text x="9" y="${(T + H - B) / 2}" text-anchor="middle" font-size="9" fill="var(--ink)" transform="rotate(-90 9 ${(T + H - B) / 2})">${eh(yl)}</text>`;
    s += `<text x="${L}" y="${H - B + 11}" font-size="8" fill="var(--muted)">${eh(fmt(x0, 2))}</text><text x="${W - R}" y="${H - B + 11}" text-anchor="end" font-size="8" fill="var(--muted)">${eh(fmt(x1, 2))}</text>`;
    s += `<text x="${L - 3}" y="${H - B}" text-anchor="end" font-size="8" fill="var(--muted)">${eh(fmt(y0, 2))}</text><text x="${L - 3}" y="${T + 6}" text-anchor="end" font-size="8" fill="var(--muted)">${eh(fmt(y1, 2))}</text>`;
    return { s, X, Y };
  }
  const axisLabel = (r, i) => t("pca.axis", { pc: "PC" + (i + 1), v: pct(r.explained[i]) });
  function scoresSvg() {
    const r = res.r, gs = groupInfo();
    const xs = r.scores.map(q => q[0]), ys = r.scores.map(q => q[1]);
    const f = frame(xs, ys, axisLabel(r, 0), axisLabel(r, 1));
    let s = `<svg class="pca-plot" id="pca-scores" viewBox="0 0 ${W} ${H}" role="img" aria-label="${eh(t("pca.scores.aria", { x: pct(r.explained[0]) + " %", y: pct(r.explained[1]) + " %" }))}">${f.s}`;
    // 95 % ellipse of a group of replicates, drawn through the data-to-pixel map
    Object.entries(r.ellipses).forEach(([g, e]) => {
      const o = gs.find(q => q.g === g); if (!o) return;
      const pts = Array.from({ length: 61 }, (_, i) => { const th = i / 60 * 2 * Math.PI, u = e.a * Math.cos(th), v = e.b * Math.sin(th);
        return [f.X(e.cx + u * Math.cos(e.angle) - v * Math.sin(e.angle)), f.Y(e.cy + u * Math.sin(e.angle) + v * Math.cos(e.angle))]; });
      s += `<polygon points="${pts.map(p => p[0].toFixed(1) + "," + p[1].toFixed(1)).join(" ")}" fill="${o.color}" fill-opacity=".12" stroke="${o.color}" stroke-width="1"><title>${eh(t("pca.ell", { g: g.replace(/\|/g, " ") }))}</title></polygon>`;
    });
    const cen = gs.filter(o => o.time != null).sort((a, b) => a.time - b.time).map(o => [f.X(o.rows.reduce((a, i) => a + xs[i], 0) / o.rows.length), f.Y(o.rows.reduce((a, i) => a + ys[i], 0) / o.rows.length), o]);
    if (cen.length > 1) s += `<polyline fill="none" stroke="var(--muted)" stroke-width="1.3" points="${cen.map(c => c[0].toFixed(1) + "," + c[1].toFixed(1)).join(" ")}"/>`;
    if (cen.length) { const a = cen[0], b = cen[cen.length - 1];
      s += `<text x="${a[0] + 6}" y="${a[1] - 6}" font-size="8" fill="var(--ink)">${eh(fmtTime(a[2].time))}</text>`;
      if (cen.length > 1) s += `<text x="${b[0] + 6}" y="${b[1] - 6}" font-size="8" fill="var(--ink)">${eh(fmtTime(b[2].time))}</text>`; }
    gs.forEach(o => o.rows.forEach(row => {
      const fl = res.files[r.samples[row]], x = f.X(xs[row]), y = f.Y(ys[row]), tip = `<title>${eh(t("pca.sample.tip", { f: fl.label || fl.file, t: fl.time == null ? "–" : fmtTime(fl.time) }))}</title>`;
      s += res.roles[r.samples[row]] === "qc" ? `<path d="M${x} ${y - 6}L${x + 6} ${y}L${x} ${y + 6}L${x - 6} ${y}Z" fill="${o.color}" stroke="var(--ink)" stroke-width=".7">${tip}</path>`
        : `<circle cx="${x}" cy="${y}" r="4.6" fill="${o.color}" stroke="var(--ink)" stroke-width=".7">${tip}</circle>`;
    }));
    return s + "</svg>";
  }
  // the values of the scores plot, for the «Dati» table (tdati.js)
  function scoresTable() {
    if (!res) return null;
    const r = res.r, nc = r.scores[0] ? r.scores[0].length : 2;
    return { head: [t("qaqc.col.file"), ...Array.from({ length: Math.min(nc, 5) }, (_, i) => "PC" + (i + 1)), t("tdati.col.group")], rowHeader: true,
      rows: r.scores.map((q, i) => { const fl = res.files[r.samples[i]] || {}; return [fl.label || fl.file || "", ...q.slice(0, 5).map(v => fmt(v, 3)), (fl.group || fl.rep || "")]; }) };
  }
  function loadingsSvg() {
    const r = res.r, xs = r.loadings.map(q => q[0]), ys = r.loadings.map(q => q[1]);
    const f = frame(xs, ys, axisLabel(r, 0), axisLabel(r, 1));
    let s = `<svg class="pca-plot" id="pca-loadings" viewBox="0 0 ${W} ${H}" role="img" aria-label="${eh(t("pca.loadings.aria", { x: pct(r.explained[0]) + " %", y: pct(r.explained[1]) + " %" }))}">${f.s}`;
    r.features.forEach((_, i) => {
      const ft = res.feats[i], on = sel === i;
      s += `<circle data-i="${i}" cx="${f.X(xs[i]).toFixed(1)}" cy="${f.Y(ys[i]).toFixed(1)}" r="${on ? 5.5 : 3.2}" fill="var(--accent)" fill-opacity="${on ? 1 : (0.2 + 0.8 * r.stability[i]).toFixed(2)}" stroke="${on ? "var(--ink)" : "none"}" stroke-width="1.2"><title>m/z ${eh(fmt(ft.mz, 4))} · RT ${eh(fmt(ft.rt, 2))}</title></circle>`;
    });
    return s + "</svg>";
  }
  function tableHtml() {
    const r = res.r, order = r.features.map((_, i) => i).sort((a, b) => Math.hypot(r.loadings[b][0], r.loadings[b][1]) - Math.hypot(r.loadings[a][0], r.loadings[a][1])).slice(0, 60);
    return `<div class="qq-scroll"><table class="qq-tab" id="pca-tab"><thead><tr><th scope="col">ID</th><th scope="col">m/z</th><th scope="col">RT</th><th scope="col">${t("pca.col.pc", { n: 1 })}</th><th scope="col">${t("pca.col.pc", { n: 2 })}</th><th scope="col">${t("pca.col.stab")}</th></tr></thead><tbody>${order.map(i => {
      const ft = res.feats[i];
      return `<tr data-i="${i}" tabindex="0" role="button" aria-label="${eh(t("pca.row.aria", { mz: fmt(ft.mz, 4), rt: fmt(ft.rt, 2) }))}"${sel === i ? ' aria-current="true" class="sel"' : ""}><td class="num">${eh(ft.id)}</td><td class="num">${fmt(ft.mz, 4)}</td><td class="num">${fmt(ft.rt, 2)}</td><td class="num">${fmt(r.loadings[i][0])}</td><td class="num">${fmt(r.loadings[i][1])}</td><td class="num">${fmt(r.stability[i], 2)}</td></tr>`; }).join("")}</tbody></table></div>`;
  }
  function stepsHtml() {
    const r = res.r, n = Object.fromEntries(r.steps.map(s => [s.step, s])), bp = BIANCO.params;
    const p = { t: n.table.n, i: n.intensity.n, p: n.presence.n, k: n.presence.min_samples, u: r.n_features };
    const flt = n.blank.applied ? t("pca.steps", { ...p, b: n.blank.n }) : t("pca.steps.noblank", p);
    return `<p class="sm">${eh(flt)}</p><p class="sm">${eh(n.blank.applied ? t("pca.blank.on", { f: bp.factor, stat: bp.stat === "mean" ? t("blank.stat.mean") : t("blank.stat.max"), n: bp.min }) : t("pca.blank.off"))}</p>
<p class="sm">${eh(t("pca.imputed", { n: r.imputed }))} ${eh(t("pca.settings", { tr: r.transform === "log" ? t("pca.tr.log") : t("pca.tr.none"), sc: r.scaling === "pareto" ? t("pca.sc.pareto") : r.scaling === "auto" ? t("pca.sc.auto") : t("pca.sc.none"), ns: r.n_samples, nf: r.n_features }))}</p>
<p class="sm">${eh(t("pca.expl", { list: r.explained.map((v, i) => `PC${i + 1} ${pct(v)} %`).join(", ") }))}</p>`;
  }
  function paintRes(box) {
    const el = box.querySelector("#pca-res");
    if (!el) return;
    if (!res) { el.innerHTML = ""; return; }
    el.innerHTML = `${stepsHtml()}<h4 class="sm" style="margin:8px 0 2px">${t("pca.scores")} <button type="button" id="pca-help" class="pca-q" aria-expanded="false" aria-controls="pca-help-text" aria-label="${eh(t("pca.help"))}">?</button></h4>
<p class="sm" id="pca-help-text" hidden>${t("pca.help.text")}</p>${scoresSvg()}${CAMPIONI.gradientLegend()}<p class="sm">${t("pca.traj")}</p>
<h4 class="sm" style="margin:8px 0 2px">${t("pca.loadings")}</h4>${loadingsSvg()}<p class="sm">${t("pca.stab.note")}</p>${tableHtml()}
<div class="qq-form"><button type="button" id="pca-xlsx">Excel</button></div>`;
  }
  function paint(box) {
    const n = nSamples(), tb = TABELLA.table;
    if (n < MIN_SAMPLES) { box.innerHTML = `<p class="sm" role="status">${t("pca.off", { min: MIN_SAMPLES, n })}</p>`; res = null; return; }
    box.innerHTML = `<p class="sm">${t("pca.intro")}</p>
<form id="pca-form" class="qq-form"><label>${t("pca.min")} <input id="pca-min" value="${st.min}" inputmode="decimal" size="7"></label><label>${t("pca.k")} <input id="pca-k" value="${st.k}" inputmode="numeric" size="3"></label>
<label>${t("pca.transform")} <select id="pca-tr"><option value="log"${st.transform === "log" ? " selected" : ""}>${t("pca.tr.log")}</option><option value="none"${st.transform === "none" ? " selected" : ""}>${t("pca.tr.none")}</option></select></label>
<label>${t("pca.scaling")} <select id="pca-sc"><option value="pareto"${st.scaling === "pareto" ? " selected" : ""}>${t("pca.sc.pareto")}</option><option value="auto"${st.scaling === "auto" ? " selected" : ""}>${t("pca.sc.auto")}</option><option value="none"${st.scaling === "none" ? " selected" : ""}>${t("pca.sc.none")}</option></select></label>
<button type="submit" id="pca-go">${t("pca.go")}</button></form>
<p class="sm" id="pca-scnote"${st.scaling === "auto" ? "" : " hidden"}>${t("pca.sc.note")}</p>${tb ? "" : `<p class="sm">${t("pca.needtable")}</p>`}
<div id="pca-msg" class="sm bad" role="alert"></div><div id="pca-res"></div>`;
    paintRes(box);
  }
  function openIonXic(mz) {
    if (!window.HR || typeof addPanel !== "function") return;
    const ht = HR.ionTrace(mz, { obs: false });
    if (!ht) return;
    const ex = E.panels.find(p => p.pcaIon);
    if (ex) { ex.traces = [ht]; if (typeof ctl === "function") ctl(ex); draw(ex); reveal(ex); }
    else { const np = addPanel("xic", { traces: [ht] }); np.pcaIon = true; reveal(np); }
  }
  let curBox = null;
  // another table: the old result no longer belongs to it
  document.addEventListener("tabella", () => { res = null; sel = null; const r = curBox && curBox.querySelector("#pca-res"); if (r) r.innerHTML = ""; });
  function select(i, box) {
    sel = i;
    const ft = res.feats[i];
    if (box) { box.querySelector("#pca-loadings").outerHTML = loadingsSvg(); box.querySelector("#pca-tab").outerHTML = tableHtml(); }
    openIonXic(ft.mz);
    if (window.PROFILO) PROFILO.show(ft.mz, ft.rt);
  }
  function mount(box) {
    curBox = box;
    paint(box);
    box.onchange = e => { if (e.target.id === "pca-sc") box.querySelector("#pca-scnote").hidden = e.target.value !== "auto"; };
    box.onsubmit = async e => {
      e.preventDefault();
      const mn = numIt(box.querySelector("#pca-min").value), k = parseInt(box.querySelector("#pca-k").value, 10), msg = box.querySelector("#pca-msg");
      st.min = mn >= 0 ? mn : 0; st.k = k >= 1 ? k : 1; st.transform = box.querySelector("#pca-tr").value; st.scaling = box.querySelector("#pca-sc").value; save(); msg.textContent = "";
      try { await run(); paintRes(box); } catch (err) { msg.textContent = String(err && err.message || err); res = null; paintRes(box); }
    };
    box.onclick = e => {
      if (e.target.id === "pca-help") { const h = box.querySelector("#pca-help-text"); h.hidden = !h.hidden; e.target.setAttribute("aria-expanded", String(!h.hidden)); return; }
      if (e.target.id === "pca-xlsx" && res) { xlsx(); return; }
      const el = e.target.closest("#pca-tab tbody tr, #pca-loadings circle[data-i]");
      if (el && res) select(+el.dataset.i, box);
    };
    box.onkeydown = e => { const tr = e.target.closest && e.target.closest("#pca-tab tbody tr"); if (tr && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); tr.click(); } };
  }
  function xlsx() {
    const r = res.r, nc = r.explained.length;
    const sc = r.samples.map((si, row) => { const f = res.files[si]; return [f.label || f.file, CAMPIONI.roleName(res.roles[si]), f.time == null ? "" : timeShown(f.time, tunitGet()), res.groups[si].replace(/\|/g, " ")].concat(r.scores[row]); });
    const ld = r.features.map((_, i) => [res.feats[i].id, res.feats[i].mz, res.feats[i].rt].concat(r.loadings[i], [r.stability[i]]));
    const pcs = Array.from({ length: nc }, (_, i) => "PC" + (i + 1));
    dlx("pca.xlsx", [{ name: "Scores", head: [t("qaqc.col.file"), t("qaqc.col.type"), t("meta.time"), t("meta.rep")].concat(pcs), rows: sc, widths: [28, 12, 10, 18] },
      { name: "Loadings", head: ["ID", "m/z", "RT"].concat(pcs, [t("pca.col.stab")]), rows: ld, widths: [10, 12, 8] },
      { name: "Variance", head: ["PC", "%"], rows: r.explained.map((v, i) => ["PC" + (i + 1), +(v * 100).toFixed(3)]), widths: [8, 10] }], { table: "pca" });
  }
  if (typeof STAT !== "undefined") STAT.add({ id: "pca", title: "pca.title", hr: true, visible: () => true, mount });
  return { mount, run, select, scoresTable, get result() { return res; }, get params() { return st; }, groupInfo, MIN_SAMPLES };
})();
window.PCA = PCA;
