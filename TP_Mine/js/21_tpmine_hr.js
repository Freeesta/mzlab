// @ts-strict
// TPMINE-PRIVATE  TP Mine, high-resolution mode: ranking of the possible TPs as a TPTAB table (42_tp_table.js) whose open row is the TPRIGA panel (44_tp_riga.js, tabs Overview, Samples,
// Identity, Quality) built from the detail of the candidate and from riga/1 of the table of the Dig; "More" (mass differences, funnel, RT alignment, MSn tree) and the exports.
// Called by 20_tpmine.js (window.TPHR.render) with its helpers; it does not talk to the worker itself.
(() => {
  "use strict";
  const CSS = `
.tp .comp{display:grid;grid-template-columns:90px 1fr 42px;gap:6px;align-items:center;font-size:12px}.tp .comp i{display:block;height:8px;background:var(--accent);border-radius:4px}
.tp .flag{color:#9a5b00;font-size:11px}
.tp .fl-s{color:#b42318;font-weight:600}.tp .fl-u{color:#2b5c8a;font-weight:600}
.tp{--sim-eq:#0072B2;--sim-sh:#D55E00;--sim-nl:#009E73}
.tp{--rta-0:#000000;--rta-1:#E69F00;--rta-2:#56B4E9;--rta-3:#009E73;--rta-4:#F0E442;--rta-5:#0072B2;--rta-6:#D55E00;--rta-7:#CC79A7}
.tp .rta-svg{display:block;width:100%;max-width:640px;height:auto}.tp .rta-svg text{fill:var(--muted);font-size:10px}.tp .rta-svg .ax{stroke:var(--line);stroke-width:1}
.tp .rta-svg .zero{stroke:var(--muted);stroke-dasharray:2 3}.tp .rta-leg{display:flex;flex-wrap:wrap;gap:2px 14px;font-size:11px;margin-top:4px}
.tp .rta-open{display:inline-block;width:8px;height:8px;border:2px solid var(--muted);border-radius:50%;vertical-align:middle;margin-right:4px}
.tp .sim-bar{display:inline-flex;width:64px;height:8px;border-radius:4px;background:var(--soft);overflow:hidden;vertical-align:middle}.tp .sim-bar i{display:block;height:100%}
.tp .sim3{display:grid;grid-template-columns:150px 1fr 1fr 44px;gap:4px 10px;font-size:12px;align-items:center;margin:6px 0}
.tp .sim3 .bar3{display:block;height:8px;border-radius:4px;background:var(--soft);overflow:hidden}.tp .sim3 .bar3 i{display:block;height:100%}
.tp .sw{display:inline-block;width:14px;height:0;border-top:3px solid;vertical-align:middle;margin-right:5px}
.tp .mnet-wrap{overflow:auto;max-height:70vh;border:1px solid var(--line);border-radius:6px;background:var(--panel)}
.tp .mn{cursor:pointer}.tp .mn:focus-visible circle,.tp .mn.sel circle{stroke-width:3.5}.tp .mn text{fill:var(--ink);font-size:10px;text-anchor:middle;pointer-events:none}
.tp .me text{fill:var(--muted);font-size:9px;text-anchor:middle}
.tp .hr-sum{margin:0 0 8px;font-size:13px}.tp .hr-sum .mut{font-size:12px}
.tp #hr-tbl .tpt-n{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap}
.tp .hr-more{margin-top:10px}.tp .hr-more>details{border:1px solid var(--line);border-radius:6px;background:var(--panel);padding:2px 10px;margin:0 0 4px}.tp .hr-more>details>summary{min-height:28px;display:flex;align-items:center;color:var(--ink)}
.tp .hr-more>details[open]{padding-bottom:8px}
.tp .hr-sg{display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:6px 14px;align-items:start}.tp .hr-sg canvas{display:block;width:100%;height:180px}
.tp .hr-idx{max-height:240px;overflow:auto;margin-top:6px;border-top:1px solid var(--line);padding-top:4px;font-size:12px}.tp .hr-idx h4{margin:6px 0 2px;font-size:12px;color:var(--muted)}
.tp .hr-idx .kv{display:grid;grid-template-columns:max-content 1fr;gap:2px 12px}.tp .hr-idx .kv b{color:var(--muted);font-weight:600}
.tp .hr-idx:focus-visible{outline:2px solid var(--accent);outline-offset:1px}`;
  /** quality of the level, 0 best: diagnostic evidence, then L3, L4, L5 @param {any} r */
  const lvQ = r => r.diagnostic ? 0 : +r.level - 2;
  const SIM_LABEL = ["equal fragments", "shifted fragments", "common neutral losses"], SIM_VAR = ["--sim-eq", "--sim-sh", "--sim-nl"];
  const simBar = sim => sim ? `<span class="sim-bar" role="img" aria-label="cosine: equal ${sim.cos[0]}, shifted ${sim.cos[1]}, neutral losses ${sim.cos[2]}" title="cosine: equal ${sim.cos[0]} · shifted ${sim.cos[1]} · common neutral losses ${sim.cos[2]}">${sim.cos.map((v, i) => `<i style="width:${Math.round(100 * v)}%;background:var(${SIM_VAR[i]})"></i>`).join("")}</span>` : "";
  /** The three components of the similarity of a product's MS2 with the parent's (also used by 20_tpmine.js for the low-resolution detail). */
  function simCard(sim) {
    const row = i => { const k = ["equal", "shifted", "loss"][i]; return `<span><span class="sw" style="border-color:var(${SIM_VAR[i]});border-top-style:${i === 0 ? "solid" : i === 1 ? "dashed" : "dotted"}"></span>${SIM_LABEL[i]}</span>
      <span class="bar3" title="cosine ${sim.cos[k].toFixed(3)}"><i style="width:${Math.round(100 * sim.cos[k])}%;background:var(${SIM_VAR[i]})"></i></span>
      <span class="bar3" title="entropy ${sim.ent[k].toFixed(3)}"><i style="width:${Math.round(100 * sim.ent[k])}%;background:var(${SIM_VAR[i]})"></i></span><span class="n">${sim.n[k]}</span>`; };
    return `<div class="sim3"><b></b><b>cosine</b><b>entropy</b><b title="matched peaks">n</b>${[0, 1, 2].map(row).join("")}
      <b>total</b><span class="n">${sim.cos.total.toFixed(3)} <span class="mut">(modified cosine)</span></span><span class="n">${sim.ent.total.toFixed(3)}</span><span></span></div>
      <div class="mut">Mass difference of the precursors ${sim.delta >= 0 ? "+" : ""}${sim.delta.toFixed(4)}. Equal: the fragment does not contain the modification. Shifted: it moved with the precursor, so it contains it. Common neutral losses: shifted fragments close to the precursor (the same small neutral is lost by both).</div>`;
  }
  // Shift of the apex of the anchor features in every file against the median over the files (piecewise linear between the nodes of the curve).
  const RTA_DASH = [[], [6, 3], [2, 3], [8, 2, 2, 2]];
  function rtAlign(a, esc) {
    if (!a || !a.rt || !a.rt.length) return "";
    const W = 560, H = 200, L = 46, T = 10, B = 30, Rt = 12, lo = Math.min(...a.rt), hi = Math.max(...a.rt, lo + 1);
    const ym = Math.max(0.05, ...a.shift.flat().map(Math.abs)), X = v => L + (v - lo) / (hi - lo) * (W - L - Rt), Y = v => T + (1 - (v + ym) / (2 * ym)) * (H - T - B);
    const line = (k, sh) => `<polyline fill="none" stroke="var(--rta-${k % 8})" stroke-width="1.6" stroke-dasharray="${RTA_DASH[Math.floor(k / 8) % 4].join(" ")}" points="${a.rt.map((r, j) => X(r).toFixed(1) + "," + Y(sh[j]).toFixed(1)).join(" ")}"/>`;
    const xt = [0, 1, 2, 3, 4].map(i => lo + (hi - lo) * i / 4), yt = [-ym, 0, ym];
    const svg = `<svg class="rta-svg" viewBox="0 0 ${W} ${H}" role="img" aria-label="${esc(`RT shift of ${a.files.length} files against the median, ${a.anchors} anchor features, largest shift ${ym.toFixed(2)} min`)}">
      <line class="ax" x1="${L}" y1="${T}" x2="${L}" y2="${H - B}"/><line class="ax" x1="${L}" y1="${H - B}" x2="${W - Rt}" y2="${H - B}"/><line class="zero" x1="${L}" y1="${Y(0)}" x2="${W - Rt}" y2="${Y(0)}"/>
      ${xt.map(v => `<text x="${X(v).toFixed(1)}" y="${H - B + 14}" text-anchor="middle">${v.toFixed(1)}</text>`).join("")}${yt.map(v => `<text x="${L - 4}" y="${(Y(v) + 3).toFixed(1)}" text-anchor="end">${v > 0 ? "+" : ""}${v.toFixed(2)}</text>`).join("")}
      <text x="${(L + W - Rt) / 2}" y="${H - 4}" text-anchor="middle">RT (min)</text><text transform="rotate(-90)" x="${-(T + H - B) / 2}" y="11" text-anchor="middle">shift (min)</text>
      ${a.shift.map((sh, k) => line(k, sh)).join("")}</svg>`;
    const leg = a.files.map((f, k) => `<span><span class="sw" style="border-top-color:var(--rta-${k % 8});border-top-style:${RTA_DASH[Math.floor(k / 8) % 4].length ? "dashed" : "solid"}"></span>${esc(f)}</span>`).join("");
    return `<details name="hr-more" id="hr-rta"><summary>Retention time alignment (${a.anchors} anchor features)</summary>
      <div class="mut">Shift of the apex of the anchors (strong, alone in their <i>m/z</i> and RT neighbourhood, present in the files) in each file against the median of the files. No signal is moved: the curve tells where a feature is expected in a file when its area is filled.</div>${svg}<div class="rta-leg">${leg}</div></details>`;
  }

  /** Columns of the table of the products: label, help, sort value and cell of each; `level` and `identity` stay fixed on the left. */
  function tableCols(/** @type {any} */ h, /** @type {Map<any, number>} */ rank, /** @type {any} */ s) {
    const { esc, fmtA } = h, TPTAB = /** @type {any} */ (window).TPTAB, ion = esc(s.parent.ion || "");
    const change = (/** @type {any} */ r) => (typeof TPTR !== "undefined") ? TPTR.change(r.derivation || r.delta || "") : esc(r.derivation || r.delta || "");
    const states = (/** @type {any} */ r) => /** @type {number[]} */ (r.series || []).map((a, k) => (r.filled || []).includes(k) ? "filled" : a > 0 ? "detected" : "absent");
    const drt = (/** @type {any} */ r) => s.parent.rt ? +(r.rt - s.parent.rt).toFixed(2) : null;
    return [
      { key: "level", label: "Lvl", help: "ft.col.level", sticky: true, sort: (/** @type {any} */ r) => lvQ(r) * 1000 - r.score, cell: (/** @type {any} */ r) => TPTAB.level(r.level), text: (/** @type {any} */ r) => `L${r.level}${r.diagnostic ? " diagnostic" : ""}` },
      { key: "identity", label: "Identity", help: "ft.col.identity", sticky: true, sort: (/** @type {any} */ r) => r.formula || "", cell: (/** @type {any} */ r) => r.formula ? `${r.derivation || r.delta ? change(r) + " · " : ""}${esc(r.formula)} ${ion}` : `<i>m/z</i> ${r.mz.toFixed(4)}`,
        text: (/** @type {any} */ r) => r.formula ? `${r.derivation || r.delta || ""}${r.derivation || r.delta ? " · " : ""}${r.formula} ${s.parent.ion || ""}` : `m/z ${r.mz.toFixed(4)}` },
      { key: "mz", label: "<i>m/z</i>", help: "ft.col.mz", num: true, sort: (/** @type {any} */ r) => r.mz, cell: (/** @type {any} */ r) => r.mz.toFixed(4) },
      { key: "rt", label: "RT", help: "ft.col.rt", num: true, sort: (/** @type {any} */ r) => r.rt, cell: (/** @type {any} */ r) => r.rt.toFixed(2) },
      { key: "trend", label: "Trend", help: "ft.col.trend", cell: (/** @type {any} */ r) => TPTAB.spark(r.series, "area"), text: (/** @type {any} */ r) => (r.series || []).map((/** @type {number} */ v) => Math.round(v)).join(" ") },
      { key: "cells", label: "Cells", help: "ft.col.cells", cell: (/** @type {any} */ r) => TPTAB.cells(states(r)), text: (/** @type {any} */ r) => states(r).map((/** @type {string} */ x) => x[0]).join("") },
      { key: "score", off: true, label: "Score", help: "ft.col.score", num: true, sort: (/** @type {any} */ r) => r.score, cell: (/** @type {any} */ r) => String(r.score) },
      { key: "pred", off: true, label: "Prev.", help: "ft.col.pred", sort: (/** @type {any} */ r) => r.pred ?? 9, cell: (/** @type {any} */ r) => r.pred ? TPTAB.pred(r.pred) : "", text: (/** @type {any} */ r) => r.pred ? `Prev. ${r.pred}` : "" },
      { key: "ppm", off: true, label: "ppm", help: "ft.col.ppm", num: true, sort: (/** @type {any} */ r) => r.ppm == null ? null : Math.abs(r.ppm), cell: (/** @type {any} */ r) => r.ppm == null ? "" : `${r.ppm >= 0 ? "+" : ""}${r.ppm}` },
      { key: "drt", off: true, label: "ΔRT", help: "ft.col.drt", num: true, sort: drt, cell: (/** @type {any} */ r) => { const v = drt(r); return v == null ? "" : `${v >= 0 ? "+" : ""}${v.toFixed(2)}`; } },
      { key: "sim", off: true, label: "Sim.", help: "ft.col.sim", sort: (/** @type {any} */ r) => r.modcos, cell: (/** @type {any} */ r) => simBar(r.sim), text: (/** @type {any} */ r) => r.modcos == null ? "" : String(r.modcos) },
      { key: "ms2", off: true, label: "MS2", help: "ft.col.ms2", num: true, sort: (/** @type {any} */ r) => r.ms2_scans, cell: (/** @type {any} */ r) => String(r.ms2_scans || "") },
      { key: "area", off: true, label: "Max area", help: "ft.col.maxarea", num: true, sort: (/** @type {any} */ r) => r.area_max, cell: (/** @type {any} */ r) => fmtA(r.area_max || 0) },
      { key: "tmax", off: true, label: "t max", help: "ft.col.tmax", num: true, sort: (/** @type {any} */ r) => r.tmax, cell: (/** @type {any} */ r) => String(r.tmax ?? "") },
      { key: "class", off: true, label: "Class", help: "ft.col.class", sort: (/** @type {any} */ r) => r.class || "", cell: (/** @type {any} */ r) => esc(r.class || "") },
      { key: "region", off: true, label: "Region", help: "ft.col.region", sort: (/** @type {any} */ r) => r.region || "", cell: (/** @type {any} */ r) => esc(r.region || "") },
      { key: "rank", off: true, label: "#", help: "ft.col.rank", num: true, sort: (/** @type {any} */ r) => rank.get(r), cell: (/** @type {any} */ r) => String(rank.get(r)) },
    ];
  }

  /** the summary line, the exports and the sections of "More" around the table @param {any} s @param {any} rows @param {(t: any) => string} esc */
  function frame(s, rows, esc) {
    const n = l => rows.filter(r => String(r.level) === l && !r.diagnostic).length, nDiag = rows.filter(r => r.diagnostic).length;
    const nMs2 = rows.filter(r => r.ms2_scans > 0).length;
    const nodes = (s.tree || []).length, nEdges = (s.mass_network || { n_edges: 0 }).n_edges;
    const sessions = [...new Set(s.sessions.labels)].map(l => l + " (" + s.sessions.labels.filter(x => x === l).length + " files)").join(", ");
    return `<div class="hr-sum"><b>Ranking of possible TPs</b> · <span id="hr-cnt">${rows.length} of ${rows.length}</span> · <span class="mut">parent ${esc(s.parent.name || "")} ${esc(s.parent.formula)} ${esc(s.parent.ion)} <i>m/z</i> ${s.parent.mz}${s.parent.rt ? ` · RT ${s.parent.rt} min` : ""} · ${s.timing.total} s · ${nDiag} diagnostic · ${n("3")} L3 · ${n("4") + n("5")} L4-L5 · ${nMs2} with MS<sup>2</sup> · ${s.n_candidates} candidates after filters · ${s.files.length} files</span></div>
      ${s.warnings.length ? `<div class="card">${s.warnings.map(w => `<div class="flag">⚠ ${esc(w)}</div>`).join("")}</div>` : ""}
      <div class="row" style="margin:0 0 6px"><button id="hr-x-xlsx">Excel (.xlsx)</button><button id="hr-x-csv">CSV</button><button id="hr-x-json">JSON</button><button id="hr-x-incl" title="Inclusion list for a second targeted injection: the best candidates without MS2">Inclusion list (CSV)</button></div>
      <div id="hr-tbl"></div>
      <div class="hr-more">
        <details name="hr-more" id="hr-m-net"><summary>Mass differences (${nEdges} edges)</summary><div id="hr-mnet"></div></details>
        <details name="hr-more" id="hr-m-funnel"><summary>Funnel and sessions (${s.funnel.length} filters)</summary>${s.funnel.map(f => `<div class="mut">${f.n.toLocaleString("en")} · ${esc(f.text)}</div>`).join("")}
          <div class="mut" style="margin-top:6px">Sessions: ${esc(sessions)}${s.sessions.factors.applied ? "; areas rescaled to the reference session" : "; no correction needed"}.</div>
          <div class="mut">Timings (s): ${esc(Object.entries(s.timing).map(([k, v]) => k + " " + v).join(" · "))}</div></details>
        ${rtAlign(s.rt_align, esc)}
        <details name="hr-more" id="hr-m-tree"><summary>MSn tree${nodes ? ` (${nodes} nodes)` : ""}</summary><div id="hr-tree"></div></details>
      </div>`;
  }

  function render(R, s, h) {
    if (!document.getElementById("tp-hr-css")) { const st = document.createElement("style"); st.id = "tp-hr-css"; st.textContent = CSS; document.head.appendChild(st); }
    const { esc, fmtA, chart, hue, call, toast, dl } = h;
    const $ = q => R.querySelector(q);
    const TPTAB = /** @type {any} */ (window).TPTAB, TPRIGA = /** @type {any} */ (window).TPRIGA, rows = s.rows, st = { sel: /** @type {number | null} */ (null), det: /** @type {Record<string, any>} */ ({}), row: /** @type {Record<string, any>} */ ({}), net: /** @type {any} */ (null) };
    R.innerHTML = frame(s, rows, esc);

    const rank = new Map(rows.map((r, i) => [r, i + 1]));
    const COLS = tableCols(h, rank, s);
    /** @param {string} key @param {string} label @param {string} help @param {(r: any) => boolean} test @param {string} [off] */
    const chip = (key, label, help, test, off) => ({ key, label, help, test, ...(off ? { off } : {}) });

    // ---- the open row: the head at once, the tabs when the detail of the candidate and riga/1 have arrived
    /** @type {Map<number, HTMLElement>} */ const heads = new Map();
    /** the sentence of the head: how much evidence there is, in words @param {any} r */
    /** @param {any} r */
    const decide = TPRIGA && (/** @type {any} */ (window).TPDEC) ? { series: () => ({ files: s.files, formula: s.parent.formula }), key: (/** @type {any} */ r) => (/** @type {any} */ (window).TPDEC).rowKey(r.mz, r.rt, true), level: (/** @type {any} */ r) => r.level } : null;
    /** the head repeats the closed row: its key cells */
    const summaryOf = (/** @type {any} */ r) => [{ label: "Neutral formula", value: r.formula || "", help: "ft.riga.neutral" }, { label: "m/z", value: r.mz.toFixed(4), help: "ft.col.mz" }, { label: "RT", value: r.rt == null ? "" : (+r.rt).toFixed(2) + " min", help: "ft.col.rt" },
      { label: "Δppm", value: r.ppm == null ? "" : (+r.ppm).toFixed(1), help: "ft.riga.ppm" }, { label: "MS2", value: r.ms2_scans ? r.ms2_scans + " scans" : "none", help: "ft.pf.ms2" }];
    const baseModel = r => ({ title: r.formula || "m/z " + r.mz, level: r.level, pred: r.pred, summary: summaryOf(r), decide: decide ? { series: decide.series(), key: decide.key(r), level: r.level } : undefined, actions: [{ key: "copy", label: "Copy", help: "ft.riga.copy", run: () => {
      const text = [r.formula || "", "m/z " + r.mz, "RT " + r.rt, "L" + r.level + (r.diagnostic ? " diagnostic" : "")].join("\t");
      Promise.resolve(navigator.clipboard && navigator.clipboard.writeText(text)).then(() => toast("Copied"), () => toast("Copy is not allowed here"));
    } }] });
    function hook() {
      return { tabs: TPRIGA.tabs, head: (/** @type {any} */ r, /** @type {HTMLElement} */ box) => { heads.set(r.id, box); TPRIGA.head(box, baseModel(r)); },
        onOpen: (/** @type {any} */ r, /** @type {HTMLElement} */ body, /** @type {string} */ tab) => { void fillTab(r, body, tab); } };
    }

    const T = TPTAB.mount($("#hr-tbl"), { id: "hr-t", label: "Possible transformation products", cols: COLS, rowId: (/** @type {any} */ r) => r.id, esc, name: (/** @type {any} */ r) => `${r.formula || "m/z " + r.mz}, m/z ${r.mz}`, columns: true, decide: decide || undefined, exportName: (s.parent.formula || "parent") + "_TPMine_HR_view",
      chips: [chip("l4", "L4 or better", "ft.chip.level", r => +r.level <= 4), chip("l3", "L3 or better", "ft.chip.level", r => +r.level <= 3), chip("diag", "diagnostic", "ft.chip.diagnostic", r => !!r.diagnostic),
        chip("ms2", "With MS2", "ft.chip.ms2", r => r.ms2_scans > 0), chip("not0", "Not at t0", "ft.chip.not0", r => !r.ctl || !r.ctl.in_t0, "ft.chip.wait"), chip("noblank", "Not in blank", "ft.chip.noblank", r => !r.ctl || !r.ctl.in_blank, "ft.chip.wait"),
        chip("region", "With region", "ft.chip.region", r => !!r.region)],
      search: (/** @type {any} */ r) => [r.formula, String(r.mz), r.region, r.delta, r.derivation].join(" "),
      onCount: (/** @type {number} */ k, /** @type {number} */ all) => { const c = $("#hr-cnt"); if (c) c.textContent = `${k} of ${all}`; },
      ...hook() });
    T.setRows(rows); T.sortBy("rank", 1);
    if (window.TPALBERO) window.TPALBERO.mount($("#hr-tree"), h, { compareId: () => st.sel });
    // the numbers of the table that the ranking does not carry (t0, blank) come from the Rust table of the products; a run served by Python has none and the two chips stay off
    call("tp_table", rows.map(r => r.id)).then(d => {
      if (!d || !d.rows) return;
      const by = new Map(d.rows.map(x => [x.row, x]));
      rows.forEach(r => { const x = by.get(r.id); if (x) r.ctl = { in_t0: x.in_t0, in_blank: x.in_blank, in_t0_ratio: x.t0_ratio ?? 0, in_blank_ratio: x.blank_ratio ?? 0 }; });
      T.enableChip("not0"); T.enableChip("noblank"); T.setRows(rows);
    }).catch(() => { /* no numbers: the chips stay off */ });


    const fmtDm = v => (v >= 0 ? "+" : "") + v.toFixed(4);
    function massNet() {
      const host = $("#hr-mnet"), net = s.mass_network, ns = st.net || (st.net = { only: "all", every: false });
      if (!net || !net.edges.length) { host.innerHTML = `<div class="mut">No candidate differs from the parent or from another ion of the series by a transformation of the list.</div>`; return; }
      const lab = n => n.id === "progenitore" ? "parent" : (n.formula || "");
      const best = net.nodes.filter(n => n.id !== "progenitore").sort((a, b) => (b.score || 0) - (a.score || 0));
      const keepIds = new Set(["progenitore", ...(ns.every ? best : best.slice(0, 60)).map(n => n.id)]);
      const edges = net.edges.filter(e => keepIds.has(e.from) && keepIds.has(e.to) && (ns.only === "all" || (ns.only === "network" ? e.kind !== "mass" : (e.from === st.sel || e.to === st.sel))));
      const nodes = net.nodes.filter(n => keepIds.has(n.id) && (n.id === "progenitore" || edges.some(e => e.from === n.id || e.to === n.id)));
      const depth = new Map([["progenitore", 0]]);
      for (let ch = true; ch;) { ch = false; for (const e of edges) if (depth.has(e.from) && !depth.has(e.to)) { depth.set(e.to, /** @type {number} */ (depth.get(e.from)) + 1); ch = true; } }
      const last = Math.max(0, ...depth.values()) + 1;
      nodes.forEach(n => { if (!depth.has(n.id)) depth.set(n.id, last); });
      const cols = [...new Set(nodes.map(n => depth.get(n.id)))].sort((a, b) => a - b), col = new Map(cols.map((d, i) => [d, i])), pos = new Map();
      cols.forEach(d => nodes.filter(n => depth.get(n.id) === d).sort((a, b) => (a.mz || 0) - (b.mz || 0)).forEach((n, i) => pos.set(n.id, { x: 60 + /** @type {number} */ (col.get(d)) * 200, y: 34 + i * 50 })));
      const W = 120 + (cols.length - 1) * 200, H = 40 + Math.max(1, ...cols.map(d => nodes.filter(n => depth.get(n.id) === d).length)) * 50;
      const showLab = edges.length <= 40;
      const line = e => {
        const a = pos.get(e.from), b = pos.get(e.to); if (!a || !b) return "";
        const dx = b.x - a.x, dy = b.y - a.y, L = Math.hypot(dx, dy) || 1, ux = dx / L, uy = dy / L, x1 = a.x + ux * 16, y1 = a.y + uy * 16, x2 = b.x - ux * 18, y2 = b.y - uy * 18;
        const stroke = e.kinetic === "against" ? "var(--bad)" : "var(--muted)", dash = e.kind === "both" ? "" : e.kind === "mass" ? "6 3" : "2 3", on = e.from === st.sel || e.to === st.sel;
        const tip = `${e.rule} (${fmtDm(e.dm)})${e.kind === "network" ? ", product network" : ""}${e.kinetic === "against" ? ", the maximum of the product is before its source" : ""}${e.shifted ? `, ${e.shifted.n} shifted fragments` : ""}`;
        return `<g class="me"><line x1="${x1.toFixed(1)}" y1="${y1.toFixed(1)}" x2="${x2.toFixed(1)}" y2="${y2.toFixed(1)}" stroke="${stroke}" stroke-width="${e.kind === "both" || on ? 2.4 : 1.3}" ${dash ? `stroke-dasharray="${dash}"` : ""} marker-end="url(#mnarr)"><title>${esc(tip)}</title></line>
          ${showLab || on ? `<text x="${((x1 + x2) / 2).toFixed(1)}" y="${((y1 + y2) / 2 - 3).toFixed(1)}">${esc(e.rule)} ${fmtDm(e.dm)}${e.shifted && e.shifted.n ? " ✓" + e.shifted.n : ""}</text>` : ""}</g>`;
      };
      const node = n => { const p = pos.get(n.id), lv = n.level, c = lv ? `var(--lv${lv})` : "var(--accent)";
        return `<g class="mn${n.id === st.sel ? " sel" : ""}" data-id="${n.id}" transform="translate(${p.x},${p.y})" tabindex="0" role="button" aria-label="${esc(lab(n))}${n.mz ? `, m/z ${n.mz}` : ""}${lv ? `, level ${lv}` : ""}">
          <circle r="14" style="fill:color-mix(in srgb,${c} 25%,var(--panel));stroke:${c}" stroke-width="2"></circle><text y="4">${n.id === "progenitore" ? "M" : lv || ""}</text><text y="29">${esc(lab(n))}</text></g>`; };
      const row = e => { const f = id => id === "progenitore" ? "parent" : ((net.nodes.find(n => n.id === id) || {}).formula || id);
        return `<tr><td>${esc(f(e.from))}</td><td>${esc(f(e.to))}</td><td>${esc(e.rule)}</td><td class="n">${fmtDm(e.dm)}</td><td>${e.scope ? esc(e.scope) : ""}</td><td>${esc(e.kinetic)}</td><td class="n">${e.shifted ? e.shifted.n : ""}</td><td>${e.kind === "both" ? "mass difference and product network" : e.kind === "mass" ? "mass difference" : "product network"}</td></tr>`; };
      host.innerHTML = `<div class="mut">${edges.length} of ${net.n_edges} edges. An arrow joins two ions whose formulas differ by a transformation of the list, from the one that peaks first. Solid: it is also the precursor chosen in the product network; dashed: only the mass difference; dotted: precursor of the network that is not a single rule (two steps or related by MS²); red: the maximum of the product is before its source. ✓n: fragments shifted by the mass difference in the MS² of both ions. The circle shows the confidence level.</div>
        <div class="row"><label class="mut" for="hr-mn-f">Show</label><select id="hr-mn-f"><option value="all">all mass differences</option><option value="network">product network only</option><option value="sel">only those of the selected candidate</option></select>
          <label><input type="checkbox" id="hr-mn-all"> all ions (not only the 60 best)</label></div>
        <div class="mnet-wrap"><svg width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" role="group" aria-label="Network of mass differences: ${nodes.length} ions, ${edges.length} edges"><defs><marker id="mnarr" markerWidth="8" markerHeight="8" refX="6" refY="4" orient="auto"><path d="M0,0 L8,4 L0,8 z" fill="var(--muted)"></path></marker></defs>
          ${edges.map(line).join("")}${nodes.map(node).join("")}</svg></div>
        <details><summary>Edges as a table (${edges.length})</summary><div class="tbl"><table><tr><th>From</th><th>To</th><th>Rule</th><th>Δ<i>m/z</i></th><th>Rule scope</th><th>Kinetics</th><th>Shifted fragments</th><th>Kind</th></tr>${edges.slice(0, 300).map(row).join("")}</table></div></details>`;
      $("#hr-mn-f").value = ns.only; $("#hr-mn-all").checked = ns.every;
      $("#hr-mn-f").onchange = e => { ns.only = e.target.value; massNet(); }; $("#hr-mn-all").onchange = e => { ns.every = e.target.checked; massNet(); };
      host.querySelectorAll("g.mn").forEach(g => { const go = () => { if (g.dataset.id !== "progenitore") show(+g.dataset.id); }; g.onclick = go; g.onkeydown = e => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); go(); } }; });
    }

    async function detail(/** @type {number} */ id) { return st.det[id] || (st.det[id] = await call("detail", id)); }
    /** riga/1 of the table of the Dig with the traces of the row (XIC of every file, MS1 at the best apex); null when the engine of the table cannot give them (the app, a Python run) */
    async function rowInfo(/** @type {number} */ id) {
      if (!(id in st.row)) st.row[id] = await call("tp_row", id).catch(() => null);
      return st.row[id];
    }
    /** opens the row of a candidate (its sheet is shown right under it); a row hidden by the filters brings the table back to "all" */
    function show(/** @type {number} */ id) {
      if (T.open(id)) return;
      T.reset(); T.open(id);
    }
    const fileName = (/** @type {number} */ k) => { const f = s.files[k]; return typeof f === "string" ? f : f && f.name ? String(f.name) : "file " + (k + 1); };
    const GROUP = /** @type {Record<string, string>} */ ({ sample: "Treated", t0: "t0", blank: "Blank", dark: "Dark", std: "Other" });

    /** the model of the panel of a row (TPRIGA): what the detail of the candidate and riga/1 hold; nothing is computed here @param {any} r @param {any} d @param {any} x */
    function model(r, d, x) {
      const row = x && x.row, tr = x && x.traces, q = row && row.quality, ms2 = d.ms2 && d.ms2.candidate && d.ms2.candidate.mz && d.ms2.candidate.mz.length ? d.ms2 : null;
      const cells = row ? row.cells : null, fl = new Set(d.row.filled || []);
      const group = /** @type {Record<string, string>} */ ({}), time = /** @type {Record<string, number | null>} */ ({});
      (cells || []).forEach((/** @type {any} */ c) => { group[c.file] = GROUP[c.type] || c.type || "Other"; time[c.file] = c.time; });
      /** @type {Record<string, string>} */ const missing = {};
      const m = /** @type {any} */ ({ ...baseModel(r), apexRt: row ? row.rt : r.rt, lo: row ? row.rt_lo : null, hi: row ? row.rt_hi : null, missing });
      if (tr && tr.xic) m.xic = tr.xic.map((/** @type {any} */ t) => ({ file: t.file, group: group[t.file] || "Treated", time: time[t.file] ?? null, rt: t.rt, y: t.y })); else missing.xic = "the XIC needs the index files of the series, which this version cannot read here";
      if (ms2) m.spectrum = { kind: "mirror", mz: ms2.candidate.mz, y: ms2.candidate.rel, ref: { mz: ms2.parent.mz, y: ms2.parent.rel } };
      else if (tr && tr.ms1 && tr.ms1.mz && tr.ms1.mz.length) m.spectrum = { kind: "ms1", mz: tr.ms1.mz, y: tr.ms1.y };
      else missing.spectrum = "no MS2 for this ion";
      m.xicTip = row ? [`RT ${row.rt.toFixed(2)} min${row.rt_lo != null && row.rt_hi != null ? ` (${row.rt_lo.toFixed(2)}-${row.rt_hi.toFixed(2)})` : ""}`, q && q.fwhm != null ? `FWHM ${q.fwhm}` : "", row.charge != null ? `charge ${row.charge}` : "",
        row.ms2 ? `MS2 from ${row.ms2.file}${row.ms2.purity != null ? ` (purity ${row.ms2.purity.toFixed(2)})` : ""}` : ""].filter(Boolean).join(" · ") + "." : "";
      const R = /** @type {any} */ (window).TPRIGA && /** @type {any} */ (window).TPRIGA.toData; if (R) m.open = { xic: () => R.xic(r.mz) };
      m.samples = cells ? cells.map((/** @type {any} */ c) => ({ file: c.file, type: GROUP[c.type] || c.type, time: c.time, area: c.area, state: c.state, apexRt: c.apex_rt, lo: row.rt_lo, hi: row.rt_hi }))
        : d.times.map((/** @type {number} */ t, /** @type {number} */ k) => ({ file: fileName(k), time: t >= 0 ? t : null, area: d.areas[k], state: fl.has(k) ? "filled" : d.areas[k] > 0 ? "detected" : "absent" }));
      const pk = (/** @type {string} */ n) => /persist/i.test(n) ? "persist" : /time zero|\bt0\b/i.test(n) ? "t0" : /grow/i.test(n) ? "grow" : /isotop/i.test(n) ? "iso" : /ms2|fragment/i.test(n) ? "ms2" : undefined;
      const proofs = d.criteria.filter((/** @type {any} */ c) => c.status === "pass" || c.status === "fail").map((/** @type {any} */ c) => ({ key: pk(c.name), ok: c.status === "pass", text: `${c.name}: ${c.text}` }));
      m.identity = { neutral: row && row.ident ? row.ident.neutral : null, adduct: (row && row.ident && row.ident.adduct) || s.parent.ion,
        candidates: [{ formula: r.formula || "", ppm: r.ppm, adduct: s.parent.ion, source: row && row.ident && row.ident.sources ? row.ident.sources.join(", ") : r.derivation ? "transformation list" : "", level: r.level, pred: r.pred, proofs }] };
      if (!r.formula) missing.candidates = "no formula within the tolerance for this ion";
      const flags = [...(q && q.flags ? q.flags : []), ...(r.flags || [])];
      m.quality = q || r.ctl || flags.length ? { shape: q ? q.shape : null, snr: q ? q.snr : null, fwhm: q ? q.fwhm : null, flags, isoM1: row ? row.iso_m1 : null,
        blank: r.ctl ? { present: r.ctl.in_blank, ratio: r.ctl.in_blank_ratio } : null, t0: r.ctl ? { present: r.ctl.in_t0, ratio: r.ctl.in_t0_ratio } : null, rejected: [] } : null;
      missing.quality = "no quality figures for this row";
      return m;
    }

    /** the tab `tab` of the open row `r` into `body`: TPRIGA draws the four parts, this adds what only the high-resolution table has (kinetics by session, score, localisation, similarity) @param {any} r @param {HTMLElement} body @param {string} tab */
    async function fillTab(r, body, tab) {
      st.sel = r.id; if (st.net && st.net.only === "sel") massNet();
      body.innerHTML = `<div class="mut">Computing...</div>`;
      let d, x; try { [d, x] = await Promise.all([detail(r.id), rowInfo(r.id)]); } catch (e) { body.innerHTML = `<div class="err">${esc(/** @type {any} */ (e).message || e)}</div>`; return; }
      if (T.openId !== r.id || T.tab !== tab) return;                         // the user opened another row or tab while this one was loading
      const m = model(r, d, x);
      if (tab === "samples") {
        body.innerHTML = `<div class="hr-sg"><div class="hr-s1"></div><div class="tpr-ch"><h4>Kinetics by session</h4><canvas id="hr-kin"></canvas><div class="row" id="hr-kleg"></div></div></div>`;
        TPRIGA.fill(/** @type {HTMLElement} */ (body.querySelector(".hr-s1")), tab, m); kinetics(d);
      } else {
        TPRIGA.fill(body, tab, m);
        if (tab === "identity") identityExtras(r, d, body);
      }
    }

    /** the kinetics of the product by session (chart and table of the data) @param {any} d */
    function kinetics(d) {
      const labs = [...new Set(d.sessions)], t = d.times, series = /** @type {any[]} */ ([]);
      labs.forEach((lab, j) => {
        const ix = t.map((/** @type {number} */ _, /** @type {number} */ i) => i).filter((/** @type {number} */ i) => d.sessions[i] === lab && t[i] >= 0).sort((/** @type {number} */ a, /** @type {number} */ b) => t[a] - t[b]);
        const fl = new Set(d.row.filled || []);
        if (ix.length) series.push({ x: ix.map((/** @type {number} */ i) => t[i]), y: ix.map((/** @type {number} */ i) => d.areas[i]), color: hue(j, Math.max(2, labs.length)), dots: true, hollow: ix.map((/** @type {number} */ i, /** @type {number} */ p) => fl.has(i) ? p : -1).filter((/** @type {number} */ p) => p >= 0), label: lab });
      });
      chart($("#hr-kin"), series, { xlabel: "treatment time (min)" });
      if (window.TDATI) TDATI.series($("#hr-kin"), { title: "Kinetics of the product by session", xl: "treatment time (min)", yl: "peak area", series: series.map(x => ({ name: x.label === "?" ? "area" : "session " + x.label, x: x.x, y: x.y })) });
      $("#hr-kleg").innerHTML = series.map(x => `<span class="mut"><span style="display:inline-block;width:14px;border-top:2px solid ${x.color};vertical-align:middle"></span> ${x.label === "?" ? "area" : "session " + esc(x.label)}</span>`).join("") +
        ((d.row.filled || []).length ? `<span class="mut"><span class="rta-open"></span> filled: area re-integrated where no peak was found (${d.row.filled.map((/** @type {number} */ k) => t[k]).join(", ")} min)</span>` : "");
    }

    /** under the table of the candidates of the Identity tab: change, region, origin, prediction, score, similarity, localisation @param {any} r @param {any} d @param {HTMLElement} body */
    function identityExtras(r, d, body) {
      const loc = d.localization, w = s.weights, kin = d.kinetics || {};
      const comp = Object.keys(w).filter(k => typeof d.components[k] === "number" && d.components[k] !== 0).map(k => `<div class="comp" title="${esc((d.components.text || {})[k] || "")}"><span>${esc(k)} <span class="mut">×${w[k]}</span></span><span><i style="width:${Math.round(100 * d.components[k])}%"></i></span><span class="mut">${d.components[k].toFixed(2)}</span></div>`).join("");
      const box = document.createElement("div"); box.className = "hr-idx"; box.tabIndex = 0; box.setAttribute("role", "region"); box.setAttribute("aria-label", "More about the candidate");
      box.innerHTML = `<div class="kv"><b>Change</b><span>${(typeof TPTR !== "undefined") ? TPTR.change(r.derivation || "-") : esc(r.derivation || "-")} <span class="mut">(${(typeof TPTR !== "undefined") ? TPTR.formula(r.delta || "") : esc(r.delta || "")})</span></span>
        <b>RT</b><span>${r.rt} min${s.parent.rt ? ` (${(r.rt - s.parent.rt) >= 0 ? "+" : ""}${(r.rt - s.parent.rt).toFixed(2)} relative to the parent)` : ""}</span>
        <b>Kinetics</b><span>${esc(kin.class_text || "-")}${kin.tmax != null ? ` · maximum at ${kin.tmax} min` : ""}${kin.onset != null ? ` · appears at ${kin.onset} min` : ""}${kin.unimodal === false ? " · not unimodal" : ""}</span>
        <b>Region</b><span>${r.region ? esc(r.region) : "<span class='mut'>not localised</span>"}${loc && loc.ok ? ` <span class="mut">· margin ${loc.margin.toFixed(2)} · ${Math.round(100 * loc.fraction_explained)}% of intensity explained</span>` : (loc && loc.note ? ` <span class="mut">${esc(loc.note)}</span>` : "")}</span>
        <b>Origin</b><span>${d.predecessor && d.predecessor.predecessor != null ? `from ${d.predecessor.predecessor === "progenitore" ? "parent" : "#" + esc(d.predecessor.predecessor)}${d.predecessor.transformation ? ", " + esc(d.predecessor.transformation) : ""}${d.predecessor.cos != null ? `, cosine ${(+d.predecessor.cos).toFixed(2)}` : ""}` : "<span class='mut'>no precursor found</span>"}</span>
        ${d.prediction ? `<b>Prediction</b><span>${r.pred ? TPTAB.pred(r.pred) : ""} ${esc(d.prediction.text)}<ul class="crit" style="margin:2px 0 0;padding-left:18px">${d.prediction.evidence.map((/** @type {string} */ e) => `<li class="mut">${esc(e)}</li>`).join("")}</ul></span>` : ""}
        ${d.isotopes ? `<b>Isotopes</b><span>${esc(d.isotopes.text || d.isotopes.status)}</span>` : ""}${r.flags.length ? `<b>Flags</b><span class="flag">${r.flags.map(esc).join("; ")}</span>` : ""}</div>
        ${comp ? `<h4>Score ${r.score}</h4>${comp}` : ""}
        ${d.similarity ? `<h4>MS<sup>2</sup> against the parent</h4>${simCard(d.similarity)}` : d.ms2 ? "" : `<div class="mut">No DDA MS² for this ion: no localisation. A second injection with an inclusion list is needed.</div>`}
        ${loc && loc.ok ? `<h4>Localisation</h4><div class="row"><button id="hr-cmp-tree">Compare with parent tree</button></div><div class="mut">${loc.n_sites} candidate sites · type ${esc(loc.type)} · ${loc.n_shifted} S fragments, ${loc.n_unshifted} U</div>
          <table><tr><th>Site</th><th>log-likelihood</th></tr>${loc.top_sites.map((/** @type {any} */ t) => `<tr><td>${esc(t.site)}</td><td class="n">${t.ll.toFixed(2)}</td></tr>`).join("")}</table>
          <table style="margin-top:6px"><tr><th><i>m/z</i></th><th>%</th><th>fragment ion formula</th><th>evidence</th></tr>${loc.evidence.map((/** @type {any} */ e) => `<tr><td class="n">${e.mz.toFixed(4)}</td><td class="n">${e.rel.toFixed(0)}</td><td>${esc(e.formula)}</td><td class="${e.kind === "S" ? "fl-s" : "fl-u"}">${e.kind === "S" ? "S (shifted)" : "U (unshifted)"}</td></tr>`).join("")}</table>` : ""}
        ${d.iimn ? `<h4>Ion family (same peak)</h4><div class="mut">${esc(d.iimn.explained_text || d.iimn.role || "")}</div>` : ""}
        ${d.isomers && d.isomers.k > 1 ? `<h4>Co-eluting isomers: ${d.isomers.k} components</h4>${d.isomers.components.map((/** @type {any} */ c, /** @type {number} */ i) => `<div class="mut">component ${i + 1}: apex at RT ${c.rt_apex.toFixed(2)} min, ${c.mz.length} MS² peaks</div>`).join("")}` : ""}`;
      body.appendChild(box);
      if (window.TPPOS && d.ms2 && d.ms2.candidate && d.ms2.candidate.mz) window.TPPOS.mount(box, { smiles: (($("#tp-smi") || {}).value || "").trim(), delta: r.delta || "", peaks: d.ms2.candidate.mz.map((/** @type {number} */ v, /** @type {number} */ i) => [v, d.ms2.candidate.rel[i]]), positive: !window.TPPOS.negative((($("#tp-add") || {}).value) || ""), tol: 10, ppm: true, label: "Change" });
      const cb = $("#hr-cmp-tree");
      if (cb) cb.onclick = () => { window.TPALBERO && window.TPALBERO.compare(r.id); const el = $("#hr-m-tree"); if (el) { el.open = true; el.scrollIntoView({ behavior: "smooth", block: "nearest" }); } };
    }

    async function exportAll(kind) {
      const name = (s.parent.formula || "parent") + "_TPMine_HR";
      const keys = Object.keys(s.weights), times = s.times;
      const head = ["#", "Level (Schymanski)", "Diagnostic evidence", "Score", "m/z", "Neutral formula", "ppm", "Possible neutral formulas", "Change", "RT (min)", "t max (min)", "Class", "Appears at (min)", "Region", "Margin", "Precursor", "Transformation", "MS2 scans", "Modified cosine", "Matched peaks", "Prediction (Löffler)", "Cosine: equal fragments", "Cosine: shifted fragments", "Cosine: common neutral losses", "Flags", "Filled areas (t, min)", ...keys.map(k => "score " + k), ...times.map(t => "Area t" + t)];
      const line = (r, i) => [i + 1, r.level, r.diagnostic ? "yes" : "", r.score, r.mz, r.formula ?? "", r.ppm ?? "", r.n_formulas ?? "", r.derivation ?? r.delta ?? "", r.rt, r.tmax ?? "", r.class ?? "", r.onset ?? "", r.region ?? "", r.margin ?? "", r.predecessor ?? "", r.transformation ?? "", r.ms2_scans, r.modcos ?? "", r.n_matched, r.pred ?? "", ...(r.sim ? r.sim.cos : ["", "", ""]), r.flags.join("; "), r.filled.map(k => times[k]).join("; "), ...keys.map(k => r.components[k]), ...r.series];
      if (kind === "json") return dl(name + ".json", JSON.stringify(s, null, 1), "application/json");
      if (kind === "csv") return dl(name + ".csv", "sep=;\n" + [head, ...rows.map(line)].map(x => x.map(v => `"${String(v ?? "").replace(/"/g, '""')}"`).join(";")).join("\n"), "text/csv");
      toast("Preparing the Excel file...");
      const top = rows.slice(0, 50), det = []; for (const r of top) det.push(await detail(r.id));
      const sheets = [{ name: "Ranking", head, rows: rows.map(line), widths: [6, 8, 9, 10, 18, 7, 9, 28, 8, 9, 18, 9, 30, 8, 12, 24, 9, 9, 9, 9, 9, 9, 9, 40, 16, ...keys.map(() => 10), ...times.map(() => 11)] },
        { name: "Criteria", head: ["#", "Neutral formula", "Criterion", "Outcome", "Detail"], rows: det.flatMap((d, i) => d.criteria.map(c => [i + 1, d.row.formula ?? "", c.name, c.status, c.text])), widths: [6, 18, 32, 8, 80] },
        { name: "Localisation", head: ["#", "Neutral formula", "Region", "Fragment m/z", "% rel.", "Fragment ion formula", "Evidence"], rows: det.flatMap((d, i) => ((d.localization && d.localization.evidence) || []).map(e => [i + 1, d.row.formula ?? "", d.row.region ?? "", e.mz, e.rel, e.formula, e.kind])), widths: [6, 18, 30, 12, 8, 18, 8] },
        { name: "Mass differences", head: ["From", "To", "Rule", "Δm/z", "Rule scope", "Kinetics", "Shifted fragments", "Share of the cosine", "Kind"], rows: ((s.mass_network || {}).edges || []).map(e => [e.from, e.to, e.rule, e.dm, e.scope ?? "", e.kinetic, e.shifted ? e.shifted.n : "", e.shifted ? e.shifted.share : "", e.kind]), widths: [12, 12, 36, 10, 12, 12, 9, 9, 10] },
        { name: "MSn tree", head: ["Node", "Level", "Parent", "Ion formula", "Precursor m/z", "ppm", "Scans", "Ghost"], rows: (s.tree || []).map(t => [t.id, t.level, t.parent ?? "", t.formula ?? "", t.prec_mz ?? "", t.ppm ?? "", t.n_scans, t.ghost ? "yes" : ""]), widths: [8, 8, 8, 18, 12, 8, 10, 9] },
        { name: "Inclusion list", head: ["m/z", "RT from", "RT to", "Neutral formula", "Score"], rows: s.inclusion.map(x => [x.mz, x.rt_from, x.rt_to, x.formula ?? "", x.score]), widths: [12, 9, 9, 18, 10] },
        { name: "Information", head: ["Item", "Value"], rows: [["Parent", s.parent.formula], ["Ion", s.parent.ion], ["m/z", s.parent.mz], ["Score weights", keys.map(k => k + " " + s.weights[k]).join(", ")], ...s.funnel.map(f => ["Filter: " + f.text, f.n]), ...s.warnings.map(x => ["Warning", x])], widths: [44, 60] }];
      dlx(name + ".xlsx", sheets);
    }
    $("#hr-x-xlsx").onclick = () => exportAll("xlsx").catch(e => toast(String(e.message || e)));
    $("#hr-x-csv").onclick = () => exportAll("csv"); $("#hr-x-json").onclick = () => exportAll("json");
    $("#hr-x-incl").onclick = async () => dl((s.parent.formula || "parent") + "_inclusion.csv", await h.callRaw("inclusion_csv", "50"), "text/csv");
    massNet();
  }

  window.TPHR = { render, simCard };
})();
