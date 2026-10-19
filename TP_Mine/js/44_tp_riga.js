// @ts-check
// TPMINE-PRIVATE  TPRIGA: what the open row of a Tabui table shows. One head (prediction, the summary of the row, actions) and four tabs (Overview, Samples, Identity, Quality), drawn from a
// row MODEL (`TpRiga`) that every table builds from its own engine. This file shows what the model holds and computes no number; a part that the model lacks is not drawn (one sentence says why).
// Charts: SVG with title, the description as help (TPHELP) and the table of the data on the T key (TDATI, no visible button); beside each chart one button opens the same ion in the Data view
// (`TPRIGA.toData`). Identity: one line per candidate (neutral and ion formula, Δ, the proofs as ✓/✗ with their help), the rest in a closed «Details». Colours only from CSS variables
// (time = viridis, areas = cividis, groups of the blanks grey); no animation. Texts in English; the helps are keys of the catalogue of Tabui (`ft.tab.*`, `ft.decided`, `ft.q.flags`) and of this file (`ft.riga.*`, `ft.pf.*`).
(() => {
  "use strict";
  const G = /** @type {any} */ (window);

  /**
   * @typedef {{ file: string, group?: string, time?: number | null, rt: number[], y: number[] }} TpXic
   * @typedef {{ kind: "ms1" | "ms2" | "mirror", mz: number[], y: number[], ref?: { mz: number[], y: number[] }, iso?: { obs?: { mz: number[], y: number[] }, exp?: { mz: number[], y: number[] } }, prec?: number, label?: string, tip?: string }} TpSpec
   * @typedef {{ file: string, type?: string, time?: number | null, area?: number | null, norm?: number | null, state?: string, apexRt?: number | null, lo?: number | null, hi?: number | null }} TpSample
   * @typedef {{ key?: string, ok: boolean | null, text: string }} TpProof  key: persist, t0, grow, iso, ms2 (a column of the candidates); any other proof goes to «Details». ok null = not tested
   * @typedef {{ name?: string, formula?: string, ion?: string | null, ppm?: number | null, dmz?: number | null, adduct?: string, source?: string, level?: string | number, pred?: number, proofs?: TpProof[] }} TpCand
   * @typedef {{ key: string, label: string, help?: string, run: (m: TpRiga) => void }} TpAction
   * @typedef {{ label: string, value: string, help?: string }} TpSum
   * @typedef {{ title?: string, level?: string | number, pred?: number, decided?: string | number | null, summary?: TpSum[], words?: string, actions?: TpAction[], xic?: TpXic[], xicTip?: string, apexRt?: number | null, lo?: number | null, hi?: number | null,
   *   composition?: { run: () => Promise<any> }, spectrum?: TpSpec | null, spectrum2?: TpSpec | null, open?: { xic?: () => void, ms2?: () => void }, samples?: TpSample[], identity?: { neutral?: number | string | null, adduct?: string, candidates?: TpCand[] } | null,
   *   quality?: { shape?: number | string | null, snr?: number | null, fwhm?: number | null, flags?: string[], isoM1?: number | string | null, blank?: any, t0?: any, rejected?: string[] } | null,
   *   decide?: { series: any, key: string, level?: string | number | null, name?: string }, missing?: Record<string, string> }} TpRiga
   */

  const CSS = `
.tp{--tpr-v0:#440154;--tpr-v1:#3b528b;--tpr-v2:#21918c;--tpr-v3:#5ec962;--tpr-v4:#addc30;--tpr-g0:#e6f4ea;--tpr-g1:#a8d5b5;--tpr-g2:#5fb27a;--tpr-g3:#2a8050;--tpr-g4:#0b4a28;--tpr-g-none:#fff;--tpr-w:#fff;--tpr-k:#111}
@media(prefers-color-scheme:dark){:root:not([data-theme=light]) .tp{--tpr-v0:#38bdf8;--tpr-v1:#14b8a6;--tpr-v2:#10b981;--tpr-v3:#84cc16;--tpr-v4:#fde047}}
:root[data-theme=dark] .tp{--tpr-v0:#38bdf8;--tpr-v1:#14b8a6;--tpr-v2:#10b981;--tpr-v3:#84cc16;--tpr-v4:#fde047}
.tp .tpr-ov{display:grid;grid-template-columns:repeat(auto-fill,minmax(300px,1fr));gap:6px 14px;align-items:start}
.tp .tpr-ch{min-width:0}.tp .tpr-ch h4{margin:0 0 2px;font-size:12px;color:var(--muted);font-weight:600}.tp .tpr-ch svg{display:block;width:100%;height:auto;max-height:170px}
.tp .tpr-ch svg text{fill:var(--muted);font-size:9px}.tp .tpr-ch svg .ax{stroke:var(--muted);fill:none;stroke-width:1}
.tp .tpr-ch svg .tr{fill:none;stroke-width:1.4}.tp .tpr-ch svg .tr-blank{stroke:var(--muted);stroke-dasharray:4 3}.tp .tpr-ch svg .tr-t0{stroke:var(--ink)}
.tp .tpr-ch svg .band{fill:var(--sel);stroke:var(--line)}.tp .tpr-ch svg .apex{stroke:var(--accent);stroke-width:1;stroke-dasharray:2 2}
.tp .tpr-ch svg .st{stroke:var(--ink);stroke-width:1.6}.tp .tpr-ch svg .st-ref{stroke:var(--muted)}.tp .tpr-ch svg .iso-o{fill:var(--accent)}.tp .tpr-ch svg .iso-e{fill:var(--panel);stroke:var(--ink);stroke-width:1}
.tp .tpr-ch svg .zero{stroke:var(--line);stroke-width:1}.tp .tpr-ch svg .st-prec{stroke:var(--accent);stroke-width:2.4;stroke-dasharray:4 2}
.tp .tpr-chh{display:flex;flex-wrap:wrap;align-items:center;justify-content:space-between;gap:2px 8px;margin:0 0 2px;min-height:24px}.tp .tpr-chh h4{margin:0}
.tp .tpr-leg{display:flex;flex-wrap:wrap;gap:0 12px;font-size:11px;color:var(--muted);margin:0}.tp .tpr-leg span{display:inline-flex;align-items:center;gap:4px;min-height:18px}
.tp .tpr-leg i{display:inline-block;width:16px;height:0;border-top:3px solid currentColor}.tp .tpr-leg i.d{border-top-style:dashed}
.tp .tpr-none{margin:4px 0;font-size:12px;color:var(--muted)}
.tp .tpr-head{display:inline-flex;flex-wrap:wrap;gap:4px 10px;align-items:center}.tp .tpr-words{flex:1 1 20em;font-size:12px}
.tp .tpr-sums{display:inline-flex;flex-wrap:wrap;gap:2px 14px;align-items:center;flex:1 1 16em;min-width:0;font-size:12px}.tp .tpr-sum{display:inline-flex;gap:4px;align-items:baseline;min-height:24px;align-items:center}.tp .tpr-sum .k{color:var(--muted)}
.tp .tpr-act{box-sizing:border-box;cursor:pointer;min-height:24px;min-width:24px;padding:0 10px;font:inherit;font-size:12px;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink)}
.tp .tpr-act:hover{background:var(--sel)}.tp .tpr-act:focus-visible,.tp .tpr-more:focus-visible,.tp .tpr-ch svg:focus-visible,.tp .tpr-cell:focus-visible,.tp .tpr-grp b:focus-visible,.tp .tpr-mxleg [tabindex]:focus-visible,.tp .tpr-mx:focus-visible,.tp .tpr-sum:focus-visible,.tp .tpr-pk:focus-visible,.tp .tpr-tw th span:focus-visible,.tp details.tpr-det>summary:focus-visible{outline:2px solid var(--accent);outline-offset:1px}
@media(pointer:coarse){.tp .tpr-act,.tp .tpr-more,.tp details.tpr-det>summary,.tp .tpr-chh{min-height:40px}}
.tp .tpr-dec{font-size:12px;color:var(--muted)}
.tp .tpr-mx{display:flex;flex-wrap:wrap;gap:4px 16px;margin:2px 0 6px}.tp .tpr-grp{display:flex;flex-direction:column;gap:1px}.tp .tpr-grp b{font-size:11px;font-weight:600;color:var(--muted)}
.tp .tpr-row{display:flex;gap:2px;flex-wrap:wrap}
.tp .tpr-cell{box-sizing:border-box;width:24px;height:24px;display:inline-flex;align-items:center;justify-content:center;border:1px solid var(--muted);border-radius:3px;background:var(--tpr-g-none);color:var(--tpr-k);font-size:13px;line-height:1}
.tp .tpr-mxleg{display:flex;flex-wrap:wrap;align-items:center;gap:2px 14px;margin:0 0 6px;font-size:11px;color:var(--muted)}.tp .tpr-mxleg .tpt-leg{margin:0}
.tp .tpr-cell.on-w{color:var(--tpr-w)}.tp .tpr-cell.on-k{color:var(--tpr-k)}
.tp .tpr-tw{max-width:100%;overflow-x:auto}.tp .tpr-tw.sc{max-height:220px;overflow:auto}
.tp table.tpr{border-collapse:collapse;font-size:12px;width:100%}.tp table.tpr th{position:sticky;top:0;background:var(--panel);text-align:left;font-weight:600;color:var(--muted);border-bottom:1px solid var(--line);padding:0 8px 0 0;white-space:nowrap}
.tp table.tpr td{height:22px;padding:0 8px 0 0;border-bottom:1px solid var(--line);white-space:nowrap}.tp table.tpr td.n,.tp table.tpr th.n{text-align:right;font-variant-numeric:tabular-nums}
.tp .tpr-id{margin:0 0 4px;font-size:12px}.tp .tpr-id b{font-weight:600}
.tp .tpr-pf{margin:0;padding:0 0 0 2px;list-style:none;font-size:12px}.tp .tpr-pf li{display:flex;gap:6px}.tp .tpr-pf .ok,.tp .tpr-pk.ok{color:var(--ok)}.tp .tpr-pf .no,.tp .tpr-pk.no{color:var(--bad)}
.tp .tpr-pk{display:inline-flex;align-items:center;justify-content:center;min-width:24px;min-height:22px;color:var(--muted)}.tp table.tpr td.c,.tp table.tpr th.c{text-align:center;padding:0 4px}
.tp details.tpr-det{margin:6px 0 0;font-size:12px}.tp details.tpr-det>summary{cursor:pointer;min-height:24px;display:inline-flex;align-items:center;color:var(--muted)}.tp details.tpr-det[open]>summary{margin-bottom:4px}
.tp .tpr-more{box-sizing:border-box;cursor:pointer;min-height:24px;min-width:24px;padding:0 8px;font:inherit;font-size:12px;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink);margin-top:4px}
.tp dl.tpr-q{display:grid;grid-template-columns:max-content 1fr;gap:3px 14px;margin:2px 0;font-size:12px}.tp dl.tpr-q dt{color:var(--muted)}.tp dl.tpr-q dd{margin:0}`;

  // ------------------------------------------------------------ texts (helps, English), given to the catalogue of the helps when there is one
  const TXT = {
    en: {
      "ft.riga.copy": "Copies the values of this row to the clipboard.",
      "ft.riga.tip": "Press T for the table of the data of this chart.",
      "ft.riga.toxic": "Opens the Data view with the XIC of this m/z, extracted from the files open there.",
      "ft.riga.toms2": "Opens the Data view on the MS2 of this precursor, at the scan drawn here.",
      "ft.riga.cands": "Candidates that fit this m/z, the one of the row first. At unit resolution several formulas share the same nominal mass: the program lists them, the chemist decides.",
      "ft.riga.neutral": "Neutral formula: the molecule, without the adduct.",
      "ft.riga.ion": "Formula of the ion that the mass spectrometer sees: the neutral formula with the adduct.",
      "ft.riga.ppm": "Observed minus calculated m/z of the ion, in parts per million.",
      "ft.riga.dmz": "Observed m/z (after the calibration offset) minus the calculated m/z of the ion, in Da. At unit resolution a few tenths of a Da do not tell two formulas apart.",
      "ft.riga.details": "Everything else the engine says about this row: the criteria in words, the MS2 fragments, the specificity, the suggested transitions.",
      "ft.pf.persist": "Persistent: a peak in enough treated samples, or in two consecutive ones.",
      "ft.pf.t0": "Absent at t0: no peak, or a much smaller one, in the sample at time zero.",
      "ft.pf.grow": "Grows over time: the area rises with the treatment time.",
      "ft.pf.iso": "Not an isotope: the peak is not the M+1 or M+2 of a stronger ion at the same RT.",
      "ft.pf.ms2": "MS2: fragments of this precursor were recorded; passed when some are explained by the parent. A dash: no MS2 for this ion.",
    },
  };
  let registered = false;
  /** gives the few helps of this file (`ft.riga.*`) to the catalogue of Tabui (`TABUI_T`, texts/40_tabui_testi.js) */
  function register() {
    const T = G.TABUI_T;
    if (T && !registered) { registered = true; Object.assign(T.en, TXT.en); }
  }
  const tt = (/** @type {string | undefined} */ k) => k && G.TPHELP ? ` data-tt="${k}"` : "";
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => /** @type {any} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  const sci = (/** @type {number} */ v) => !Number.isFinite(v) ? "–" : v === 0 ? "0" : Math.abs(v) >= 1e4 || Math.abs(v) < 0.01 ? v.toExponential(1) : String(+v.toPrecision(3));
  const fix = (/** @type {any} */ v, /** @type {number} */ d) => typeof v === "number" && Number.isFinite(v) ? v.toFixed(d) : "–";
  const fin = (/** @type {any} */ v) => typeof v === "number" && Number.isFinite(v);

  // ------------------------------------------------------------ colours (CSS variables only)
  /** a point u in 0..1 of a scale of five CSS variables (`--tpr-v0..4`): a mix of the two stops around it */
  function ramp(/** @type {"v"} */ k, /** @type {number} */ u) {
    const x = Math.max(0, Math.min(1, u)) * 4, i = Math.min(3, Math.floor(x)), f = Math.round((x - i) * 100);
    return `color-mix(in srgb,var(--tpr-${k}${i + 1}) ${f}%,var(--tpr-${k}${i}))`;
  }
  /** @param {string | undefined} type */
  const isBlank = type => /^(blank|dark|bianco|buio)/i.test(type || "");
  /** the class of a sample: 0 blank or dark, 1 t0, 2 timed, 3 other */
  const klass = (/** @type {TpSample} */ s) => isBlank(s.type) ? 0 : (/^t0/i.test(s.type || "") || s.time === 0) ? 1 : fin(s.time) ? 2 : 3;
  const GNAME = ["Blank / dark", "t0", "Times", "Other"];

  // ------------------------------------------------------------ chart frame
  /** @param {HTMLElement} box @param {string} title @param {string} desc @param {string} svg @param {{ head: string[], rows: string[][] }} table @param {{ label: string, help: string, run: () => void } | null} [go] the button that opens it in Data */
  function chart(box, title, desc, svg, table, go) {
    box.innerHTML = `<div class="tpr-chh"><h4>${esc(title)}</h4>${go ? `<button type="button" class="tpr-act tpr-go"${tt(go.help)}>${esc(go.label)}</button>` : ""}</div>${svg}`;
    const el = /** @type {SVGElement | null} */ (box.querySelector("svg")); if (!el) return;
    el.setAttribute("role", "img"); el.setAttribute("aria-label", title);
    if (G.TPHELP) { el.setAttribute("data-tt", "ft.riga.tip"); el.setAttribute("data-tt-text", `${desc} ${G.TT ? G.TT("ft.riga.tip") : ""}`.trim()); }
    if (G.TDATI) { G.TDATI.describe(el, desc); G.TDATI.addButton(el, () => ({ title, head: table.head, rows: table.rows }), true); }
    else el.setAttribute("tabindex", "0");
    const b = /** @type {HTMLButtonElement | null} */ (box.querySelector(".tpr-go")); if (b && go) b.addEventListener("click", () => go.run());
  }
  /** @param {HTMLElement} box @param {string} what @param {string} [why] */
  function none(box, what, why) { const p = document.createElement("p"); p.className = "tpr-none"; p.textContent = `${what}: ${why || "not available for this row"}.`; box.appendChild(p); }

  // XIC of all the samples: colour = time, blank/dark grey dashed, t0 black, edges of the peak shaded, line at the apex
  const W = 340, H = 150, M = { l: 40, r: 8, t: 8, b: 22 };
  /** @param {HTMLElement} box @param {TpRiga} m */
  function xicChart(box, m) {
    const xs = /** @type {TpXic[]} */ (m.xic || []).filter(t => t.rt && t.rt.length > 1);
    if (!xs.length) { none(box, "XIC", m.missing && m.missing.xic); return; }
    let x0 = Infinity, x1 = -Infinity, ym = 0;
    for (const t of xs) { x0 = Math.min(x0, t.rt[0]); x1 = Math.max(x1, t.rt[t.rt.length - 1]); for (const v of t.y) if (v > ym) ym = v; }
    if (!(x1 > x0)) x1 = x0 + 1; if (!(ym > 0)) ym = 1;
    const X = (/** @type {number} */ t) => M.l + (t - x0) / (x1 - x0) * (W - M.l - M.r), Y = (/** @type {number} */ v) => H - M.b - v / ym * (H - M.t - M.b);
    const times = xs.map(t => t.time).filter(fin).map(Number), tmin = Math.min(...times), tmax = Math.max(...times);
    /** @param {TpXic} t */
    const kind = t => isBlank(t.group) ? "blank" : (/^t0/i.test(t.group || "") || t.time === 0) ? "t0" : "time";
    const colour = (/** @type {TpXic} */ t) => fin(t.time) && tmax > tmin ? ramp("v", (+t.time - tmin) / (tmax - tmin)) : ramp("v", 0.5);
    let s = `<svg viewBox="0 0 ${W} ${H}" preserveAspectRatio="xMidYMid meet">`;
    const lo = fin(m.lo) ? Math.max(x0, /** @type {number} */ (m.lo)) : null, hi = fin(m.hi) ? Math.min(x1, /** @type {number} */ (m.hi)) : null;
    if (lo != null && hi != null && hi > lo) s += `<rect class="band" x="${X(lo).toFixed(1)}" y="${M.t}" width="${(X(hi) - X(lo)).toFixed(1)}" height="${H - M.t - M.b}"/>`;
    s += `<path class="ax" d="M${M.l},${M.t}V${H - M.b}H${W - M.r}"/>`;
    for (const t of xs) {
      const d = t.rt.map((r, i) => (i ? "L" : "M") + X(r).toFixed(1) + "," + Y(t.y[i] || 0).toFixed(1)).join(""), k = kind(t);
      s += `<path class="tr${k === "blank" ? " tr-blank" : k === "t0" ? " tr-t0" : ""}"${k === "time" ? ` style="stroke:${colour(t)}"` : ""} d="${d}"/>`;
    }
    if (fin(m.apexRt) && /** @type {number} */ (m.apexRt) >= x0 && /** @type {number} */ (m.apexRt) <= x1) s += `<line class="apex" x1="${X(/** @type {number} */ (m.apexRt)).toFixed(1)}" x2="${X(/** @type {number} */ (m.apexRt)).toFixed(1)}" y1="${M.t}" y2="${H - M.b}"/>`;
    s += `<text x="${M.l}" y="${H - 8}">${esc(x0.toFixed(2))}</text><text x="${W - M.r}" y="${H - 8}" text-anchor="end">${esc(x1.toFixed(2))} min</text><text x="${M.l - 3}" y="${M.t + 8}" text-anchor="end">${esc(sci(ym))}</text><text x="${M.l - 3}" y="${H - M.b}" text-anchor="end">0</text></svg>`;
    // one legend entry per group (not per file)
    const groups = /** @type {Map<string, TpXic>} */ (new Map()); for (const t of xs) { const g = t.group || "samples"; if (!groups.has(g)) groups.set(g, t); }
    const leg = `<p class="tpr-leg" aria-hidden="true">${[...groups].map(([g, t]) => `<span style="color:${kind(t) === "time" ? colour(t) : "var(--muted)"}"><i${kind(t) === "blank" ? ` class="d"` : ""}${kind(t) === "t0" ? ` style="color:var(--ink)"` : ""}></i><span style="color:var(--ink)">${esc(g)}</span></span>`).join("")}</p>`;
    const stride = Math.max(1, Math.ceil(xs.reduce((a, t) => a + t.rt.length, 0) / 600)), rows = /** @type {string[][]} */ ([]);
    for (const t of xs) for (let i = 0; i < t.rt.length; i += stride) rows.push([t.file, t.group || "", fin(t.time) ? String(t.time) : "", t.rt[i].toFixed(3), String(Math.round(t.y[i] || 0))]);
    const apex = fin(m.apexRt) ? `, apex at ${/** @type {number} */ (m.apexRt).toFixed(2)} min` : "", edges = lo != null && hi != null && hi > lo ? `, peak edges ${lo.toFixed(2)} to ${hi.toFixed(2)} min (shaded)` : "";
    const go = m.open && m.open.xic ? { label: "XIC in Data", help: "ft.riga.toxic", run: m.open.xic } : null;
    chart(box, "XIC, all samples", `Extracted ion chromatograms of ${xs.length} samples in ${groups.size} groups, colour = time of the sample${apex}${edges}.${m.xicTip ? " " + m.xicTip : ""}`, s, { head: ["File", "Group", "Time", "RT (min)", "Intensity"], rows }, go);
    box.insertAdjacentHTML("beforeend", leg);
  }

  // spectrum: MS1 with the expected isotopes (hollow) and the observed ones (filled), MS2, or MS2 against a reference (mirror)
  const SW = 340, SH = 150;
  /** @param {HTMLElement} box @param {TpRiga} m @param {boolean} [second] the second spectrum (`spectrum2`) */
  function specChart(box, m, second) {
    const sp = second ? m.spectrum2 : m.spectrum;
    if (!sp || !sp.mz || !sp.mz.length) { none(box, "Spectrum", m.missing && m.missing.spectrum); return; }
    const mirror = sp.kind === "mirror" && sp.ref && sp.ref.mz.length > 0, all = [...sp.mz, ...(mirror && sp.ref ? sp.ref.mz : []), ...(sp.iso && sp.iso.exp ? sp.iso.exp.mz : [])];
    if (fin(sp.prec)) all.push(/** @type {number} */ (sp.prec));
    const lo = Math.min(...all), hi = Math.max(...all), pad = (hi - lo || 1) * 0.06, a = lo - pad, b = hi + pad;
    const top = M.t, bot = SH - M.b, mid = mirror ? (top + bot) / 2 : bot, up = mid - top;
    const X = (/** @type {number} */ v) => M.l + (v - a) / (b - a) * (SW - M.l - M.r);
    const norm = (/** @type {number[]} */ y) => { const mx = Math.max(...y, 1e-12); return y.map(v => v / mx); };
    const yo = norm(sp.y), isoE = sp.iso && sp.iso.exp, isoO = sp.iso && sp.iso.obs, hasIso = sp.kind === "ms1" && (isoE || isoO);
    let s = `<svg viewBox="0 0 ${SW} ${SH}" preserveAspectRatio="xMidYMid meet"><path class="ax" d="M${M.l},${top}V${bot}H${SW - M.r}"/>${mirror ? `<line class="zero" x1="${M.l}" x2="${SW - M.r}" y1="${mid}" y2="${mid}"/>` : ""}`;
    sp.mz.forEach((v, i) => { s += `<line class="st" x1="${X(v).toFixed(1)}" x2="${X(v).toFixed(1)}" y1="${mid}" y2="${(mid - yo[i] * up * 0.92).toFixed(1)}"/>`; });
    if (mirror && sp.ref) { const yr = norm(sp.ref.y), half = bot - mid; sp.ref.mz.forEach((v, i) => { s += `<line class="st st-ref" x1="${X(v).toFixed(1)}" x2="${X(v).toFixed(1)}" y1="${mid}" y2="${(mid + yr[i] * half * 0.92).toFixed(1)}"/>`; }); }
    if (hasIso) {   // the expected isotope pattern: hollow bars; the observed ones: filled (both relative to their own most intense peak)
      if (isoE) { const ye = norm(isoE.y); isoE.mz.forEach((v, i) => { const h = ye[i] * up * 0.92; s += `<rect class="iso-e" x="${(X(v) - 3.5).toFixed(1)}" y="${(mid - h).toFixed(1)}" width="7" height="${h.toFixed(1)}"/>`; }); }
      if (isoO) { const y2 = norm(isoO.y); isoO.mz.forEach((v, i) => { const h = y2[i] * up * 0.92; s += `<rect class="iso-o" x="${(X(v) - 2).toFixed(1)}" y="${(mid - h).toFixed(1)}" width="4" height="${h.toFixed(1)}"/>`; }); }
    }
    if (fin(sp.prec)) { const px = X(/** @type {number} */ (sp.prec)).toFixed(1); s += `<line class="st st-prec" x1="${px}" x2="${px}" y1="${mid}" y2="${top}"/><text x="${px}" y="${top + 8}" text-anchor="${+px > SW / 2 ? "end" : "start"}">precursor ${esc(/** @type {number} */ (sp.prec).toFixed(2))}</text>`; }
    if (sp.kind === "ms2" && fin(sp.prec)) s += `<text x="${M.l - 3}" y="${top + 8}" text-anchor="end">${esc(sci(Math.max(...sp.y)))}</text><text x="${M.l - 3}" y="${mid}" text-anchor="end">0</text>`;
    const order = yo.map((v, i) => i).sort((p, q) => yo[q] - yo[p]).slice(0, 4);
    for (const i of order) s += `<text x="${X(sp.mz[i]).toFixed(1)}" y="${(mid - yo[i] * up * 0.92 - 2).toFixed(1)}" text-anchor="middle">${esc(sp.mz[i].toFixed(sp.kind === "ms1" ? 4 : 2))}</text>`;
    s += `<text x="${M.l}" y="${SH - 8}">${esc(a.toFixed(1))}</text><text x="${SW - M.r}" y="${SH - 8}" text-anchor="end">${esc(b.toFixed(1))} m/z</text></svg>`;
    const title = sp.label || (sp.kind === "ms1" ? "MS1 at the apex" : mirror ? "MS2 against the reference" : "MS2");
    const rows = /** @type {string[][]} */ ([]);
    if (fin(sp.prec)) rows.push(["precursor", /** @type {number} */ (sp.prec).toFixed(4), ""]);
    sp.mz.slice(0, 300).forEach((v, i) => rows.push([mirror ? "observed" : sp.kind === "ms1" ? "observed" : "MS2", v.toFixed(4), String(+(yo[i] * 100).toFixed(1))]));
    if (mirror && sp.ref) { const yr = norm(sp.ref.y); sp.ref.mz.slice(0, 300).forEach((v, i) => rows.push(["reference", v.toFixed(4), String(+(yr[i] * 100).toFixed(1))])); }
    if (isoE) { const ye = norm(isoE.y); isoE.mz.forEach((v, i) => rows.push(["expected isotope", v.toFixed(4), String(+(ye[i] * 100).toFixed(1))])); }
    const desc = `${title}: ${sp.mz.length} peaks between m/z ${lo.toFixed(2)} and ${hi.toFixed(2)}${mirror && sp.ref ? `, ${sp.ref.mz.length} reference peaks below the axis` : ""}${hasIso ? "; hollow bars are the expected isotopes, filled bars the observed ones" : ""}.`;
    const go = sp.kind !== "ms1" && m.open && m.open.ms2 ? { label: "MS2 in Data", help: "ft.riga.toms2", run: m.open.ms2 } : null;
    chart(box, title, sp.tip ? `${desc} ${sp.tip}` : desc, s, { head: ["Series", "m/z", "Relative intensity"], rows }, go);
  }

  // ------------------------------------------------------------ head
  /** @type {HTMLElement | null} */ let lastHead = null;
  /** the box of the head that was drawn last; FT-08 puts the control of the decided level into `TPRIGA.slot("decided")` @param {string} name */
  const slot = name => lastHead ? /** @type {HTMLElement | null} */ (lastHead.querySelector(`[data-slot="${name}"]`)) : null;
  /** @param {HTMLElement} box @param {TpRiga} m */
  function head(box, m) {
    register(); lastHead = box;
    const dd = G.TPDEC && m.decide ? G.TPDEC.get(m.decide.series, m.decide.key) : null, decided = dd && dd.level ? dd.level : m.decided;   // FT-08: the decision of the chemist
    const acts = (m.actions || []).map(a => `<button type="button" class="tpr-act" data-act="${esc(a.key)}"${tt(a.help)}>${esc(a.label)}</button>`).join("");
    // the badge of the level only with the decision of the chemist (it follows it); the closed row above already shows it otherwise. «Decided:» only for a table that gives it
    const dec = m.decide ? `<span class="tpr-dec" data-slot="decided"${tt("ft.decided")}${G.TPHELP ? ` tabindex="0"` : ""}>Decided: ${decided != null && decided !== "" ? esc(String(decided)) : "—"}</span>`
      : "decided" in m ? `<span class="tpr-dec"${tt("ft.decided")}${G.TPHELP ? ` tabindex="0"` : ""}>Decided: ${decided != null && decided !== "" ? esc(String(decided)) : "—"}</span>` : "";
    const lv = m.decide && m.level != null && m.level !== "" && G.TPTAB ? G.TPTAB.level(m.level, { decided }) : "";
    const sums = (m.summary || []).filter(x => x && x.value !== "" && x.value != null);
    const sum = sums.length ? `<span class="tpr-sums" role="group" aria-label="Summary of the row">${sums.map(x => `<span class="tpr-sum"${tt(x.help)}${x.help && G.TPHELP ? ` tabindex="0"` : ""}><span class="k">${esc(x.label)}</span> <span>${esc(x.value)}</span></span>`).join("")}</span>`
      : m.words ? `<span class="tpr-words">${esc(m.words)}</span>` : "";
    box.innerHTML = `<span class="tpr-head">${lv}${fin(m.pred) && G.TPTAB ? G.TPTAB.pred(m.pred) : ""}${dec}</span>` + sum + acts;
    if (G.TPDEC && m.decide) { const sl = slot("decided"); if (sl) { sl.removeAttribute("tabindex"); sl.removeAttribute("data-tt"); sl.textContent = ""; sl.appendChild(G.TPDEC.control(m.decide.series, { ...m.decide, name: m.title }, null)); } }
    if (G.TPDEC && m.decide) {   // a decision (here or in the Identity tab) changes the badge of the head
      const old = /** @type {any} */ (box)._tpdec; if (old) window.removeEventListener(G.TPDEC.EVENT, old);
      const on = (/** @type {Event} */ e) => {
        if (!box.isConnected) { window.removeEventListener(G.TPDEC.EVENT, on); return; }
        const d = /** @type {CustomEvent} */ (e).detail, dc = m.decide; if (!dc || d.fp !== G.TPDEC.key(dc.series) || d.rowKey !== dc.key) return;
        const x = G.TPDEC.get(dc.series, dc.key), bd = box.querySelector(".tpr-head > .lvb:not([class*=pd-])");
        if (bd && G.TPTAB) bd.outerHTML = G.TPTAB.level(m.level, { decided: x && x.level ? x.level : null });
      };
      /** @type {any} */ (box)._tpdec = on; window.addEventListener(G.TPDEC.EVENT, on);
    }
    box.querySelectorAll("button[data-act]").forEach(b => b.addEventListener("click", () => { const a = (m.actions || []).find(x => x.key === /** @type {HTMLElement} */ (b).dataset.act); if (a) a.run(m); }));
  }

  // ------------------------------------------------------------ tabs
  const TABS = [
    { key: "overview", label: "Overview", help: "ft.tab.overview" },
    { key: "samples", label: "Samples", help: "ft.tab.samples" },
    { key: "identity", label: "Identity", help: "ft.tab.identity" },
    { key: "quality", label: "Quality", help: "ft.tab.quality" },
  ];

  /** @param {HTMLElement} panel @param {TpRiga} m */
  function overview(panel, m) {
    const g = document.createElement("div"); g.className = "tpr-ov"; panel.appendChild(g);
    const a = document.createElement("div"), b = document.createElement("div"); a.className = b.className = "tpr-ch"; g.append(a, b);
    xicChart(a, m); specChart(b, m);
    if (m.spectrum2 && m.spectrum2.mz && m.spectrum2.mz.length) { const c = document.createElement("div"); c.className = "tpr-ch"; g.appendChild(c); specChart(c, m, true); }
  }

  /** @param {HTMLElement} panel @param {TpRiga} m */
  function samples(panel, m) {
    const ss = m.samples || [];
    if (!ss.length) { none(panel, "Samples", m.missing && m.missing.samples); return; }
    const sc = G.TPTAB ? G.TPTAB.areaScale(ss.map(s => s.area)) : (/** @type {any} */ () => null);
    const wordOf = (/** @type {string | undefined} */ st) => { const x = /** @type {HTMLElement} */ (document.createElement("div")); x.innerHTML = G.TPTAB ? G.TPTAB.state(st || "na") : ""; const l = x.firstElementChild; return l ? l.getAttribute("aria-label") || "" : st || ""; };
    const sym = (/** @type {string | undefined} */ st) => { const x = document.createElement("div"); x.innerHTML = G.TPTAB ? G.TPTAB.state(st || "na") : ""; return x.textContent || ""; };
    const order = ss.map((s, i) => i).sort((p, q) => klass(ss[p]) - klass(ss[q]) || (+(ss[p].time ?? 0)) - (+(ss[q].time ?? 0)));
    let mx = `<div class="tpr-mx" role="group" aria-label="Samples by group">`;
    for (const k of [0, 1, 2, 3]) {
      const ix = order.filter(i => klass(ss[i]) === k); if (!ix.length) continue;
      mx += `<div class="tpr-grp"><b${G.TPHELP ? ` tabindex="0"` : ""}${tt("ft.riga.g" + k)}>${GNAME[k]}</b><div class="tpr-row">${ix.map(i => {
        const s = ss[i], pr = G.TPTAB ? G.TPTAB.paint(sc(s.area)) : { cls: "", style: "" };
        const lab = `${s.file}: area ${fin(s.area) ? sci(/** @type {number} */ (s.area)) : "none"}, ${wordOf(s.state)}`;
        const tip = `${s.file} · ${s.type || GNAME[k]}${fin(s.time) ? `, time ${s.time}` : ""} · area ${fin(s.area) ? sci(/** @type {number} */ (s.area)) : "none"} · peak ${wordOf(s.state)}`;
        return `<span class="tpr-cell${pr.cls}" role="img" tabindex="0" aria-label="${esc(lab)}"${G.TPHELP ? ` data-tt="ft.riga.cell" data-tt-text="${esc(tip)}"` : ""}${pr.style}><span aria-hidden="true">${esc(sym(s.state))}</span></span>`;
      }).join("")}</div></div>`;
    }
    const hasNorm = ss.some(s => fin(s.norm));
    const mxBox = document.createElement("div"); mxBox.innerHTML = mx + `</div>`; const mxEl = /** @type {HTMLElement} */ (mxBox.firstElementChild); panel.appendChild(mxEl);
    if (G.TPTAB) {
      const used = ["detected", "filled", "targeted", "absent", "na"];
      const lg = document.createElement("div"); lg.className = "tpr-mxleg";
      lg.innerHTML = `<span class="tpr-mxnote">White = none, darker green = larger area (log scale, relative to the maximum of the row); symbol = peak state. Filled = area re-read by the program where the peak was expected.</span>${G.TPTAB.legend(used)}`;
      panel.appendChild(lg);
      if (G.TPHELP) G.TPHELP.attach(lg);
    }
    const head = ["File", "Type", "Time", "Area", ...(hasNorm ? ["Normalized"] : []), "State", "Apex RT", "Edges"];
    const rowsOf = ss.map(s => [s.file, s.type || "", fin(s.time) ? String(s.time) : "", fin(s.area) ? sci(/** @type {number} */ (s.area)) : "–", ...(hasNorm ? [fin(s.norm) ? sci(/** @type {number} */ (s.norm)) : "–"] : []), wordOf(s.state), fix(s.apexRt, 2),
      fin(s.lo) && fin(s.hi) ? `${fix(s.lo, 2)}–${fix(s.hi, 2)}` : "–"]);
    const num = (/** @type {number} */ i) => ["Time", "Area", "Normalized", "Apex RT"].includes(head[i]);
    const tw = document.createElement("div"); tw.className = "tpr-tw" + (ss.length > 10 ? " sc" : "");
    tw.innerHTML = `<table class="tpr"><caption class="sr-only">Samples of the row</caption><thead><tr>${head.map((h, i) => `<th scope="col"${num(i) ? ` class="n"` : ""}>${h}</th>`).join("")}</tr></thead><tbody>${rowsOf.map(r => `<tr>${r.map((c, i) => `<td${num(i) ? ` class="n"` : ""}>${esc(c)}</td>`).join("")}</tr>`).join("")}</tbody></table>`;
    if (ss.length > 10) { tw.tabIndex = 0; tw.setAttribute("role", "region"); tw.setAttribute("aria-label", "Samples table"); }
    panel.appendChild(tw);
    const d = `Matrix of ${ss.length} samples in groups (blank or dark, t0, times, other). Colour = area on a log scale, the symbol says the state.`;
    if (G.TDATI) { G.TDATI.describe(mxEl, d); mxEl.setAttribute("role", "group"); mxEl.setAttribute("aria-label", "Samples by group"); }
  }

  /** the proofs drawn as columns of the candidates: key, header, help */
  const PF = /** @type {[string, string, string][]} */ ([["persist", "Persistent", "ft.pf.persist"], ["t0", "Not at t0", "ft.pf.t0"], ["grow", "Grows", "ft.pf.grow"], ["iso", "Not isotope", "ft.pf.iso"], ["ms2", "MS2", "ft.pf.ms2"]]);
  const pfWord = (/** @type {boolean | null} */ ok) => ok == null ? "not tested" : ok ? "passed" : "failed";
  /** the closed box «Details» at the end of a tab: what is long goes here (a table adds its own parts to it) @param {HTMLElement} panel */
  function details(panel) {
    let d = /** @type {HTMLDetailsElement | null} */ (panel.querySelector(":scope > details.tpr-det"));
    if (!d) { d = document.createElement("details"); d.className = "tpr-det"; d.innerHTML = `<summary${tt("ft.riga.details")}>Details</summary><div class="tpr-detb"></div>`; panel.appendChild(d); }
    return /** @type {HTMLElement} */ (d.querySelector(".tpr-detb"));
  }
  /** @param {HTMLElement} panel @param {TpRiga} m */
  function identity(panel, m) {
    const id = m.identity;
    if (G.TPDEC && m.decide) { const p = document.createElement("p"); p.className = "tpr-id"; p.append("Decided level ", G.TPDEC.control(m.decide.series, { ...m.decide, name: m.title }, null)); panel.appendChild(p); }
    if (!id) { none(panel, "Identity", m.missing && m.missing.identity); return; }
    const cs = id.candidates || [];
    if (!cs.length) { none(panel, "Candidates", m.missing && m.missing.candidates || "no candidate for this row"); return; }
    const ad = id.adduct || "[M+H]+", cols = PF.filter(([k]) => cs.some(c => (c.proofs || []).some(p => p.key === k)));
    const hasName = cs.some(c => c.name && c.name !== c.formula), hasIon = cs.some(c => c.ion), hasPpm = cs.some(c => fin(c.ppm)), hasDmz = !hasPpm && cs.some(c => fin(c.dmz));
    const th = (/** @type {string} */ label, /** @type {string} */ help, /** @type {string} */ cls) => `<th scope="col"${cls ? ` class="${cls}"` : ""}><span${tt(help)}${G.TPHELP ? ` tabindex="0"` : ""}>${label}</span></th>`;
    const proof = (/** @type {TpCand} */ c, /** @type {string} */ k, /** @type {string} */ name, /** @type {string} */ key) => {
      const p = (c.proofs || []).find(x => x.key === k), ok = p ? p.ok : null, w = p ? pfWord(ok) : "not tested";
      return `<td class="c"><span class="tpr-pk${ok == null ? "" : ok ? " ok" : " no"}" role="img" aria-label="${esc(`${name}: ${w}`)}"${G.TPHELP ? ` tabindex="0" data-tt="${key}" data-tt-text="${esc(`${name}: ${w}.${p && p.text ? " " + p.text : ""}`)}"` : ""}>${ok == null ? "–" : ok ? "✓" : "✗"}</span></td>`;
    };
    const one = (/** @type {TpCand} */ c) => `<tr>${hasName ? `<td>${esc(c.name || "–")}${fin(c.pred) && G.TPTAB ? " " + G.TPTAB.pred(/** @type {number} */ (c.pred)) : ""}</td>` : ""}<td>${esc(c.formula || "–")}${!hasName && fin(c.pred) && G.TPTAB ? " " + G.TPTAB.pred(/** @type {number} */ (c.pred)) : ""}</td>`
      + (hasIon ? `<td>${esc(c.ion || "–")}</td>` : "") + (hasPpm ? `<td class="n">${fix(c.ppm, 1)}</td>` : hasDmz ? `<td class="n">${fin(c.dmz) ? (/** @type {number} */ (c.dmz) >= 0 ? "+" : "") + /** @type {number} */ (c.dmz).toFixed(2) : "–"}</td>` : "")
      + cols.map(([k, name, key]) => proof(c, k, name, key)).join("") + `</tr>`;
    const head = (hasName ? th("Candidate", "ft.riga.cands", "") : "") + th("Neutral formula", hasName ? "ft.riga.neutral" : "ft.riga.cands", "") + (hasIon ? th(`${esc(ad)} formula`, "ft.riga.ion", "") : "")
      + (hasPpm ? th("Δppm", "ft.riga.ppm", "n") : hasDmz ? th("Δm/z (Da)", "ft.riga.dmz", "n") : "") + cols.map(([, name, help]) => th(name, help, "c")).join("");
    const SHOW = 5, shown = cs.slice(0, SHOW), rest = cs.slice(SHOW);
    const tw = document.createElement("div"); tw.className = "tpr-tw" + (cs.length > 10 ? " sc" : "");
    tw.innerHTML = `<table class="tpr tpr-cand"><caption class="sr-only">Candidates</caption><thead><tr>${head}</tr></thead><tbody>${shown.map(one).join("")}</tbody></table>`;
    if (cs.length > 10) { tw.tabIndex = 0; tw.setAttribute("role", "region"); tw.setAttribute("aria-label", "Candidates table"); }
    panel.appendChild(tw);
    if (rest.length) {
      const bt = document.createElement("button"); bt.type = "button"; bt.className = "tpr-more"; bt.textContent = `+${rest.length}`; bt.setAttribute("aria-expanded", "false"); bt.setAttribute("aria-label", `Show ${rest.length} more candidates`);
      bt.addEventListener("click", () => { const open = bt.getAttribute("aria-expanded") === "true"; const tb = /** @type {HTMLElement} */ (tw.querySelector("tbody")); tb.innerHTML = (open ? shown : cs).map(one).join(""); bt.setAttribute("aria-expanded", String(!open)); bt.textContent = open ? `+${rest.length}` : "−"; bt.setAttribute("aria-label", open ? `Show ${rest.length} more candidates` : "Show fewer candidates"); });
      panel.appendChild(bt);
    }
    // on request: the neutral mass, the source of each candidate and every criterion in words
    const box = details(panel), all = cs.flatMap(c => (c.proofs || []).map(p => ({ c, p }))), firstOnly = cs.length > 1 && cs.slice(1).every(c => !(c.proofs || []).length);
    let h = fin(id.neutral) || (id.neutral != null && id.neutral !== "") ? `<p class="tpr-id"><b>Neutral mass</b> ${esc(fin(id.neutral) ? /** @type {number} */ (id.neutral).toFixed(4) : id.neutral)} · <b>Adduct</b> ${esc(ad)}</p>` : "";
    const src = cs.filter(c => c.source);
    if (src.length) h += `<p class="tpr-id"><b>Source</b> ${src.map(c => esc(`${c.name || c.formula || ""}: ${c.source}`)).join("; ")}</p>`;
    const seen = new Set(), pfs = all.filter(({ p }) => { const k = (p.key || "") + "|" + p.text; if (seen.has(k)) return false; seen.add(k); return true; });
    if (pfs.length) h += `<ul class="tpr-pf" aria-label="Criteria">${pfs.map(({ c, p }) => `<li><span class="${p.ok == null ? "" : p.ok ? "ok" : "no"}" role="img" aria-label="${pfWord(p.ok)}">${p.ok == null ? "–" : p.ok ? "✓" : "✗"}</span><span>${esc((cs.length > 1 && !firstOnly ? (c.name || c.formula || "") + ": " : "") + p.text)}</span></li>`).join("")}</ul>`;
    box.insertAdjacentHTML("beforeend", h);
  }

  /** the words of the quality flags of the engine */
  const FLAG = /** @type {Record<string, string>} */ ({ shape: "peak shape", snr: "low S/N", "s/n": "low S/N", points: "few points", few_points: "few points", "few points": "few points" });
  /** presence in the blank or in t0: a flag, a ratio or both @param {any} v */
  function presence(v) {
    if (v == null) return "–";
    if (typeof v === "boolean") return v ? "present" : "absent";
    if (typeof v === "number") return `ratio ${sci(v)}`;
    if (typeof v === "object") return `${v.present == null ? "" : v.present ? "present" : "absent"}${fin(v.ratio) ? `${v.present == null ? "" : ", "}ratio ${sci(v.ratio)}` : ""}` || "–";
    return String(v);
  }
  /** @param {HTMLElement} panel @param {TpRiga} m */
  function quality(panel, m) {
    const q = m.quality;
    if (!q) { none(panel, "Quality", m.missing && m.missing.quality); return; }
    const flags = (q.flags || []).map(f => FLAG[f] || f), dt = (/** @type {string} */ t, /** @type {string} */ v, /** @type {string} */ k) => `<dt${k ? tt(k) + (G.TPHELP ? ` tabindex="0"` : "") : ""}>${t}</dt><dd>${v}</dd>`;
    panel.insertAdjacentHTML("beforeend", `<dl class="tpr-q">${dt("Shape", esc(q.shape == null ? "–" : fin(q.shape) ? fix(q.shape, 2) : q.shape), "")}${dt("S/N", fin(q.snr) ? sci(/** @type {number} */ (q.snr)) : "–", "")}${dt("FWHM", fin(q.fwhm) ? esc(String(q.fwhm)) : "–", "")}`
      + dt("Warnings", flags.length ? esc(flags.join(", ")) : "none", "ft.q.flags") + dt("M+1/M", q.isoM1 == null ? "–" : esc(fin(q.isoM1) ? fix(q.isoM1, 2) : q.isoM1), "") + dt("In the blank", esc(presence(q.blank)), "") + dt("In t0", esc(presence(q.t0)), "")
      + dt("Left out of the view by", (q.rejected || []).length ? esc((q.rejected || []).join(", ")) : "nothing", "") + `</dl>`);
  }

  // ------------------------------------------------------------ to the Data view: the ion of the row opened there at once (openXic of pannelli.js, goMs2 and ms2Spectrum of tabs.js)
  const say = (/** @type {string} */ t) => { if (typeof G.toast === "function") G.toast(t); };
  const base = (/** @type {any} */ s) => (String(s || "").split(/[\\/]/).pop() || "").replace(/\.(mzml|mzxml|mzidx|raw|wiff|d)$/i, "").toLowerCase();
  /** the files open in the Data view (of the kind `kind`, or all) @param {string} [kind] @returns {any[]} */
  const dataFiles = kind => typeof E !== "undefined" ? E.files.filter((/** @type {any} */ f) => !f.gone && (!kind || f.kind === kind)) : [];
  /** the XIC of `mz` in a new panel of the Data view, from the files open there, without the dialog @param {number} mz */
  function toDataXic(mz) {
    if (typeof openXic !== "function" || typeof setView !== "function" || typeof tabFiles !== "function") { say("The Data view is not ready."); return false; }
    const fs = dataFiles().filter(f => f.kind !== "ms2");
    if (!fs.length) { say("Open the files in the Data view first: the XIC is extracted from them."); return false; }
    setView("data");
    if (E.tab === "ms2" || !tabFiles(E.tab).length) setTab(fs[0].kind, true);
    openXic(null, { mz, diretto: true });
    return true;
  }
  /** the MS2 of the precursor `prec` in the Data view: its chromatogram and product-ion spectrum, at the scan of `rt` (of the file `file`) or at the highest one
   * @param {number} prec @param {number | null} [rt] @param {string} [file] */
  async function toDataMs2(prec, rt, file) {
    if (typeof ms2Exps !== "function" || typeof setView !== "function") { say("The Data view is not ready."); return false; }
    const ex = /** @type {any[]} */ (ms2Exps()).filter(x => x.prec != null).sort((a, b) => Math.abs(a.prec - prec) - Math.abs(b.prec - prec))[0];
    if (!ex || Math.abs(ex.prec - prec) > 0.6) return toInfusion(prec, file);
    setView("data");
    const f = file ? dataFiles("ms2").find(x => base(x.name || x.file) === base(file)) : null;
    if (fin(rt) && typeof goMs2 === "function") await goMs2({ prec: ex.prec, rt, k2: f ? f.k : ex.k });
    else { setTab("ms2", true); await ms2Spectrum(ex.prec); }
    return true;
  }

  /** a file of one precursor is an infusion in the Data view: its tab, with the node of the precursor nearest to `prec` in its tree
   * @param {number} prec @param {string} [file] */
  async function toInfusion(prec, file) {
    const near = (/** @type {any} */ f) => (f.ms2_exps || []).some((/** @type {any} */ x) => x.prec != null && Math.abs(x.prec - prec) <= 0.6);
    const fs = dataFiles("infusion").filter(near), f = fs.find(x => file && base(x.name || x.file) === base(file)) || fs[0];
    if (!f || typeof INF === "undefined") { say(`No MS2 of precursor m/z ${prec.toFixed(2)} among the files open in the Data view.`); return false; }
    setView("data"); if (typeof activeFile === "function") activeFile(f.k); else setTab("infusion", true);      // the file shown and active: its tab lays out
    await new Promise(r => setTimeout(r, 700));             // the tab lays out its cells first (INF.layout), as goMs2 waits for the MS2 tab
    const c = E.panels.find((/** @type {any} */ q) => q.type === "chrom" && q.tab === "infusion"), j = await INF.tree(f.k);
    const n = (j && j.nodi || []).filter((/** @type {any} */ x) => x.prec != null).sort((/** @type {any} */ a, /** @type {any} */ b) => Math.abs(a.prec - prec) - Math.abs(b.prec - prec))[0];
    if (c && n) await INF.pick(c, n.id);
    return true;
  }

  /** draws the tab `tab` of the model `m` into `panel` (which it empties)
   * @param {HTMLElement} panel @param {string} tab @param {TpRiga} m */
  function fill(panel, tab, m) {
    register(); panel.innerHTML = "";
    if (tab === "samples") samples(panel, m); else if (tab === "identity") { identity(panel, m); if (G.TPCOMP && m.composition) G.TPCOMP.mount(panel, m.composition); } else if (tab === "quality") quality(panel, m); else overview(panel, m);
  }

  /** the options of `TPTAB.mount` that put these panels in the open row: `TPRIGA.hook(r => model)` @param {(r: any) => TpRiga} model */
  function hook(model) { return { tabs: TABS, head: (/** @type {any} */ r, /** @type {HTMLElement} */ box) => head(box, model(r)), onOpen: (/** @type {any} */ r, /** @type {HTMLElement} */ p, /** @type {string} */ t) => fill(p, t, model(r)) }; }

  if (!document.getElementById("tpr-css")) { const s = document.createElement("style"); s.id = "tpr-css"; s.textContent = CSS; document.head.appendChild(s); }
  G.TPRIGA = { tabs: TABS, head, fill, slot, hook, details, texts: TXT, toData: { xic: toDataXic, ms2: toDataMs2 } };
})();
