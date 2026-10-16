// @ts-check
// TPMINE-PRIVATE  TPRIGA: what the open row of a Tabui table shows. One head (level, prediction, decided, words, actions) and four tabs (Overview, Samples, Identity, Quality), drawn from a
// row MODEL (`TpRiga`) that every table builds from its own engine. This file shows what the model holds and computes no number; a part that the model lacks is not drawn (one sentence says why).
// Charts: SVG with title, description and the table of the data (TDATI); colours only from CSS variables (time = viridis, areas = cividis, groups of the blanks grey); no animation. Texts in English;
// the helps are keys of the catalogue of Tabui (`ft.tab.*`, `ft.decided`, `ft.adduct`, `ft.q.flags`, `ft.showdata`); `ft.riga.copy` is its own.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);

  /**
   * @typedef {{ file: string, group?: string, time?: number | null, rt: number[], y: number[] }} TpXic
   * @typedef {{ kind: "ms1" | "ms2" | "mirror", mz: number[], y: number[], ref?: { mz: number[], y: number[] }, iso?: { obs?: { mz: number[], y: number[] }, exp?: { mz: number[], y: number[] } }, prec?: number, label?: string }} TpSpec
   * @typedef {{ file: string, type?: string, time?: number | null, area?: number | null, norm?: number | null, state?: string, apexRt?: number | null, lo?: number | null, hi?: number | null }} TpSample
   * @typedef {{ ok: boolean, text: string }} TpProof
   * @typedef {{ name?: string, formula?: string, ppm?: number | null, adduct?: string, source?: string, level?: string | number, pred?: number, proofs?: TpProof[] }} TpCand
   * @typedef {{ key: string, label: string, help?: string, run: (m: TpRiga) => void }} TpAction
   * @typedef {{ title?: string, level?: string | number, pred?: number, decided?: string | number | null, words?: string, actions?: TpAction[], xic?: TpXic[], apexRt?: number | null, lo?: number | null, hi?: number | null,
   *   spectrum?: TpSpec | null, spectrum2?: TpSpec | null, line?: string, samples?: TpSample[], identity?: { neutral?: number | string | null, adduct?: string, candidates?: TpCand[] } | null,
   *   quality?: { shape?: number | string | null, snr?: number | null, fwhm?: number | null, flags?: string[], isoM1?: number | string | null, blank?: any, t0?: any, rejected?: string[] } | null,
   *   decide?: { series: any, key: string, level?: string | number | null, name?: string }, ms2text?: string, missing?: Record<string, string> }} TpRiga
   */

  const CSS = `
.tp{--tpr-v0:#440154;--tpr-v1:#3b528b;--tpr-v2:#21918c;--tpr-v3:#5ec962;--tpr-v4:#addc30;--tpr-c0:#00204d;--tpr-c1:#575c6d;--tpr-c2:#a69d75;--tpr-c3:#e4cf5b;--tpr-c4:#ffea46;--tpr-w:#fff;--tpr-k:#111}
@media(prefers-color-scheme:dark){:root:not([data-theme=light]) .tp{--tpr-v0:#38bdf8;--tpr-v1:#14b8a6;--tpr-v2:#10b981;--tpr-v3:#84cc16;--tpr-v4:#fde047}}
:root[data-theme=dark] .tp{--tpr-v0:#38bdf8;--tpr-v1:#14b8a6;--tpr-v2:#10b981;--tpr-v3:#84cc16;--tpr-v4:#fde047}
.tp .tpr-ov{display:grid;grid-template-columns:repeat(auto-fill,minmax(300px,1fr));gap:6px 14px;align-items:start}
.tp .tpr-ch{min-width:0}.tp .tpr-ch h4{margin:0 0 2px;font-size:12px;color:var(--muted);font-weight:600}.tp .tpr-ch svg{display:block;width:100%;height:auto;max-height:170px}
.tp .tpr-ch svg text{fill:var(--muted);font-size:9px}.tp .tpr-ch svg .ax{stroke:var(--muted);fill:none;stroke-width:1}
.tp .tpr-ch svg .tr{fill:none;stroke-width:1.4}.tp .tpr-ch svg .tr-blank{stroke:var(--muted);stroke-dasharray:4 3}.tp .tpr-ch svg .tr-t0{stroke:var(--ink)}
.tp .tpr-ch svg .band{fill:var(--sel);stroke:var(--line)}.tp .tpr-ch svg .apex{stroke:var(--accent);stroke-width:1;stroke-dasharray:2 2}
.tp .tpr-ch svg .st{stroke:var(--ink);stroke-width:1.6}.tp .tpr-ch svg .st-ref{stroke:var(--muted)}.tp .tpr-ch svg .iso-o{fill:var(--accent)}.tp .tpr-ch svg .iso-e{fill:var(--panel);stroke:var(--ink);stroke-width:1}
.tp .tpr-ch svg .zero{stroke:var(--line);stroke-width:1}.tp .tpr-ch svg .st-prec{stroke:var(--accent);stroke-width:2.4;stroke-dasharray:4 2}
.tp .tpr-ms2t{margin:6px 0 0;font-size:12px}.tp .tpr-ms2t .tpr-txt{display:block;user-select:text;-webkit-user-select:text;white-space:normal;overflow-wrap:anywhere;padding:4px 6px;border:1px solid var(--line);border-radius:5px;background:var(--panel)}
.tp .tpr-ms2t button{box-sizing:border-box;cursor:pointer;min-height:24px;min-width:24px;margin-top:3px;padding:0 10px;font:inherit;font-size:12px;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink)}
@media(pointer:coarse){.tp .tpr-ms2t button{min-height:40px}}
.tp .tpr-ch .tdbtn{margin:2px 0}
.tp .tpr-leg{display:flex;flex-wrap:wrap;gap:0 12px;font-size:11px;color:var(--muted);margin:0}.tp .tpr-leg span{display:inline-flex;align-items:center;gap:4px;min-height:18px}
.tp .tpr-leg i{display:inline-block;width:16px;height:0;border-top:3px solid currentColor}.tp .tpr-leg i.d{border-top-style:dashed}
.tp .tpr-line{margin:4px 0 0;font-size:12px;color:var(--muted)}
.tp .tpr-none{margin:4px 0;font-size:12px;color:var(--muted)}
.tp .tpr-head{display:inline-flex;flex-wrap:wrap;gap:4px 10px;align-items:center}.tp .tpr-words{flex:1 1 20em;font-size:12px}
.tp .tpr-act{box-sizing:border-box;cursor:pointer;min-height:24px;min-width:24px;padding:0 10px;font:inherit;font-size:12px;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink)}
.tp .tpr-act:hover{background:var(--sel)}.tp .tpr-act:focus-visible,.tp .tpr-more:focus-visible,.tp .tpr-ch svg:focus-visible,.tp .tpr-mx:focus-visible{outline:2px solid var(--accent);outline-offset:1px}
.tp .tpr-dec{font-size:12px;color:var(--muted)}
.tp .tpr-mx{display:flex;flex-wrap:wrap;gap:4px 16px;margin:2px 0 6px}.tp .tpr-grp{display:flex;flex-direction:column;gap:1px}.tp .tpr-grp b{font-size:11px;font-weight:600;color:var(--muted)}
.tp .tpr-row{display:flex;gap:2px;flex-wrap:wrap}
.tp .tpr-cell{box-sizing:border-box;width:24px;height:24px;display:inline-flex;align-items:center;justify-content:center;border:1px solid var(--muted);border-radius:3px;background:var(--panel);color:var(--muted);font-size:13px;line-height:1}
.tp .tpr-cell.on-w{color:var(--tpr-w)}.tp .tpr-cell.on-k{color:var(--tpr-k)}
.tp .tpr-tw{max-width:100%;overflow-x:auto}.tp .tpr-tw.sc{max-height:220px;overflow:auto}
.tp table.tpr{border-collapse:collapse;font-size:12px;width:100%}.tp table.tpr th{position:sticky;top:0;background:var(--panel);text-align:left;font-weight:600;color:var(--muted);border-bottom:1px solid var(--line);padding:0 8px 0 0;white-space:nowrap}
.tp table.tpr td{height:22px;padding:0 8px 0 0;border-bottom:1px solid var(--line);white-space:nowrap}.tp table.tpr td.n,.tp table.tpr th.n{text-align:right;font-variant-numeric:tabular-nums}
.tp .tpr-id{margin:0 0 4px;font-size:12px}.tp .tpr-id b{font-weight:600}
.tp .tpr-pf{margin:0;padding:0 0 0 2px;list-style:none;font-size:12px}.tp .tpr-pf li{display:flex;gap:6px}.tp .tpr-pf .ok{color:var(--ok)}.tp .tpr-pf .no{color:var(--bad)}
.tp .tpr-more{box-sizing:border-box;cursor:pointer;min-height:24px;min-width:24px;padding:0 8px;font:inherit;font-size:12px;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink);margin-top:4px}
.tp dl.tpr-q{display:grid;grid-template-columns:max-content 1fr;gap:3px 14px;margin:2px 0;font-size:12px}.tp dl.tpr-q dt{color:var(--muted)}.tp dl.tpr-q dd{margin:0}`;

  // ------------------------------------------------------------ texts (helps; it/en), given to the catalogue of the helps when there is one
  const TXT = {
    it: { "ft.riga.copy": "Copia negli appunti i valori di questa riga." },
    en: { "ft.riga.copy": "Copies the values of this row to the clipboard." },
  };
  let registered = false;
  /** gives the few helps of this file (`ft.riga.*`) to the catalogue of Tabui (`TABUI_T`, texts/40_tabui_testi.js) */
  function register() {
    const T = G.TABUI_T;
    if (T && !registered) { registered = true; for (const l of /** @type {("it" | "en")[]} */ (["it", "en"])) if (T[l]) Object.assign(T[l], TXT[l]); }
  }
  const tt = (/** @type {string | undefined} */ k) => k && G.TPHELP ? ` data-tt="${k}"` : "";
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => /** @type {any} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  const sci = (/** @type {number} */ v) => !Number.isFinite(v) ? "–" : v === 0 ? "0" : Math.abs(v) >= 1e4 || Math.abs(v) < 0.01 ? v.toExponential(1) : String(+v.toPrecision(3));
  const fix = (/** @type {any} */ v, /** @type {number} */ d) => typeof v === "number" && Number.isFinite(v) ? v.toFixed(d) : "–";
  const fin = (/** @type {any} */ v) => typeof v === "number" && Number.isFinite(v);

  // ------------------------------------------------------------ colours (CSS variables only)
  /** a point u in 0..1 of a scale of five CSS variables (`--tpr-v0..4` or `--tpr-c0..4`): a mix of the two stops around it */
  function ramp(/** @type {"v" | "c"} */ k, /** @type {number} */ u) {
    const x = Math.max(0, Math.min(1, u)) * 4, i = Math.min(3, Math.floor(x)), f = Math.round((x - i) * 100);
    return `color-mix(in srgb,var(--tpr-${k}${i + 1}) ${f}%,var(--tpr-${k}${i}))`;
  }
  /** @param {string | undefined} type */
  const isBlank = type => /^(blank|dark|bianco|buio)/i.test(type || "");
  /** the class of a sample: 0 blank or dark, 1 t0, 2 timed, 3 other */
  const klass = (/** @type {TpSample} */ s) => isBlank(s.type) ? 0 : (/^t0/i.test(s.type || "") || s.time === 0) ? 1 : fin(s.time) ? 2 : 3;
  const GNAME = ["Blank / dark", "t0", "Times", "Other"];

  // ------------------------------------------------------------ chart frame
  /** @param {HTMLElement} box @param {string} title @param {string} desc @param {string} svg @param {{ head: string[], rows: string[][] }} table */
  function chart(box, title, desc, svg, table) {
    box.innerHTML = `<h4>${esc(title)}</h4>${svg}`;
    const el = /** @type {SVGElement | null} */ (box.querySelector("svg")); if (!el) return;
    el.setAttribute("role", "img"); el.setAttribute("aria-label", title);
    if (G.TDATI) { G.TDATI.describe(el, desc); G.TDATI.addButton(el, () => ({ title, head: table.head, rows: table.rows })); }
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
    chart(box, "XIC, all samples", `Extracted ion chromatograms of ${xs.length} samples in ${groups.size} groups, colour = time of the sample${apex}${edges}.`, s, { head: ["File", "Group", "Time", "RT (min)", "Intensity"], rows });
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
    chart(box, title, desc, s, { head: ["Series", "m/z", "Relative intensity"], rows });
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
    box.innerHTML = `<span class="tpr-head">${m.level != null && m.level !== "" && G.TPTAB ? G.TPTAB.level(m.level, { decided }) : ""}${fin(m.pred) && G.TPTAB ? G.TPTAB.pred(m.pred) : ""}`
      + `<span class="tpr-dec" data-slot="decided"${tt("ft.decided")}${G.TPHELP ? ` tabindex="0"` : ""}>Decided: ${decided != null && decided !== "" ? esc(String(decided)) : "—"}</span></span>`
      + (m.words ? `<span class="tpr-words">${esc(m.words)}</span>` : "") + acts;
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
    if (m.ms2text) ms2Text(panel, m.ms2text);
    if (m.line) { const p = document.createElement("p"); p.className = "tpr-line"; p.textContent = m.line; panel.appendChild(p); }
  }

  /** the sentence of the fragments found: one selectable text with a button that copies it @param {HTMLElement} panel @param {string} text */
  function ms2Text(panel, text) {
    const d = document.createElement("div"); d.className = "tpr-ms2t";
    const t = document.createElement("span"); t.className = "tpr-txt"; t.id = "tpr-ms2-text"; t.textContent = text; t.tabIndex = 0; t.setAttribute("role", "group"); t.setAttribute("aria-label", "Fragment ions found in the MS2 of this row");
    const b = document.createElement("button"); b.type = "button"; b.id = "tpr-ms2-copy"; b.textContent = "Copy text"; b.setAttribute("aria-label", "Copy the fragment ions sentence");
    const say = document.createElement("span"); say.className = "tpr-none"; say.setAttribute("role", "status"); say.setAttribute("aria-live", "polite");
    b.onclick = () => Promise.resolve(navigator.clipboard && navigator.clipboard.writeText(text)).then(() => { say.textContent = " Copied"; }, () => { const r = document.createRange(); r.selectNodeContents(t); const g = window.getSelection(); if (g) { g.removeAllRanges(); g.addRange(r); } say.textContent = " Selected: press Ctrl+C"; });
    d.append(t, b, say); panel.appendChild(d);
  }

  /** @param {HTMLElement} panel @param {TpRiga} m */
  function samples(panel, m) {
    const ss = m.samples || [];
    if (!ss.length) { none(panel, "Samples", m.missing && m.missing.samples); return; }
    const areas = ss.map(s => s.area).filter(a => fin(a) && /** @type {number} */ (a) > 0).map(Number), lmin = areas.length ? Math.log10(Math.min(...areas)) : 0, lmax = areas.length ? Math.log10(Math.max(...areas)) : 1;
    const wordOf = (/** @type {string | undefined} */ st) => { const x = /** @type {HTMLElement} */ (document.createElement("div")); x.innerHTML = G.TPTAB ? G.TPTAB.state(st || "na") : ""; const l = x.firstElementChild; return l ? l.getAttribute("aria-label") || "" : st || ""; };
    const sym = (/** @type {string | undefined} */ st) => { const x = document.createElement("div"); x.innerHTML = G.TPTAB ? G.TPTAB.state(st || "na") : ""; return x.textContent || ""; };
    const order = ss.map((s, i) => i).sort((p, q) => klass(ss[p]) - klass(ss[q]) || (+(ss[p].time ?? 0)) - (+(ss[q].time ?? 0)));
    let mx = `<div class="tpr-mx" role="group" aria-label="Samples by group">`;
    for (const k of [0, 1, 2, 3]) {
      const ix = order.filter(i => klass(ss[i]) === k); if (!ix.length) continue;
      mx += `<div class="tpr-grp"><b>${GNAME[k]}</b><div class="tpr-row">${ix.map(i => {
        const s = ss[i], has = fin(s.area) && /** @type {number} */ (s.area) > 0, u = has ? (lmax > lmin ? (Math.log10(/** @type {number} */ (s.area)) - lmin) / (lmax - lmin) : 0.5) : null;
        const lab = `${s.file}: area ${fin(s.area) ? sci(/** @type {number} */ (s.area)) : "none"}, ${wordOf(s.state)}`;
        return `<span class="tpr-cell${u == null ? "" : u < 0.5 ? " on-w" : " on-k"}" role="img" aria-label="${esc(lab)}"${u == null ? "" : ` style="background:${ramp("c", u)}"`}><span aria-hidden="true">${esc(sym(s.state))}</span></span>`;
      }).join("")}</div></div>`;
    }
    const hasNorm = ss.some(s => fin(s.norm));
    const mxBox = document.createElement("div"); mxBox.innerHTML = mx + `</div>`; const mxEl = /** @type {HTMLElement} */ (mxBox.firstElementChild); panel.appendChild(mxEl);
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

  /** @param {HTMLElement} panel @param {TpRiga} m */
  function identity(panel, m) {
    const id = m.identity;
    if (G.TPDEC && m.decide) { const p = document.createElement("p"); p.className = "tpr-id"; p.append("Decided level ", G.TPDEC.control(m.decide.series, { ...m.decide, name: m.title }, null)); panel.appendChild(p); }
    if (!id) { none(panel, "Identity", m.missing && m.missing.identity); return; }
    const cs = id.candidates || [];
    panel.insertAdjacentHTML("beforeend", `<p class="tpr-id"><b>Neutral mass</b> ${esc(fin(id.neutral) ? /** @type {number} */ (id.neutral).toFixed(4) : id.neutral ?? "–")} · <span${G.TPHELP ? ` tabindex="0"` : ""}${tt("ft.adduct")}><b>Adduct</b> ${esc(id.adduct || "–")}</span></p>`);
    if (!cs.length) { none(panel, "Candidates", m.missing && m.missing.candidates || "no candidate for this row"); return; }
    const one = (/** @type {TpCand} */ c) => {
      const lv = c.level != null && c.level !== "" && G.TPTAB ? G.TPTAB.level(c.level) : "", pd = fin(c.pred) && G.TPTAB ? G.TPTAB.pred(/** @type {number} */ (c.pred)) : "";
      const pf = (c.proofs || []).length ? `<ul class="tpr-pf">${(c.proofs || []).map(p => `<li><span class="${p.ok ? "ok" : "no"}" role="img" aria-label="${p.ok ? "passed" : "failed"}">${p.ok ? "✓" : "✗"}</span><span>${esc(p.text)}</span></li>`).join("")}</ul>` : "";
      return `<tr><td>${esc(c.name || c.formula || "–")}${c.name && c.formula ? ` <span class="muted">${esc(c.formula)}</span>` : ""}</td><td class="n">${fix(c.ppm, 1)}</td><td>${esc(c.adduct || "")}</td><td>${esc(c.source || "")}</td><td>${lv} ${pd}</td></tr>${pf ? `<tr><td colspan="5">${pf}</td></tr>` : ""}`;
    };
    const SHOW = 3, shown = cs.slice(0, SHOW), rest = cs.slice(SHOW);
    const tw = document.createElement("div"); tw.className = "tpr-tw" + (cs.length > 10 ? " sc" : "");
    tw.innerHTML = `<table class="tpr"><caption class="sr-only">Candidates</caption><thead><tr><th scope="col">Candidate</th><th scope="col" class="n">Δppm</th><th scope="col">Adduct</th><th scope="col">Source</th><th scope="col">Level</th></tr></thead><tbody>${shown.map(one).join("")}</tbody></table>`;
    panel.appendChild(tw);
    if (rest.length) {
      const bt = document.createElement("button"); bt.type = "button"; bt.className = "tpr-more"; bt.textContent = `+${rest.length}`; bt.setAttribute("aria-expanded", "false"); bt.setAttribute("aria-label", `Show ${rest.length} more candidates`);
      bt.addEventListener("click", () => { const open = bt.getAttribute("aria-expanded") === "true"; const tb = /** @type {HTMLElement} */ (tw.querySelector("tbody")); tb.innerHTML = (open ? shown : cs).map(one).join(""); bt.setAttribute("aria-expanded", String(!open)); bt.textContent = open ? `+${rest.length}` : "−"; bt.setAttribute("aria-label", open ? `Show ${rest.length} more candidates` : "Show fewer candidates"); });
      panel.appendChild(bt);
    }
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

  /** draws the tab `tab` of the model `m` into `panel` (which it empties)
   * @param {HTMLElement} panel @param {string} tab @param {TpRiga} m */
  function fill(panel, tab, m) {
    register(); panel.innerHTML = "";
    if (tab === "samples") samples(panel, m); else if (tab === "identity") identity(panel, m); else if (tab === "quality") quality(panel, m); else overview(panel, m);
  }

  /** the options of `TPTAB.mount` that put these panels in the open row: `TPRIGA.hook(r => model)` @param {(r: any) => TpRiga} model */
  function hook(model) { return { tabs: TABS, head: (/** @type {any} */ r, /** @type {HTMLElement} */ box) => head(box, model(r)), onOpen: (/** @type {any} */ r, /** @type {HTMLElement} */ p, /** @type {string} */ t) => fill(p, t, model(r)) }; }

  if (!document.getElementById("tpr-css")) { const s = document.createElement("style"); s.id = "tpr-css"; s.textContent = CSS; document.head.appendChild(s); }
  G.TPRIGA = { tabs: TABS, head, fill, slot, hook, texts: TXT };
})();
