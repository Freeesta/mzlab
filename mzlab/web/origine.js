"use strict";
// "Da dove viene questo ione?" (Origin of an ion). Classic script, loaded after explore.js.
// ONE window (like openXic) that shows the EVIDENCE computed by mzlab/ionfamily.py (/api/origin) and never a verdict:
//   (a) normalised XICs of the ion, of the candidate precursor and of the co-eluting ions the student picks,
//   (b) table of the co-eluting ions sorted by correlation, with the difference in apex time and in mass (neutral losses are hints),
//   (c) F against P (area of the ion against area of the candidate precursor, one point per scan) with the fitted line and R2,
//   (d) behaviour across samples (kinetics), each metric with a two-line explanation, honest warnings, the student's own hypothesis
//       (saved in the notebook) and an Excel export. The only "conclusion" on the page is the one the student writes.
const OG = { res: null, sel: new Set(), traces: new Map(), busy: 0 };
const ogMsg = w => (w && w.key ? I18N.t(w.key, w.params) : String(w));                // a warning from the server: {key, params}
const ogRole = r => (r.label_key ? I18N.t(r.label_key, r.params) : r.label || r.role);   // a role of a co-eluting ion
(function () {
  const st = document.createElement("style");
  st.textContent = `#ogdlg{width:min(1100px,96vw);max-width:96vw;max-height:92vh;border:1px solid var(--line);border-radius:8px;padding:0;background:var(--panel);color:var(--text)}
#ogdlg::backdrop{background:rgba(0,0,0,.35)}#ogdlg .ogh{display:flex;align-items:center;gap:8px;padding:10px 14px;border-bottom:1px solid var(--line);position:sticky;top:0;background:var(--panel);z-index:2}
#ogdlg .ogh h3{margin:0;font-size:16px;flex:1}#ogdlg .ogb{padding:10px 14px 14px;overflow:auto;max-height:calc(92vh - 52px)}
#ogdlg .ogf{display:flex;flex-wrap:wrap;gap:8px 14px;align-items:flex-end}#ogdlg label{font-size:12px;color:var(--muted);display:flex;flex-direction:column;gap:2px}
#ogdlg input,#ogdlg select,#ogdlg textarea{font:inherit;padding:3px 6px;border:1px solid var(--line);border-radius:4px;background:var(--panel);color:var(--text)}
#ogdlg input.mzf{width:90px}#ogdlg section{margin-top:14px;border-top:1px solid var(--line);padding-top:10px}#ogdlg section h4{margin:0 0 2px;font-size:14px}
#ogdlg .why{font-size:12px;color:var(--muted);margin:0 0 6px;max-width:80ch}#ogdlg canvas{width:100%;height:230px;display:block}
#ogdlg table{border-collapse:collapse;font-size:12px;width:100%}#ogdlg th,#ogdlg td{padding:3px 6px;border-bottom:1px solid var(--line);text-align:right}
#ogdlg th:first-child,#ogdlg td:first-child{text-align:left}#ogdlg tr.sel td{background:var(--sel)}#ogdlg tbody tr{cursor:pointer}
#ogdlg .oggrid{display:grid;grid-template-columns:1fr 1fr;gap:14px}@media(max-width:800px){#ogdlg .oggrid{grid-template-columns:1fr}}
#ogdlg .ogw{background:rgba(230,159,0,.14);border-left:3px solid #E69F00;padding:5px 9px;margin:4px 0;font-size:12px}
#ogdlg .chips{display:flex;flex-wrap:wrap;gap:8px;font-size:12px;margin:4px 0}#ogdlg .chips i{display:inline-block;width:10px;height:10px;border-radius:2px;margin-right:4px}
#ogdlg .kv{display:flex;flex-wrap:wrap;gap:4px 18px;font-size:13px;margin:4px 0}#ogdlg .kv b{font-variant-numeric:tabular-nums}
#ogdlg textarea{width:100%;min-height:70px;box-sizing:border-box}`;
  document.head.appendChild(st);
})();
const ogN = (v, d = 3) => v == null || !isFinite(v) ? "-" : (Math.abs(v) >= 1e4 || (v !== 0 && Math.abs(v) < 1e-3) ? v.toExponential(2) : (+v.toFixed(d)).toString());
const OG_COLORS = ["#0072B2", "#D55E00", "#009E73", "#CC79A7", "#E69F00", "#56B4E9", "#7a3b9b", "#8a8a8a"];

function ogDialog() {
  let d = document.getElementById("ogdlg"); if (d) return d;
  d = document.createElement("dialog"); d.id = "ogdlg";
  d.innerHTML = `<div class="ogh"><h3>${I18N.t("og.title")}</h3><button id="og-x" title="${I18N.t("common.close")}">&times;</button></div>
<div class="ogb"><p class="why">${I18N.t("og.intro")}</p>
<div class="ogf">
<label><span>${I18N.t("og.lbl.ion")}</span><input id="og-mz" class="mzf" inputmode="decimal" autocomplete="off"></label>
<label><span>${I18N.t("og.lbl.par")}</span><input id="og-par" class="mzf" inputmode="decimal" autocomplete="off" placeholder="${I18N.t("og.ph.par")}"></label>
<label>${I18N.t("og.lbl.f")}<input id="og-f" autocomplete="off" placeholder="${I18N.t("og.ph.f")}"></label>
<label>${I18N.t("og.lbl.r0")}<input id="og-r0" class="mzf" inputmode="decimal" autocomplete="off" placeholder="${I18N.t("og.ph.auto")}"></label>
<label>${I18N.t("og.lbl.r1")}<input id="og-r1" class="mzf" inputmode="decimal" autocomplete="off"></label>
<label>${I18N.t("og.lbl.k")}<select id="og-k"></select></label>
<button id="og-go" class="imp">${I18N.t("og.go")}</button></div>
<div id="og-err" class="ogw" hidden></div><div id="og-out"></div></div>`;
  document.body.appendChild(d);
  d.querySelector("#og-x").onclick = () => d.close();
  d.querySelector("#og-go").onclick = () => ogRun();
  d.querySelectorAll("input").forEach(i => i.onkeydown = e => { if (e.key === "Enter") { e.preventDefault(); ogRun(); } });
  return d;
}
function openOrigin(pre = {}) {
  const d = ogDialog(), full = E.files.filter(f => f.kind === "full" && !f.gone && f.type !== "blank");
  if (!full.length) return info(I18N.t("og.needFull"));
  const q = id => d.querySelector(id);
  q("#og-k").innerHTML = full.map(f => `<option value="${f.k}">${EH(f.label)}</option>`).join("");
  const kk = full.find(f => f.k === pre.k) || full[0]; q("#og-k").value = kk.k;
  q("#og-mz").value = pre.mz != null ? fmz(pre.mz) : "";
  if (pre.parent != null) q("#og-par").value = fmz(pre.parent);
  q("#og-r0").value = pre.rt0 != null ? (+pre.rt0).toFixed(2) : ""; q("#og-r1").value = pre.rt1 != null ? (+pre.rt1).toFixed(2) : "";
  q("#og-err").hidden = true; q("#og-out").innerHTML = ""; OG.res = null; OG.sel = new Set(); OG.traces = new Map();
  d.showModal();
  (q("#og-par").value ? q("#og-go") : q("#og-par")).focus();
}
async function ogRun() {
  const d = document.getElementById("ogdlg"), q = id => d.querySelector(id), err = q("#og-err");
  const mz = numMz(q("#og-mz").value), par = numMz(q("#og-par").value), r0 = numIt(q("#og-r0").value), r1 = numIt(q("#og-r1").value);
  err.hidden = true;
  if (mz == null || par == null) { err.textContent = I18N.t("og.needBoth"); err.hidden = false; return; }
  if (mz === par) { err.textContent = I18N.t("og.differ"); err.hidden = false; return; }
  q("#og-go").disabled = true; const t0 = Date.now(), webNote = window.QQQ_BROWSER ? " " + I18N.t("og.webNote") : "";
  q("#og-out").innerHTML = `<p class="why" id="og-wait">${I18N.t("og.wait")}${webNote}</p>`;
  const tick = setInterval(() => { const w = document.getElementById("og-wait"); if (w) w.firstChild.textContent = I18N.t("og.waitN", { s: Math.round((Date.now() - t0) / 1000) }); }, 1000);
  const my = ++OG.busy;
  try {
    let u = `api/origin?k=${q("#og-k").value}&mz=${mz}&parent=${par}`;
    if (isFinite(r0) && isFinite(r1) && r1 > r0) u += `&rt0=${r0}&rt1=${r1}`;
    if (q("#og-f").value.trim()) u += `&formula=${encodeURIComponent(q("#og-f").value.trim())}`;
    const res = await J(u); if (my !== OG.busy) return;
    OG.res = res; OG.k = +q("#og-k").value; OG.sel = new Set(); OG.traces = new Map();
    ogRender();
  } catch (e) { q("#og-out").innerHTML = ""; err.textContent = I18N.t("og.failed", { message: e.message }); err.hidden = false; }
  finally { clearInterval(tick); q("#og-go").disabled = false; }
}

// ---- small canvas plots (same helpers as the rest of the program: setup, frame, nice, M)
function ogPlot(cv, o) {
  const { g, W, H } = setup(cv); g.clearRect(0, 0, W, H);
  if (window.TDATI) TDATI.series(cv, o);                    // description and table of the data (tdati.js)
  const xs = o.series.flatMap(s => s.x), ys = o.series.flatMap(s => s.y).filter(isFinite);
  if (!xs.length) { g.fillStyle = css("--muted"); g.fillText(I18N.t("og.noData"), M.l, 30); return; }
  let x0 = o.x0 ?? Math.min(...xs), x1 = o.x1 ?? Math.max(...xs), y0 = o.y0 ?? Math.min(0, ...ys), y1 = o.y1 ?? Math.max(...ys) * 1.05;
  if (x1 === x0) { x0 -= 1; x1 += 1; } if (y1 === y0) y1 = y0 + 1;
  const X = v => M.l + (v - x0) / (x1 - x0) * (W - M.l - M.r), Y = v => H - M.b - (v - y0) / (y1 - y0) * (H - M.t - M.b);
  frame(g, W, H, nice(x0, x1, 7).map(t => [X(t), +t.toFixed(2)]), nice(y0, y1, 4).map(t => [Y(t), ogN(t, 2)]), o.xl, o.yl);
  g.save(); g.beginPath(); g.rect(M.l, M.t, W - M.l - M.r, H - M.t - M.b); g.clip();
  o.series.forEach(s => {
    g.strokeStyle = g.fillStyle = s.color; g.lineWidth = s.w || 1.5;
    if (s.dash) g.setLineDash(s.dash);
    if (s.line !== false) { g.beginPath(); s.x.forEach((v, i) => { if (!isFinite(s.y[i])) return; i ? g.lineTo(X(v), Y(s.y[i])) : g.moveTo(X(v), Y(s.y[i])); }); g.stroke(); }
    g.setLineDash([]);
    if (s.pts) s.x.forEach((v, i) => { if (isFinite(s.y[i])) { g.beginPath(); g.arc(X(v), Y(s.y[i]), s.r || 3, 0, 7); g.fill(); } });
  });
  g.restore();
}

async function ogXic(k, mz) {
  const key = `${k}|${mz}`; if (OG.traces.has(key)) return OG.traces.get(key);
  const d = await getXic(k, mz, 0.35, 1); OG.traces.set(key, d); return d;
}
async function ogDrawXics() {
  const R = OG.res, cv = document.querySelector("#og-xic"), chips = document.querySelector("#og-chips"); if (!cv || !R) return;
  const items = [{ mz: R.mz, name: I18N.t("og.leg.ion", { mz: fmz(R.mz) }), color: OG_COLORS[0], w: 2.2 }, { mz: R.parent_mz, name: I18N.t("og.leg.par", { mz: fmz(R.parent_mz) }), color: OG_COLORS[1], w: 2.2 }];
  [...OG.sel].forEach((m, i) => items.push({ mz: m, name: I18N.t("og.leg.co", { mz: fmz(m) }), color: OG_COLORS[2 + (i % 6)], w: 1.3, dash: [4, 3] }));
  const half = Math.max(0.6, (R.parent_fwhm_s || 8) / 60 * 5), c = R.parent_apex_rt ?? ((R.window[0] + R.window[1]) / 2), x0 = c - half, x1 = c + half;
  const series = [];
  for (const it of items) {
    const t = await ogXic(OG.k, it.mz); if (!t) continue;
    let mx = 0; t.rt.forEach((v, i) => { if (v >= x0 && v <= x1 && t.y[i] > mx) mx = t.y[i]; });
    series.push({ x: t.rt, y: t.y.map(v => mx > 0 ? v / mx : 0), color: it.color, w: it.w, dash: it.dash, name: it.name });
  }
  ogPlot(cv, { series, x0, x1, y0: 0, y1: 1.08, xl: I18N.t("og.ax.rt"), yl: I18N.t("og.ax.norm") });
  chips.innerHTML = series.map(s => `<span><i style="background:${s.color}"></i>${EH(s.name)}</span>`).join("");
}

// High-resolution files only, and only with mzFinder on: its list of proofs (adduct step, co-elution, fragment in the MS2 of the parent, ratio across samples).
// A list of measurements, never a label (mzFinder is English-only, so these texts are not in the catalogs).
async function ogEvidence(R, box) {
  if (!R.hr || !window.TPCALL || !box) return;
  try {
    const ev = await window.TPCALL("origin_evidence", { report: Object.fromEntries(["mz", "parent_mz", "profile", "ratio", "ratio_constancy", "kinetics", "ms2_peaks"].map(k => [k, R[k]])), pol: R.pol || 1 });
    if (R !== OG.res || !ev || !ev.length) return;
    const mark = { evidence: "+", against: "-", hint: "~", info: "·" };
    box.innerHTML = `<h4>Proofs (mzFinder)</h4><p class="why">What the data say about the link between the two ions, one proof per line (+ supports, - against, ~ only a hint). They are measurements for you to weigh, not a conclusion.</p><ul>${ev.map(e => `<li><b aria-hidden="true">${mark[e.strength] || "·"}</b> ${EH(e.text)}</li>`).join("")}</ul>`;
    box.hidden = false;
  } catch (_) { /* mzFinder not ready: the report stays as it is */ }
}

function ogRender() {
  const R = OG.res, d = document.getElementById("ogdlg"), out = d.querySelector("#og-out"), P = R.profile || {}, F = R.ratio || {}, K = R.kinetics || {}, C = R.ratio_constancy || {};
  const warn = [...new Set([...(R.warnings || []), ...(P.warnings || []), ...(K.warnings || [])].map(ogMsg))];
  if (P.reliable === false) warn.push(I18N.t("og.warn.unreliable"));
  if (P.x_has_peak === false) warn.push(I18N.t("og.warn.noPeak"));
  const cands = (R.candidates || []).slice().sort((a, b) => (b.pearson ?? -2) - (a.pearson ?? -2));
  const loss = m => { const dm = R.parent_mz - m; return Math.abs(dm) < 0.5 ? "-" : dm > 0 ? I18N.t("og.heavier", { dm: ogN(dm, 1) }) : I18N.t("og.lighter", { dm: ogN(dm, 1) }); };
  const lossHint = m => { const dm = R.parent_mz - m; return dm > 1.5 ? I18N.t("og.lossHint", { dm: ogN(dm, 1), lost: ogN(dm, 0) }) : ""; };
  out.innerHTML = `${warn.map(w => `<div class="ogw">${EH(w)}</div>`).join("")}
<section><h4>${I18N.t("og.s1.title")}</h4>
<p class="why">${I18N.t("og.s1.why")}</p>
<canvas id="og-xic"></canvas><div id="og-chips" class="chips"></div>
<div class="kv"><span>${I18N.t("og.kv.pearson")} <b>${ogN(P.pearson_w)}</b></span><span>${I18N.t("og.kv.ci")} <b>${P.corr_ci ? ogN(P.corr_ci[0], 2) + " - " + ogN(P.corr_ci[1], 2) : "-"}</b></span><span>${I18N.t("og.kv.apex")} <b>${ogN(P.apex_diff_s, 2)} s</b> (±${ogN(P.apex_diff_sigma_s, 2)})</span><span>${I18N.t("og.kv.fwhm")} <b>${ogN(P.fwhm_ratio, 2)}</b></span><span>${I18N.t("og.kv.scans")} <b>${ogN(P.n_scans, 0)}</b></span></div>
<p class="why">${I18N.t("og.s1.why2")}</p></section>
<section><h4>${I18N.t("og.s2.title")}</h4>
<p class="why">${I18N.t("og.s2.why")}</p>
<div style="max-height:260px;overflow:auto"><table><thead><tr><th><i>m/z</i></th><th>${I18N.t("og.t2.dm")}</th><th>Pearson</th><th>${I18N.t("og.t2.apex")}</th><th>${I18N.t("og.t2.area")}</th><th>${I18N.t("og.t2.notes")}</th></tr></thead><tbody id="og-tb">${cands.map(c => `<tr data-m="${c.mz}" title="${EH(lossHint(c.mz))}"><td>${ogN(c.mz, 2)}</td><td>${loss(c.mz)}</td><td>${ogN(c.pearson, 3)}</td><td>${ogN(c.apex_diff_s, 2)}</td><td>${ogN(c.area_rel, 3)}</td><td>${(c.roles || []).map(r => EH(ogRole(r))).join("; ")}</td></tr>`).join("") || `<tr><td colspan="6">${I18N.t("og.t2.none")}</td></tr>`}</tbody></table></div></section>
<section><h4>${I18N.t("og.s3.title")}</h4>
<p class="why">${I18N.t("og.s3.why")}</p>
<div class="oggrid"><canvas id="og-fp"></canvas><div>
<div class="kv"><span>${I18N.t("og.kv.slope")} <b>${ogN(F.r_tls, 3)}</b> ±${ogN(F.r_se, 3)}</span><span>R<sup>2</sup> <b>${ogN(F.r2, 3)}</b></span><span>${I18N.t("og.kv.intercept")} <b>${ogN(F.intercept_rel, 3)}</b></span><span>${I18N.t("og.kv.points")} <b>${ogN(F.n, 0)}</b></span><span>${I18N.t("og.kv.resid")} <b>${ogN(F.resid_rel, 3)}</b></span></div>
<p class="why">${I18N.t("og.s3.why2")}</p>
<div class="kv"><span>${I18N.t("og.kv.cochran")} <b>${ogN(C.q, 2)}</b> (p = <b>${ogN(C.p, 3)}</b>)</span><span>${I18N.t("og.kv.cv")} <b>${ogN(C.cv, 3)}</b></span></div>
<p class="why">${I18N.t("og.s3.why3")}</p></div></div></section>
<section><h4>${I18N.t("og.s4.title")}</h4>
<p class="why">${I18N.t("og.s4.why")}</p>
<div class="oggrid"><canvas id="og-kin"></canvas><div>
<div class="kv"><span>${I18N.t("og.kv.decay")} <b>${ogN(K.parent_decay && K.parent_decay.k, 4)}</b> /min</span><span>${I18N.t("og.kv.half")} <b>${ogN(K.parent_decay && K.parent_decay.half_life, 1)}</b> min</span><span>${I18N.t("og.kv.spearman")} <b>${ogN(K.spearman_parent, 2)}</b></span></div>
<div class="kv"><span>${I18N.t("og.kv.mTracks")} AICc <b>${ogN(K.models && K.models.tracks_parent && K.models.tracks_parent.aicc, 1)}</b></span><span>${I18N.t("og.kv.mDecay")} <b>${ogN(K.models && K.models.decay && K.models.decay.aicc, 1)}</b></span><span>${I18N.t("og.kv.mForm")} <b>${ogN(K.models && K.models.formation_decay && K.models.formation_decay.aicc, 1)}</b></span></div>
<p class="why">${I18N.t("og.s4.why2")}</p></div></div></section>
<section id="og-ev" hidden></section>
<section><h4>${I18N.t("og.s5.title")}</h4>
<p class="why">${I18N.t("og.s5.why")}</p>
<textarea id="og-hyp" placeholder="${I18N.t("og.hyp.ph")}"></textarea>
<p style="margin:8px 0 0"><button id="og-xlsx">${I18N.t("og.export")}</button></p></section>
<p class="why">${EH(R.note ? ogMsg(R.note) : "")}</p>`;
  d.querySelectorAll("#og-tb tr[data-m]").forEach(tr => tr.onclick = () => {
    const m = +tr.dataset.m; if (OG.sel.has(m)) OG.sel.delete(m); else OG.sel.add(m);
    tr.classList.toggle("sel", OG.sel.has(m)); ogDrawXics();
  });
  const key = `${fmz(R.mz)}|${fmz(R.parent_mz)}`; NB.origin = NB.origin || {};
  const ta = d.querySelector("#og-hyp"); ta.value = NB.origin[key] || "";
  ta.oninput = () => { NB.origin[key] = ta.value; nbSave(); };
  d.querySelector("#og-xlsx").onclick = () => ogXlsx();
  ogEvidence(R, d.querySelector("#og-ev"));
  requestAnimationFrame(() => {
    ogDrawXics();
    const pts = F.points || {}, p = pts.p || [], f = pts.f || [], fit = pts.fit || [];
    const order = p.map((v, i) => i).sort((a, b) => p[a] - p[b]);
    ogPlot(d.querySelector("#og-fp"), { series: [{ x: p, y: f, color: OG_COLORS[0], pts: true, line: false, r: 2.5 }, { x: order.map(i => p[i]), y: order.map(i => fit[i]), color: OG_COLORS[1], w: 1.6 }], x0: 0, y0: 0, xl: I18N.t("og.ax.areaP"), yl: I18N.t("og.ax.ionF") });
    const sm = R.samples || [], kt = K.times || sm.map(s => s.time);
    ogPlot(d.querySelector("#og-kin"), { series: [{ x: kt, y: K.parent_norm || [], color: OG_COLORS[1], pts: true, w: 1.6 }, { x: kt, y: K.ion_norm || [], color: OG_COLORS[0], pts: true, w: 1.6 }], y0: 0, xl: I18N.t("og.ax.treat"), yl: I18N.t("og.ax.rel") });
  });
}
function ogXlsx() {
  const R = OG.res; if (!R) return; const P = R.profile || {}, F = R.ratio || {}, K = R.kinetics || {}, C = R.ratio_constancy || {};
  const key = `${fmz(R.mz)}|${fmz(R.parent_mz)}`, pts = F.points || {};
  dlx(I18N.t("og.xlsx.prefix") + fmz(R.mz) + I18N.t("og.xlsx.from") + fmz(R.parent_mz), [
    { name: I18N.t("og.x.summary"), head: [I18N.t("og.x.measure"), I18N.t("lib.all.value")], widths: [46, 20], rows: [[I18N.t("og.x.ionMz"), R.mz], [I18N.t("og.x.parMz"), R.parent_mz], [I18N.t("og.x.ref"), R.ref_label || ""], [I18N.t("og.x.rt0"), R.window ? R.window[0] : null], [I18N.t("og.x.rt1"), R.window ? R.window[1] : null], [I18N.t("og.x.apex"), R.parent_apex_rt], [I18N.t("og.x.pearson"), P.pearson_w], [I18N.t("og.x.apexDiff"), P.apex_diff_s], [I18N.t("og.x.sigma"), P.apex_diff_sigma_s], [I18N.t("og.x.fwhm"), P.fwhm_ratio], [I18N.t("og.x.slope"), F.r_tls], [I18N.t("og.x.slopeErr"), F.r_se], [I18N.t("og.x.r2"), F.r2], [I18N.t("og.x.intercept"), F.intercept_rel], [I18N.t("og.x.q"), C.q], [I18N.t("og.x.p"), C.p], [I18N.t("og.x.k"), K.parent_decay ? K.parent_decay.k : null], [I18N.t("og.x.hyp"), (NB.origin && NB.origin[key]) || ""], ...[...(R.warnings || []), ...(P.warnings || []), ...(K.warnings || [])].map(w => [I18N.t("og.x.warning"), ogMsg(w)])] },
    { name: I18N.t("og.x.coeluting"), head: ["m/z", I18N.t("og.x.dm"), "Pearson", I18N.t("og.x.dApex"), I18N.t("og.x.area"), I18N.t("lst.col.note")], widths: [10, 20, 10, 14, 14, 40], rows: (R.candidates || []).map(c => [c.mz, R.parent_mz - c.mz, c.pearson, c.apex_diff_s, c.area_rel, (c.roles || []).map(ogRole).join("; ")]) },
    { name: I18N.t("og.x.samples"), head: [I18N.t("og.x.sample"), I18N.t("load.table.time"), I18N.t("og.x.areaCand"), I18N.t("og.x.areaIon"), I18N.t("og.x.ratio"), I18N.t("og.x.error")], widths: [24, 12, 16, 16, 14, 12], rows: (R.samples || []).map(s => [s.label, s.time, s.area_parent, s.area_ion, s.ratio, s.ratio_se]) },
    { name: I18N.t("og.x.fp"), head: [I18N.t("og.x.pCand"), I18N.t("og.x.fIon"), I18N.t("og.x.fLine")], widths: [16, 16, 16], rows: (pts.p || []).map((v, i) => [v, pts.f[i], (pts.fit || [])[i]]) },
    { name: I18N.t("og.x.kin"), head: [I18N.t("load.table.time"), I18N.t("og.x.candNorm"), I18N.t("og.x.ionNorm"), I18N.t("og.x.ratio2")], widths: [12, 22, 20, 12], rows: (K.times || []).map((t, i) => [t, (K.parent_norm || [])[i], (K.ion_norm || [])[i], (K.ratio || [])[i]]) }]);
}
