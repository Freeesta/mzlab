"use strict";
// QA/QC panel of the drawer («QA/QC»): measures of the series, never verdicts (NORMAN guide, Hollender et al. 2023; BP4NTA).
//   - type of each file (sample, blank, QC, standard, dark): guessed from the name (project.py + QC / dark words), corrected by hand
//   - control ions chosen by the user (internal standards; in HR also known ions): RT found and RT shift, area, area relative to the
//     median, observed m/z and error in ppm (HR only), drift along the order of the files, CV of the areas, mass accuracy
//   - sample / blank ratio of a chosen ion (3:1 or 10:1) and, in HR, of the features of an mzFinder table
// Low resolution: only RT and area of the standards the user chose and the blanks; no list of ions of the sample (principle 1).
// The numbers come from /api/qaqc (mzlab/qaqc.py); the state (roles, ions, threshold) is saved in localStorage qqq.qaqc and in the workflow file.
const QAQC = (() => {
  const KEY = "qqq.qaqc";
  const ROLES = ["sample", "blank", "qc", "standard", "dark"];
  const MARK = { sample: "c", blank: "s", qc: "d", standard: "t", dark: "x" };      // circle, square, diamond, triangle, cross: not only colour
  const COL = ["var(--lv1)", "var(--lv3)", "var(--lv2a)", "var(--lv4)", "var(--lv2b)", "var(--lv5)"];
  const t = (k, p) => I18N.t(k, p);
  const Q1 = s => document.querySelector(s);
  const eh = s => String(s == null ? "" : s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const isHr = () => !!(window.HR && typeof BANCO !== "undefined" && BANCO.on());
  const blankState = () => ({ roles: {}, ions: [], thr: 3, win: 0.5, ratio: { text: "", rt: "" }, seq: 1 });
  let st = blankState();
  let res = { ions: [], ratio: null, feat: null, at: null };      // last results, read by the study report
  try { Object.assign(st, JSON.parse(localStorage.getItem(KEY) || "{}")); } catch (_) { /* none */ }
  const save = () => { try { localStorage.setItem(KEY, JSON.stringify(st)); } catch (_) { /* no storage */ } };

  // ---------------------------------------------------------------- the series and the type of each file
  const series = () => (typeof E !== "undefined" && E.files ? E.files : []).filter(f => !f.gone && (f.lv || 1) === 1 && f.kind !== "mrm" && f.kind !== "empty");
  function guessRole(f) {
    const n = String(f.file || "") + " " + String(f.label || "");
    if (/(^|[_\-\s.])(qc|pool|pooled)([_\-\s.\d]|$)/i.test(n)) return "qc";
    if (/(^|[_\-\s.])(dark|buio)([_\-\s.\d]|$)/i.test(n)) return "dark";
    return f.type === "blank" ? "blank" : f.type === "standard" ? "standard" : "sample";
  }
  const roleOf = f => (ROLES.includes(st.roles[f.file]) ? st.roles[f.file] : guessRole(f));
  const setRole = (file, role) => { if (ROLES.includes(role)) { st.roles[file] = role; save(); } };       // the sample table (campioni.js) writes the type of a file here
  const roleName = r => (r === "blank" ? t("qaqc.role.blank") : r === "qc" ? t("qaqc.role.qc") : r === "standard" ? t("qaqc.role.standard") : r === "dark" ? t("qaqc.role.dark") : t("qaqc.role.sample"));

  // ---------------------------------------------------------------- ions: m/z or formula, RT expected
  async function resolveMz(text) {
    const s = String(text || "").trim().replace(",", ".");
    if (/^\d+(\.\d+)?$/.test(s)) return { mz: parseFloat(s), formula: null };
    const j = await getFormula(s, "[M+H]+");
    return { mz: isHr() ? j.mz5 : j.mz, formula: j.formula };
  }
  function window_(mz) {                                        // [center, half width, unit] of the extraction window
    if (isHr()) return [mz, (typeof UIP !== "undefined" && UIP.hrPpm) || 5, "ppm"];
    const w = xicWin(mz, false);
    return [(w[0] + w[1]) / 2, (w[1] - w[0]) / 2, "da"];
  }
  async function query(mz, rt, ratio) {
    const fs = series();
    if (!fs.length) throw new Error(t("qaqc.err.nofiles"));
    const [c, tol, unit] = window_(mz);
    const q = `k=${fs.map(f => f.k).join(",")}&roles=${fs.map(roleOf).join(",")}&mz=${c}&rt=${rt}&win=${st.win}&tol=${tol}&unit=${unit}&level=1&thr=${st.thr}` + (ratio ? "&ratio=1" : "");
    return { files: fs, roles: fs.map(roleOf), r: await J("api/qaqc?" + q) };
  }

  async function addIon(text, rt, label, kind) {
    const r = await resolveMz(text);
    const x = numIt(rt);
    if (!(r.mz > 0) || !(x >= 0)) throw new Error(t("qaqc.err.input"));
    st.ions.push({ id: st.seq++, kind: kind === "known" && isHr() ? "known" : "is", text: String(text).trim(), mz: r.mz, rt: x, label: label || r.formula || String(r.mz) });
    save();
  }

  // ---------------------------------------------------------------- run
  async function run() {
    const out = [];
    for (const ion of st.ions) {
      try { const q = await query(ion.mz, ion.rt, false); out.push({ ion, files: q.files.map(f => f.file), roles: q.roles, r: q.r }); }
      catch (e) { out.push({ ion, error: String(e && e.message || e) }); }
    }
    res.ions = out;
    res.at = new Date().toISOString();
    return res;
  }
  async function runRatio() {
    const text = (Q1("#qq-rin") || {}).value ?? st.ratio.text, rt = (Q1("#qq-rrt") || {}).value ?? st.ratio.rt;
    st.ratio = { text, rt }; save();
    const r = await resolveMz(text), x = numIt(rt);
    if (!(r.mz > 0) || !(x >= 0)) throw new Error(t("qaqc.err.input"));
    const q = await query(r.mz, x, true);
    res.ratio = { text, mz: r.mz, rt: x, thr: st.thr, files: q.files.map(f => f.file), roles: q.roles, r: q.r };
    return res.ratio;
  }
  async function runFeatures() {
    const tb = window.UNTUI && UNTUI.state && UNTUI.state.table;
    if (!tb || !isHr()) return null;
    const names = (tb.samples || []).map(s => (typeof s === "string" ? s : s.name));
    const roleByName = Object.fromEntries(series().map(f => [f.file, roleOf(f)]));
    const base = n => String(n).replace(/\.mzml$/i, "").toLowerCase();
    const byBase = Object.fromEntries(series().map(f => [base(f.file), roleOf(f)]));
    const roleFor = n => roleByName[n] || byBase[base(n)] || "sample";
    const blanks = names.filter(n => roleFor(n) === "blank"), samples = names.filter(n => !["blank", "dark"].includes(roleFor(n)));
        const roles = names.map(roleFor);
    // same rule as the Statistics section «Difference from the reference», with the blanks (mean area) as the reference
    const j = await fetch("api/stat/reference", { method: "POST", headers: { "content-type": "application/json" },
      body: JSON.stringify({ matrix: tb.features.map(f => names.map(n => (f.areas && f.areas[n]) || 0)), roles, times: names.map(() => null), reference: roles.flatMap((r, i) => (r === "blank" ? [i] : [])), factor: st.thr, stat: "mean" }) }).then(r => r.json());
    if (j.error) throw I18N.err(j);
    j.rows = j.rows.map((q, i) => { const f = tb.features[i]; return { key: f.key, mz: f.mz, rt: f.rt, blank_area: q.ref_level, sample_area: q.treated_max, ratio: q.ratio, absent_in_blank: q.ratio == null, at_least: q.ratio == null ? q.treated_max > 0 : q.ratio >= st.thr }; });
    res.feat = { thr: st.thr, blanks, samples, rows: j.rows };
    return res.feat;
  }

  // ---------------------------------------------------------------- drawing
  const fmt = (v, d = 2) => (v == null || !isFinite(v) ? "–" : Math.abs(v) >= 1e4 || (v !== 0 && Math.abs(v) < 0.01) ? v.toExponential(2) : v.toFixed(d));
  function mark(kind, x, y, col, lab) {
    const s = 3.6;
    const a = `fill="${col}" stroke="var(--ink)" stroke-width=".6"`;
    const g = kind === "s" ? `<rect x="${x - s}" y="${y - s}" width="${2 * s}" height="${2 * s}" ${a}/>`
      : kind === "d" ? `<path d="M${x} ${y - s - 1}L${x + s + 1} ${y}L${x} ${y + s + 1}L${x - s - 1} ${y}Z" ${a}/>`
      : kind === "t" ? `<path d="M${x} ${y - s - 1}L${x + s + 1} ${y + s}L${x - s - 1} ${y + s}Z" ${a}/>`
      : kind === "x" ? `<path d="M${x - s} ${y - s}L${x + s} ${y + s}M${x + s} ${y - s}L${x - s} ${y + s}" stroke="${col}" stroke-width="2" fill="none"/>`
      : `<circle cx="${x}" cy="${y}" r="${s}" ${a}/>`;
    return `<g>${lab ? `<title>${eh(lab)}</title>` : ""}${g}</g>`;
  }
  // one value per file along the order of the series; `lines`: [{label, vals, color}], roles of the files for the shape of the marks
  function chart(title, lines, roles, ylab) {
    const W = 300, H = 120, L = 38, R = 6, T = 8, B = 20, n = roles.length;
    const all = lines.flatMap(l => l.vals).filter(v => v != null && isFinite(v));
    if (!all.length || n < 2) return "";
    let lo = Math.min(...all), hi = Math.max(...all);
    if (lo === hi) { lo -= 1; hi += 1; }
    const pad = (hi - lo) * 0.08; lo -= pad; hi += pad;
    const X = i => L + (n === 1 ? 0 : (i * (W - L - R)) / (n - 1)), Y = v => T + (1 - (v - lo) / (hi - lo)) * (H - T - B);
    let s = `<svg class="qq-chart" viewBox="0 0 ${W} ${H}" role="img" aria-label="${eh(title)}"><title>${eh(title)}</title>`;
    s += `<line x1="${L}" y1="${H - B}" x2="${W - R}" y2="${H - B}" stroke="var(--line)"/><line x1="${L}" y1="${T}" x2="${L}" y2="${H - B}" stroke="var(--line)"/>`;
    s += `<text x="${L - 3}" y="${T + 4}" text-anchor="end" font-size="8" fill="var(--muted)">${eh(fmt(hi - pad, 2))}</text><text x="${L - 3}" y="${H - B}" text-anchor="end" font-size="8" fill="var(--muted)">${eh(fmt(lo + pad, 2))}</text>`;
    s += `<text x="${(L + W - R) / 2}" y="${H - 4}" text-anchor="middle" font-size="8" fill="var(--muted)">${eh(t("qaqc.chart.x"))}</text>`;
    s += `<text x="6" y="${(T + H - B) / 2}" font-size="8" fill="var(--muted)" transform="rotate(-90 6 ${(T + H - B) / 2})" text-anchor="middle">${eh(ylab)}</text>`;
    lines.forEach(l => {
      const pts = l.vals.map((v, i) => (v == null || !isFinite(v) ? null : [X(i), Y(v), i, v])).filter(Boolean);
      if (pts.length > 1) s += `<polyline fill="none" stroke="${l.color}" stroke-width="1.2" stroke-dasharray="${l.dash || ""}" points="${pts.map(p => p[0].toFixed(1) + "," + p[1].toFixed(1)).join(" ")}"/>`;
      pts.forEach(p => { s += mark(MARK[roles[p[2]]] || "c", p[0], p[1], l.color, `${l.label} · #${p[2] + 1} · ${roleName(roles[p[2]])} · ${fmt(p[3], 3)}`); });
    });
    return s + "</svg>";
  }
  function table(head, rows) {
    return `<table class="qq-tab"><thead><tr>${head.map(h => `<th scope="col">${eh(h)}</th>`).join("")}</tr></thead><tbody>${rows.map(r => `<tr>${r.map((c, i) => `<td${i ? ' class="num"' : ""}>${i ? eh(c) : c}</td>`).join("")}</tr>`).join("")}</tbody></table>`;
  }
  function ionBlock(o, i) {
    const ion = o.ion;
    if (o.error) return `<div class="qq-ion"><b>${eh(ion.label)}</b> <span class="sm bad">${eh(o.error)}</span></div>`;
    const r = o.r, hr = isHr(), rows = r.rows;
    const head = [t("qaqc.col.file"), t("qaqc.col.type"), "RT", "ΔRT", t("qaqc.col.area"), t("qaqc.col.rel")].concat(hr ? ["Δppm"] : []);
    const body = o.files.map((f, k) => {
      const x = rows[k];
      return [`<span class="qq-n">${k + 1}</span> ${eh(f)}`, roleName(o.roles[k])].concat(x ? [fmt(x.rt, 3), fmt(x.drt, 3), fmt(x.area, 0), fmt(x.rel, 2)].concat(hr ? [fmt(x.dppm, 2)] : []) : ["–", "–", "–", "–", "–"].concat(hr ? ["–"] : []));
    });
    const sum = [];
    sum.push(`<li>${t("qaqc.sum.cv", { v: fmt(r.cv_area, 1) })}</li>`);
    if (r.cv_area_qc != null) sum.push(`<li>${t("qaqc.sum.cvqc", { v: fmt(r.cv_area_qc, 1) })}</li>`);
    sum.push(`<li>${t("qaqc.sum.drift", { v: fmt(r.rt_drift_per_file, 4) })}</li>`);
    if (hr && r.mass_accuracy) sum.push(`<li>${t("qaqc.sum.ppm", { mean: fmt(r.mass_accuracy.mean, 2), max: fmt(r.mass_accuracy.max_abs, 2), n: r.mass_accuracy.n })}</li>`);
    const col = COL[i % COL.length];
    const c1 = chart(t("qaqc.chart.rt", { ion: ion.label }), [{ label: ion.label, vals: rows.map(x => (x ? x.drt : null)), color: col }], o.roles, "ΔRT (min)");
    const c2 = chart(t("qaqc.chart.area", { ion: ion.label }), [{ label: ion.label, vals: rows.map(x => (x ? x.rel : null)), color: col }], o.roles, t("qaqc.col.rel"));
    return `<div class="qq-ion"><div class="qq-h"><b>${eh(ion.label)}</b> <span class="sm">m/z ${eh(fmt(ion.mz, 4))} · RT ${eh(fmt(ion.rt, 2))} · ${t(ion.kind === "known" ? "qaqc.kind.known" : "qaqc.kind.is")}</span> <button type="button" data-rm="${ion.id}" aria-label="${eh(t("qaqc.remove", { n: ion.label }))}">×</button></div>${c1}${c2}<ul class="sm qq-sum">${sum.join("")}</ul><div class="qq-scroll">${table(head, body)}</div></div>`;
  }
  function ratioBlock(rt) {
    if (!rt) return "";
    const rows = rt.files.map((f, k) => {
      const a = rt.r.rows[k], q = rt.r.ratio.rows[k];
      return [eh(f), roleName(rt.roles[k]), a ? fmt(a.area, 0) : "0", rt.roles[k] === "blank" ? "–" : !q ? "–" : q.absent_in_blank ? t("qaqc.ratio.absent") : fmt(q.ratio, 1), rt.roles[k] === "blank" || !q ? "–" : t(q.at_least ? "qaqc.yes" : "qaqc.no")];
    });
    return `<div class="qq-ion"><div class="sm">${t("qaqc.ratio.sum", { mz: fmt(rt.mz, 4), thr: rt.thr, blank: fmt(rt.r.ratio.blank_area, 0) })}</div><div class="qq-scroll">${table([t("qaqc.col.file"), t("qaqc.col.type"), t("qaqc.col.area"), t("qaqc.ratio.col"), t("qaqc.ratio.atleast", { thr: rt.thr })], rows)}</div></div>`;
  }
  function featBlock(f) {
    if (!f) return "";
    const rows = f.rows.filter(x => x.ratio != null || x.absent_in_blank).sort((a, b) => (b.absent_in_blank ? 1e99 : b.ratio) - (a.absent_in_blank ? 1e99 : a.ratio));
    const n = rows.filter(x => x.at_least).length;
    return `<div class="qq-ion"><div class="sm">${t("qaqc.feat.sum", { n, tot: f.rows.length, thr: f.thr, b: f.blanks.length })}</div><div class="qq-scroll">${table(["m/z", "RT", t("qaqc.col.area"), t("qaqc.ratio.col"), t("qaqc.ratio.atleast", { thr: f.thr })], rows.slice(0, 200).map(x => [fmt(x.mz, 4), fmt(x.rt, 2), fmt(x.sample_area, 0), x.absent_in_blank ? t("qaqc.ratio.absent") : fmt(x.ratio, 1), t(x.at_least ? "qaqc.yes" : "qaqc.no")]))}</div></div>`;
  }

  // ---------------------------------------------------------------- panel
  function paint(box) {
    const hr = isHr(), fs = series();
    const roleRows = fs.map((f, k) => `<tr><td><span class="qq-n">${k + 1}</span> ${eh(f.label || f.file)}</td><td><select data-role="${eh(f.file)}" aria-label="${eh(t("qaqc.role.of", { f: f.label || f.file }))}">${ROLES.map(r => `<option value="${r}"${roleOf(f) === r ? " selected" : ""}>${eh(roleName(r))}</option>`).join("")}</select></td></tr>`).join("");
    const hasTable = hr && window.UNTUI && UNTUI.state && UNTUI.state.table;
    box.innerHTML = `<p class="sm qq-intro">${t(hr ? "qaqc.intro.hr" : "qaqc.intro.lr")}</p>
<details open><summary>${t("qaqc.types")}</summary><table class="qq-tab"><tbody>${roleRows}</tbody></table></details>
<details open><summary>${t("qaqc.ions")}</summary>
<form id="qq-add" class="qq-form"><label>${t("qaqc.in.mz")} <input id="qq-in" autocomplete="off" required></label><label>${t("qaqc.in.rt")} <input id="qq-rt" inputmode="decimal" size="5" required></label>
${hr ? `<label>${t("qaqc.in.kind")} <select id="qq-kind"><option value="is">${t("qaqc.kind.is")}</option><option value="known">${t("qaqc.kind.known")}</option></select></label>` : ""}
<label>${t("qaqc.in.win")} <input id="qq-win" inputmode="decimal" size="4" value="${st.win}"></label><button type="submit" id="qq-addb">${t("qaqc.add")}</button></form>
<div id="qq-msg" class="sm bad" role="alert"></div><div id="qq-res"></div></details>
<details open><summary>${t("qaqc.ratio")}</summary>
<form id="qq-rform" class="qq-form"><label>${t("qaqc.in.mz")} <input id="qq-rin" value="${eh(st.ratio.text)}" autocomplete="off" required></label><label>${t("qaqc.in.rt")} <input id="qq-rrt" value="${eh(st.ratio.rt)}" inputmode="decimal" size="5" required></label>
<label>${t("qaqc.thr")} <select id="qq-thr"><option value="3"${st.thr === 3 ? " selected" : ""}>3:1</option><option value="10"${st.thr === 10 ? " selected" : ""}>10:1</option></select></label>
<button type="submit">${t("qaqc.ratio.go")}</button>${hasTable ? `<button type="button" id="qq-feat">${t("qaqc.feat.go")}</button>` : ""}</form>
<div class="qq-form"><button type="button" id="qq-rep-html">${t("qaqc.rep.html")}</button><button type="button" id="qq-rep-json">${t("qaqc.rep.json")}</button></div>
<div id="qq-rres">${ratioBlock(res.ratio)}${featBlock(res.feat)}</div></details>`;
    paintIons(box);
  }
  function paintIons(box) {
    const el = box.querySelector("#qq-res");
    if (el) el.innerHTML = res.ions.map(ionBlock).join("") || `<p class="sm">${t("qaqc.none")}</p>`;
  }
  async function refresh(box) {
    const msg = box.querySelector("#qq-msg");
    try { await run(); if (msg) msg.textContent = ""; } catch (e) { if (msg) msg.textContent = String(e && e.message || e); }
    paintIons(box);
    if (window.WORKFLOW && WORKFLOW.logOp) WORKFLOW.logOp("qaqc_measure", { n: st.ions.length });
  }
  function mount(box) {
    paint(box);
    if (st.ions.length) refresh(box);
    box.onchange = e => {
      const r = e.target.closest("select[data-role]");
      if (r) { st.roles[r.dataset.role] = r.value; save(); refresh(box); }
      if (e.target.id === "qq-thr") { st.thr = +e.target.value; save(); if (res.ratio) runRatio().then(() => paint(box)).catch(() => {}); }
      if (e.target.id === "qq-win") { const v = numIt(e.target.value); if (v > 0) { st.win = v; save(); refresh(box); } }
    };
    box.onclick = e => {
      const b = e.target.closest("button[data-rm]");
      if (b) { st.ions = st.ions.filter(i => String(i.id) !== b.dataset.rm); res.ions = res.ions.filter(o => String(o.ion.id) !== b.dataset.rm); save(); paintIons(box); }
      if (e.target.id === "qq-rep-html" || e.target.id === "qq-rep-json") RAPPORTO.export(e.target.id === "qq-rep-json" ? "json" : "html").catch(err => { box.querySelector("#qq-msg").textContent = String(err && err.message || err); });
      if (e.target.id === "qq-feat") runFeatures().then(() => paint(box)).catch(err => { box.querySelector("#qq-msg").textContent = String(err && err.message || err); });
    };
    box.onsubmit = async e => {
      e.preventDefault();
      const msg = box.querySelector("#qq-msg");
      try {
        if (e.target.id === "qq-add") {
          await addIon(box.querySelector("#qq-in").value, box.querySelector("#qq-rt").value, "", (box.querySelector("#qq-kind") || {}).value);
          e.target.reset(); await refresh(box);
        } else {
          await runRatio(); paint(box); if (window.WORKFLOW && WORKFLOW.logOp) WORKFLOW.logOp("qaqc_ratio", { thr: st.thr });
        }
        msg.textContent = "";
      } catch (err) { msg.textContent = String(err && err.message || err); }
    };
  }

  // ---------------------------------------------------------------- workflow file and report
  const exportState = () => ({ ruoli: { ...st.roles }, ioni: st.ions.map(i => ({ ...i })), soglia: st.thr, finestra_rt: st.win, rapporto: { ...st.ratio } });
  function importState(s) {
    if (!s || typeof s !== "object") return;
    st = { ...blankState(), roles: { ...(s.ruoli || {}) }, ions: (s.ioni || []).map(i => ({ ...i })), thr: s.soglia === 10 ? 10 : 3, win: +s.finestra_rt > 0 ? +s.finestra_rt : 0.5, ratio: { text: "", rt: "", ...(s.rapporto || {}) } };
    st.seq = 1 + Math.max(0, ...st.ions.map(i => i.id || 0));
    res = { ions: [], ratio: null, feat: null, at: null };
    save();
  }

  return { mount, run, setRole, runRatio, runFeatures, addIon, exportState, importState, series, roleOf, roleName, guessRole, get state() { return st; }, get results() { return res; }, chart, ROLES };
})();
window.QAQC = QAQC;
