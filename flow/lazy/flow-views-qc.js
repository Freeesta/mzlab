// @ts-check
// Flow (administrator mode): the view of a massacc/1 (qc.mass_accuracy): one row per file (offset, spread, residual, the warnings), the mass error of every reference ion against RT and against m/z with the data table of each chart,
// and the standards over the files. It only shows the numbers the step measured: no verdict. Colours are CSS variables (Okabe-Ito --gv1..8) and each file also has its own marker shape; the charts zoom and move with buttons.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const NS = "http://www.w3.org/2000/svg";
  const fx = (/** @type {any} */ n, d = 2) => n === null || n === undefined || n === "" || !Number.isFinite(Number(n)) ? "" : String(Math.round(Number(n) * 10 ** d) / 10 ** d);
  const WORDS = /** @type {Record<string, string>} */ ({
    "warn.qc.mass.offset": "mass offset above the limit", "warn.qc.mass.spread": "spread above the limit", "warn.qc.mass.is_rt": "internal standard retention shifted",
    "warn.qc.mass.is_area": "internal standard area varies", "warn.qc.mass.few_refs": "too few reference ions to assess the file",
  });
  /** marker shapes, one per file (the second cue after colour): paths centred on 0,0 */
  const SHAPES = ["M-4 -4h8v8h-8z", "M0 -5L5 4H-5z", "M0 -5L5 0L0 5L-5 0z", "M-5 -1.5h3.5V-5h3v3.5H5v3H1.5V5h-3V1.5H-5z", "M-5 4L0 -5L5 4z M-5 -4H5L0 5z", "M-4.5 -4.5L4.5 4.5M4.5 -4.5L-4.5 4.5", "M0 -5V5M-5 0H5", "M-4 -4h8v8h-8z M-1.5 -1.5h3v3h-3z"];

  const sv = (/** @type {string} */ tag, /** @type {Record<string, any>} */ attrs) => { const e = document.createElementNS(NS, tag); for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, String(v)); return e; };
  /** `n` ticks over [a, b] at round values @param {number} a @param {number} b @param {number} n */
  const ticks = (a, b, n) => {
    const raw = (b - a) / Math.max(1, n), mag = 10 ** Math.floor(Math.log10(raw || 1)), step = [1, 2, 5, 10].map(m => m * mag).find(s => s >= raw) || mag, out = [];
    for (let v = Math.ceil(a / step) * step; v <= b + step * 1e-9; v += step) out.push(Math.round(v / step) * step);
    return out;
  };

  /** a scatter of the error of the reference ions: `series` = [{ name, k (file number), points: [[x, y, label]] }]; the window of x is moved and zoomed with buttons, y follows the points in view
   * @param {HTMLElement} host @param {{ title: string, xl: string, yl: string, series: { name: string, k: number, points: number[][] }[], head: string[], rows: string[][], digits: number }} o */
  const scatter = (host, o) => {
    const { el } = F, W = 640, H = 260, L = 52, R = 12, T = 12, B = 34, all = o.series.flatMap(s => s.points);
    const box = el("figure", { class: "flx-one" }), svg = sv("svg", { viewBox: `0 0 ${W} ${H}`, class: "flx-sticks", role: "img", "aria-label": o.title });
    if (!all.length) { host.append(el("p", { class: "fl-note", text: `${o.title}: no reference ion was found.` })); return; }
    const xs = all.map(p => p[0]), lo = Math.min(...xs), hi = Math.max(...xs), span = (hi - lo) || 1;
    /** @type {[number, number]} */ let win = [lo - span * 0.04, hi + span * 0.04];
    const home = /** @type {[number, number]} */ ([...win]);
    const draw = () => {
      svg.replaceChildren();
      const inside = all.filter(p => p[0] >= win[0] && p[0] <= win[1]), top = Math.max(1, ...inside.map(p => Math.abs(p[1]))) * 1.1;
      const x = (/** @type {number} */ v) => L + (v - win[0]) / (win[1] - win[0]) * (W - L - R), y = (/** @type {number} */ v) => T + (1 - (v + top) / (2 * top)) * (H - T - B);
      svg.append(sv("line", { x1: L, y1: H - B, x2: W - R, y2: H - B, class: "flx-ax" }), sv("line", { x1: L, y1: T, x2: L, y2: H - B, class: "flx-ax" }), sv("line", { x1: L, y1: y(0), x2: W - R, y2: y(0), class: "flx-prec" }));
      for (const t of ticks(win[0], win[1], 6)) { const e = sv("text", { x: x(t), y: H - B + 14, "text-anchor": "middle", class: "flx-tick" }); e.textContent = fx(t, 2); svg.append(e); }
      for (const t of ticks(-top, top, 5)) { const e = sv("text", { x: L - 5, y: y(t) + 4, "text-anchor": "end", class: "flx-tick" }); e.textContent = fx(t, 1); svg.append(e); }
      const xt = sv("text", { x: (L + W - R) / 2, y: H - 3, "text-anchor": "middle", class: "flx-tick" }); xt.textContent = o.xl;
      const yt = sv("text", { x: 12, y: (T + H - B) / 2, "text-anchor": "middle", class: "flx-tick", transform: `rotate(-90 12 ${(T + H - B) / 2})` }); yt.textContent = o.yl;
      svg.append(xt, yt);
      for (const s of o.series) for (const p of s.points) {
        if (p[0] < win[0] || p[0] > win[1]) continue;
        const colour = `var(--gv${(s.k % 8) + 1})`, g = sv("g", { transform: `translate(${x(p[0]).toFixed(1)} ${y(p[1]).toFixed(1)})` });
        g.append(sv("path", { d: SHAPES[s.k % 8], fill: s.k % 8 >= 4 ? "none" : colour, stroke: colour, "stroke-width": 1.6 }));
        const t = sv("title", {}); t.textContent = `${s.name}: ${fx(p[0], o.digits)}, ${fx(p[1], 2)} ppm`; g.append(t); svg.append(g);
      }
    };
    /** @param {number} f zoom factor around the centre */
    const zoom = f => { const c = (win[0] + win[1]) / 2, h = (win[1] - win[0]) / 2 * f; win = [c - h, c + h]; draw(); };
    /** @param {number} f share of the window to move by */
    const move = f => { const d = (win[1] - win[0]) * f; win = [win[0] + d, win[1] + d]; draw(); };
    const btn = (/** @type {string} */ text, /** @type {string} */ name, /** @type {() => void} */ fn) => el("button", { type: "button", class: "fl-b", style: "min-width:32px;min-height:32px", "aria-label": name, title: name, onclick: fn, text });
    const legend = el("ul", { class: "fl-note", "aria-label": "Files of the chart", style: "list-style:none;display:flex;flex-wrap:wrap;gap:4px 12px;padding:0;margin:4px 0" },
      ...o.series.map(s => { const m = sv("svg", { width: 14, height: 14, viewBox: "-7 -7 14 14", "aria-hidden": "true" }); const c = `var(--gv${(s.k % 8) + 1})`; m.append(sv("path", { d: SHAPES[s.k % 8], fill: s.k % 8 >= 4 ? "none" : c, stroke: c, "stroke-width": 1.6 })); return el("li", {}, m, ` ${s.name}`); }));
    box.append(el("figcaption", { class: "fl-note", text: o.title }), svg, el("div", { class: "flx-live-ctl", role: "group", "aria-label": `Zoom and move: ${o.title}` },
      btn("+", "Zoom in", () => zoom(0.5)), btn("−", "Zoom out", () => zoom(2)), btn("←", "Move left", () => move(-0.25)), btn("→", "Move right", () => move(0.25)), btn("Reset", "Show everything", () => { win = [...home]; draw(); })), legend);
    host.append(box); draw();
    if (G.TDATI) {
      G.TDATI.addButton(svg, () => ({ title: o.title, head: o.head, rows: o.rows, rowHeader: false }));
      G.TDATI.describe(svg, `${o.title}: ${all.length} reference ions in ${o.series.length} files, error in ppm against ${o.xl}.`);
      const open = /** @type {HTMLElement | undefined} */ ((/** @type {any} */ (svg))._tdBtn);
      if (open) { open.style.minHeight = "32px"; open.style.minWidth = "32px"; }
    } else F.drawTable(host, { caption: `${o.title}, data`, head: o.head, rows: o.rows, label: `Data of ${o.title}` });
  };

  /** the corrected table, when this browser kept it: a button that saves it @param {HTMLElement} host @param {string} text */
  const tableButton = async (host, text) => {
    const k = F.runners && F.runners.web && F.runners.web.kit, p = k && k.poolIfAny && k.poolIfAny();
    if (!p || !F.webQc) return;
    const path = F.webQc.tablePath(F.webQc.hex(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text))));
    if (!await F.liveJobs.has(p, path)) return;
    host.append(F.btn("export", "Save the table with the m/z corrected", async () => {
      const a = document.createElement("a"), url = URL.createObjectURL(new Blob([await p.fs.read(path)], { type: "application/octet-stream" }));
      a.href = url; a.download = "corrected.mzfeat"; a.click(); setTimeout(() => URL.revokeObjectURL(url), 1000);
    }, "Save the corrected table"));
  };

  F.registerView({ id: "massacc", title: "Mass accuracy", types: ["qc"], accepts: (/** @type {any} */ item) => /\.json$/i.test(item.label), open: async (/** @type {HTMLElement} */ host, /** @type {any} */ item) => {
    const { el } = F, text = await item.text(), j = JSON.parse(text);
    if (j.format !== "massacc/1") { host.append(el("p", { class: "fl-note", text: `${item.label} is not a mass accuracy report.` })); return; }
    const files = j.files || [];
    F.drawTable(host, { caption: `${item.label}: mass error by file`, label: `Mass error by file in ${item.label}`,
      head: ["File", "MS1 scans", "Reference ions", "Offset (ppm)", "Spread (ppm)", "Residual after the model (ppm)", "Tolerance that holds (ppm)", "Model", "Corrected", "Warnings"],
      rows: files.map((/** @type {any} */ f) => [f.file, String(f.ms1_scans), f.assessable ? String(f.n_refs) : `${f.n_refs} (not assessable)`, fx(f.offset_ppm), fx(f.spread_ppm), fx(f.residual_ppm), fx(f.tolerance_ppm, 0), f.model || "", f.corrected ? "yes" : "no", (f.warnings || []).map((/** @type {string} */ w) => WORDS[w] || w).join("; ")]) });
    const rows = (/** @type {(f: any, p: any) => (string|number)[]} */ cell) => files.flatMap((/** @type {any} */ f) => (f.points || []).map((/** @type {any} */ p) => cell(f, p)));
    const series = (/** @type {(p: any) => number} */ xOf) => files.map((/** @type {any} */ f, /** @type {number} */ k) => ({ name: f.file, k, points: (f.points || []).map((/** @type {any} */ p) => [xOf(p), p.err_ppm]) }));
    const head = ["File", "Reference ion", "Role", "m/z", "RT (min)", "Error (ppm)", "Intensity", "Outlier"];
    const data = rows((f, p) => [f.file, p.name, p.role, fx(p.mz, 4), fx(p.rt, 3), fx(p.err_ppm, 2), fx(p.intensity, 0), p.outlier ? "yes" : "no"]);
    scatter(host, { title: "Mass error against RT", xl: "RT (min)", yl: "Error (ppm)", series: series((/** @type {any} */ p) => p.rt), head, rows: data, digits: 3 });
    scatter(host, { title: "Mass error against m/z", xl: "m/z", yl: "Error (ppm)", series: files.map((/** @type {any} */ f, /** @type {number} */ k) => ({ name: f.file, k, points: (f.points || []).map((/** @type {any} */ p) => [p.mz, p.err_ppm]) })), head, rows: data, digits: 4 });
    const std = (j.batch && j.batch.istd_area) || [];
    if (std.length) F.drawTable(host, { caption: "Internal standards over the files", label: "Internal standards over the files", head: ["Standard", "Files", "Median area", "Relative spread"], rows: std.map((/** @type {any} */ s) => [s.name, String(s.files), fx(s.median_area, 0), fx(s.rsd, 3)]) });
    const bar = el("div", { class: "fl-results" });
    host.append(bar);
    await tableButton(bar, text);
  } });
})();
