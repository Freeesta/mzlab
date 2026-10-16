// @ts-check
// TPMINE-PRIVATE  TPGRUPPI_VIEW: the «Compare conditions» view of Tabui. It receives a `gruppi/1` (the output of the Rust module `crates/mzlab-feat/src/gruppi/`) and draws it: the matrix TP × condition,
// the UpSet of the sets, the chart of the chosen TP (grouped bars with one time per condition, overlaid curves with more times, small multiples beyond 8 conditions), the table of the parent per
// condition and the exports. It computes nothing: every number (areas, ratios, sets, shapes, Prev.) is read from the JSON. Texts in English (Tabui has no `lang/` keys); colours only from CSS variables
// (Okabe-Ito for the first 8 conditions); every chart has its table of the data (TDATI); no animation.
//   TPGRUPPI_VIEW.mount(el, gruppi, { parentName, onOpenRow(rowId) }) → { destroy(), setFilter(value), selectRow(rowId), state(), csv(kind) }
(() => {
  "use strict";
  const G = /** @type {any} */ (window);

  /**
   * @typedef {{ t: number, area: number | null, sd: number | null, n: number }} GKin
   * @typedef {{ id: string, present: boolean, n_det: number, n_rep: number, area: number | null, area_sd: number | null, cv: number | null, ratio_ref: number | null, shape: string | null, t_max: number | null, t_half_rise: number | null, rate_rel: number | null, kin: GKin[], prev: Record<string, string> }} GCell
   * @typedef {{ row: number, tp_name: string, mz: number, polarity: string, rt: number, neutral_formula: string | null, ion_formula?: string | null, ppm: number | null, other_polarity: number | null, present_in: string[], only_in: string | null, in_all: boolean, cond: GCell[] }} GRow
   * @typedef {{ id: string, factors: Record<string, string>, files: number[], times: number[], replicates: number, controls: Record<string, number[]>, parent: { k: number | null, half_life: number | null, remaining: number | null } }} GCond
   * @typedef {{ format: string, version: number, reference: string | null, norm: string, conditions: GCond[], rows: GRow[], sets: { conditions: string[], rows: number[] }[], warnings: { code: string, condition?: string }[], parent_name?: string }} Gruppi
   * @typedef {{ parentName?: string, onOpenRow?: (rowId: number) => void }} GOpts
   */

  // ------------------------------------------------------------------ formatting
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"']/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c] || c));
  /** scientific notation with a two-digit exponent: 1.23e+06 */
  const sci = (/** @type {number | null | undefined} */ v, d = 2) => (v == null || !isFinite(v) ? "—" : v === 0 ? "0" : v.toExponential(d).replace(/e([+-])(\d)$/, "e$10$2"));
  const num = (/** @type {number | null | undefined} */ v, d = 3) => (v == null || !isFinite(v) ? "" : String(+v.toFixed(d)));
  const ratioText = (/** @type {GRow} */ r, /** @type {GCell} */ c, /** @type {string | null} */ ref) =>
    r.only_in === c.id ? "only here" : c.id === ref || c.ratio_ref == null ? "—" : "×" + (c.ratio_ref >= 10 ? c.ratio_ref.toFixed(0) : +c.ratio_ref.toPrecision(2));
  const slug = (/** @type {string} */ s) => String(s || "parent").toLowerCase().replace(/[^a-z0-9.+-]+/g, "-").replace(/^-+|-+$/g, "") || "parent";
  const today = () => { const d = new Date(); return `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, "0")}${String(d.getDate()).padStart(2, "0")}`; };
  const niceStep = (/** @type {number} */ r) => { const p = Math.pow(10, Math.floor(Math.log10(r))), m = r / p; return (m < 1.5 ? 1 : m < 3.5 ? 2 : m < 7.5 ? 5 : 10) * p; };

  const PREV_SYM = { "Prev. likely": "✓", "Prev. possible": "?", "Prev. unlikely": "✗" };
  const PREV_LABEL = { absent_t0: "Absent in T0", absent_blank: "Absent in blank", in_parent_std: "In the parent standard", above_min: "Above minimum area", ms2: "MS2" };
  const WARN = {
    refMissing: (/** @type {string} */ c) => `The reference condition «${c}» has no data: ratios are not shown.`,
    condNoControls: (c) => `Condition «${c}» has no T0 or blank of its own and no shared control.`,
    condOnePoint: (c) => `Condition «${c}» has a single point: no kinetic shape.`,
    condNoReplicates: (c) => `Condition «${c}» has no replicates (n = 1): no standard deviation.`,
    mixedTimes: (c) => `Condition «${c}» has different times from the others: ratios use the highest common time.`,
  };
  const prevTitle = (/** @type {GCell} */ c) => {
    const p = c.prev || {};
    return Object.keys(PREV_LABEL).filter(k => p[k] != null).map(k => `${PREV_LABEL[k]}: ${p[k]}`).join("; ") + (p.verdict ? ` → ${p.verdict}` : "");
  };

  // ------------------------------------------------------------------ styles (page) and the rules of the charts (also written into the exported SVG, variables resolved)
  const SVGCSS = `.gv-svg text{font:11px system-ui,sans-serif;fill:var(--ink)}.gv-svg .mut{fill:var(--muted)}.gv-svg .ax{stroke:var(--muted);stroke-width:1;fill:none}.gv-svg .grid{stroke:var(--line);stroke-width:1}
.gv-svg .ln{fill:none;stroke-width:2.2}.gv-svg .par{fill:none;stroke-width:1.4;stroke-dasharray:5 4;opacity:.85}.gv-svg .bar{stroke:var(--ink);stroke-width:.6}.gv-svg .sd{stroke:var(--ink);stroke-width:1.5}
.gv-svg .dot{fill:var(--ink)}.gv-svg .on{fill:var(--accent)}.gv-svg .off{fill:var(--line)}.gv-svg .link{stroke:var(--ink);stroke-width:2}.gv-svg .tot{fill:var(--muted)}.gv-svg .setbar{fill:var(--ink)}.gv-svg .setbar.sel{fill:var(--accent)}
.gv-svg .hit{fill:transparent;cursor:pointer}.gv-svg .hit:hover{fill:var(--sel)}.gv-svg .hit:focus-visible{fill:var(--sel);stroke:var(--accent);stroke-width:2;outline:none}`;
  const CSS = `.gv{--gv1:#0072B2;--gv2:#E69F00;--gv3:#009E73;--gv4:#D55E00;--gv5:#CC79A7;--gv6:#56B4E9;--gv7:#F0E442;--gv8:var(--ink);font-size:13px;color:var(--ink);width:100%;max-width:100%;min-width:0;contain:inline-size;position:relative}
.gv *{box-sizing:border-box}.gv h3{margin:14px 0 4px;font-size:14px}.gv h4{margin:6px 0 2px;font-size:12px;color:var(--muted)}.gv .sr-only{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap}
.gv .gv-bar{display:flex;flex-wrap:wrap;gap:6px 10px;align-items:center;margin:6px 0}.gv .gv-bar label{color:var(--muted);font-size:12px}
.gv button,.gv select{min-height:24px;min-width:24px;font:inherit;font-size:12px;padding:2px 10px;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink);cursor:pointer}
.gv button:hover{background:var(--sel)}.gv button:focus-visible,.gv select:focus-visible,.gv .gv-sc:focus-visible,.gv a:focus-visible{outline:2px solid var(--accent);outline-offset:1px}.gv button[aria-pressed=true]{background:var(--sel);border-color:var(--accent)}
.gv .gv-warn{margin:4px 0;padding:4px 10px;list-style:none;border-left:3px solid var(--warn);background:color-mix(in srgb,var(--warn) 12%,var(--panel));font-size:12px}.gv .gv-warn li{margin:2px 0}
.gv .gv-sc{position:relative;max-width:100%;overflow:auto;max-height:480px;border:1px solid var(--line);border-radius:5px}.gv .gv-sc.free{max-height:none}
.gv table{border-collapse:collapse;font-size:12px;width:100%}.gv th,.gv td{padding:2px 8px;border-bottom:1px solid var(--line);text-align:left;white-space:nowrap;vertical-align:top}.gv thead th{position:sticky;top:0;background:var(--panel);z-index:1}
.gv td.n,.gv th.n{text-align:right;font-variant-numeric:tabular-nums}.gv tr.sel td,.gv tr.sel th{background:var(--sel)}.gv th[scope=row]{font-weight:600}
.gv .gv-hb{border:0;background:none;padding:0 2px;font-weight:600;min-height:24px;display:inline-flex;align-items:center;gap:3px}.gv .gv-hb:hover{background:var(--sel)}
.gv .gv-nm{scroll-margin-top:34px;border:0;background:none;padding:0 2px;text-align:left;font-weight:600;min-height:24px;text-decoration:underline dotted}
.gv .ref{font-size:10px;font-weight:400;color:var(--muted);margin-left:4px}.gv .gv-pres{font-size:13px;margin-right:4px}.gv .gv-rt{color:var(--muted);margin-left:6px}
.gv .gv-pv{display:inline-block;margin-left:6px;padding:0 5px;border:1px solid var(--muted);border-radius:8px;font-size:10px;line-height:15px;cursor:help}
.gv .gv-pv.likely{border-color:var(--ok)}.gv .gv-pv.unlikely{border-color:var(--bad)}
.gv .gv-sub{display:block;color:var(--muted);font-size:11px}
.gv .gv-leg{display:flex;flex-wrap:wrap;gap:2px 14px;margin:4px 0;padding:0;list-style:none;font-size:12px}.gv .gv-leg li{display:inline-flex;align-items:center;gap:4px;min-height:20px}
.gv .gv-multi{display:grid;grid-template-columns:repeat(auto-fill,minmax(150px,1fr));gap:6px}.gv .gv-multi figure{margin:0;border:1px solid var(--line);border-radius:5px;padding:2px 4px}.gv .gv-multi figcaption{font-size:11px;font-weight:600}
.gv .gv-fig{position:relative;max-width:100%;overflow:auto}.gv .gv-fig svg{display:block;max-width:none;height:auto}.gv .tdbtn{margin:2px 0}
${SVGCSS.replace(/\.gv-svg/g, ".gv .gv-svg")}`;
  const COLORS = Array.from({ length: 8 }, (_, i) => `var(--gv${i + 1})`);
  /** a marker per condition so that the colour is never the only cue */
  const marker = (/** @type {number} */ i, /** @type {number} */ x, /** @type {number} */ y, /** @type {string} */ col, r = 4) => {
    const s = i % 5, f = `fill="${col}" stroke="var(--ink)" stroke-width=".7"`;
    if (s === 0) return `<circle cx="${x}" cy="${y}" r="${r}" ${f}/>`;
    if (s === 1) return `<rect x="${x - r}" y="${y - r}" width="${2 * r}" height="${2 * r}" ${f}/>`;
    if (s === 2) return `<path d="M${x} ${y - r - 1}L${x + r + 1} ${y + r}L${x - r - 1} ${y + r}Z" ${f}/>`;
    if (s === 3) return `<path d="M${x} ${y - r - 1}L${x + r + 1} ${y}L${x} ${y + r + 1}L${x - r - 1} ${y}Z" ${f}/>`;
    return `<path d="M${x - r} ${y}H${x + r}M${x} ${y - r}V${y + r}" stroke="${col}" stroke-width="3" fill="none"/>`;
  };

  // ------------------------------------------------------------------ downloads
  /** the SVG as a standalone file: the rules of the charts are written in it and the CSS variables are replaced by the colours they have now */
  function standalone(/** @type {SVGElement} */ svg) {
    const cs = getComputedStyle(svg.closest(".gv") || document.documentElement), val = (/** @type {string} */ n) => cs.getPropertyValue(n).trim() || "#000";
    const resolve = (/** @type {string} */ s) => { let t = s; for (let k = 0; k < 3; k++) t = t.replace(/var\((--[\w-]+)\)/g, (_, n) => val(n)); return t; };
    const vb = (svg.getAttribute("viewBox") || "0 0 300 200").split(/\s+/).map(Number);
    const clone = /** @type {SVGElement} */ (svg.cloneNode(true));
    clone.removeAttribute("style"); clone.removeAttribute("tabindex"); clone.setAttribute("xmlns", "http://www.w3.org/2000/svg");
    clone.setAttribute("width", String(vb[2])); clone.setAttribute("height", String(vb[3]));
    const bg = `<rect width="${vb[2]}" height="${vb[3]}" fill="${val("--panel")}"/>`;
    const out = clone.outerHTML.replace(/^(<svg[^>]*>)/, `$1<style>${SVGCSS.replace(/\.gv-svg/g, "svg")}</style>${bg}`);
    return `<?xml version="1.0" encoding="UTF-8"?>\n` + resolve(out);
  }
  function download(/** @type {string} */ name, /** @type {any} */ body, /** @type {string} */ type) {
    if (typeof G.dl === "function") return G.dl(name, body, type, false);
    const a = document.createElement("a"); a.href = URL.createObjectURL(new Blob([body], { type })); a.download = name; document.body.appendChild(a); a.click(); a.remove();
  }
  async function pngOf(/** @type {SVGElement} */ svg) {
    const text = standalone(svg), vb = (svg.getAttribute("viewBox") || "0 0 300 200").split(/\s+/).map(Number), k = 2;
    const img = new Image(); img.src = "data:image/svg+xml;charset=utf-8," + encodeURIComponent(text);
    await img.decode();
    const cv = document.createElement("canvas"); cv.width = vb[2] * k; cv.height = vb[3] * k;
    const g = /** @type {CanvasRenderingContext2D} */ (cv.getContext("2d")); g.scale(k, k); g.drawImage(img, 0, 0, vb[2], vb[3]);
    return await new Promise(res => cv.toBlob(b => res(b), "image/png"));
  }
  const csvCell = (/** @type {any} */ v) => { const s = v == null ? "" : String(v); return /[",\n\r]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s; };
  const csvText = (/** @type {any[][]} */ rows) => rows.map(r => r.map(csvCell).join(",")).join("\r\n") + "\r\n";

  // ------------------------------------------------------------------ the component
  function filterPart(/** @type {any} */ V) {
    const { g, el, opts, conds, ids, ref, rows, byRow, parent, hasF, hasP, st, cellOf, color, Q } = V;
    // ---- filter: «Only in…», «In all», «Present in…», or a set picked in the UpSet; all from `sets`
    const setRows = (/** @type {(s: { conditions: string[] }) => boolean} */ test) => { const out = new Set(); g.sets.filter(test).forEach(s => s.rows.forEach(x => out.add(x))); return out; };
    function allowed() {
      const f = st.filter;
      if (f === "all") return null;
      if (f === "inall") return setRows(s => s.conditions.length === ids.length);
      if (f.startsWith("only:")) return setRows(s => s.conditions.length === 1 && s.conditions[0] === f.slice(5));
      if (f.startsWith("in:")) return setRows(s => s.conditions.includes(f.slice(3)));
      if (f.startsWith("set:")) { const s = g.sets[+f.slice(4)]; return new Set(s ? s.rows : []); }
      return null;
    }
    function visibleRows() {
      const ok = allowed(); let v = ok ? rows.filter(r => ok.has(r.row)) : rows.slice();
      const { col, dir } = st.sort;
      if (col) {
        const key = (/** @type {GRow} */ r) => col === "name" ? r.tp_name : col === "mz" ? r.mz : col === "rt" ? r.rt : (cellOf(r, col)?.area ?? null);
        v.sort((a, b) => { const x = key(a), y = key(b); if (x == null && y == null) return 0; if (x == null) return 1; if (y == null) return -1; return (x < y ? -1 : x > y ? 1 : 0) * dir; });
      }
      return v;
    }
    const filterLabel = (/** @type {string} */ f) => f === "all" ? "All rows" : f === "inall" ? "In all conditions" : f.startsWith("only:") ? `Only in ${f.slice(5)}` : f.startsWith("in:") ? `Present in ${f.slice(3)}` : f.startsWith("set:") ? `Set: ${(g.sets[+f.slice(4)]?.conditions || []).join(" + ")}` : f;

    function drawWarnings() {
      const w = g.warnings || [];
      Q("#gv-warn").innerHTML = w.length ? `<ul class="gv-warn" role="status" aria-label="Warnings">${w.map(x => `<li>${esc((WARN[x.code] || (() => x.code))(x.condition || ""))}</li>`).join("")}</ul>` : "";
    }
    function drawFilterSelect() {
      const opts2 = [["all", "All rows"], ["inall", "In all conditions"], ...ids.map(c => [`only:${c}`, `Only in ${c}`]), ...ids.map(c => [`in:${c}`, `Present in ${c}`])];
      if (st.filter.startsWith("set:")) opts2.push([st.filter, filterLabel(st.filter)]);
      const sel = /** @type {HTMLSelectElement} */ (Q("#gv-flt"));
      sel.innerHTML = opts2.map(([v, l]) => `<option value="${esc(v)}">${esc(l)}</option>`).join(""); sel.value = st.filter;
    }

    Object.assign(V, { visibleRows, filterLabel, drawFilterSelect, drawWarnings });
  }

  function matrixPart(/** @type {any} */ V) {
    const { g, el, opts, conds, ids, ref, rows, byRow, parent, hasF, hasP, st, cellOf, color, Q } = V;
    // ---- matrix
    function cellHtml(/** @type {GRow} */ r, /** @type {GCell} */ c) {
      const pv = c.present && c.prev && c.prev.verdict ? `<span class="gv-pv ${c.prev.verdict.split(" ")[1] || ""}" title="${esc(prevTitle(c))}"><span aria-hidden="true">Prev. ${PREV_SYM[c.prev.verdict] || ""}</span><span class="sr-only">${esc(prevTitle(c))}</span></span>` : "";
      const rep = c.n_rep > 1 ? `<span class="gv-sub">n = ${c.n_det}/${c.n_rep}${c.cv != null ? `, CV ${(c.cv * 100).toFixed(0)}%` : ""}</span>` : "";
      return `<td><span class="gv-pres" aria-hidden="true">${c.present ? "●" : "○"}</span><span class="sr-only">${c.present ? "present" : "absent"}, </span>${c.present ? `<span title="area">${sci(c.area)}</span>` : `<span class="gv-sub" style="display:inline" title="area of the filled cell">${c.kin.some(k => k.area) ? sci(c.kin[c.kin.length - 1].area) : "—"}</span>`}` +
        ` <span title="ratio to the reference${ref ? ` (${esc(ref)})` : ""}">${esc(ratioText(r, c, ref))}</span>${pv}${rep}</td>`;
    }
    const sortMark = (/** @type {string} */ col) => (st.sort.col === col ? (st.sort.dir > 0 ? "▲" : "▼") : "");
    const th = (/** @type {string} */ col, /** @type {string} */ label, /** @type {string} */ cls = "", /** @type {string} */ extra = "") =>
      `<th scope="col" class="${cls}" aria-sort="${st.sort.col === col ? (st.sort.dir > 0 ? "ascending" : "descending") : "none"}"><button type="button" class="gv-hb" data-sort="${esc(col)}" title="Sort by ${esc(label)}">${esc(label)}${extra}<span aria-hidden="true">${sortMark(col)}</span></button></th>`;
    function drawMatrix() {
      const vr = V.visibleRows();
      const head = `<tr>${th("name", "TP")}${th("mz", "m/z", "n")}${th("rt", "RT (min)", "n")}${hasF ? "<th scope=\"col\">Neutral formula</th><th scope=\"col\">[M+H]+ formula</th>" : ""}${hasP ? "<th scope=\"col\" class=\"n\">Δ ppm</th>" : ""}` +
        `${ids.map(c => th(c, c, "", c === ref ? '<span class="ref">ref</span>' : "")).join("")}</tr>`;
      const body = vr.map(r => `<tr data-row="${r.row}"${st.tp === r.row ? ' class="sel"' : ""}><th scope="row"><button type="button" class="gv-nm" data-open="${r.row}" aria-label="${esc(r.tp_name)}, m/z ${num(r.mz, 2)}, RT ${num(r.rt, 2)}${r.polarity === "-" ? ", ESI negative" : ""}. Show the chart">${esc(r.tp_name)}</button></th>` +
        `<td class="n">${num(r.mz, 2)}${r.polarity === "-" ? ' <span title="negative polarity">(−)</span>' : ""}</td><td class="n">${num(r.rt, 2)}</td>${hasF ? `<td>${esc(r.neutral_formula || "")}</td><td>${esc(r.ion_formula || "")}</td>` : ""}${hasP ? `<td class="n">${num(r.ppm, 1)}</td>` : ""}` +
        `${ids.map(id => { const c = cellOf(r, id); return c ? cellHtml(r, c) : "<td>—</td>"; }).join("")}</tr>`).join("");
      Q("#gv-mx").innerHTML = `<table><caption class="sr-only">Comparison matrix: TP by condition. ● present, ○ absent; area, ratio to the reference, Prev. suggestion.</caption><thead>${head}</thead><tbody>${body || `<tr><td colspan="${3 + ids.length}">No rows match this filter.</td></tr>`}</tbody></table>`;
      Q("#gv-nv").textContent = `${vr.length} of ${rows.length} rows · ${V.filterLabel(st.filter)}`;
      Q("#gv-n").textContent = `${vr.length} of ${rows.length} rows`;
    }

    Object.assign(V, { drawMatrix });
  }

  function upsetPart(/** @type {any} */ V) {
    const { g, el, opts, conds, ids, ref, rows, byRow, parent, hasF, hasP, st, cellOf, color, Q } = V;
    // ---- UpSet
    const SLOT = 30, LBLW = 112, TOTW = 96, TOPH = 120, RH = 24, MAXSETS = 20;
    function drawUpset() {
      const box = Q("#gv-up"); const sets = g.sets.slice(0, MAXSETS), n = ids.length;
      if (!sets.length) { box.innerHTML = `<p class="gv-sub">No TP is present in any condition.</p>`; return; }
      const totals = ids.map(c => rows.filter(r => r.present_in.includes(c)).length), tmax = Math.max(1, ...totals), smax = Math.max(1, ...sets.map(s => s.rows.length));
      const x0 = LBLW + TOTW + 8, W = x0 + sets.length * SLOT + 8, H = TOPH + n * RH + 6;
      const sx = (/** @type {number} */ i) => x0 + i * SLOT + SLOT / 2;
      let s = `<svg class="gv-svg" id="gv-upsvg" viewBox="0 0 ${W} ${H}" style="width:${Math.round(W * st.zoom)}px" role="group" aria-label="UpSet plot of the sets of conditions">`;
      ids.forEach((c, k) => {
        const y = TOPH + k * RH, w = totals[k] / tmax * (TOTW - 26);
        s += `<rect class="tot" x="${TOTW - w}" y="${y + 5}" width="${w}" height="${RH - 10}"/><text x="${TOTW - w - 3}" y="${y + 16}" text-anchor="end" class="mut">${totals[k]}</text><text x="${TOTW + 8}" y="${y + 16}">${esc(c.length > 14 ? c.slice(0, 13) + "…" : c)}${c === ref ? " (ref)" : ""}</text>`;
        if (k % 2 === 0) s += `<rect x="${x0}" y="${y}" width="${sets.length * SLOT}" height="${RH}" fill="var(--sel)" opacity=".35"/>`;
      });
      sets.forEach((set, i) => {
        const idx = g.sets.indexOf(set), h = set.rows.length / smax * (TOPH - 28), sel = st.filter === `set:${idx}`;
        const on = ids.map((c, k) => (set.conditions.includes(c) ? k : -1)).filter(k => k >= 0);
        s += `<rect class="setbar${sel ? " sel" : ""}" x="${sx(i) - 7}" y="${TOPH - 6 - h}" width="14" height="${h}"/><text x="${sx(i)}" y="${TOPH - 10 - h}" text-anchor="middle">${set.rows.length}</text>`;
        if (on.length > 1) s += `<line class="link" x1="${sx(i)}" x2="${sx(i)}" y1="${TOPH + on[0] * RH + RH / 2}" y2="${TOPH + on[on.length - 1] * RH + RH / 2}"/>`;
        ids.forEach((c, k) => { s += `<circle class="${on.includes(k) ? "dot" : "off"}" cx="${sx(i)}" cy="${TOPH + k * RH + RH / 2}" r="${on.includes(k) ? 5 : 3.5}"/>`; });
        s += `<rect class="hit" tabindex="0" role="button" aria-pressed="${sel}" data-set="${idx}" x="${x0 + i * SLOT}" y="0" width="${SLOT}" height="${H}" aria-label="${esc(set.conditions.join(" + "))}: ${set.rows.length} rows. Press to filter the matrix"><title>${esc(set.conditions.join(" + "))}: ${set.rows.length} rows</title></rect>`;
      });
      box.innerHTML = `<div class="gv-bar"><button type="button" data-zoom="-" aria-label="Zoom out the UpSet plot">−</button><button type="button" data-zoom="+" aria-label="Zoom in the UpSet plot">+</button><button type="button" data-dl="up-svg">UpSet (SVG)</button><button type="button" data-dl="up-png">UpSet (PNG)</button>` +
        `${g.sets.length > MAXSETS ? `<span class="gv-sub">${MAXSETS} largest of ${g.sets.length} sets shown</span>` : ""}</div><div class="gv-fig gv-sc free" tabindex="0" role="region" aria-label="UpSet plot, scrollable">${s}</svg></div>`;
      const svg = /** @type {SVGElement} */ (Q("#gv-upsvg"));
      if (G.TDATI) {
        G.TDATI.describe(svg, `UpSet plot: ${sets.length} sets of conditions; the largest has ${sets[0].rows.length} rows.`);
        G.TDATI.addButton(svg, () => ({ title: "UpSet: sets of conditions", head: ["Conditions", "TPs", "TP names"], rowHeader: true,
          rows: g.sets.map(x => [x.conditions.join(" + "), String(x.rows.length), x.rows.map(i => byRow.get(i)?.tp_name || i).join("; ")]), note: "Totals per condition: " + ids.map((c, k) => `${c} ${totals[k]}`).join(", ") }));
      }
    }

    Object.assign(V, { drawUpset });
  }

  function tpPart(/** @type {any} */ V) {
    const { g, el, opts, conds, ids, ref, rows, byRow, parent, hasF, hasP, st, cellOf, color, Q } = V;
    // ---- chart of the chosen TP
    const timesOf = (/** @type {GRow} */ r) => [...new Set(r.cond.flatMap(c => c.kin.map(k => k.t)))].sort((a, b) => a - b);
    const maxArea = (/** @type {GRow} */ r) => Math.max(0, ...r.cond.flatMap(c => c.kin.map(k => (k.area || 0) + (k.sd || 0))));
    function yAxis(/** @type {number} */ ymax, /** @type {number} */ L, /** @type {number} */ R, /** @type {number} */ T, /** @type {number} */ B, /** @type {string} */ mode) {
      const Y = (/** @type {number} */ v) => B - v / ymax * (B - T), step = niceStep(ymax / 4); let s = `<line class="ax" x1="${L}" y1="${T}" x2="${L}" y2="${B}"/><line class="ax" x1="${L}" y1="${B}" x2="${R}" y2="${B}"/>`;
      for (let v = 0; v <= ymax * 1.001; v += step) s += `<line class="grid" x1="${L}" x2="${R}" y1="${Y(v)}" y2="${Y(v)}"/><text class="mut" x="${L - 4}" y="${Y(v) + 4}" text-anchor="end">${mode === "pct" ? Math.round(v) : sci(v, 1)}</text>`;
      return { s, Y };
    }
    /** one plot: grouped bars (one time per condition) or curves; `cs` = the conditions drawn */
    function plot(/** @type {GRow} */ r, /** @type {string[]} */ cs, /** @type {{ ymax: number, small?: boolean }} */ o) {
      const ts = timesOf(r), bars = ts.length <= 1, pct = st.mode === "pct", small = !!o.small, top = maxArea(r) || 1;
      const val = (/** @type {number | null} */ a) => (a == null ? 0 : pct ? a / top * 100 : a);
      const ymax = pct ? 100 : o.ymax, W = bars ? Math.max(small ? 150 : 320, 60 + cs.length * 46) : (small ? 190 : 420), H = small ? 120 : 230, L = small ? 44 : 56, R = W - 10, T = 12, B = H - (bars ? 56 : 30);
      const { s: ax, Y } = yAxis(ymax, L, R, T, B, pct ? "pct" : "area");
      let s = `<svg class="gv-svg" viewBox="0 0 ${W} ${H}" role="img" aria-label="${esc(r.tp_name)} across conditions" ${small ? "" : `style="width:${Math.round(W * st.zoom)}px"`}>${ax}`;
      if (bars) {
        const slot = (R - L) / cs.length;
        cs.forEach(id => {
          const i = ids.indexOf(id), c = cellOf(r, id), k = c ? c.kin[c.kin.length - 1] : null, x = L + (cs.indexOf(id) + 0.5) * slot, a = c && c.present ? val(k ? k.area : c.area) : 0;
          if (a > 0) s += `<rect class="bar" x="${x - Math.min(16, slot / 2 - 3)}" y="${Y(a)}" width="${2 * Math.min(16, slot / 2 - 3)}" height="${B - Y(a)}" fill="${color(i)}"/>`;
          else s += `<text class="mut" x="${x}" y="${B - 4}" text-anchor="middle">–</text>`;
          if (a > 0 && k && k.sd) s += `<line class="sd" x1="${x}" x2="${x}" y1="${Y(val(k.area - k.sd))}" y2="${Y(val(k.area + k.sd))}"/><line class="sd" x1="${x - 4}" x2="${x + 4}" y1="${Y(val(k.area + k.sd))}" y2="${Y(val(k.area + k.sd))}"/><line class="sd" x1="${x - 4}" x2="${x + 4}" y1="${Y(val(k.area - k.sd))}" y2="${Y(val(k.area - k.sd))}"/>`;
          s += `<text x="${x}" y="${B + 12}" text-anchor="end" transform="rotate(-35 ${x} ${B + 12})">${esc(id.length > 12 ? id.slice(0, 11) + "…" : id)}</text>`;
        });
      } else {
        const t0 = ts[0], t1 = ts[ts.length - 1], X = (/** @type {number} */ t) => L + (t - t0) / (t1 - t0 || 1) * (R - L - 6) + 3;
        ts.forEach(t => { s += `<text class="mut" x="${X(t)}" y="${B + 14}" text-anchor="middle">${t}</text>`; });
        s += `<text class="mut" x="${(L + R) / 2}" y="${H - 4}" text-anchor="middle">Time (min)</text>`;
        cs.forEach(id => {
          const i = ids.indexOf(id), c = cellOf(r, id), pts = (c ? c.kin : []).filter(k => k.area != null);
          if (pts.length > 1) s += `<path class="ln" stroke="${color(i)}" d="${pts.map((k, j) => `${j ? "L" : "M"}${X(k.t).toFixed(1)} ${Y(val(k.area)).toFixed(1)}`).join("")}"/>`;
          pts.forEach(k => { if (k.sd) s += `<line class="sd" x1="${X(k.t)}" x2="${X(k.t)}" y1="${Y(val(k.area - k.sd))}" y2="${Y(val(k.area + k.sd))}"/>`; s += marker(i, +X(k.t).toFixed(1), +Y(val(k.area)).toFixed(1), color(i), small ? 3 : 4); });
          const pk = conds[i] && conds[i].parent.k;
          if (pct && pk && ts[0] === 0) s += `<path class="par" stroke="${color(i)}" d="${ts.map((t, j) => `${j ? "L" : "M"}${X(t).toFixed(1)} ${Y(100 * Math.exp(-pk * t)).toFixed(1)}`).join("")}"/>`;
        });
      }
      return s + "</svg>";
    }
    function tpData(/** @type {GRow} */ r) {
      const top = maxArea(r) || 1, rowsT = [];
      for (const c of r.cond) for (const k of c.kin) rowsT.push([c.id, String(k.t), k.area == null ? "" : sci(k.area, 3), k.sd == null ? "" : sci(k.sd, 3), String(k.n), k.area == null ? "" : num(k.area / top * 100, 1)]);
      return { title: `${r.tp_name}: area per condition`, head: ["Condition", "Time (min)", "Area", "Area SD", "n", "% of max"], rows: rowsT, rowHeader: true };
    }
    function drawTp() {
      const box = Q("#gv-tp"), r = st.tp != null ? byRow.get(st.tp) : null;
      if (!r) { box.innerHTML = `<p class="gv-sub">Choose a TP in the matrix.</p>`; return; }
      const many = ids.length > 8, ts = timesOf(r);
      const legend = many ? "" : `<ul class="gv-leg" aria-label="Conditions">${ids.map((c, i) => `<li><svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">${marker(i, 7, 7, color(i), 4)}</svg>${esc(c)}${c === ref ? " (ref)" : ""}</li>`).join("")}${ts.length > 1 && st.mode === "pct" ? `<li><svg width="26" height="10" aria-hidden="true"><path class="par" d="M1 5H25" stroke="var(--ink)" fill="none" stroke-width="1.6" stroke-dasharray="5 4"/></svg>Parent (first-order fit from k)</li>` : ""}</ul>`;
      const top = maxArea(r) || 1;
      let figs;
      if (many) figs = `<div class="gv-multi" role="group" aria-label="${esc(r.tp_name)}: one small chart per condition">${ids.map(c => `<figure><figcaption>${esc(c)}${c === ref ? " (ref)" : ""}</figcaption>${plot(r, [c], { ymax: top * 1.08, small: true })}</figure>`).join("")}</div>`;
      else figs = `<div class="gv-fig gv-sc free" tabindex="0" role="region" aria-label="Chart of ${esc(r.tp_name)}, scrollable" id="gv-tpfig">${plot(r, ids, { ymax: top * 1.08 })}</div>`;
      box.innerHTML = `<div class="gv-bar"><label for="gv-tpsel">TP</label><select id="gv-tpsel">${rows.map(x => `<option value="${x.row}">${esc(x.tp_name)} (m/z ${num(x.mz, 1)}, RT ${num(x.rt, 1)})</option>`).join("")}</select>` +
        `<span role="group" aria-label="Value shown"><button type="button" data-mode="area" aria-pressed="${st.mode === "area"}">Area</button><button type="button" data-mode="pct" aria-pressed="${st.mode === "pct"}">% of max</button></span>` +
        `<button type="button" data-zoom="-" aria-label="Zoom out the chart">−</button><button type="button" data-zoom="+" aria-label="Zoom in the chart">+</button><button type="button" data-dl="tp-svg">Chart (SVG)</button><button type="button" data-dl="tp-png">Chart (PNG)</button>` +
        `${many ? `<button type="button" data-tpdata>Data</button>` : ""}</div>` +
        `<p class="gv-sub">${ts.length > 1 ? "Curves: area against time, one per condition." : "Bars: area at the end point (error bar = standard deviation)."} ${esc(r.tp_name)}, m/z ${num(r.mz, 2)}, RT ${num(r.rt, 2)} min.</p>${legend}${figs}`;
      /** @type {HTMLSelectElement} */ (Q("#gv-tpsel")).value = String(r.row);
      const svgs = [...box.querySelectorAll("svg.gv-svg")].filter(x => !x.closest(".gv-leg"));
      if (G.TDATI) svgs.forEach((svg, i) => {
        G.TDATI.describe(svg, `${r.tp_name}: ${ts.length > 1 ? "area against time" : "area at the end point"} for ${ids.length} conditions.`);
        G.TDATI.addButton(svg, () => tpData(r), many || i > 0); svg.setAttribute("tabindex", "0");
      });
      const tb = box.querySelector("[data-tpdata]"); if (tb) tb.addEventListener("click", () => G.TDATI && G.TDATI.table(tpData(r)));
    }

    // ---- parent per condition
    function drawParent() {
      Q("#gv-par").innerHTML = `<table><caption class="sr-only">Parent compound per condition</caption><thead><tr><th scope="col">Condition</th><th scope="col" class="n">k (1/min)</th><th scope="col" class="n">t½ (min)</th><th scope="col" class="n">Remaining (%)</th></tr></thead><tbody>` +
        conds.map(c => `<tr><th scope="row">${esc(c.id)}${c.id === ref ? '<span class="ref">ref</span>' : ""}</th><td class="n">${c.parent.k == null ? "—" : sci(c.parent.k, 2)}</td><td class="n">${c.parent.half_life == null ? "—" : num(c.parent.half_life, 1)}</td><td class="n">${c.parent.remaining == null ? "—" : num(c.parent.remaining * 100, 0)}</td></tr>`).join("") + `</tbody></table>`;
    }

    Object.assign(V, { drawTp, drawParent });
  }

  function exportPart(/** @type {any} */ V) {
    const { g, el, opts, conds, ids, ref, rows, byRow, parent, hasF, hasP, st, cellOf, color, Q } = V;
    // ---- exports
    function longCsv() {
      const head = ["TP", "m/z", "RT (min)", "Polarity", ...(hasF ? ["Neutral formula", "[M+H]+ formula"] : []), ...(hasP ? ["Δ ppm"] : []), "Condition", "Time (min)", "Present", "Area", "Area SD", "n", "Ratio to reference", "Shape", "Prev."];
      /** @type {any[][]} */ const out = [head];
      for (const r of V.visibleRows()) for (const c of r.cond) for (const k of c.kin.length ? c.kin : [/** @type {any} */ ({ t: "", area: c.area, sd: c.area_sd, n: c.n_rep })])
        out.push([r.tp_name, r.mz, r.rt, r.polarity, ...(hasF ? [r.neutral_formula, r.ion_formula] : []), ...(hasP ? [r.ppm] : []), c.id, k.t, c.present ? "yes" : "no", k.area, k.sd, k.n, c.ratio_ref, c.shape, c.prev && c.prev.verdict]);
      return csvText(out);
    }
    function wideCsv() {
      const head = ["TP", "m/z", "RT (min)", "Polarity", ...(hasF ? ["Neutral formula", "[M+H]+ formula"] : []), ...(hasP ? ["Δ ppm"] : []), "Present in", "Only in", ...ids.flatMap(c => [`${c} present`, `${c} area`, `${c} ratio to reference`, `${c} Prev.`])];
      /** @type {any[][]} */ const out = [head];
      for (const r of V.visibleRows()) out.push([r.tp_name, r.mz, r.rt, r.polarity, ...(hasF ? [r.neutral_formula, r.ion_formula] : []), ...(hasP ? [r.ppm] : []), r.present_in.join("; "), r.only_in, ...ids.flatMap(id => { const c = cellOf(r, id); return c ? [c.present ? "yes" : "no", c.area, c.ratio_ref, c.prev && c.prev.verdict] : ["", "", "", ""]; })]);
      return csvText(out);
    }
    const setsCsv = () => csvText([["Conditions", "Number of TPs", "TP names"], ...g.sets.map(s => [s.conditions.join(" + "), s.rows.length, s.rows.map(i => byRow.get(i)?.tp_name || i).join("; ")])]);
    const csv = (/** @type {string} */ kind) => (kind === "long" ? longCsv() : kind === "wide" ? wideCsv() : kind === "sets" ? setsCsv() : JSON.stringify(g, null, 1));
    const fileName = (/** @type {string} */ kind) => ({ long: `tabui_conditions_${parent}_${today()}.csv`, wide: `tabui_conditions-wide_${parent}_${today()}.csv`, sets: `tabui_sets_${parent}_${today()}.csv`, json: `tabui_gruppi_${parent}_${today()}.json` }[kind]);
    const tpFile = (/** @type {string} */ ext) => { const r = st.tp != null ? byRow.get(st.tp) : null; return `tabui_tp-${r ? num(r.mz, 1) : "x"}_conditions.${ext}`; };
    async function doDownload(/** @type {string} */ what) {
      const svg = /** @type {SVGElement | null} */ (what.startsWith("up") ? el.querySelector("#gv-upsvg") : el.querySelector("#gv-tp svg.gv-svg:not(.gv-leg svg)"));
      if (!svg) return;
      const base = what.startsWith("up") ? `tabui_upset_${parent}` : tpFile("").slice(0, -1);
      if (what.endsWith("svg")) download(`${base}.svg`, standalone(svg), "image/svg+xml");
      else download(`${base}.png`, await pngOf(svg), "image/png");
    }

    Object.assign(V, { csv, fileName, doDownload });
  }

  function eventPart(/** @type {any} */ V) {
    const { g, el, opts, conds, ids, ref, rows, byRow, parent, hasF, hasP, st, cellOf, color, Q } = V;
    // ---- events
    const redraw = () => { V.drawFilterSelect(); V.drawMatrix(); V.drawUpset(); };
    el.addEventListener("click", ev => {
      const t = /** @type {HTMLElement} */ (ev.target), b = t.closest ? /** @type {HTMLElement | null} */ (t.closest("[data-sort],[data-open],[data-set],[data-mode],[data-zoom],[data-dl],[data-ex]")) : null; if (!b) return;
      const d = b.dataset;
      if (d.sort != null) { st.sort = { col: d.sort, dir: st.sort.col === d.sort ? -st.sort.dir : 1 }; V.drawMatrix(); el.querySelector(`[data-sort="${CSS_ESC(d.sort)}"]`)?.focus?.(); }
      else if (d.open != null) { select(+d.open, true); }
      else if (d.set != null) { const f = `set:${d.set}`; st.filter = st.filter === f ? "all" : f; redraw(); el.querySelector(`[data-set="${d.set}"]`)?.focus?.(); }
      else if (d.mode) { st.mode = d.mode; V.drawTp(); el.querySelector(`[data-mode="${d.mode}"]`)?.focus?.(); }
      else if (d.zoom) { st.zoom = Math.max(0.6, Math.min(3, st.zoom * (d.zoom === "+" ? 1.25 : 0.8))); const host = b.closest("#gv-up") ? "up" : "tp"; if (host === "up") V.drawUpset(); else V.drawTp(); el.querySelector(`#gv-${host} [data-zoom="${d.zoom}"]`)?.focus?.(); }
      else if (d.dl) V.doDownload(d.dl);
      else if (d.ex) download(V.fileName(d.ex), V.csv(d.ex), d.ex === "json" ? "application/json" : "text/csv;charset=utf-8");
    });
    const CSS_ESC = (/** @type {string} */ s) => (window.CSS && window.CSS.escape ? window.CSS.escape(s) : s.replace(/"/g, '\\"'));
    el.addEventListener("change", ev => {
      const t = /** @type {HTMLSelectElement} */ (ev.target);
      if (t.id === "gv-flt") { st.filter = t.value; redraw(); el.querySelector("#gv-flt")?.focus?.(); }
      else if (t.id === "gv-tpsel") select(+t.value, false);
    });
    el.addEventListener("keydown", ev => {
      const t = /** @type {HTMLElement} */ (ev.target);
      if (t.matches && t.matches("button.gv-nm")) {                                  // matrix: up/down between the TP names, Home/End
        const all = [...el.querySelectorAll("button.gv-nm")], i = all.indexOf(t), j = ev.key === "ArrowDown" ? i + 1 : ev.key === "ArrowUp" ? i - 1 : ev.key === "Home" ? 0 : ev.key === "End" ? all.length - 1 : -1;
        if (j >= 0 && all[j]) { ev.preventDefault(); /** @type {HTMLElement} */ (all[j]).focus(); all[j].scrollIntoView({ block: "nearest" }); }
      } else if (t.matches && t.matches("rect.hit")) {                                // UpSet: left/right between the bars, Enter/Space filter
        const all = [...el.querySelectorAll("rect.hit")], i = all.indexOf(t), j = ev.key === "ArrowRight" ? i + 1 : ev.key === "ArrowLeft" ? i - 1 : ev.key === "Home" ? 0 : ev.key === "End" ? all.length - 1 : -1;
        if (j >= 0 && all[j]) { ev.preventDefault(); /** @type {SVGElement} */ (all[j]).focus(); }
        else if (ev.key === "Enter" || ev.key === " ") { ev.preventDefault(); t.dispatchEvent(new MouseEvent("click", { bubbles: true })); }
      }
    });
    function select(/** @type {number} */ id, /** @type {boolean} */ open) {
      st.tp = id; el.querySelectorAll("#gv-mx tr.sel").forEach(x => x.classList.remove("sel")); el.querySelector(`#gv-mx tr[data-row="${id}"]`)?.classList.add("sel");
      V.drawTp(); if (open && opts.onOpenRow) opts.onOpenRow(id);
    }

    Object.assign(V, { select, redraw });
  }

  function mount(/** @type {HTMLElement} */ el, /** @type {Gruppi} */ g, /** @type {GOpts} */ opts = {}) {
    if (!document.getElementById("gv-css")) { const st = document.createElement("style"); st.id = "gv-css"; st.textContent = CSS; document.head.appendChild(st); }
    const conds = g.conditions, ids = conds.map(c => c.id), ref = g.reference, rows = g.rows, byRow = new Map(rows.map(r => [r.row, r]));
    const parent = slug(opts.parentName || g.parent_name || "parent");
    const hasF = rows.some(r => r.neutral_formula || r.ion_formula), hasP = rows.some(r => r.ppm != null);
    /** @type {{ filter: string, sort: { col: string, dir: number }, tp: number | null, mode: string, zoom: number }} */
    const st = { filter: "all", sort: { col: "", dir: 1 }, tp: rows.length ? rows[0].row : null, mode: "area", zoom: 1 };
    const cellOf = (/** @type {GRow} */ r, /** @type {string} */ id) => r.cond.find(c => c.id === id);
    const color = (/** @type {number} */ i) => COLORS[i % 8];

    /** @type {any} */
    const V = { g, el, opts, conds, ids, ref, rows, byRow, parent, hasF, hasP, st, cellOf, color, Q: null };
    // ---- skeleton
    el.innerHTML = `<div class="gv" data-gv>
<div id="gv-warn"></div>
<div class="gv-bar" role="group" aria-label="Exports">
 <button type="button" data-ex="long" title="One row per TP × condition × time (rows now shown)">Comparison (CSV, long)</button><button type="button" data-ex="wide" title="One row per TP (rows now shown)">Comparison (CSV, wide)</button>
 <button type="button" data-ex="sets" title="The sets of conditions and their rows">Sets (CSV)</button><button type="button" data-ex="json" title="The complete gruppi/1 output">gruppi/1 (JSON)</button></div>
<h3 id="gv-h-mx">Comparison matrix</h3>
<div class="gv-bar"><label for="gv-flt">Filter</label><select id="gv-flt"></select><span id="gv-n" role="status" aria-live="polite" class="sr-only"></span><span id="gv-nv" class="gv-sub"></span></div>
<div class="gv-sc" id="gv-mx" tabindex="0" role="region" aria-labelledby="gv-h-mx"></div>
<h3>UpSet: which conditions share each TP</h3><div id="gv-up"></div>
<h3>Chosen TP across conditions</h3><div id="gv-tp"></div>
<h3>Parent compound per condition</h3><div class="gv-sc free" id="gv-par"></div></div>`;

    const Q = (/** @type {string} */ s) => /** @type {any} */ (el.querySelector(s));
    V.Q = Q;
    filterPart(V); matrixPart(V); upsetPart(V); tpPart(V); exportPart(V); eventPart(V);
    V.drawWarnings(); V.redraw(); V.drawTp(); V.drawParent();
    return {
      destroy() { el.innerHTML = ""; },
      setFilter(/** @type {string} */ f) { st.filter = f; V.redraw(); },
      selectRow(/** @type {number} */ id) { V.select(id, false); },
      state() { return { filter: st.filter, sort: { ...st.sort }, tp: st.tp, mode: st.mode, visible: V.visibleRows().map(r => r.row) }; },
      csv: V.csv,
    };
  }

  G.TPGRUPPI_VIEW = { mount, slug, sci };
})();
