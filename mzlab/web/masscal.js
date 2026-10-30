// @ts-check
// Mass calibration check of a high-resolution file (TIC menu «Check calibration (contaminants)…», azione chrom.masscal in composizione.js): asks api/masscal for the
// mass error of the contaminant ions that stay in the MS1 scans (massacc/1, the Rust engine; the lab list contaminanti.json) and shows it as a table, two charts
// (error against m/z and against RT, each with its data table) and a summary line. Numbers only: the threshold colours the errors, it never gives a verdict. Loaded on first use.
const T = (/** @type {string} */ k, /** @type {Record<string, unknown>} */ p = {}) => I18N.t(k, p);
const KEY = () => (typeof NOMI !== "undefined" ? NOMI.KEY("masscal") : "mzlab.masscal");
const MIN_REFS = 3;                                       // as the step: fewer references are listed but no offset is given
const COLS = ["masscal.col.ion", "masscal.col.mz", "masscal.col.obs", "masscal.col.err", "masscal.col.scans", "masscal.col.int", "masscal.col.rt"];
const W = 380, H = 220, M = { l: 46, r: 12, t: 12, b: 34 };

/** @typedef {{ name: string, mz: number, rt: number, err_ppm: number, intensity: number, scans: number, outlier: boolean }} Point */
/** @typedef {{ polarity: number, n_refs: number, assessable: boolean, offset_ppm: number | null, spread_ppm: number | null, model: string | null, coef: Record<string, number> | null, points: Point[] }} FileReport */

/** the settings kept between visits: { thr, minInt } */
function saved() {
  try { const v = JSON.parse(localStorage.getItem(KEY()) || "{}"); return { thr: v.thr > 0 ? +v.thr : 3, minInt: v.minInt >= 0 ? +v.minInt : 1e4 }; } catch (_) { return { thr: 3, minInt: 1e4 }; }
}
/** @param {{ thr: number, minInt: number }} s */
function keep(s) { try { localStorage.setItem(KEY(), JSON.stringify(s)); } catch (_) { /* private window: the settings stay for this panel only */ } }

/** @param {number} v @param {number} d */
const fx = (v, d) => (Number.isFinite(v) ? v.toFixed(d) : "–");
/** a signed ppm @param {number} v */
const sg = v => (Number.isFinite(v) ? (v > 0 ? "+" : v < 0 ? "−" : "") + Math.abs(v).toFixed(2) : "–");

/**
 * The scatter of the error against `xs`, with the threshold lines; the points past the threshold are diamonds in the second colour (shape and colour).
 * @param {{ xs: number[], ys: number[], names: string[], thr: number, xl: string, title: string }} o
 */
function plot(o) {
  const ymax = Math.max(o.thr, ...o.ys.map(Math.abs)) * 1.15 || 1;
  const lo = Math.min(...o.xs), hi = Math.max(...o.xs), span = hi > lo ? hi - lo : 1;
  const px = (/** @type {number} */ x) => M.l + (hi > lo ? (x - lo) / span : 0.5) * (W - M.l - M.r);
  const py = (/** @type {number} */ y) => M.t + (1 - (y + ymax) / (2 * ymax)) * (H - M.t - M.b);
  const yt = [-ymax, -ymax / 2, 0, ymax / 2, ymax].map(v => `<text x="${M.l - 6}" y="${py(v) + 4}" text-anchor="end" class="mc-tk">${fx(v, 1)}</text>`).join("");
  const xt = [0, 1, 2, 3, 4].map(i => { const v = lo + (span * i) / 4; return `<text x="${px(v)}" y="${H - M.b + 16}" text-anchor="middle" class="mc-tk">${fx(v, span < 20 ? 2 : 0)}</text>`; }).join("");
  const dots = o.xs.map((x, i) => {
    const y = o.ys[i], out = Math.abs(y) > o.thr, cx = px(x), cy = py(y), tip = `<title>${EH(o.names[i])}: ${sg(y)} ppm</title>`;
    return out ? `<path d="M${cx} ${cy - 6}L${cx + 6} ${cy}L${cx} ${cy + 6}L${cx - 6} ${cy}Z" class="mc-out">${tip}</path>` : `<circle cx="${cx}" cy="${cy}" r="4.5" class="mc-in">${tip}</circle>`;
  }).join("");
  const line = (/** @type {number} */ v, /** @type {string} */ c) => `<line x1="${M.l}" x2="${W - M.r}" y1="${py(v)}" y2="${py(v)}" class="${c}"/>`;
  return `<svg viewBox="0 0 ${W} ${H}" width="${W}" height="${H}" class="mc-plot">${line(0, "mc-zero")}${line(o.thr, "mc-thr")}${line(-o.thr, "mc-thr")}
    <rect x="${M.l}" y="${M.t}" width="${W - M.l - M.r}" height="${H - M.t - M.b}" class="mc-box"/>${yt}${xt}${dots}
    <text x="${(W + M.l - M.r) / 2}" y="${H - 4}" text-anchor="middle" class="mc-ax">${EH(o.xl)}</text>
    <text transform="translate(12 ${(H - M.b + M.t) / 2}) rotate(-90)" text-anchor="middle" class="mc-ax">${EH(T("masscal.ax.err"))}</text></svg>`;
}

/** @param {FileReport} f @param {string} file */
function csv(f, file) {
  const q = (/** @type {string} */ s) => '"' + s.replace(/"/g, '""') + '"';
  const head = ["file", "ion", "theoretical_mz", "observed_mz", "error_ppm", "scans", "intensity", "rt_min"].join(",");
  return [head, ...f.points.map(p => [q(file), q(p.name), p.mz.toFixed(6), (p.mz * (1 + p.err_ppm * 1e-6)).toFixed(6), p.err_ppm.toFixed(3), p.scans, p.intensity, p.rt.toFixed(3)].join(","))].join("\n") + "\n";
}

/** @param {number} k index of the file in E.files */
export async function open(k) {
  const file = E.files[k];
  if (!file) return;
  const opener = /** @type {HTMLElement | null} */ (document.activeElement);
  const old = document.getElementById("masscal"); if (old) old.remove();
  const set = saved();
  const d = document.createElement("dialog"); d.id = "masscal"; d.className = "tdati masscal"; d.setAttribute("aria-labelledby", "masscal-h");
  d.innerHTML = `<div class="td-hd"><h3 id="masscal-h">${EH(T("masscal.title", { file: file.label }))}</h3><button type="button" class="bt" data-mc="x" aria-label="${EH(T("masscal.close"))}" title="${EH(T("masscal.close"))}">&times;</button></div>
    <p class="sm muted">${EH(T("masscal.intro"))}</p>
    <div class="mc-ctl"><label>${T("masscal.thr", { input: `<input type="number" id="mc-thr" min="0.1" step="0.5" value="${set.thr}" aria-label="${EH(T("masscal.thr", { input: "" }))}">` })}</label>
      <label>${T("masscal.minint", { input: `<input type="number" id="mc-int" min="0" step="1000" value="${set.minInt}" aria-label="${EH(T("masscal.minint", { input: "" }))}">` })}</label>
      <button type="button" class="go" data-mc="run">${EH(T("masscal.run"))}</button></div>
    <p id="mc-sum" class="mc-sum" role="status" aria-live="polite"></p><p id="mc-trend" class="sm muted"></p>
    <div id="mc-out"></div>
    <p class="sm muted">${EH(T("masscal.limits"))}</p>
    <div class="td-ft"><button type="button" class="go" data-mc="csv" disabled>${EH(T("masscal.csv"))}</button><button type="button" class="go" data-mc="x">${EH(T("masscal.close"))}</button></div>`;
  document.body.appendChild(d);
  const $ = (/** @type {string} */ s) => /** @type {HTMLElement} */ (d.querySelector(s));
  const close = () => { d.close(); d.remove(); if (opener && document.contains(opener)) opener.focus(); };
  d.querySelectorAll('[data-mc="x"]').forEach(b => { /** @type {HTMLElement} */ (b).onclick = close; });
  d.addEventListener("cancel", e => { e.preventDefault(); close(); });
  /** @type {FileReport | null} */ let rep = null;
  const thrNow = () => { const v = +/** @type {HTMLInputElement} */ ($("#mc-thr")).value; return v > 0 ? v : 3; };

  function draw() {
    const out = $("#mc-out"), sum = $("#mc-sum"), trend = $("#mc-trend"), btn = /** @type {HTMLButtonElement} */ ($('[data-mc="csv"]'));
    btn.disabled = !rep || !rep.points.length;
    trend.textContent = "";
    if (!rep || !rep.points.length) { out.innerHTML = ""; sum.textContent = T("masscal.none"); return; }
    const thr = thrNow(), pts = [...rep.points].sort((a, b) => a.mz - b.mz), n = pts.length;
    sum.textContent = rep.assessable && rep.offset_ppm != null ? T("masscal.sum", { n, off: sg(rep.offset_ppm), spread: fx(rep.spread_ppm ?? NaN, 2) }) : T("masscal.sum.few", { n, min: MIN_REFS });
    const c = rep.coef;
    if (c && "slope_ppm_per_100_mz" in c) trend.textContent = "slope_ppm_per_min" in c
      ? T("masscal.trend", { mz: sg(c.slope_ppm_per_100_mz), rt: sg(c.slope_ppm_per_min), model: rep.model })
      : T("masscal.trend.mz", { mz: sg(c.slope_ppm_per_100_mz), model: rep.model });
    const rows = pts.map(p => `<tr><th scope="row">${EH(p.name)}</th><td>${fx(p.mz, 5)}</td><td>${fx(p.mz * (1 + p.err_ppm * 1e-6), 5)}</td>
      <td class="${Math.abs(p.err_ppm) > thr ? "mc-hi" : ""}">${sg(p.err_ppm)}${Math.abs(p.err_ppm) > thr ? ` <span aria-hidden="true">◆</span><span class="sr-only">${EH(T("masscal.beyond"))}</span>` : ""}</td>
      <td>${p.scans}</td><td>${p.intensity.toExponential(2)}</td><td>${fx(p.rt, 2)}</td></tr>`).join("");
    const errs = pts.map(p => p.err_ppm), names = pts.map(p => p.name);
    const plots = [["masscal.plot.mz", pts.map(p => p.mz), "m/z"], ["masscal.plot.rt", pts.map(p => p.rt), T("masscal.col.rt")]].map(([key, xs, xl]) => ({ key: String(key), xs: /** @type {number[]} */ (xs), xl: String(xl) }));
    out.innerHTML = `<div class="mc-plots">${plots.map((p, i) => `<figure><figcaption>${EH(T(p.key))}</figcaption><div data-plot="${i}">${plot({ xs: p.xs, ys: errs, names, thr, xl: p.xl, title: T(p.key) })}</div></figure>`).join("")}</div>
      <div class="td-sc" tabindex="0" role="region" aria-label="${EH(T("masscal.title", { file: file.label }))}"><table><caption class="sr-only">${EH(T("masscal.title", { file: file.label }))}</caption>
      <thead><tr>${COLS.map(c => `<th scope="col">${EH(T(c))}</th>`).join("")}</tr></thead><tbody>${rows}</tbody></table></div>`;
    plots.forEach((p, i) => {
      const svg = out.querySelector(`[data-plot="${i}"] svg`);
      if (!svg || !window.TDATI) return;
      svg.setAttribute("aria-label", T(p.key));
      TDATI.describe(/** @type {any} */ (svg), T("masscal.plot.desc", { title: T(p.key), n, a: sg(Math.min(...errs)), b: sg(Math.max(...errs)) }));
      TDATI.addButton(/** @type {any} */ (svg), () => ({ title: T(p.key), head: [T("masscal.col.ion"), p.xl, T("masscal.col.err")], rowHeader: true, rows: pts.map((q, j) => [q.name, TDATI.num(p.xs[j], 4), TDATI.num(q.err_ppm, 3)]) }));
    });
  }

  async function run() {
    const s = { thr: thrNow(), minInt: Math.max(0, +/** @type {HTMLInputElement} */ ($("#mc-int")).value || 0) };
    keep(s);
    $("#mc-sum").textContent = T("masscal.busy"); $("#mc-out").innerHTML = ""; rep = null;
    try {
      const r = await fetch(`api/masscal?k=${k}&min_intensity=${s.minInt}`), j = await r.json();
      if (!r.ok) throw I18N.err(j);
      rep = j.files && j.files[0];
    } catch (e_) { const e = /** @type {Error} */ (e_); $("#mc-sum").innerHTML = `<span class="fail">${EH(e.message)}</span>`; $('[data-mc="csv"]').setAttribute("disabled", ""); return; }
    draw();
  }
  $('[data-mc="run"]').onclick = run;
  $("#mc-thr").addEventListener("change", () => { keep({ thr: thrNow(), minInt: saved().minInt }); draw(); });
  $('[data-mc="csv"]').onclick = () => { if (rep) dl(`mass-calibration-${String(file.label).replace(/[^\w.-]+/g, "_")}.csv`, csv(rep, String(file.label)), "text/csv"); };
  d.showModal();
  $('[data-mc="x"]').focus();
  await run();
}
