// @ts-check
// TPMINE-PRIVATE  TPRETE_VIEW: the network of the products of Tabui. It receives a `rete/1` (the output of the Rust module `crates/mzlab-feat/src/rete/`) and draws it: the parent and the products in columns
// by step, an arc for every rule that takes one ion formula to another, the reason of the chosen arc under the chart and the table of the arcs. It computes nothing: columns, places, rules, errors and the
// reasons in words are read from the JSON. Texts in English (Tabui has no `lang/` keys); colours only from CSS variables; shape and text as well as colour; every chart has its table (TDATI); no animation.
//   TPRETE_VIEW.mount(el, rete, { onSelect(rowId) }) → { destroy(), selectRow(rowId), state(), csv() }
// A network is not an identification: nothing here names a product or raises a level.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);

  /**
   * @typedef {{ id: number, row: number, mz: number, rt: number, neutral_formula: string, ion_formula: string, kinship: number, step: number, pos: number }} RNode
   * @typedef {{ from: number, to: number, rule: string, scope: string, delta: string, dm: number, ppm: number, delta_rt: number, kinetic: string, ms2: { equal: number[], shifted: number[], n_equal: number, n_shifted: number } | null, reason: string }} RArc
   * @typedef {{ schema: string, params: { ppm: number, min_fragments: number }, parent: number, columns: number, nodes: RNode[], arcs: RArc[], n_arcs: number }} Rete
   */

  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"']/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c] || c));
  const KIN = ["none", "doubtful", "possible", "probable", "certain"];
  const KIN_NOTE = "Kinship with the parent from the proofs of the series; a network is evidence of kinship, never an identification.";
  const COL_W = 190, ROW_H = 66, PAD_X = 60, PAD_T = 44, R = 11, HIT = 40;

  const CSS = `.rv{font-size:13px;color:var(--ink);width:100%;max-width:100%;min-width:0;contain:inline-size;position:relative}.rv *{box-sizing:border-box}
.rv h3{margin:12px 0 4px;font-size:14px}.rv .rv-bar{display:flex;flex-wrap:wrap;gap:6px 10px;align-items:center;margin:6px 0}
.rv button{min-height:24px;min-width:24px;font:inherit;font-size:12px;padding:2px 10px;border:1px solid var(--line);border-radius:5px;background:var(--panel);color:var(--ink);cursor:pointer}
.rv button:hover{background:var(--sel)}.rv button:focus-visible,.rv .rv-fig:focus-visible{outline:2px solid var(--accent);outline-offset:1px}.rv button[aria-pressed=true]{background:var(--sel);border-color:var(--accent)}
.rv .rv-fig{position:relative;max-width:100%;overflow:auto;border:1px solid var(--line);border-radius:5px}.rv .rv-fig svg{display:block;max-width:none;height:auto}
.rv .rv-leg{display:flex;flex-wrap:wrap;gap:2px 14px;margin:4px 0;padding:0;list-style:none;font-size:12px}.rv .rv-leg li{display:inline-flex;align-items:center;gap:4px;min-height:20px}
.rv .rv-why{margin:6px 0;padding:4px 10px;border-left:3px solid var(--accent);background:var(--panel);min-height:24px}.rv .rv-sub{color:var(--muted);font-size:11px}
.rv ul.rv-arcs{margin:4px 0;padding:0;list-style:none;display:flex;flex-direction:column;gap:2px}.rv ul.rv-arcs button{width:100%;text-align:left}
.rv .rv-svg text{font:11px system-ui,sans-serif;fill:var(--ink);paint-order:stroke;stroke:var(--panel);stroke-width:3px;stroke-linejoin:round}.rv .rv-svg .mut{fill:var(--muted)}
.rv .rv-svg .hit{fill:transparent;cursor:pointer}.rv .rv-svg g.nd:focus-visible .hit{stroke:var(--accent);stroke-width:2}.rv .rv-svg .arc{fill:none;stroke:var(--ink)}.rv .rv-svg .arc.sel{stroke:var(--accent)}
.rv .rv-svg .ahit{fill:none;stroke:transparent;stroke-width:24px;cursor:pointer}.rv .rv-svg .glyph{fill:var(--ink);stroke:var(--ink);stroke-width:1.6}.rv .rv-svg .glyph.hollow{fill:var(--panel)}
.rv .rv-svg .ring{fill:none;stroke:var(--accent);stroke-width:4}.rv .rv-svg .colh{fill:var(--muted)}`;

  /** the shape of a node: filled = certain, left half = probable, dashed = possible, empty = doubtful or none, square = the parent */
  const glyph = (/** @type {RNode} */ n, /** @type {boolean} */ parent, /** @type {boolean} */ sel) => {
    let s = "";
    if (parent) s = `<rect class="glyph" x="${-R}" y="${-R}" width="${2 * R}" height="${2 * R}"/>`;
    else if (n.kinship >= 4) s = `<circle class="glyph" r="${R}"/>`;
    else if (n.kinship === 3) s = `<circle class="glyph hollow" r="${R}"/><path class="glyph" d="M0 ${-R}A${R} ${R} 0 0 0 0 ${R}Z"/>`;
    else if (n.kinship === 2) s = `<circle class="glyph hollow" r="${R}" stroke-dasharray="3 2.5"/>`;
    else s = `<circle class="glyph hollow" r="${R}"/>`;
    return s + (sel ? `<circle class="ring" r="${R + 5}"/>` : "");
  };
  const mzText = (/** @type {RNode} */ n) => n.mz.toFixed(4);
  const evidence = (/** @type {RArc} */ a) => (a.ms2 ? a.ms2.n_equal + a.ms2.n_shifted : null);
  const nodeLabel = (/** @type {RNode} */ n, /** @type {boolean} */ parent) =>
    `${parent ? "Parent" : "Product"} m/z ${mzText(n)}, RT ${n.rt.toFixed(2)}, ion formula ${n.ion_formula}, ${parent ? "step 0" : `step ${n.step}, kinship ${KIN[n.kinship] || "none"}`}`;

  const csvCell = (/** @type {any} */ v) => { const s = v == null ? "" : String(v); return /[",\n\r]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s; };
  const HEAD = ["From m/z", "To m/z", "Rule", "Scope", "Δ ion formula", "Δm (Da)", "Δ ppm", "ΔRT (min)", "Time order", "Equal fragments", "Shifted fragments", "Reason"];

  function mount(/** @type {HTMLElement} */ el, /** @type {Rete} */ g, /** @type {{ onSelect?: (rowId: number) => void }} */ opts = {}) {
    if (!document.getElementById("rv-css")) { const s = document.createElement("style"); s.id = "rv-css"; s.textContent = CSS; document.head.appendChild(s); }
    const nodes = g.nodes, byId = new Map(nodes.map(n => [n.id, n]));
    const st = { node: /** @type {number | null} */ (null), arc: /** @type {number | null} */ (null) };
    const name = (/** @type {number} */ id) => { const n = byId.get(id); return n ? mzText(n) : String(id); };
    const arcRow = (/** @type {RArc} */ a) => [name(a.from), name(a.to), a.rule, a.scope, a.delta, a.dm.toFixed(4), a.ppm.toFixed(2), a.delta_rt.toFixed(2), a.kinetic === "consistent" ? "source first" : a.kinetic === "against" ? "source later" : "not ordered", a.ms2 ? a.ms2.n_equal : "", a.ms2 ? a.ms2.n_shifted : "", a.reason];
    const xy = (/** @type {RNode} */ n) => ({ x: PAD_X + n.step * COL_W, y: PAD_T + n.pos * ROW_H });
    const rowsMax = Math.max(1, ...nodes.map(n => n.pos + 1));
    const W = PAD_X + 100 + Math.max(0, g.columns - 1) * COL_W, H = PAD_T + rowsMax * ROW_H;

    el.innerHTML = `<div class="rv" data-rv><div class="rv-bar"><button type="button" data-copy>Copy data</button><span role="status" data-note class="rv-sub"></span></div>
      <div class="rv-fig" data-fig></div><ul class="rv-leg" aria-label="Legend"></ul><p class="rv-why" role="status" data-why></p><h3>Arcs</h3><ul class="rv-arcs" data-arcs></ul></div>`;
    const Q = (/** @type {string} */ s) => /** @type {HTMLElement} */ (el.querySelector(s));

    function arcPath(/** @type {RArc} */ a) {
      const p = byId.get(a.from), q = byId.get(a.to);
      if (!p || !q) return "";
      const A = xy(p), B = xy(q);
      if (p.step === q.step) { const bulge = 46 + 10 * Math.abs(p.pos - q.pos); return `M${A.x + R} ${A.y}C${A.x + bulge} ${A.y} ${B.x + bulge} ${B.y} ${B.x + R} ${B.y}`; }
      const dir = q.step > p.step ? 1 : -1, mid = (A.x + B.x) / 2;
      return `M${A.x + dir * R} ${A.y}C${mid} ${A.y} ${mid} ${B.y} ${B.x - dir * (R + 5)} ${B.y}`;
    }
    function arcMid(/** @type {RArc} */ a) {
      const p = byId.get(a.from), q = byId.get(a.to);
      if (!p || !q) return { x: 0, y: 0 };
      const A = xy(p), B = xy(q);
      return p.step === q.step ? { x: A.x + 46 + 10 * Math.abs(p.pos - q.pos) * 0.75, y: (A.y + B.y) / 2 } : { x: (A.x + B.x) / 2, y: (A.y + B.y) / 2 - 5 };
    }
    function svgText() {
      let s = `<svg class="rv-svg" viewBox="0 0 ${W} ${H}" width="${W}" height="${H}" role="group" aria-label="Network of the products: ${nodes.length} nodes, ${g.arcs.length} arcs"><defs><marker id="rv-ah" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="9" markerHeight="9" markerUnits="userSpaceOnUse" orient="auto"><path d="M0 0L10 5L0 10z" fill="var(--ink)"/></marker></defs>`;
      for (let c = 0; c < g.columns; c++) s += `<text class="colh" x="${PAD_X + c * COL_W}" y="18" text-anchor="middle">${c === 0 ? "Parent" : `Step ${c}`}</text>`;
      g.arcs.forEach((a, i) => {
        const e = evidence(a), sel = st.arc === i, d = arcPath(a), w = (1.4 + 0.9 * Math.min(e ?? 0, 5) + (sel ? 1 : 0)).toFixed(1);
        s += `<g><path class="arc${sel ? " sel" : ""}" d="${d}" stroke-width="${w}"${e == null || e === 0 ? ' stroke-dasharray="6 4"' : ""} marker-end="url(#rv-ah)"/><path class="ahit" d="${d}" data-arc="${i}"/></g>`;
        const m = arcMid(a);
        s += `<text x="${m.x.toFixed(1)}" y="${m.y.toFixed(1)}" text-anchor="middle">${esc(a.delta)}</text>`;
      });
      nodes.forEach((n, i) => {
        const { x, y } = xy(n), parent = n.id === g.parent, sel = st.node === n.id;
        s += `<g class="nd" data-node="${n.id}" role="button" tabindex="${(st.node == null ? i === 0 : sel) ? 0 : -1}" aria-pressed="${sel}" aria-label="${esc(nodeLabel(n, parent))}" transform="translate(${x} ${y})"><rect class="hit" x="${-HIT / 2}" y="${-HIT / 2}" width="${HIT}" height="${HIT}"/>${glyph(n, parent, sel)}<text y="${R + 16}" text-anchor="middle">${mzText(n)}</text><text class="mut" y="${R + 28}" text-anchor="middle">RT ${n.rt.toFixed(2)}</text></g>`;
      });
      return s + "</svg>";
    }
    function arcList() {
      const touching = (/** @type {RArc} */ a) => st.node == null || a.from === st.node || a.to === st.node;
      Q("[data-arcs]").innerHTML = g.arcs.map((a, i) => ({ a, i })).filter(x => touching(x.a)).map(({ a, i }) =>
        `<li><button type="button" data-arcbtn="${i}" aria-pressed="${st.arc === i}">${esc(name(a.from))} → ${esc(name(a.to))} · ${esc(a.rule)} · ${esc(a.delta)}</button></li>`).join("") || `<li class="rv-sub">No arc.</li>`;
    }
    function why() {
      const a = st.arc != null ? g.arcs[st.arc] : null;
      const list = a ? [a] : st.node != null ? g.arcs.filter(x => x.from === st.node || x.to === st.node) : [];
      Q("[data-why]").innerHTML = list.length ? list.map(x => `${esc(name(x.from))} → ${esc(name(x.to))}, ${esc(x.rule)} (${esc(x.delta)}): ${esc(x.reason)}`).join("<br>") : "Choose a node or an arc to read why it is drawn.";
    }
    function legend() {
      const sh = (/** @type {string} */ body) => `<svg width="26" height="26" viewBox="-13 -13 26 26" aria-hidden="true" class="rv-svg">${body}</svg>`;
      const mk = (/** @type {number} */ k) => glyph(/** @type {RNode} */ ({ kinship: k }), false, false);
      Q(".rv-leg").innerHTML = [[`<rect class="glyph" x="-11" y="-11" width="22" height="22"/>`, "parent"], [mk(4), "certain"], [mk(3), "probable"], [mk(2), "possible"], [mk(1), "doubtful or none"]]
        .map(([b, t]) => `<li>${sh(b)}${t}</li>`).join("") + `<li>Line width: fragments shared with the parent; dashed: no MS2 compared</li><li class="rv-sub">${esc(KIN_NOTE)}</li>`;
    }
    function draw() {
      const fig = Q("[data-fig]");
      fig.innerHTML = svgText();
      const svg = /** @type {SVGElement} */ (fig.querySelector("svg"));
      if (G.TDATI) {
        G.TDATI.describe(svg, `Network of the products: ${nodes.length} nodes in ${g.columns} columns by step, ${g.arcs.length} arcs; each arc is a rule that takes one ion formula to another.`);
        G.TDATI.addButton(svg, () => ({ title: "Network of the products: arcs", head: HEAD, rows: g.arcs.map(arcRow), rowHeader: true }));
      }
      arcList(); why();
    }
    function csv() { return [HEAD, ...g.arcs.map(arcRow)].map(r => r.map(csvCell).join(",")).join("\r\n") + "\r\n"; }
    function select(/** @type {number | null} */ id, /** @type {boolean} */ notify) {
      const keep = el.contains(document.activeElement);
      st.node = id; st.arc = null; draw();
      if (keep && id != null) /** @type {HTMLElement | null} */ (el.querySelector(`[data-node="${id}"]`))?.focus();
      if (notify && id != null && opts.onSelect) opts.onSelect(id);
    }
    function move(/** @type {number} */ id, /** @type {string} */ key) {
      const n = byId.get(id); if (!n) return;
      const col = (/** @type {number} */ s) => nodes.filter(x => x.step === s);
      let to = null;
      if (key === "ArrowDown" || key === "ArrowUp") to = col(n.step).find(x => x.pos === n.pos + (key === "ArrowDown" ? 1 : -1));
      else {
        const next = col(n.step + (key === "ArrowRight" ? 1 : -1));
        to = next.sort((a, b) => Math.abs(a.pos - n.pos) - Math.abs(b.pos - n.pos))[0];
      }
      if (to) /** @type {HTMLElement | null} */ (el.querySelector(`[data-node="${to.id}"]`))?.focus();
    }
    function onClick(/** @type {Event} */ e) {
      const t = /** @type {Element} */ (e.target);
      const node = t.closest("[data-node]"), arc = t.closest("[data-arc],[data-arcbtn]");
      if (node) select(+(node.getAttribute("data-node") || 0), true);
      else if (arc) { st.arc = +(arc.getAttribute("data-arc") ?? arc.getAttribute("data-arcbtn") ?? 0); draw(); }
      else if (t.closest("[data-copy]")) {
        const note = Q("[data-note]"), text = csv();
        const done = () => { note.textContent = `Copied ${g.arcs.length} arcs.`; };
        if (navigator.clipboard) navigator.clipboard.writeText(text).then(done, () => { note.textContent = "Copy not allowed here."; }); else note.textContent = "Copy not available here.";
      }
    }
    function onKey(/** @type {KeyboardEvent} */ e) {
      const nd = /** @type {Element} */ (e.target).closest("[data-node]");
      if (!nd) return;
      const id = +(nd.getAttribute("data-node") || 0);
      if (e.key === "Enter" || e.key === " ") { e.preventDefault(); select(id, true); }
      else if (e.key.startsWith("Arrow")) { e.preventDefault(); move(id, e.key); }
    }
    el.addEventListener("click", onClick); el.addEventListener("keydown", onKey);
    legend(); draw();
    return {
      destroy() { el.removeEventListener("click", onClick); el.removeEventListener("keydown", onKey); el.innerHTML = ""; },
      selectRow(/** @type {number | null} */ id) { select(id != null && byId.has(id) ? id : null, false); },
      state: () => ({ node: st.node, arc: st.arc }),
      csv,
    };
  }

  G.TPRETE_VIEW = { mount };
})();
