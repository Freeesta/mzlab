// @ts-check
// TPMINE-PRIVATE  Transformation products and provenance for the printable report of the Feature table (window.TPREPORT), used by 38_feature_export.js: `collect` reads the rows in the view from the
// table of the products (`tptab/1`, session.tpTable) and the formulas of the best hypothesis (`interp/1`, session.interp); `section` lays them out as HTML (tables, then one SVG time course per
// product with its data table right below). Every number is the engine's; the page only lays them out. The levels are shown as proposed and as decided by the chemist, never merged. Texts in English.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => /** @type {any} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  /** most products drawn in the report; the rest are in the CSV */
  const MAX_DRAWN = 50;
  /** words of the codes of `tptab/1` (status, kinship, route, notes) */
  const STATUS = ["Product", "Control", "In-source fragment", "Formed, not explained", "Parent"];
  const KINSHIP = ["None", "Doubtful", "Possible", "Probable", "Certain"];
  const VIA = /** @type {[number, string][]} */ ([[1, "Rule"], [2, "Cleavage"], [4, "Imported"], [8, "MS2"], [16, "Network"]]);
  const NOTES = /** @type {[number, string][]} */ ([[1, "t0 present"], [2, "ISF co-elutes"], [4, "ISF of a product"], [8, "Chimeric MS2"], [16, "In the blank"], [32, "In t0"], [64, "In the dark control"], [128, "No t0"]]);
  const bits = (/** @type {[number, string][]} */ table, /** @type {number | null} */ v) => (v == null ? "" : table.filter(b => v & b[0]).map(b => b[1]).join(", "));
  const num = (/** @type {any} */ v, d = 4) => (v == null ? "" : String(+Number(v).toFixed(d)));
  const sci = (/** @type {any} */ v) => (v == null ? "" : Math.abs(v) >= 1e4 || (v !== 0 && Math.abs(v) < 0.01) ? Number(v).toExponential(2) : String(+Number(v).toPrecision(4)));
  const th = (/** @type {string[]} */ h) => `<tr>${h.map(x => `<th scope="col">${esc(x)}</th>`).join("")}</tr>`;
  const tr = (/** @type {any[]} */ c) => `<tr>${c.map(x => `<td>${esc(x)}</td>`).join("")}</tr>`;
  const kv = (/** @type {string} */ k, /** @type {any} */ v) => `<tr><th scope="row">${esc(k)}</th><td>${esc(v)}</td></tr>`;

  /**
   * the data of the section: rows of the products in the view, with the formulas of the hypothesis and the levels; null when the table has no block `tp/1` or the session cannot give it
   * @param {{ session: any, spec: () => string, names: string[], tpInfo?: (id: number) => { level?: string | null, pred?: number | null } }} o
   */
  async function collect(o) {
    const s = o.session;
    if (!s.tpTable) return null;
    let tp; try { tp = await s.tpTable(); } catch (_) { return null; }
    if (!tp || !tp.rows) return null;
    const inView = new Set((await s.rows(o.spec())).slice(1).map((/** @type {any[]} */ x) => x[0]));
    const rows = tp.rows.filter((/** @type {any} */ r) => inView.has(r.id));
    const shown = rows.filter((/** @type {any} */ r) => r.status === 0 || r.status === 4);
    /** @type {Map<number, { neutral: string, ion: string }>} */ const formulas = new Map();
    if (s.interp) await Promise.all(shown.map(async (/** @type {any} */ r) => {
      try { const c = (await s.interp(r.id)).change; if (c) formulas.set(r.id, { neutral: c.neutral_formula || "", ion: c.ion_formula || "" }); } catch (_) { /* the row keeps its cells empty */ }
    }));
    const D = G.TPDEC, series = { files: o.names, formula: "" };
    const decided = (/** @type {any} */ r) => { const d = D && D.get(series, D.rowKey(r.mz, r.rt, true)); return d && d.level ? D.text(d.level) + (d.note ? ` (${d.note})` : "") : ""; };
    return { times: tp.times || [], rows: rows.map((/** @type {any} */ r) => {
      const i = o.tpInfo ? o.tpInfo(r.id) || {} : {}, f = formulas.get(r.id);
      return { ...r, neutral: f ? f.neutral : "", ion: f ? f.ion : "", level: i.level ? (G.LIVELLI ? G.LIVELLI.tag(i.level) : String(i.level)) : "", pred: i.pred, decided: decided(r) };
    }) };
  }

  /** the time course of a product as an SVG (points joined by a line, scaled to its own maximum) @param {number[]} times @param {(number | null)[]} ys @param {string} label */
  function svg(times, ys, label) {
    const W = 320, H = 110, L = 8, B = 16, pts = times.map((t, i) => [t, ys[i]]).filter(p => p[1] != null && Number.isFinite(p[0]));
    if (!pts.length) return "";
    const t0 = Math.min(...pts.map(p => p[0])), t1 = Math.max(...pts.map(p => p[0])), ymax = Math.max(...pts.map(p => /** @type {number} */ (p[1])), 0) || 1;
    const X = (/** @type {number} */ t) => L + (t1 > t0 ? (t - t0) / (t1 - t0) : 0.5) * (W - 2 * L), Y = (/** @type {number} */ y) => H - B - (y / ymax) * (H - B - 8);
    const line = pts.map(p => `${X(p[0]).toFixed(1)},${Y(/** @type {number} */ (p[1])).toFixed(1)}`).join(" ");
    return `<svg viewBox="0 0 ${W} ${H}" width="${W}" height="${H}" role="img" aria-label="${esc(label)}"><title>${esc(label)}</title>`
      + `<line x1="${L}" y1="${H - B}" x2="${W - L}" y2="${H - B}" stroke="var(--line)"/><line x1="${L}" y1="8" x2="${L}" y2="${H - B}" stroke="var(--line)"/>`
      + `<polyline points="${line}" fill="none" stroke="var(--c1)" stroke-width="2"/>`
      + pts.map(p => `<circle cx="${X(p[0]).toFixed(1)}" cy="${Y(/** @type {number} */ (p[1])).toFixed(1)}" r="3" fill="var(--c2)"/>`).join("")
      + `<text x="${L}" y="${H - 3}" font-size="10" fill="var(--ink)">${esc(num(t0, 2))}</text><text x="${W - L}" y="${H - 3}" font-size="10" text-anchor="end" fill="var(--ink)">${esc(num(t1, 2))} min</text></svg>`;
  }

  /** the section «Transformation products» (empty string without data) @param {any} d result of `collect` */
  function section(d) {
    if (!d) return "";
    const prod = d.rows.filter((/** @type {any} */ r) => r.status === 0 || r.status === 4), unexp = d.rows.filter((/** @type {any} */ r) => r.status === 3);
    const head = ["ID", "m/z", "RT (min)", "Neutral formula", "Ion formula", "Status", "Kinship", "Route", "Prev. N", "Level proposed", "Level decided", "Notes"];
    let h = `<h2>Transformation products</h2><p>${prod.length} rows of the view with status product or parent, ${unexp.length} formed and not explained. The levels proposed come from the engine; the level decided is the one the chemist typed, if any.</p>`
      + `<h3>Products</h3><table>${th(head)}${prod.map((/** @type {any} */ r) => tr([r.id, num(r.mz), num(r.rt, 2), r.neutral, r.ion, STATUS[r.status] || "", KINSHIP[r.kinship] || "", bits(VIA, r.via), r.pred ? `Prev. ${r.pred}` : "", r.level, r.decided, bits(NOTES, r.notes)])).join("")}</table>`
      + `<h3>Formed, not explained</h3>` + (unexp.length ? `<table>${th(["ID", "m/z", "RT (min)", "Detected in files", "Max area", "Notes"])}${unexp.map((/** @type {any} */ r) => tr([r.id, num(r.mz), num(r.rt, 2), r.n_det, sci(r.max_area), bits(NOTES, r.notes)])).join("")}</table>` : "<p>None.</p>");
    const drawn = prod.filter((/** @type {any} */ r) => r.status === 0 && r.series && r.series.some((/** @type {any} */ y) => y != null)).sort((/** @type {any} */ a, /** @type {any} */ b) => (b.kinship || 0) - (a.kinship || 0)).slice(0, MAX_DRAWN);
    if (drawn.length) h += `<h3>Time courses</h3><p>Area of the treated samples against time, one chart per product with its data. ${prod.filter((/** @type {any} */ r) => r.status === 0).length > drawn.length ? `The first ${MAX_DRAWN} by kinship are drawn; the others are in the CSV.` : ""}</p>`
      + drawn.map((/** @type {any} */ r) => {
        const label = `ID ${r.id}, m/z ${num(r.mz)}, RT ${num(r.rt, 2)} min`;
        return `<figure><figcaption>${esc(label)}</figcaption>${svg(d.times, r.series, "Time course of " + label)}<table><caption>Data of the chart: ${esc(label)}</caption>${th(["Time (min)", "Area"])}${d.times.map((/** @type {number} */ t, /** @type {number} */ i) => tr([num(t, 2), sci(r.series[i])])).join("")}</table></figure>`;
      }).join("");
    return h;
  }

  /** the provenance block: versions, recipe, table, samples and inputs @param {any} r `report/1` @param {{ profile?: string, recipe_sha256?: string, rules?: string, inputs?: { name: string, sha256: string }[] } | null} p */
  function provenance(r, p) {
    const q = p || {};
    return `<h2>Provenance</h2><table>${th(["Item", "Value"])}${kv("mzlab-feat", r.crate)}${kv("Rules", q.rules || "not recorded")}${kv("Profile", q.profile || "not recorded")}`
      + `<tr><th scope="row">Recipe SHA-256</th><td><code>${esc(q.recipe_sha256 || "not recorded")}</code></td></tr><tr><th scope="row">Table SHA-256</th><td><code>${esc(r.tabella_sha256)}</code></td></tr>`
      + `<tr><th scope="row">Samples SHA-256</th><td><code>${esc(r.campioni_sha256)}</code></td></tr>${(q.inputs || []).map(x => `<tr><th scope="row">Input ${esc(x.name)}</th><td><code>${esc(x.sha256)}</code></td></tr>`).join("")}`
      + `<tr><th scope="row">Date</th><td>${esc(new Date().toISOString().slice(0, 10))}</td></tr></table>`;
  }

  G.TPREPORT = { collect, section, provenance, svg, MAX_DRAWN };
})();
