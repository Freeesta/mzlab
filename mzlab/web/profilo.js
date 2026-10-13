// @ts-strict
"use strict";
// Time profile of ONE ion chosen by the user («Statistica» panel): height or area of its XIC in every file, in the order of the file times,
// the points joined by a line, the table under the chart (copy / Excel). The measure is the integration that is already there (autoEdges +
// integ of picchi.js, the same numbers as the integration table); nothing is fitted or smoothed.
//   Low resolution: only for an m/z the student types, raw values; no automatic normalization, no list of ions (principle 1).
//   High resolution: optionally divided by the internal standard the user defined in QA/QC (ratio per file).
// Order, normalization and the points of the line come from /api/stat/profile (mzlab/statistica.py); the page draws.
const PROFILO = (() => {
  const KEY = "qqq.profilo";
  const t = (k, p) => I18N.t(k, p);
  const eh = s => EH(s == null ? "" : s);
  const isHr = () => !!(window.HR && typeof BANCO !== "undefined" && BANCO.on());
  const qaOn = () => !!(window.FEATURES && FEATURES.qaqc);      // «Normalise on» (internal standards of QA/QC) exists only with QA/QC on
  let st = { text: "", rt: "", metric: "area", istd: "" };
  let res = null;                                              // last result {ion, rows, series, normalized, metric}
  Object.assign(st, lsObj(KEY));
  const save = () => { try { localStorage.setItem(KEY, JSON.stringify(st)); } catch (_) { /* no storage */ } };
  const series = () => (window.CAMPIONI ? CAMPIONI.files() : []).filter(f => f.kind !== "mrm");
  const fmt = (v, d = 2) => (v == null || !isFinite(v) ? "–" : Math.abs(v) >= 1e4 || (v !== 0 && Math.abs(v) < 0.01) ? v.toExponential(2) : v.toFixed(d));

  async function resolveMz(text) {
    const s = String(text || "").trim().replace(",", ".");
    if (/^\d+(\.\d+)?$/.test(s)) return { mz: parseFloat(s), formula: null };
    const j = await getFormula(s, "[M+H]+");
    return { mz: isHr() ? j.mz5 : j.mz, formula: j.formula };
  }
  // height and area of the peak of the ion at `rt` (minutes) in one file, as the integration of the XIC panel does; null where there is no peak
  async function measure(f, mz, rt) {
    const [c, w] = window.HR ? HR.xicArgs(f, { mz, ion: true, obs: false }, { tol: 0.5 }) : (() => { const x = xicWin(mz, false); return [(x[0] + x[1]) / 2, (x[1] - x[0]) / 2]; })();
    const d = await getXic(f.k, c, w, f.lv || 1);
    const s = { x: d.rt, y: d.y, k: f.k, key: `x|${f.k}|${mz}` };
    if (!s.x || s.x.length < 9) return null;
    const ed = autoEdges(s, rt, 0);
    if (!ed) return null;
    const m = integ(s, ed[0], ed[1]);
    return m.area > 0 ? { rt: m.rt, area: m.area, height: m.height } : null;
  }
  const rowOf = (f, m) => ({ file: f.file, label: f.label || f.file, time: f.time ?? null, role: window.QAQC ? QAQC.roleOf(f) : f.type, rt: m ? m.rt : null, area: m ? m.area : null, height: m ? m.height : null });

  async function run(text, rtText) {
    const r = await resolveMz(text), x = numIt(rtText);
    if (!(r.mz > 0) || !(x >= 0)) throw new Error(t("prof.err.input"));
    const fs = series();
    if (!fs.length) throw new Error(t("qaqc.err.nofiles"));
    const pts = [];
    for (const f of fs) pts.push(rowOf(f, await measure(f, r.mz, x)));
    let istd = null, istdIon = null;
    if (isHr() && qaOn() && st.istd !== "") {
      istdIon = (window.QAQC ? QAQC.state.ions : []).find(i => String(i.id) === String(st.istd));
      if (istdIon) { istd = {}; for (const f of fs) { const m = await measure(f, istdIon.mz, istdIon.rt); istd[f.file] = m ? { area: m.area, height: m.height } : {}; } }
    }
    const j = await fetch("api/stat/profile", { method: "POST", headers: { "content-type": "application/json" },
      body: JSON.stringify({ points: pts.map(p => ({ file: p.file, time: p.time, rt: p.rt, area: p.area, height: p.height })), metric: st.metric, istd }) }).then(q => q.json());
    if (j.error) throw I18N.err(j);
    const by = Object.fromEntries(pts.map(p => [p.file, p]));
    res = { ion: { text, mz: r.mz, rt: x, label: r.formula || String(r.mz) }, istd: istdIon, metric: j.metric, normalized: j.normalized, series: j.series,
      rows: j.rows.map(o => ({ ...o, label: by[o.file].label, role: by[o.file].role })) };
    if (window.WORKFLOW && WORKFLOW.logOp) WORKFLOW.logOp("stat_profile", { mz: r.mz, rt: x, metric: j.metric });
    return res;
  }

  // ---------------------------------------------------------------- drawing
  const colorOf = file => { const f = (typeof E !== "undefined" ? E.files : []).find(x => x.file === file); return (f && f.color) || "var(--accent)"; };
  function chart(r) {
    const key = r.normalized ? "norm" : "value", pts = r.series.map(i => r.rows[i]);
    if (!pts.length) return `<p class="sm">${t("prof.nopoints")}</p>`;
    const u = tunitGet(), W = 300, H = 140, L = 44, R = 8, T = 8, B = 24;
    const xs = pts.map(p => p.time), ys = pts.map(p => p[key]);
    let x0 = Math.min(...xs), x1 = Math.max(...xs), y0 = Math.min(0, ...ys), y1 = Math.max(...ys);
    if (x1 === x0) { x0 -= 1; x1 += 1; }
    if (y1 === y0) y1 = y0 + 1;
    const X = v => L + (v - x0) / (x1 - x0) * (W - L - R), Y = v => T + (1 - (v - y0) / (y1 - y0)) * (H - T - B);
    const lab = r.normalized ? t("prof.y.norm") : t(r.metric === "area" ? "prof.y.area" : "prof.y.height");
    let s = `<svg class="prof-chart" viewBox="0 0 ${W} ${H}" role="img" aria-label="${eh(t("prof.chart.aria", { ion: r.ion.label }))}"><title>${eh(t("prof.chart.aria", { ion: r.ion.label }))}</title>`;
    s += `<line x1="${L}" y1="${H - B}" x2="${W - R}" y2="${H - B}" stroke="var(--line)"/><line x1="${L}" y1="${T}" x2="${L}" y2="${H - B}" stroke="var(--line)"/>`;
    s += `<text x="${L - 3}" y="${T + 4}" text-anchor="end" font-size="8" fill="var(--muted)">${eh(fmt(y1))}</text><text x="${L - 3}" y="${H - B}" text-anchor="end" font-size="8" fill="var(--muted)">${eh(fmt(y0))}</text>`;
    s += `<text x="${L}" y="${H - B + 10}" font-size="8" fill="var(--muted)">${eh(fmt(timeShown(x0, u), 1))}</text><text x="${W - R}" y="${H - B + 10}" text-anchor="end" font-size="8" fill="var(--muted)">${eh(fmt(timeShown(x1, u), 1))}</text>`;
    s += `<text x="${(L + W - R) / 2}" y="${H - 3}" text-anchor="middle" font-size="8" fill="var(--muted)">${eh(t("meta.time"))} (${eh(tunitLabel(u))})</text>`;
    s += `<text x="7" y="${(T + H - B) / 2}" font-size="8" fill="var(--muted)" transform="rotate(-90 7 ${(T + H - B) / 2})" text-anchor="middle">${eh(lab)}</text>`;
    if (pts.length > 1) s += `<polyline fill="none" stroke="var(--muted)" stroke-width="1.2" points="${pts.map(p => X(p.time).toFixed(1) + "," + Y(p[key]).toFixed(1)).join(" ")}"/>`;
    pts.forEach(p => {
      const x = X(p.time), y = Y(p[key]), c = colorOf(p.file), tip = `<title>${eh(p.label)} · ${eh(fmtTime(p.time))} · ${eh(fmt(p[key], 3))}</title>`;
      s += p.role === "qc" ? `<path d="M${x} ${y - 5}L${x + 5} ${y}L${x} ${y + 5}L${x - 5} ${y}Z" fill="${c}" stroke="var(--ink)" stroke-width=".6">${tip}</path>`
        : `<circle cx="${x}" cy="${y}" r="3.8" fill="${c}" stroke="var(--ink)" stroke-width=".6">${tip}</circle>`;
    });
    return s + "</svg>";
  }
  function head(r) {
    return [t("qaqc.col.file"), t("qaqc.col.type"), t("meta.time") + " (" + tunitLabel(tunitGet()) + ")", "RT", t("qaqc.col.area"), t("prof.col.height")].concat(r.normalized ? [t("prof.col.norm")] : []);
  }
  function body(r) {
    const u = tunitGet();
    return r.rows.map((o, k) => [o.label, CAMPIONI.roleName(o.role), o.time == null ? "" : timeShown(o.time, u), o.rt == null ? "" : +o.rt.toFixed(3), o.area == null ? "" : o.area, o.height == null ? "" : o.height].concat(r.normalized ? [o.norm == null ? "" : o.norm] : []));
  }
  function tableHtml(r) {
    const b = body(r);
    return `<div class="qq-scroll"><table class="qq-tab" id="prof-tab"><thead><tr>${head(r).map(h => `<th scope="col">${eh(h)}</th>`).join("")}</tr></thead><tbody>${b.map((row, k) => `<tr${r.series.includes(k) ? "" : ' class="sm"'}>${row.map((c, i) => `<td${i > 1 ? ' class="num"' : ""}>${i > 1 && typeof c === "number" ? eh(fmt(c, i === 2 || i === 3 ? 2 : 1)) : eh(c)}</td>`).join("")}</tr>`).join("")}</tbody></table></div>`;
  }
  const tsv = r => [head(r)].concat(body(r)).map(l => l.map(c => String(c).replace(/[\t\r\n]+/g, " ")).join("\t")).join("\n") + "\n";

  // ---------------------------------------------------------------- the section
  function paint(box) {
    const hr = isHr(), ions = hr && qaOn() && window.QAQC ? QAQC.state.ions.filter(i => i.kind !== "known") : [];
    box.innerHTML = `<p class="sm">${t(hr ? (qaOn() ? "prof.intro.hr" : "prof.intro.hr.plain") : "prof.intro.lr")}</p>
<form id="prof-form" class="qq-form"><label>${t("qaqc.in.mz")} <input id="prof-mz" value="${eh(st.text)}" autocomplete="off" required></label><label>${t("qaqc.in.rt")} <input id="prof-rt" value="${eh(st.rt)}" inputmode="decimal" size="5" required></label>
<label>${t("prof.metric")} <select id="prof-metric"><option value="area"${st.metric === "area" ? " selected" : ""}>${t("qaqc.col.area")}</option><option value="height"${st.metric === "height" ? " selected" : ""}>${t("prof.col.height")}</option></select></label>
${hr && qaOn() ? `<label>${t("prof.istd")} <select id="prof-istd"><option value="">${t("prof.istd.none")}</option>${ions.map(i => `<option value="${i.id}"${String(i.id) === String(st.istd) ? " selected" : ""}>${eh(i.label)}</option>`).join("")}</select></label>` : ""}
<button type="submit" id="prof-go">${t("prof.go")}</button></form>
${hr && qaOn() && !ions.length ? `<p class="sm">${t("prof.istd.hint")}</p>` : ""}
<div id="prof-msg" class="sm bad" role="alert"></div><div id="prof-res"></div>`;
    paintRes(box);
  }
  function paintRes(box) {
    const el = box.querySelector("#prof-res");
    if (!el) return;
    if (!res) { el.innerHTML = ""; return; }
    el.innerHTML = `<div class="sm">${t("prof.sum", { mz: fmt(res.ion.mz, 4), rt: fmt(res.ion.rt, 2) })}${res.normalized && res.istd ? " " + t("prof.sum.istd", { istd: eh(res.istd.label) }) : ""}</div>${chart(res)}${window.CAMPIONI ? CAMPIONI.gradientLegend() : ""}
${tableHtml(res)}<div class="qq-form"><button type="button" id="prof-copy">${t("meta.copy")}</button><button type="button" id="prof-xlsx">Excel</button></div><div id="prof-note" class="sm" role="status"></div>`;
  }
  function mount(box) {
    paint(box);
    box.onchange = e => {
      if (e.target.id === "prof-metric") { st.metric = e.target.value; save(); }
      if (e.target.id === "prof-istd") { st.istd = e.target.value; save(); }
    };
    box.onclick = e => {
      if (e.target.id === "prof-copy" && res) {
        const note = box.querySelector("#prof-note");
        (navigator.clipboard ? navigator.clipboard.writeText(tsv(res)) : Promise.reject()).then(() => { note.textContent = t("meta.copied"); }, () => { note.textContent = t("meta.copy.fail"); });
      }
      if (e.target.id === "prof-xlsx" && res) dlx("profile.xlsx", [{ name: "Profile", head: head(res), rows: body(res), widths: [28, 12, 12, 10, 14, 14, 14] }]);
    };
    box.onsubmit = async e => {
      e.preventDefault();
      st.text = box.querySelector("#prof-mz").value; st.rt = box.querySelector("#prof-rt").value; save();
      const msg = box.querySelector("#prof-msg"), go = box.querySelector("#prof-go");
      go.disabled = true; msg.textContent = "";
      try { await run(st.text, st.rt); paintRes(box); }
      catch (err) { msg.textContent = String(err && err.message || err); }
      go.disabled = false;
    };
  }
  // used by the other tools (the PCA loadings): fill the form, open the panel and run
  async function show(mz, rt) {
    st.text = String(mz); st.rt = String(rt); save();
    if (window.BARRA) BARRA.setTab("stat", { noFocus: true, instant: true });
    const box = document.querySelector("#stat-profile");
    if (box) { paint(box); try { await run(st.text, st.rt); paintRes(box); } catch (err) { const m = box.querySelector("#prof-msg"); if (m) m.textContent = String(err && err.message || err); } }
    return res;
  }

  if (typeof STAT !== "undefined") STAT.add({ id: "profile", title: "prof.title", visible: () => true, mount });
  return { mount, run, show, measure, get state() { return st; }, get result() { return res; }, tsv };
})();
window.PROFILO = PROFILO;
