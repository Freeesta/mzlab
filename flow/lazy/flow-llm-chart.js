// @ts-check
// Flow (administrator mode): draws a chart of the assistant (`grafico/1` with its rows, docs/agenti/llm.md) as SVG: line, point, stick, bar, box, hist and heat marks, axes with units,
// Okabe-Ito series from the CSS variables --oi1..8 plus a dash or a shape per series (readable without colour), log scale, panels per facet. It decides nothing: Rust chose the view, the fields and the
// rows. The figure around it (table, zoom, CSV) is flow-llm-chart-ui.js. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const NS = "http://www.w3.org/2000/svg";
  const W = 640, H = 300, M = { l: 62, r: 14, t: 12, b: 46 };
  const DASH = ["", "7 3", "2 3", "9 3 2 3", "", "7 3", "2 3", "9 3 2 3"];
  const SHAPES = ["M-4 0a4 4 0 1 0 8 0a4 4 0 1 0-8 0", "M-4-4h8v8h-8z", "M0-5L5 4h-10z", "M0-5L5 0 0 5-5 0z", "M-4-1h3v-3h2v3h3v2h-3v3h-2v-3h-3z", "M-4-4L4 4M4-4L-4 4", "M-5 3h10L0-5z", "M-4 0h8M0-4v8"];

  const CSS = `
.flx-chart{--oi1:#0072B2;--oi2:#E69F00;--oi3:#009E73;--oi4:#D55E00;--oi5:#CC79A7;--oi6:#56B4E9;--oi7:#9a8a00;--oi8:#555;color:var(--ink)}
@media (prefers-color-scheme:dark){:root:not([data-theme="light"]) .flx-chart{--oi1:#56B4E9;--oi2:#F0E442;--oi3:#34d399;--oi4:#fb923c;--oi5:#f472b6;--oi6:#38bdf8;--oi7:#fde047;--oi8:#cbd5e1}}
:root[data-theme="dark"] .flx-chart{--oi1:#56B4E9;--oi2:#F0E442;--oi3:#34d399;--oi4:#fb923c;--oi5:#f472b6;--oi6:#38bdf8;--oi7:#fde047;--oi8:#cbd5e1}
.flx-chart svg{width:100%;height:auto;display:block;background:var(--bg)}.flx-chart .ax{stroke:var(--muted);stroke-width:1;fill:none}.flx-chart .gr{stroke:var(--line);stroke-width:.5}
.flx-chart text{fill:var(--ink);font-size:10px}.flx-chart .lb{fill:var(--muted)}.flx-chart .sr{fill:none;stroke-width:1.6}.flx-chart .fl{stroke:none}`;

  /** @param {string} tag @param {Record<string, any>} [a] @param {...(Node | string | null)} kids */
  const sv = (tag, a = {}, ...kids) => {
    const n = document.createElementNS(NS, tag);
    for (const k in a) n.setAttribute(k, String(a[k]));
    for (const c of kids) if (c) n.append(c);
    return n;
  };
  /** a short number for a tick @param {number} v */
  const fmt = v => (Math.abs(v) >= 1e5 || (v !== 0 && Math.abs(v) < 1e-2) ? v.toExponential(1) : String(+v.toPrecision(4)));
  /** about `n` round ticks between lo and hi @param {number} lo @param {number} hi @param {number} n */
  const ticks = (lo, hi, n = 5) => {
    if (!(hi > lo)) return [lo];
    const raw = (hi - lo) / n, p = 10 ** Math.floor(Math.log10(raw)), f = raw / p, step = (f < 1.5 ? 1 : f < 3.5 ? 2 : f < 7.5 ? 5 : 10) * p;
    const out = [];
    for (let v = Math.ceil(lo / step) * step; v <= hi + step * 1e-9; v += step) out.push(+v.toPrecision(12));
    return out;
  };

  /** the data of a chart by field name: `col(name)` gives the values, `field(name)` its description @param {any} chart @param {any[][]} rows */
  const frame = (chart, rows) => {
    const names = chart.data.fields.map((/** @type {any} */ f) => f.name);
    return {
      rows,
      field: (/** @type {string} */ n) => chart.data.fields[names.indexOf(n)],
      col: (/** @type {string} */ n) => rows.map(r => r[names.indexOf(n)]),
      at: (/** @type {string} */ n) => names.indexOf(n),
    };
  };

  /** a linear or log scale from a domain to a pixel range @param {number} d0 @param {number} d1 @param {number} r0 @param {number} r1 @param {boolean} [log] */
  const scale = (d0, d1, r0, r1, log = false) => {
    const t = log ? Math.log10 : (/** @type {number} */ v) => v, a = t(d0), b = t(d1) === a ? a + 1 : t(d1);
    return (/** @type {number} */ v) => r0 + ((t(v) - a) / (b - a)) * (r1 - r0);
  };

  /** the distinct series of a frame: key, index, style @param {any} chart @param {ReturnType<typeof frame>} fr */
  const seriesOf = (chart, fr) => {
    const f = chart.encoding.color && chart.mark !== "heat" ? chart.encoding.color.field : chart.encoding.detail ? chart.encoding.detail.field : null;
    /** @type {Map<string, any[][]>} */ const m = new Map();
    for (const r of fr.rows) { const k = f ? String(r[fr.at(f)]) : ""; (m.get(k) || m.set(k, []).get(k) || []).push(r); }
    return { byColour: !!(chart.encoding.color && chart.mark !== "heat"), list: [...m].map(([key, rows], i) => ({ key, rows, i })) };
  };
  /** the attributes of a series line: colour from the variable, dash by index; a series beyond the colours (detail) is drawn neutral @param {boolean} coloured @param {number} i */
  const look = (coloured, i) => ({ stroke: coloured ? `var(--oi${(i % 8) + 1})` : "var(--oi8)", "stroke-dasharray": coloured ? DASH[i % 8] : "", "stroke-opacity": coloured ? 1 : 0.55 });

  /** axes, grid and labels around a plot area. `xs` is a function of the x tick value, `cat` the labels when x is nominal @param {SVGElement} g @param {any} o */
  const axes = (g, o) => {
    const { xt, yt, xs, ys, xl, yl, cat } = o;
    g.append(sv("rect", { x: M.l, y: M.t, width: W - M.l - M.r, height: H - M.t - M.b, fill: "none", class: "ax" }));
    for (const v of yt) g.append(sv("line", { x1: M.l, x2: W - M.r, y1: ys(v), y2: ys(v), class: "gr" }), sv("text", { x: M.l - 4, y: ys(v) + 3, "text-anchor": "end" }, fmt(v)));
    if (cat) cat.forEach((/** @type {string} */ c, /** @type {number} */ i) => { if (cat.length <= 24 || i % Math.ceil(cat.length / 24) === 0) g.append(sv("text", { x: xs(i), y: H - M.b + 12, "text-anchor": "middle", class: "lb" }, c.length > 12 ? c.slice(0, 11) + "…" : c)); });
    else for (const v of xt) g.append(sv("line", { x1: xs(v), x2: xs(v), y1: M.t, y2: H - M.b, class: "gr" }), sv("text", { x: xs(v), y: H - M.b + 12, "text-anchor": "middle" }, fmt(v)));
    g.append(sv("text", { x: (M.l + W - M.r) / 2, y: H - 6, "text-anchor": "middle", class: "lb" }, xl), sv("text", { transform: `translate(12 ${(M.t + H - M.b) / 2}) rotate(-90)`, "text-anchor": "middle", class: "lb" }, yl));
  };
  /** @param {any} f field */
  const label = f => (f ? f.name + (f.unit ? ` (${f.unit})` : "") : "");

  /** a point marker at (x, y) of series i @param {number} x @param {number} y @param {number} i @param {boolean} coloured */
  const marker = (x, y, i, coloured) => sv("path", { d: SHAPES[i % 8], transform: `translate(${x.toFixed(1)} ${y.toFixed(1)})`, fill: i % 8 === 5 || i % 8 === 7 ? "none" : coloured ? `var(--oi${(i % 8) + 1})` : "var(--oi8)", stroke: coloured ? `var(--oi${(i % 8) + 1})` : "var(--oi8)", "stroke-width": 1.2 });

  /** line, point and stick marks over a continuous x @param {SVGElement} g @param {any} chart @param {ReturnType<typeof frame>} fr @param {any} xs @param {any} ys */
  const xyMarks = (g, chart, fr, xs, ys) => {
    const { byColour, list } = seriesOf(chart, fr), xi = fr.at(chart.encoding.x.field), yi = fr.at(chart.encoding.y.field), base = ys.base;
    for (const s of list) {
      const pts = s.rows.filter(r => Number.isFinite(r[yi]) && (!chart.log_y || r[yi] > 0)).map(r => [xs(r[xi]), ys(r[yi])]);
      if (chart.mark === "line") g.append(sv("path", { d: pts.map((p, k) => (k ? "L" : "M") + p[0].toFixed(1) + " " + p[1].toFixed(1)).join(""), class: "sr", ...look(byColour, s.i) }));
      else if (chart.mark === "stick") g.append(sv("path", { d: pts.map(p => `M${p[0].toFixed(1)} ${base}V${p[1].toFixed(1)}`).join(""), class: "sr", ...look(byColour, s.i) }));
      else for (const p of pts) g.append(marker(p[0], p[1], s.i, byColour));
    }
  };

  /** the categories of a nominal x, in order of appearance @param {ReturnType<typeof frame>} fr @param {string} name */
  const categories = (fr, name) => [...new Set(fr.col(name).map(String))];

  /** the quartiles of a sorted list @param {number[]} v */
  const quart = v => {
    const q = (/** @type {number} */ p) => { const k = (v.length - 1) * p, a = Math.floor(k); return v[a] + (v[Math.min(a + 1, v.length - 1)] - v[a]) * (k - a); };
    const q1 = q(0.25), q3 = q(0.75), lo = q1 - 1.5 * (q3 - q1), hi = q3 + 1.5 * (q3 - q1);
    return { q1, med: q(0.5), q3, min: v.find(x => x >= lo) ?? v[0], max: [...v].reverse().find(x => x <= hi) ?? v[v.length - 1] };
  };

  /** bar and box marks over categories @param {SVGElement} g @param {any} chart @param {ReturnType<typeof frame>} fr @param {string[]} cats @param {any} xs @param {any} ys @param {number} bw */
  const catMarks = (g, chart, fr, cats, xs, ys, bw) => {
    const xi = fr.at(chart.encoding.x.field), yi = fr.at(chart.encoding.y.field);
    cats.forEach((c, k) => {
      const v = fr.rows.filter(r => String(r[xi]) === c && Number.isFinite(r[yi])).map(r => r[yi]).sort((a, b) => a - b);
      if (!v.length) return;
      const x = xs(k);
      if (chart.mark === "bar") {
        const top = ys(v.reduce((a, b) => a + b, 0));
        g.append(sv("rect", { x: x - bw / 2, y: Math.min(top, ys.base), width: bw, height: Math.abs(ys.base - top), fill: "var(--oi1)", class: "fl" }));
        return;
      }
      const b = quart(v);
      g.append(sv("rect", { x: x - bw / 2, y: ys(b.q3), width: bw, height: Math.max(1, ys(b.q1) - ys(b.q3)), fill: "var(--oi1)", "fill-opacity": 0.35, stroke: "var(--oi1)", "stroke-width": 1.4 }),
        sv("path", { d: `M${x - bw / 2} ${ys(b.med)}h${bw}M${x} ${ys(b.q3)}V${ys(b.max)}M${x} ${ys(b.q1)}V${ys(b.min)}`, stroke: "var(--oi1)", "stroke-width": 2, fill: "none" }));
    });
  };

  /** histogram: bins over the range, one bar per bin, series overlaid with transparency @param {SVGElement} g @param {any} chart @param {ReturnType<typeof frame>} fr */
  const histogram = (g, chart, fr) => {
    const xi = fr.at(chart.encoding.x.field), all = fr.rows.map(r => r[xi]).filter(Number.isFinite);
    const lo = Math.min(...all), hi = Math.max(...all), n = Math.max(5, Math.min(30, Math.ceil(Math.log2(all.length + 1)) + 1)), w = (hi - lo || 1) / n;
    const { byColour, list } = seriesOf(chart, fr);
    const counts = list.map(s => { const c = Array(n).fill(0); for (const r of s.rows) if (Number.isFinite(r[xi])) c[Math.min(n - 1, Math.floor((r[xi] - lo) / w))]++; return c; });
    const xs = scale(lo, lo + w * n, M.l, W - M.r), top = Math.max(1, ...counts.flat()), ys = scale(0, top, H - M.b, M.t);
    axes(g, { xt: ticks(lo, lo + w * n), yt: ticks(0, top), xs, ys, xl: label(fr.field(chart.encoding.x.field)), yl: "count" });
    list.forEach((s, k) => counts[k].forEach((c, b) => { if (c) g.append(sv("rect", { x: xs(lo + b * w) + 0.5, y: ys(c), width: Math.max(1, xs(lo + (b + 1) * w) - xs(lo + b * w) - 1), height: ys(0) - ys(c), fill: byColour ? `var(--oi${(s.i % 8) + 1})` : "var(--oi1)", "fill-opacity": list.length > 1 ? 0.5 : 0.85, class: "fl" })); }));
  };

  /** heat map: nominal x and y, the quantity in `color` as the opacity of the cell @param {SVGElement} g @param {any} chart @param {ReturnType<typeof frame>} fr */
  const heat = (g, chart, fr) => {
    const [x, y, c] = [chart.encoding.x.field, chart.encoding.y.field, chart.encoding.color.field], cx = categories(fr, x), cy = categories(fr, y), ci = fr.at(c);
    const vals = fr.rows.map(r => r[ci]).filter(Number.isFinite), lo = Math.min(...vals), hi = Math.max(...vals);
    const xs = scale(0, cx.length, M.l, W - M.r), ys = scale(0, cy.length, M.t, H - M.b);
    axes(g, { xt: [], yt: [], xs: (/** @type {number} */ i) => xs(i + 0.5), ys, xl: x, yl: y, cat: cx });
    const xi = fr.at(x), yi = fr.at(y);
    for (const r of fr.rows) {
      const a = cx.indexOf(String(r[xi])), b = cy.indexOf(String(r[yi]));
      g.append(sv("rect", { x: xs(a), y: ys(b), width: Math.max(1, xs(1) - xs(0)), height: Math.max(1, ys(1) - ys(0)), fill: "var(--oi4)", "fill-opacity": hi > lo ? 0.08 + 0.92 * ((r[ci] - lo) / (hi - lo)) : 0.6, class: "fl" }));
    }
  };

  /** the extent of the numbers of a column, widened when flat @param {number[]} v @param {boolean} [log] */
  const extent = (v, log = false) => {
    const a = log ? v.filter(x => x > 0) : v.filter(Number.isFinite);
    if (!a.length) return [0, 1];
    const lo = Math.min(...a), hi = Math.max(...a);
    return hi > lo ? [lo, hi] : [lo - 1, hi + 1];
  };

  /** one panel (the whole chart or one facet): the SVG with its marks @param {any} chart @param {any[][]} rows @param {string} [title] */
  const panel = (chart, rows, title) => {
    const fr = frame(chart, rows), g = sv("g"), enc = chart.encoding, mark = chart.mark;
    if (mark === "hist") histogram(g, chart, fr);
    else if (mark === "heat") heat(g, chart, fr);
    else {
      const yf = fr.field(enc.y.field), xf = fr.field(enc.x.field), nominalX = xf.type === "nominal", cats = nominalX ? categories(fr, enc.x.field) : [];
      const [y0, y1] = extent(fr.col(enc.y.field), !!chart.log_y), lo = chart.log_y ? y0 : mark === "line" || mark === "point" ? y0 : Math.min(0, y0);
      const ys = Object.assign(scale(lo, y1, H - M.b, M.t, !!chart.log_y), { base: H - M.b });
      let xs, xt = [];
      if (nominalX) xs = scale(-0.5, cats.length - 0.5, M.l, W - M.r);
      else { const [a, b] = extent(fr.col(enc.x.field)); xs = scale(a, b, M.l, W - M.r); xt = ticks(a, b); }
      axes(g, { xt, yt: chart.log_y ? [y0, y1].map(v => +v.toPrecision(2)) : ticks(lo, y1), xs, ys, xl: label(xf), yl: label(yf), cat: nominalX ? cats : null });
      if (nominalX) catMarks(g, chart, fr, cats, xs, ys, Math.max(4, Math.min(60, ((W - M.l - M.r) / cats.length) * 0.7)));
      else xyMarks(g, chart, fr, xs, ys);
    }
    const s = sv("svg", { viewBox: `0 0 ${W} ${H}`, role: "img", "aria-label": `${chart.title}${title ? " — " + title : ""}. ${chart.why}` }, g);
    if (title) g.append(sv("text", { x: M.l + 6, y: M.t + 12, style: "font-weight:600" }, title));
    return s;
  };

  /** the legend of the series (colour, dash and shape): HTML so that it wraps and reads aloud @param {any} chart @param {any[][]} rows */
  const legend = (chart, rows) => {
    const fr = frame(chart, rows), { byColour, list } = seriesOf(chart, fr);
    if (!byColour || list.length < 2) return null;
    const ul = F.el("ul", { class: "flx-chart-legend", "aria-label": "Series" });
    for (const s of list) {
      const key = sv("svg", { width: 28, height: 12, "aria-hidden": "true" }, sv("path", { d: "M0 6H28", class: "sr", ...look(true, s.i) }), marker(14, 6, s.i, true));
      ul.append(F.el("li", {}, key, F.el("span", { text: " " + s.key })));
    }
    return ul;
  };

  /** the SVG panels of a chart: one, or one per value of the facet @param {any} chart @param {any[][]} rows @returns {{panels: SVGElement[], legend: HTMLElement | null}} */
  const drawChart = (chart, rows) => {
    const style = document.createElement("style"); style.textContent = CSS;
    if (!document.querySelector("style[data-flx-chart]")) { style.dataset.flxChart = "1"; document.head.append(style); }
    const facet = chart.facet, fr = frame(chart, rows);
    if (!facet) return { panels: [panel(chart, rows)], legend: legend(chart, rows) };
    const fi = fr.at(facet), vals = categories(fr, facet);
    return { panels: vals.map(v => panel(chart, rows.filter(r => String(r[fi]) === v), v)), legend: legend(chart, rows) };
  };
  F.llmChartDraw = drawChart;
  F.llmChartExtent = extent;
})();
