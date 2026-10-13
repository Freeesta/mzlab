// @ts-check
// PFAS evidence for the feature table (high resolution only; «Statistica» panel, after the Kendrick mass defect): the candidate formula of every ion
// (/api/composition, the Rust engine), MD/C against m/C, and the diagnostic fragments and neutral losses of the MS2 scans that selected it. A VIEW of
// CANDIDATES with their evidence (counts and ppm errors): the program never says what an ion is. A click on a point or a row opens the XIC and the MS2 scan.
// Module loaded after the classic scripts (carica.js); the arithmetic is in pfas-calc.js.
import { carbons, mdc, pickFormula, search } from "./pfas-calc.js";

const w = /** @type {any} */ (window);
const KEY = "qqq.pfas";
const MAX_FEATURES = 300, MAX_ROWS = 200, POOL = 4;
const t = (/** @type {string} */ k, /** @type {any} */ p) => I18N.t(k, p);
const eh = (/** @type {any} */ s) => EH(s == null ? "" : s);
const fmt = (/** @type {any} */ v, d = 3) => (v == null || !isFinite(v) ? "–" : v.toFixed(d));
const ELEMENTS = "C:1-30,H:0-10,F:3-40,O:0-6,N:0-1,S:0-1";
const st = Object.assign({ ion: "[M-H]-", tol: 3, ms2tol: 5, minRel: 1 }, lsObj(KEY));
const save = () => { try { localStorage.setItem(KEY, JSON.stringify(st)); } catch (_) { /* no storage */ } };

/** @typedef {{i: number, id: string, mz: number, rt: number|null, formula: string|null, ppm: number|null, nCand: number, c: number, mc: number|null, mdc: number|null, series: number|null, ms2: any}} Row */
/** @type {{rows: Row[], pol: number, nMs2: number}|null} */
let res = null;
let selRow = /** @type {number|null} */ (null), curBox = /** @type {HTMLElement|null} */ (null), seq = 0;

// ------------------------------------------------------------------ computation
const ms2Files = () => E.files.filter((/** @type {any} */ f) => f.kind === ESP.ids[1] && !f.gone && HR.isHr(f, 2));

/** best MS2 scan (most fragments and losses) of the files that selected the ion near its retention time */
async function ms2Evidence(/** @type {Row} */ r, pol, files) {
  let best = null;
  for (const f of files) {
    const D = await DDA.get(f.k);
    const cand = [];
    for (let j = 0; j < D.sid.length; j++) {
      const c = D.tgt[j] != null ? D.tgt[j] : D.prec[j];
      if (c == null) continue;
      const hit = D.lo[j] != null && D.hi[j] != null ? r.mz >= D.lo[j] && r.mz <= D.hi[j] : Math.abs(c - r.mz) <= 0.02;
      if (hit && (r.rt == null || Math.abs(D.rt[j] - r.rt) <= 0.5)) cand.push([r.rt == null ? 0 : Math.abs(D.rt[j] - r.rt), j]);
    }
    cand.sort((a, b) => a[0] - b[0]);
    for (const [, j] of cand.slice(0, 3)) {
      const sp = await memo(`pfs|${f.k}|${D.sid[j]}`, () => J(`api/scan?k=${f.k}&sid=${D.sid[j]}` + HR.q()));
      const ev = search(sp, r.mz, pol, { tolPpm: st.ms2tol, minRel: st.minRel });
      if (!best || ev.n > best.ev.n) best = { k: f.k, sid: D.sid[j], rt: D.rt[j], prec: sp.prec != null ? sp.prec : r.mz, ev };
    }
  }
  return best;
}
async function compute(/** @type {HTMLElement} */ box) {
  const tb = TABELLA.table, my = ++seq;
  if (!tb) return;
  const pol = /-$/.test(st.ion) ? -1 : 1, feats = tb.features.slice(0, MAX_FEATURES), kmd = w.KMD && w.KMD.result;
  /** @type {Row[]} */
  const rows = feats.map((/** @type {any} */ f, /** @type {number} */ i) => {
    const k = kmd && kmd.rows.find((/** @type {any} */ x) => x.i === i);
    return { i, id: f.id, mz: +f.mz, rt: f.rt == null ? null : +f.rt, formula: null, ppm: null, nCand: 0, c: 0, mc: null, mdc: null, series: k ? k.series : null, ms2: null };
  });
  const files = ms2Files(), msg = /** @type {HTMLElement|null} */ (box.querySelector("#pfas-msg"));
  let done = 0, next = 0;
  const work = async () => {
    while (next < rows.length && my === seq) {
      const r = rows[next++];
      try {
        const q = new URLSearchParams({ mz: String(r.mz), ion: st.ion, tol: String(st.tol), unit: "ppm", max: "50", elements: ELEMENTS });
        const j = await J("api/composition?" + q.toString()), p = pickFormula(j.results);
        if (p) { r.formula = p.formula; r.ppm = p.ppm; r.nCand = p.n; r.c = carbons(p.formula); const m = mdc(r.mz, r.c); if (m) { r.mc = m.mc; r.mdc = m.mdc; } }
        if (files.length) r.ms2 = await ms2Evidence(r, pol, files);
      } catch (e) { if (msg) msg.textContent = String(e && /** @type {any} */ (e).message || e); }
      if (++done % 10 === 0 && msg && my === seq) msg.textContent = t("pfas.progress", { n: done, m: rows.length });
    }
  };
  if (msg) msg.textContent = t("pfas.progress", { n: 0, m: rows.length });
  await Promise.all(Array.from({ length: POOL }, work));
  if (my !== seq) return;
  if (msg) msg.textContent = "";
  res = { rows, pol, nMs2: files.length };
  selRow = null;
  if (w.WORKFLOW && WORKFLOW.logOp) WORKFLOW.logOp("stat_pfas", { ion: st.ion, tol: st.tol, ms2tol: st.ms2tol, n: rows.length, withMs2: rows.filter(r => r.ms2).length });
  paintRes(box);
}

// ------------------------------------------------------------------ the plot: m/C (x) against MD/C (y)
const W = 320, H = 250, L = 52, R = 10, T = 10, B = 34;
function plotSvg(/** @type {NonNullable<typeof res>} */ rs) {
  const pts = rs.rows.filter(r => r.mc != null);
  if (!pts.length) return "";
  const xs = pts.map(r => /** @type {number} */ (r.mc)), ys = pts.map(r => /** @type {number} */ (r.mdc));
  let x0 = Math.min(...xs), x1 = Math.max(...xs), y0 = Math.min(...ys), y1 = Math.max(...ys);
  const px = (x1 - x0 || 1) * 0.06, py = (y1 - y0 || 0.01) * 0.12;
  x0 -= px; x1 += px; y0 -= py; y1 += py;
  const X = (/** @type {number} */ v) => L + (v - x0) / (x1 - x0) * (W - L - R), Y = (/** @type {number} */ v) => T + (1 - (v - y0) / (y1 - y0)) * (H - T - B);
  let s = `<svg class="pca-plot" id="pfas-plot" viewBox="0 0 ${W} ${H}" role="img" aria-label="${eh(t("pfas.plot.aria", { n: pts.length }))}">`
    + `<line x1="${L}" y1="${H - B}" x2="${W - R}" y2="${H - B}" stroke="var(--line)"/><line x1="${L}" y1="${T}" x2="${L}" y2="${H - B}" stroke="var(--line)"/>`
    + `<text x="${(L + W - R) / 2}" y="${H - 6}" text-anchor="middle" font-size="9" fill="var(--ink)">${eh(t("pfas.axis.mc"))}</text>`
    + `<text x="9" y="${(T + H - B) / 2}" text-anchor="middle" font-size="9" fill="var(--ink)" transform="rotate(-90 9 ${(T + H - B) / 2})">${eh(t("pfas.axis.mdc"))}</text>`
    + `<text x="${L}" y="${H - B + 11}" font-size="8" fill="var(--muted)">${eh(fmt(x0, 1))}</text><text x="${W - R}" y="${H - B + 11}" text-anchor="end" font-size="8" fill="var(--muted)">${eh(fmt(x1, 1))}</text>`
    + `<text x="${L - 3}" y="${H - B}" text-anchor="end" font-size="8" fill="var(--muted)">${eh(fmt(y0, 4))}</text><text x="${L - 3}" y="${T + 6}" text-anchor="end" font-size="8" fill="var(--muted)">${eh(fmt(y1, 4))}</text>`;
  pts.forEach(r => {
    const x = X(/** @type {number} */ (r.mc)), y = Y(/** @type {number} */ (r.mdc)), on = selRow === r.i, ev = r.ms2 ? r.ms2.ev.n : 0, ser = r.series;
    const col = ser ? PAL[(ser - 1) % PAL.length] || "var(--accent)" : "var(--muted)", rr = on ? 5.6 : 3.8;
    const mark = ev > 0 ? `<rect x="${(x - rr).toFixed(1)}" y="${(y - rr).toFixed(1)}" width="${2 * rr}" height="${2 * rr}"` : `<circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="${rr}"`;
    s += `<g data-i="${r.i}" tabindex="0" role="button" aria-label="${eh(t("pfas.row.aria", { mz: fmt(r.mz, 4), rt: fmt(r.rt, 2) }))}">`
      + `<circle cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="12" fill="transparent"/>`
      + `${mark} fill="${col}" fill-opacity="${ser ? 1 : 0.55}" stroke="var(--ink)" stroke-width="${on ? 1.6 : 0.5}"/><title>${eh(t("pfas.point.tip", { mz: fmt(r.mz, 4), f: r.formula || "–", mdc: fmt(r.mdc, 4), n: ev }))}</title></g>`;
  });
  return s + "</svg>";
}
function plotTable() {
  if (!res) return null;
  return { head: ["ID", "m/z", "RT", t("pfas.col.formula"), "m/C", "MD/C", t("pfas.col.evidence")], rowHeader: true,
    rows: res.rows.filter(r => r.mc != null).map(r => [r.id, fmt(r.mz, 4), fmt(r.rt, 2), r.formula || "", fmt(r.mc, 3), fmt(r.mdc, 5), r.ms2 ? r.ms2.ev.n : 0]) };
}

// ------------------------------------------------------------------ tables
const evText = (/** @type {any} */ ev) => ev.frags.map((/** @type {any} */ f) => `${f.label} ${fmt(f.obs, 4)} (${fmt(f.ppm, 1)} ppm)`).join("; ") || "–";
const lossText = (/** @type {any} */ ev) => ev.losses.map((/** @type {any} */ l) => `${l.label} ${fmt(l.dm, 4)} (${fmt(l.ppm, 1)} ppm)`).join("; ") || "–";
function formulaTable(/** @type {NonNullable<typeof res>} */ rs) {
  const list = rs.rows.slice().sort((a, b) => (b.mc != null ? 1 : 0) - (a.mc != null ? 1 : 0) || (a.mdc ?? 0) - (b.mdc ?? 0)), shown = list.slice(0, MAX_ROWS);
  return `<div class="qq-scroll"><table class="qq-tab" id="pfas-tab"><thead><tr><th scope="col">ID</th><th scope="col">m/z</th><th scope="col">RT</th><th scope="col">${t("pfas.col.formula")}</th>`
    + `<th scope="col">ppm</th><th scope="col">${t("pfas.col.ncand")}</th><th scope="col">m/C</th><th scope="col">MD/C</th><th scope="col">${t("kmd.col.series")}</th></tr></thead><tbody>`
    + shown.map(r => `<tr data-i="${r.i}" tabindex="0" role="button" aria-label="${eh(t("pfas.row.aria", { mz: fmt(r.mz, 4), rt: fmt(r.rt, 2) }))}"${selRow === r.i ? ' aria-current="true" class="sel"' : ""}>`
      + `<td class="num">${eh(r.id)}</td><td class="num">${fmt(r.mz, 4)}</td><td class="num">${fmt(r.rt, 2)}</td><td>${eh(r.formula || "–")}</td><td class="num">${fmt(r.ppm, 1)}</td>`
      + `<td class="num">${r.nCand || "–"}</td><td class="num">${fmt(r.mc, 3)}</td><td class="num">${fmt(r.mdc, 5)}</td><td class="num">${r.series || "–"}</td></tr>`).join("")
    + `</tbody></table></div>` + (list.length > shown.length ? `<p class="sm">${eh(t("kmd.more", { n: shown.length, m: list.length }))}</p>` : "");
}
function evidenceTable(/** @type {NonNullable<typeof res>} */ rs) {
  const list = rs.rows.filter(r => r.ms2).sort((a, b) => b.ms2.ev.n - a.ms2.ev.n || a.mz - b.mz).slice(0, MAX_ROWS);
  if (!rs.nMs2) return `<p class="sm" role="status">${eh(t("pfas.noms2file"))}</p>`;
  if (!list.length) return `<p class="sm" role="status">${eh(t("pfas.noms2"))}</p>`;
  return `<div class="qq-scroll"><table class="qq-tab" id="pfas-ev"><thead><tr><th scope="col">ID</th><th scope="col">m/z</th><th scope="col">RT</th><th scope="col">${t("pfas.col.frags")}</th>`
    + `<th scope="col">${t("pfas.col.losses")}</th><th scope="col">${t("pfas.col.cf2")}</th><th scope="col">${t("pfas.col.evidence")}</th></tr></thead><tbody>`
    + list.map(r => `<tr data-i="${r.i}" tabindex="0" role="button" aria-label="${eh(t("pfas.row.aria", { mz: fmt(r.mz, 4), rt: fmt(r.rt, 2) }))}"${selRow === r.i ? ' aria-current="true" class="sel"' : ""}>`
      + `<td class="num">${eh(r.id)}</td><td class="num">${fmt(r.mz, 4)}</td><td class="num">${fmt(r.rt, 2)}</td><td>${eh(evText(r.ms2.ev))}</td><td>${eh(lossText(r.ms2.ev))}</td>`
      + `<td class="num">${r.ms2.ev.cf2}</td><td class="num">${r.ms2.ev.n}</td></tr>`).join("") + `</tbody></table></div>`;
}

function paintRes(/** @type {HTMLElement} */ box) {
  const el = box.querySelector("#pfas-res");
  if (!el) return;
  if (!res) { el.innerHTML = ""; return; }
  const withF = res.rows.filter(r => r.formula).length, withEv = res.rows.filter(r => r.ms2 && r.ms2.ev.n > 0).length;
  el.innerHTML = `<p class="sm" role="status">${eh(t("pfas.sum", { n: res.rows.length, f: withF, e: withEv }))}</p>${plotSvg(res)}<p class="sm">${t("pfas.plot.note")}</p>`
    + `<h4 class="sm" style="margin:8px 0 2px">${t("pfas.formulas.title")}</h4>${formulaTable(res)}<p class="sm">${t("pfas.formulas.note")}</p>`
    + `<h4 class="sm" style="margin:8px 0 2px">${t("pfas.ev.title")}</h4>${evidenceTable(res)}<p class="sm">${t("pfas.ev.note")}</p>`
    + `<div class="qq-form"><button type="button" id="pfas-xlsx">Excel</button></div>`;
}

// ------------------------------------------------------------------ links to the XIC and the MS2 spectrum
function openIon(/** @type {Row} */ r) {
  if (!w.HR || typeof addPanel !== "function") return;
  const ht = HR.ionTrace(r.mz, { obs: false });
  if (ht) {
    const ex = E.panels.find((/** @type {any} */ p) => p.pfasIon);
    if (ex) { ex.traces = [ht]; if (typeof ctl === "function") ctl(ex); draw(ex); reveal(ex); }
    else { const np = addPanel("xic", { traces: [ht] }); np.pfasIon = true; reveal(np); }
  }
  if (r.ms2) {
    const o = { k: r.ms2.k, level: 2, prec: r.ms2.prec, r0: r.ms2.rt - 1e-4, r1: r.ms2.rt + 1e-4, title: "MS2 " + fmt(r.mz, 4) };
    const ex = E.panels.find((/** @type {any} */ p) => p.pfasMs2);
    if (ex) { Object.assign(ex, o); if (typeof ctl === "function") ctl(ex); draw(ex); reveal(ex); }
    else { const np = addPanel("spec", o); np.pfasMs2 = true; reveal(np); }
  }
}
function selectRow(/** @type {number} */ i, /** @type {HTMLElement} */ box) {
  selRow = i;
  const r = res && res.rows.find(x => x.i === i);
  if (!r) return;
  paintRes(box);
  openIon(r);
  if (w.PROFILO && r.rt != null) PROFILO.show(r.mz, r.rt);
}

// ------------------------------------------------------------------ section
function paint(/** @type {HTMLElement} */ box) {
  const tb = TABELLA.table;
  box.innerHTML = `<p class="sm">${t("pfas.intro")}</p>`
    + `<form id="pfas-form" class="qq-form"><label>${t("pfas.ion")} <select id="pfas-ion">${["[M-H]-", "[M+H]+"].map(a => `<option value="${a}"${a === st.ion ? " selected" : ""}>${a}</option>`).join("")}</select></label>`
    + `<label>${t("pfas.tol")} <input id="pfas-tol" value="${st.tol}" inputmode="decimal" size="3"></label><label>${t("pfas.ms2tol")} <input id="pfas-ms2tol" value="${st.ms2tol}" inputmode="decimal" size="3"></label>`
    + `<label>${t("pfas.minrel")} <input id="pfas-minrel" value="${st.minRel}" inputmode="decimal" size="3"></label><button type="submit" id="pfas-go">${t("kmd.go")}</button></form>`
    + `${tb ? "" : `<p class="sm">${t("kmd.needtable")}</p>`}<div id="pfas-msg" class="sm" role="status"></div><div id="pfas-res"></div>`;
  paintRes(box);
}
function xlsx() {
  if (!res) return;
  dlx("pfas.xlsx", [{ name: "Features", head: ["ID", "m/z", "RT", "Formula", "ppm", "N", "C", "m/C", "MD/C", t("kmd.col.series")], rows: res.rows.map(r => [r.id, r.mz, r.rt, r.formula || "", r.ppm == null ? "" : r.ppm, r.nCand, r.c, r.mc == null ? "" : r.mc, r.mdc == null ? "" : r.mdc, r.series || ""]), widths: [10, 12, 8, 16, 8, 6, 6, 10, 10, 8] },
    { name: "MS2", head: ["ID", "m/z", "RT", t("pfas.col.frags"), t("pfas.col.losses"), t("pfas.col.cf2"), t("pfas.col.evidence")], rows: res.rows.filter(r => r.ms2).map(r => [r.id, r.mz, r.rt, evText(r.ms2.ev), lossText(r.ms2.ev), r.ms2.ev.cf2, r.ms2.ev.n]), widths: [10, 12, 8, 40, 30, 8, 10] },
    { name: "Parameters", head: [t("kmd.param"), t("kmd.value")], rows: [[t("pfas.ion"), st.ion], [t("pfas.tol"), st.tol], [t("pfas.ms2tol"), st.ms2tol], [t("pfas.minrel"), st.minRel], ["Elements", ELEMENTS]], widths: [30, 40] }]);
}
function mount(/** @type {HTMLElement} */ box) {
  curBox = box;
  paint(box);
  box.onsubmit = e => {
    e.preventDefault();
    const num = (/** @type {string} */ id, /** @type {number} */ d) => { const v = numIt(/** @type {HTMLInputElement} */ (box.querySelector(id)).value); return v > 0 ? v : d; };
    st.ion = /** @type {HTMLSelectElement} */ (box.querySelector("#pfas-ion")).value; st.tol = num("#pfas-tol", 3); st.ms2tol = num("#pfas-ms2tol", 5); st.minRel = num("#pfas-minrel", 1); save();
    if (!TABELLA.table) return;
    compute(box);
  };
  box.onclick = e => {
    const tg = /** @type {HTMLElement} */ (e.target);
    if (tg.id === "pfas-xlsx") { xlsx(); return; }
    const el = /** @type {HTMLElement|null} */ (tg.closest("#pfas-tab tbody tr, #pfas-ev tbody tr, #pfas-plot g[data-i]"));
    if (el && res) selectRow(+(el.dataset.i || 0), box);
  };
  box.onkeydown = e => { const el = /** @type {HTMLElement|null} */ (/** @type {HTMLElement} */ (e.target).closest && /** @type {HTMLElement} */ (e.target).closest("#pfas-tab tbody tr, #pfas-ev tbody tr, #pfas-plot g[data-i]")); if (el && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); el.dispatchEvent(new MouseEvent("click", { bubbles: true })); } };
}
document.addEventListener("tabella", () => { seq++; res = null; selRow = null; if (curBox && curBox.isConnected) paint(curBox); });
if (typeof STAT !== "undefined") STAT.add({ id: "pfas", title: "pfas.title", hr: true, visible: () => true, mount });
TDATI.watch("svg#pfas-plot", (/** @type {Element} */ _el, /** @type {string} */ title) => { const d = plotTable(); return d ? { title, ...d } : null; });
w.PFAS = { mount, plotTable, selectRow, get result() { return res; }, get params() { return st; } };
