// @ts-check
// Flow (administrator mode): the figure around a chart of the assistant (`grafico/1`): title and reason, the SVG panels of flow-llm-chart.js, zoom and pan with buttons and keys (+ - [ ] 0, no
// dragging), a legend, the table of the same data under the chart (every row, same numbers), «Download CSV», and the «Charts without the model» menu (the charts Rust builds from the open files).
// English only; the colours come from CSS variables, never from the model.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const CSS = `
.flx-chart{border:1px solid var(--line);border-radius:8px;padding:6px;background:var(--panel);display:flex;flex-direction:column;gap:4px}.flx-chart:focus-visible{outline:2px solid var(--accent)}
.flx-chart h4{margin:0;font-size:13px}.flx-chart p{margin:0;font-size:12px;color:var(--muted)}.flx-chart .row{display:flex;flex-wrap:wrap;gap:4px;align-items:center}
.flx-chart .fl-b,.flx-chart summary{min-width:24px;min-height:24px;cursor:pointer}@media (pointer:coarse){.flx-chart .fl-b,.flx-chart summary{min-width:40px;min-height:40px}}
.flx-chart-legend{display:flex;flex-wrap:wrap;gap:2px 10px;margin:0;padding:0;list-style:none;font-size:11px}.flx-chart-legend li{display:inline-flex;align-items:center;gap:2px}
.flx-chart-table{max-height:180px;overflow:auto}.flx-chart table{border-collapse:collapse;font-size:11px;width:100%}.flx-chart th,.flx-chart td{border:1px solid var(--line);padding:1px 5px;text-align:right}
.flx-chart th{position:sticky;top:0;background:var(--soft)}.flx-chart caption{text-align:left;font-weight:600}
.flx-chart-presets .row{margin-top:4px}`;
  const PRESETS = /** @type {Record<string, [string, string]>} */ ({
    tic: ["TIC of all files", "The TIC of every open file, overlaid."],
    width: ["Peak widths", "How wide the detected peaks are, in seconds."],
    drift: ["RT drift of the QC", "How far the retention time of the QC injections moved from the first one."],
    cv: ["CV of the QC", "The spread of the QC areas of each row, in percent."],
    blank: ["Sample to blank ratio", "How much larger the sample area is than the blank area, per row."],
  });
  let count = 0;
  /** a button that shows a text (no icon in the set): its accessible name is `name` @param {string} name @param {string} text @param {(e: Event) => void} fn */
  const tb = (name, text, fn) => F.el("button", { type: "button", class: "fl-b", title: name, "aria-label": name, onclick: fn, text });
  const style = () => {
    if (document.querySelector("style[data-flx-chart-ui]")) return;
    const s = document.createElement("style"); s.dataset.flxChartUi = "1"; s.textContent = CSS; document.head.append(s);
  };

  /** the CSV text of the rows of a chart, with the units in the header @param {any} chart */
  const csv = chart => {
    const q = (/** @type {any} */ v) => (typeof v === "string" && /[",\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : String(v));
    return [chart.data.fields.map((/** @type {any} */ f) => q(f.name + (f.unit ? ` (${f.unit})` : ""))).join(","), ...chart.data.rows.map((/** @type {any[]} */ r) => r.map(q).join(","))].join("\n") + "\n";
  };

  /** the table of the data: caption, header with units, every row @param {any} chart */
  const dataTable = chart => {
    const { el } = F, t = el("table", { "aria-label": `Data of ${chart.title}` }, el("caption", { text: chart.title }));
    t.append(el("thead", {}, el("tr", {}, ...chart.data.fields.map((/** @type {any} */ f) => el("th", { scope: "col", text: f.name + (f.unit ? ` (${f.unit})` : "") })))));
    t.append(el("tbody", {}, ...chart.data.rows.map((/** @type {any[]} */ r) => el("tr", {}, ...r.map(v => el("td", { text: typeof v === "number" ? String(+v.toPrecision(6)) : String(v) }))))));
    return el("div", { class: "flx-chart-table", tabindex: "0", role: "region", "aria-label": `Table of the data of ${chart.title}` }, t);
  };

  /** the rows inside the window [a, b] (fractions of the x extent: values for a number, positions for a category) @param {any} chart @param {number} a @param {number} b */
  const windowed = (chart, a, b) => {
    const xi = chart.data.fields.findIndex((/** @type {any} */ f) => f.name === chart.encoding.x.field), nominal = chart.data.fields[xi].type === "nominal", rows = chart.data.rows;
    if (a <= 0 && b >= 1) return rows;
    if (nominal) {
      const cats = [...new Set(rows.map((/** @type {any[]} */ r) => String(r[xi])))], keep = new Set(cats.slice(Math.floor(a * cats.length), Math.max(Math.floor(a * cats.length) + 1, Math.ceil(b * cats.length))));
      return rows.filter((/** @type {any[]} */ r) => keep.has(String(r[xi])));
    }
    const [lo, hi] = F.llmChartExtent(rows.map((/** @type {any[]} */ r) => r[xi])), x0 = lo + a * (hi - lo), x1 = lo + b * (hi - lo);
    return rows.filter((/** @type {any[]} */ r) => r[xi] >= x0 && r[xi] <= x1);
  };

  /** the figure of a chart: SVG, legend, controls and table; `chart.mark === "heat"` or `hist` zoom the same way @param {any} chart @returns {HTMLElement} */
  const figure = chart => {
    style();
    const { el, btn } = F, id = "flx-chart-" + ++count;
    let a = 0, b = 1;
    const plot = el("div", { class: "flx-chart-plot" }), status = el("span", { class: "fl-meta", role: "status", "aria-live": "polite" });
    const draw = () => {
      const { panels, legend } = F.llmChartDraw(chart, windowed(chart, a, b));
      plot.replaceChildren(...panels, ...(legend ? [legend] : []));
      status.textContent = a <= 0 && b >= 1 ? "Whole range" : `Showing ${(a * 100).toFixed(0)}% to ${(b * 100).toFixed(0)}% of the range`;
    };
    /** @param {number} f zoom factor around the centre @param {number} s pan in widths */
    const move = (f, s) => {
      const w = Math.min(1, Math.max(0.05, (b - a) * f)), c = (a + b) / 2 + s * (b - a);
      a = Math.max(0, Math.min(1 - w, c - w / 2)); b = a + w; draw();
    };
    const csvBtn = btn("export", "Download CSV", () => {
      const url = URL.createObjectURL(new Blob([csv(chart)], { type: "text/csv" })), link = el("a", { href: url, download: (chart.title || "chart").replace(/[^\w.-]+/g, "_") + ".csv" });
      link.click(); setTimeout(() => URL.revokeObjectURL(url), 1000);
    }, "Download CSV");
    const bar = el("div", { class: "row", role: "group", "aria-label": "Zoom and pan" },
      btn("plus", "Zoom in", () => move(0.5, 0)), btn("minus", "Zoom out", () => move(2, 0)), tb("Pan left", "◀", () => move(1, -0.25)), tb("Pan right", "▶", () => move(1, 0.25)),
      btn("reset", "Reset zoom", () => { a = 0; b = 1; draw(); }), csvBtn, status);
    const fig = el("figure", { class: "flx-chart", id, tabindex: "0", role: "group", "aria-label": `${chart.title}. ${chart.why}`, "data-mark": chart.mark, "data-ref": chart.data.ref },
      el("h4", { text: chart.title }), el("p", { text: chart.why }), plot, bar,
      el("details", { open: true }, el("summary", { text: "Table of the data" }), dataTable(chart)));
    fig.style.margin = "0";
    fig.addEventListener("keydown", e => {
      if (/** @type {HTMLElement} */ (e.target).closest("table, button, summary")) return;
      const k = e.key;
      if (k === "+" || k === "=") move(0.5, 0); else if (k === "-") move(2, 0); else if (k === "[") move(1, -0.25); else if (k === "]") move(1, 0.25); else if (k === "0") { a = 0; b = 1; draw(); } else return;
      e.preventDefault();
    });
    draw();
    return fig;
  };

  /** the answer «no view can show this»: text, never a chart @param {any} chart */
  const noView = chart => F.el("p", { class: "flx-chart-none", role: "note", text: "No chart can show this: " + chart.why });

  /** @param {any} chart the text of a preset: its title is a key of PRESETS */
  const labelled = chart => {
    const p = PRESETS[chart.title];
    return p ? { ...chart, title: p[0], why: p[1] } : chart;
  };

  /** the menu of the charts built without a model; `presets()` gives the `grafico/1` list, `show(node)` puts a figure in the conversation @param {() => Promise<any[]>} presets @param {(node: HTMLElement) => void} show */
  const presetMenu = (presets, show) => {
    const { el } = F, row = el("div", { class: "row", role: "group", "aria-label": "Charts without the model" });
    const box = el("details", { class: "flx-chart-presets", id: "flx-llm-charts" }, el("summary", { text: "Charts without the model" }), row);
    box.addEventListener("toggle", async () => {
      if (!(/** @type {HTMLDetailsElement} */ (box)).open) return;
      row.replaceChildren(el("span", { class: "fl-meta", text: "Looking at the open files…" }));
      /** @type {any[]} */ let list = [];
      try { list = await presets(); } catch (_) { /* no data: nothing to offer */ }
      row.replaceChildren(...(list.length ? list.map(c => { const l = labelled(c); return tb(`Draw: ${l.title}`, l.title, () => show(figure(l))); }) : [el("span", { class: "fl-meta", text: "No open file has data for a chart yet." })]));
    });
    return box;
  };

  /** @type {Record<string, (q: any) => string>} */
  const MSG = {
    "err.llm.chart.schema": q => `The chart is not a valid grafico/1 (${q.field}).`,
    "err.llm.chart.title": q => `The title must have 1 to ${q.max} characters.`,
    "err.llm.chart.ref": q => `There is no view ${q.ref}.`,
    "err.llm.chart.mark": q => `The mark ${q.mark} does not exist.`,
    "err.llm.chart.field": q => `The view ${q.ref} has no field ${q.field} for ${q.channel}.`,
    "err.llm.chart.type": q => `The ${q.mark} mark does not take a ${q.kind} field for ${q.channel}.`,
    "err.llm.chart.color": q => `Colour needs a nominal field with at most ${q.max} values (${q.field}).`,
    "err.llm.chart.detail": q => `Detail needs a nominal field with at most ${q.max} values (${q.field}).`,
    "err.llm.chart.facet": q => `Facet needs a nominal field with at most ${q.max} values (${q.field}).`,
    "err.llm.chart.style": q => `The chart carries a style (${q.field}); colours and styles belong to the program.`,
    "err.llm.chart.rows": () => "A chart points to a view; it never carries its own rows.",
    "err.llm.chart.why": q => `The reason must have 1 to ${q.max} characters.`,
    "err.llm.chart.sources": () => "The data of the charts could not be read.",
  };
  const prev = F.message;
  F.message = (/** @type {any} */ p) => (MSG[p.key] ? MSG[p.key](p.params || {}) : prev(p));

  F.llmChart = { figure, noView, presetMenu, csv, labelled, css: CSS };
})();
