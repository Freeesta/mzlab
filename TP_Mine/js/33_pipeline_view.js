// @ts-strict
// TPMINE-PRIVATE  View of the result of the feature pipeline (window.PIPEV): reads the .mzfeat of a series (table + section `evid`, evid/1), builds the rows and draws them in a virtualized
// list. With the gate of quality open (PIPE.gateState) the unit is the identity: the column of the contributions of every proof (never the total alone), 1/N, the states of the cells per
// file (D detected, T targeted, F filled, A absent), the filter «discarded» with the reason code, the proofs «against» in red. Without the gate only the rows of the table (traces and
// families) are listed and nothing about candidates reaches the DOM. Pure functions first (tested by tests/test_pipeline_view.py with node), DOM after. Texts in English (mzFinder convention).
(() => {
  "use strict";
  const G = typeof window !== "undefined" ? window : globalThis;
  const esc = s => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const STATE = ["D", "T", "F", "A"], STATE_NAME = ["detected", "targeted", "filled", "absent"];
  const TYPES = ["massa", "isotopi", "MS2", "frammenti", "rete", "cinetica", "controlli", "RT", "frequenza", "esterne"];
  const TYPE_NAME = { massa: "mass", isotopi: "isotopes", MS2: "MS2", frammenti: "fragments", rete: "network", cinetica: "kinetics", controlli: "controls", RT: "RT", frequenza: "frequency", esterne: "external" };
  const REASON = { EV_AGAINST_OPEN: "a proof against is open", EV_INDISTINGUISHABLE: "cannot be told apart from another candidate", EV_NO_MS2: "no MS2", EV_WEAK_MS2: "weak MS2 (impure precursor)", EV_KINETICS_INFO: "kinetics is informative only", EV_NO_PROOF: "no proof in favour", EV_ONLY_FREQUENCY: "only the frequency, which proves nothing about the identity" };
  const ROW_H = 30;

  // ------------------------------------------------------------ the .mzfeat of a series (MZFEAT01: magic, u64 length of the meta, meta JSON, arrays aligned to 64 bytes)
  const TYPED = { f64: Float64Array, f32: Float32Array, u32: Uint32Array, u16: Uint16Array, u8: Uint8Array, i8: Int8Array };
  function readFeat(bytes) {
    const b = bytes instanceof Uint8Array ? new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.length) : new Uint8Array(bytes);     // a plain Uint8Array (Buffer.slice would be a view)
    if (b.length < 16 || String.fromCharCode(...b.subarray(0, 8)) !== "MZFEAT01") throw Object.assign(new Error("not a .mzfeat"), { key: "error.badMzfeat" });
    const n = Number(new DataView(b.buffer, b.byteOffset, b.length).getBigUint64(8, true)), meta = JSON.parse(new TextDecoder().decode(b.subarray(16, 16 + n)));
    const start = Math.ceil((16 + n) / 64) * 64, data = b.slice(start).buffer, out = { meta, rows: meta.rows || 0, files: meta.files || 0 };
    for (const a of meta.arrays || []) out[a.name] = new TYPED[a.dtype](data, a.offset, a.count);
    out.evid = out.evid && out.evid.length ? JSON.parse(new TextDecoder().decode(out.evid)) : null;
    return out;
  }

  /** the rows of the table, by m/z: the position of the row of a member (m/z and RT given by evid/1), -1 when not found */
  function locator(f) {
    const ix = Array.from({ length: f.rows }, (_, i) => i).sort((a, b) => f.mz[a] - f.mz[b] || a - b);
    return (mz, rt) => {
      let lo = 0, hi = ix.length;
      while (lo < hi) { const m = (lo + hi) >> 1; if (f.mz[ix[m]] < mz * (1 - 2e-6)) lo = m + 1; else hi = m; }
      for (let k = lo; k < ix.length && f.mz[ix[k]] <= mz * (1 + 2e-6); k++) if (Math.abs(f.rt[ix[k]] - rt) < 2e-3) return ix[k];
      return -1;
    };
  }
  const cellsOf = (f, row) => Array.from({ length: f.files }, (_, j) => ({ state: f.state[row * f.files + j], area: f.area[row * f.files + j] }));

  /** the points of a proof as a chip: type, points; «against» flagged (always 0 points, never compensated) */
  const chip = p => ({ type: p.tipo, name: TYPE_NAME[p.tipo] || p.tipo, points: p.punti, against: p.segno === "contro", sign: p.segno });

  /** the identities of evid/1 with their cells; `discarded` = nothing in favour; the reasons are the codes of the engine (or EV_NO_PROOF) */
  function identities(f) {
    const find = locator(f), out = [];
    for (const x of f.evid ? f.evid.identita : []) {
      const ref = (x.membri || []).find(m => m.ruolo === "M") || (x.membri || [])[0] || null, row = ref ? find(ref.mz, ref.rt) : -1;
      const reasons = (x.motivi || []).map(m => m.codice);
      out.push({ id: x.id, mass: x.massa_neutra ?? null, formula: x.formula || "", mz: ref ? ref.mz : null, rt: ref ? ref.rt : null, members: (x.membri || []).length, total: x.totale, against: x.contro,
        n: x.n, p: x.probabilita, level: x.livello_max, hypothesis: x.ipotesi, chips: TYPES.map(t => (x.contributi || []).find(p => p.tipo === t)).filter(Boolean).map(chip),
        proofs: x.contributi || [], reasons: x.totale > 0 || reasons.length ? reasons : ["EV_NO_PROOF"], discarded: !(x.totale > 0), arcs: x.archi || [], member: x.membri || [], row, cells: row >= 0 ? cellsOf(f, row) : [] });
    }
    return out;
  }
  /** without the gate: the rows of the table, no hypothesis, no proof */
  function plainRows(f) {
    return Array.from({ length: f.rows }, (_, i) => ({ id: i + 1, mz: f.mz[i], rt: f.rt[i], members: (f.member_start ? f.member_start[i + 1] - f.member_start[i] : 1) || 1, row: i, cells: cellsOf(f, i), chips: [], discarded: false, reasons: [] }));
  }

  /** the filter of the list: text (id, m/z, formula), only the discarded or hide them */
  function filter(list, { text = "", discarded = false } = {}) {
    const q = text.trim().toLowerCase();
    return list.filter(r => r.discarded === discarded && (!q || String(r.id).includes(q) || (r.mz != null && r.mz.toFixed(4).includes(q)) || (r.formula || "").toLowerCase().includes(q)));
  }

  /** a table of the model UNT (program «mzlab») from the rows of the series: the features enter Match, levels, identities and the exports like any other table */
  function toUnt(f, samples, pol) {
    const rows = Array.from({ length: f.rows }, (_, i) => {
      const c = cellsOf(f, i);
      return { id: i + 1, mz: f.mz[i], rt: f.rt[i], areas: c.map(x => (x.area > 0 ? x.area : 0)), detected: c.map((x, j) => (x.state <= 1 ? j : -1)).filter(j => j >= 0) };
    });
    return G.UNT.fromRows(rows, samples, { name: "mzFinder pipeline", pol });
  }

  // ------------------------------------------------------------ DOM: virtualized list
  const strip = cells => `<span class="pv-cells" role="img" aria-label="${esc(cells.map((c, j) => `file ${j + 1} ${STATE_NAME[c.state]}`).join(", "))}">`
    + cells.map(c => `<span class="pv-c pv-c${c.state}" aria-hidden="true">${STATE[c.state]}</span>`).join("") + "</span>";
  const chips = r => r.chips.map(c => `<span class="pv-chip${c.against ? " pv-against" : ""}">${c.against ? "&#10005; " : ""}${esc(c.name)} ${c.against ? "against" : "+" + c.points}</span>`).join("");
  const reasonText = r => r.reasons.map(c => `<span class="pv-why" title="${esc(REASON[c] || c)}">${esc(c)}</span>`).join(" ");

  function rowHtml(r, i, gated, sel) {
    const cls = "pv-row" + (i === sel ? " pv-sel" : "") + (r.against ? " pv-has-against" : "");
    if (!gated) return `<div class="${cls}" role="row" id="pv-r${i}" aria-rowindex="${i + 2}" style="top:${i * ROW_H}px"><span role="cell">${r.id}</span><span role="cell" class="n">${r.mz.toFixed(4)}</span><span role="cell" class="n">${r.rt.toFixed(2)}</span><span role="cell" class="n">${r.members}</span><span role="cell">${strip(r.cells)}</span></div>`;
    return `<div class="${cls}" data-cand="1" role="row" id="pv-r${i}" aria-rowindex="${i + 2}" style="top:${i * ROW_H}px"><span role="cell">ID${r.id}</span><span role="cell" class="n">${r.mass == null ? "" : r.mass.toFixed(4)}</span>`
      + `<span role="cell">${esc(r.formula)}</span><span role="cell" class="n">${r.total}</span><span role="cell" class="pv-ch">${chips(r)}</span><span role="cell" class="n" title="1/N: N candidates the proofs cannot tell apart">1/${r.n}</span>`
      + `<span role="cell">${strip(r.cells)}</span><span role="cell">${reasonText(r)}</span></div>`;
  }
  const HEAD_PLAIN = ["ID", "m/z", "RT (min)", "Members", "Cells"], HEAD_CAND = ["ID", "Neutral mass", "Formula", "Points", "Contributions", "1/N", "Cells", "Reason"];

  /** mounts the list in `el`; `opts.onPick(row)` on click or Enter. Returns {set(list), pick(i), count()} */
  function mount(el, { gated, onPick = /** @type {(row?: any, i?: number) => void} */ (() => {}) }) {
    const head = gated ? HEAD_CAND : HEAD_PLAIN;
    el.innerHTML = `<div class="pv-head" role="rowgroup"><div role="row" class="pv-hrow">${head.map(h => `<span role="columnheader">${h}</span>`).join("")}</div></div><div class="pv-scroll" role="rowgroup"><div class="pv-space"><div class="pv-win"></div></div></div>`;
    el.setAttribute("role", "grid"); el.setAttribute("aria-label", "Identities"); el.setAttribute("aria-rowcount", "0"); el.tabIndex = 0;
    el.classList.add("pv-grid"); el.classList.toggle("pv-cand", gated);
    const sc = el, hd = el.querySelector(".pv-head"), space = el.querySelector(".pv-space"), win = el.querySelector(".pv-win");
    let list = [], sel = -1, raf = 0;
    const draw = () => {
      raf = 0;
      const h = (sc.clientHeight || 400) - hd.offsetHeight, a = Math.max(0, Math.floor(sc.scrollTop / ROW_H) - 4), b = Math.min(list.length, Math.ceil((sc.scrollTop + h) / ROW_H) + 4);
      let s = ""; for (let i = a; i < b; i++) s += rowHtml(list[i], i, gated, sel);
      win.innerHTML = s;
    };
    const later = () => { if (!raf) raf = requestAnimationFrame(draw); };
    const pick = i => {
      if (i < 0 || i >= list.length) return;
      sel = i; const top = i * ROW_H, vis = sc.clientHeight - hd.offsetHeight;
      if (top < sc.scrollTop) sc.scrollTop = top; else if (top + ROW_H > sc.scrollTop + vis) sc.scrollTop = top + ROW_H - vis;
      draw(); el.setAttribute("aria-activedescendant", "pv-r" + i); onPick(list[i], i);
    };
    sc.addEventListener("scroll", later);
    sc.addEventListener("click", e => { const r = e.target.closest(".pv-row"); if (r) pick(Math.round(parseInt(r.style.top, 10) / ROW_H)); });
    el.addEventListener("keydown", e => {
      const k = { ArrowDown: 1, ArrowUp: -1, PageDown: 10, PageUp: -10 }[e.key];
      if (k) { e.preventDefault(); pick(Math.min(list.length - 1, Math.max(0, (sel < 0 ? 0 : sel) + k))); }
      else if (e.key === "Home" || e.key === "End") { e.preventDefault(); pick(e.key === "Home" ? 0 : list.length - 1); }
      else if (e.key === "Enter" && sel >= 0) { e.preventDefault(); onPick(list[sel], sel); }
    });
    return { set(l) { list = l; sel = -1; space.style.height = l.length * ROW_H + "px"; el.setAttribute("aria-rowcount", String(l.length + 1)); sc.scrollTop = 0; draw(); }, pick, count: () => list.length, scroller: sc };
  }

  const CSS = `#pipe{--pv-d:#0072b2;--pv-t:#e69f00;--pv-f:#56b4e9;--pv-a:#767676}.pv-hrow,.pv-row{display:grid;gap:6px;align-items:center;padding:0 6px}
.pv-hrow,.pv-row{grid-template-columns:56px 90px 80px 70px minmax(90px,1fr)}.pv-cand .pv-hrow,.pv-cand .pv-row{grid-template-columns:56px 90px 110px 50px minmax(160px,1fr) 44px minmax(90px,150px) 150px}
.pv-hrow{font-size:11px;color:var(--muted);border-bottom:1px solid var(--line);min-height:24px}.pv-grid{height:min(420px,50vh);overflow:auto;position:relative;border:1px solid var(--line);border-radius:6px}.pv-head{position:sticky;top:0;z-index:1;background:var(--bg)}.pv-space{position:relative}.pv-win{position:absolute;left:0;right:0;top:0}
.pv-row{position:absolute;left:0;right:0;height:${ROW_H}px;font-size:12px;cursor:pointer;border-bottom:1px solid var(--line);white-space:nowrap;overflow:hidden}.pv-row.pv-sel{background:var(--sel);box-shadow:inset 3px 0 0 var(--accent)}.pv-row .n{text-align:right}
.pv-chip{display:inline-block;padding:0 4px;margin-right:3px;border:1px solid var(--line);border-radius:4px;font-size:10px}.pv-chip.pv-against{color:var(--bad);border-color:var(--bad);font-weight:600}
.pv-c{display:inline-block;width:14px;height:14px;margin-right:1px;text-align:center;font-size:9px;line-height:14px;color:#fff;border-radius:2px}.pv-c0{background:var(--pv-d)}.pv-c1{background:var(--pv-t);color:#000}.pv-c2{background:var(--pv-f);color:#000}.pv-c3{background:var(--pv-a)}
.pv-why{font-size:10px;color:var(--muted)}.pv-det{margin-top:8px;border:1px solid var(--line);border-radius:6px;padding:8px}.pv-det table{border-collapse:collapse;font-size:12px}.pv-det td,.pv-det th{padding:2px 8px;text-align:left}.pv-det .against{color:var(--bad);font-weight:600}
.pv-bar{display:flex;gap:10px;align-items:center;flex-wrap:wrap;margin:6px 0}.pv-bar label{display:flex;gap:4px;align-items:center;min-height:24px}.pv-bar input[type=search]{min-height:28px}`;

  G.PIPEV = { readFeat, locator, identities, plainRows, filter, toUnt, mount, strip, chips, CSS, ROW_H, STATE, STATE_NAME, TYPES, REASON };
  if (typeof module !== "undefined") module.exports = G.PIPEV;
})();
