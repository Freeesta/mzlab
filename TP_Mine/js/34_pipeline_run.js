// TPMINE-PRIVATE  «Run» section of the panel Pipeline (window.PIPER): runs L1-L9 (feat-run.js, pool of PIPE) on every .mzidx in OPFS, shows the result with PIPEV (list of identities or, without the
// gate of quality, of rows), the detail of a row (proofs, cells per file, XIC and MS2 of the file chosen, from the routes api/xic and api/spectrum of the file open in Data) and sends the table to
// the window of the feature tables (UNT, model «mzlab»). Texts in English (mzFinder convention).
(() => {
  "use strict";
  const esc = s => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const st = { ctx: null, data: null, list: [], view: null, files: [], gated: false, out: null, sel: null, res: {} };
  const $ = q => st.ctx.q(q);
  const PV = () => window.PIPEV;

  const html = () => `<details open><summary>Run on all the files</summary>
    <div class="par"><label>Parent formula<input id="pipe-pf" type="text" placeholder="e.g. C9H11NO2" autocomplete="off"><span class="why">only with the gate open: hypotheses of transformation</span></label>
      <label>Parent m/z<input id="pipe-pmz" type="number" step="any"></label><label>Parent RT (min)<input id="pipe-prt" type="number" step="any"></label>
      <label>Treatment time of each file (min, empty = control)<input id="pipe-times" type="text" autocomplete="off"></label></div>
    <div class="row"><button type="button" id="pipe-run-go" class="imp">Run</button><span id="pipe-run-msg" class="mut" role="status"></span></div>
    <div id="pipe-res"></div></details>`;

  function mount(box, ctx) {
    st.ctx = ctx; box.innerHTML = html(); st.box = box;
    $("#pipe-run-go").onclick = run;
    const sync = async () => { st.files = await ctx.listIdx(); $("#pipe-times").value = st.files.map(n => { const m = /_t(\d+)(?:min)?\.mzidx$/.exec(n); return m ? String(+m[1]) : ""; }).join(","); };
    ctx.onRefresh(sync); sync();
  }
  const msg = (t, err) => { const e = $("#pipe-run-msg"); if (e) { e.textContent = t || ""; e.className = err ? "err" : "mut"; } };

  /** the parent of the proofs (null without the gate, the formula or the rules) */
  async function parentOf(files) {
    const f = $("#pipe-pf").value.trim(), rules = window.QTOOLS.ctx.files["regole_tp.json"];
    if (!st.gated || !f || !rules) return null;
    const num = id => { const v = parseFloat($(id).value); return Number.isFinite(v) ? v : NaN; }, t = $("#pipe-times").value.split(",").map(s => s.trim());
    return { rules: typeof rules === "string" ? new TextEncoder().encode(rules) : rules, parent: f, parentMz: num("#pipe-pmz"), parentRt: num("#pipe-prt"), minutes: files.map((_, i) => (t[i] === "" || t[i] == null ? NaN : +t[i])) };
  }
  async function run() {
    if (st.busy) return;
    const files = st.files.map(n => ({ name: n, path: "mzlab/idx/" + n }));
    if (!files.length) return msg("No .mzidx file: open an mzML in Data first.", true);
    st.busy = true; st.gated = st.ctx.gate().ok; $("#pipe-run-go").disabled = true; $("#pipe-res").innerHTML = "";
    try {
      const pool = await st.ctx.pool(), url = URL.createObjectURL(new Blob([window.QTOOLS.ctx.files["feat-run.js"]], { type: "text/javascript" })), mod = await import(url);
      const out = await mod.runFeatPipeline(pool, { files, recipe: st.ctx.recipe(), parent: await parentOf(files), onPhase: p => msg(p.state === "start" ? p.phase + "..." : "") });
      st.out = out; st.data = PV().readFeat(out.evid || out.table); st.names = files.map(f => f.name.replace(/\.mzidx$/, ""));
      msg(`${st.data.rows.toLocaleString("en")} rows in ${(out.totalMs / 1000).toFixed(1)} s`); show();
    } catch (e) { msg(st.ctx.msg(e.key || "error.featFailed", e.params) + (e.key ? "" : " " + (e.message || "")), true); }
    st.busy = false; $("#pipe-run-go").disabled = false;
  }

  // ------------------------------------------------------------ result: toolbar, list, detail
  function show() {
    const d = st.data, cand = st.gated && d.evid, all = cand ? PV().identities(d) : PV().plainRows(d);
    st.all = all; st.cand = cand;
    $("#pipe-res").innerHTML = `<div class="pv-bar"><label>Filter<input type="search" id="pv-q" aria-label="Filter by ID, m/z or formula"></label>`
      + (cand ? `<label><input type="checkbox" id="pv-dis"> Discarded (<span id="pv-nd">${all.filter(r => r.discarded).length}</span>)</label>` : "")
      + `<button type="button" id="pv-unt">Send to the feature tables</button><span class="mut" id="pv-n"></span></div>`
      + (cand ? "" : `<div class="mut">Pipeline not validated: only traces and families are shown, no candidates.</div>`)
      + `<div id="pv-list" class="pv-wrap"></div><div id="pv-det" class="pv-det" hidden></div>`;
    st.view = PV().mount($("#pv-list"), { gated: cand, onPick: detail });
    $("#pv-q").oninput = refilter; if (cand) $("#pv-dis").onchange = refilter;
    $("#pv-unt").onclick = toUnt; refilter();
  }
  function refilter() {
    const opt = { text: $("#pv-q").value, discarded: !!(st.cand && $("#pv-dis").checked) };
    st.list = st.cand ? PV().filter(st.all, opt) : st.all.filter(r => !opt.text || String(r.id).includes(opt.text) || r.mz.toFixed(4).includes(opt.text));
    st.view.set(st.list); $("#pv-n").textContent = `${st.list.length.toLocaleString("en")} shown`;
  }
  async function toUnt() {
    const t = PV().toUnt(st.data, st.names, st.ctx.values().polarity);
    try { t.file.sha256 = await window.UNT.sha256Hex(st.out.evid || st.out.table); } catch (_) { t.file.sha256 = ""; }
    if (window.UNTUI && window.UNTUI.adopt) { window.UNTUI.adopt(t); msg(`${t.features.length.toLocaleString("en")} features sent to the feature tables.`); }
  }

  const f4 = x => (x == null ? "" : Number(x).toPrecision(4));
  function proofsTable(r) {
    return `<table><caption class="mut">Contributions: one row per proof; a proof against is worth 0 points and no other proof offsets it</caption><tr><th>Proof</th><th>Value</th><th>Threshold</th><th>Points</th><th>Sign</th><th>Source</th></tr>`
      + r.proofs.map(p => `<tr${p.segno === "contro" ? ' class="against"' : ""}><td>${esc(p.tipo)}</td><td>${f4(p.valore)}</td><td>${f4(p.soglia)}</td><td>${p.punti}</td><td>${p.segno === "contro" ? "&#10005; against" : esc(p.segno)}</td><td>${esc(p.fonte)}</td></tr>`).join("") + `</table>`;
  }
  function cellsTable(r) {
    return `<table><caption class="mut">States of the cells per file</caption><tr><th>File</th><th>State</th><th>Area</th></tr>`
      + r.cells.map((c, j) => `<tr><td>${esc(st.names[j])}</td><td>${PV().STATE[c.state]} ${PV().STATE_NAME[c.state]}</td><td>${c.area > 0 ? Number(c.area).toExponential(2) : ""}</td></tr>`).join("") + `</table>`;
  }
  function detail(r) {
    const box = $("#pv-det"); st.sel = r; box.hidden = false;
    const best = r.cells.reduce((b, c, j) => (c.state <= 1 && c.area > (r.cells[b] ? r.cells[b].area : -1) ? j : b), 0);
    box.innerHTML = `<h4>${st.cand ? "ID" + r.id + (r.formula ? " " + esc(r.formula) : "") : "Row " + r.id} · m/z ${r.mz.toFixed(4)} · RT ${r.rt.toFixed(2)} min</h4>`
      + (st.cand ? `<p class="mut">Hypothesis with its proofs, not an identification. ${r.reasons.map(c => esc(PV().REASON[c] || c)).join("; ")}.</p>${proofsTable(r)}` : "") + cellsTable(r)
      + `<div class="row"><label>File <select id="pv-file">${st.names.map((n, j) => `<option value="${j}"${j === best ? " selected" : ""}>${esc(n)}</option>`).join("")}</select></label></div><div class="charts" id="pv-ch"><div class="ch a"></div><div class="ch b"></div></div>`;
    $("#pv-file").onchange = () => traces(r); traces(r);
  }

  // ------------------------------------------------------------ XIC and MS2 of the file open in Data (existing routes)
  const getJson = async u => { const r = await fetch(u); if (!r.ok) throw new Error(String(r.status)); return r.json(); };
  async function sessionK(name) {
    const s = await getJson("api/state"), base = n => String(n || "").replace(/\.(mzml|mzidx|raw|d)$/i, "").toLowerCase();
    const f = (s.files || []).find(x => base(x.name) === base(name)); return f ? f.k : null;
  }
  function lineSvg(x, y, apex) {
    const W = 320, H = 150, M = { l: 34, r: 8, t: 8, b: 22 }, x0 = x[0], x1 = x[x.length - 1] || x0 + 1, ym = Math.max(1, ...y);
    const X = t => M.l + (t - x0) / (x1 - x0 || 1) * (W - M.l - M.r), Y = v => H - M.b - v / ym * (H - M.t - M.b);
    return `<svg viewBox="0 0 ${W} ${H}"><path class="ax" d="M${M.l},${M.t}V${H - M.b}H${W - M.r}"/><path class="ln" d="${x.map((t, i) => (i ? "L" : "M") + X(t).toFixed(1) + "," + Y(y[i]).toFixed(1)).join("")}"/>`
      + (apex != null ? `<line class="mk" x1="${X(apex).toFixed(1)}" x2="${X(apex).toFixed(1)}" y1="${M.t}" y2="${H - M.b}"/>` : "") + `<text x="${M.l}" y="${H - 8}">${x0.toFixed(2)}</text><text x="${W - M.r}" y="${H - 8}" text-anchor="end">${x1.toFixed(2)}</text></svg>`;
  }
  function sticksSvg(mz, y) {
    const W = 320, H = 150, M = { l: 34, r: 8, t: 8, b: 22 }, x0 = Math.min(...mz), x1 = Math.max(...mz), ym = Math.max(1, ...y), X = v => M.l + (v - x0) / (x1 - x0 || 1) * (W - M.l - M.r);
    return `<svg viewBox="0 0 ${W} ${H}"><path class="ax" d="M${M.l},${M.t}V${H - M.b}H${W - M.r}"/>` + mz.map((v, i) => `<line class="ln" x1="${X(v).toFixed(1)}" x2="${X(v).toFixed(1)}" y1="${H - M.b}" y2="${(H - M.b - y[i] / ym * (H - M.t - M.b)).toFixed(1)}"/>`).join("")
      + `<text x="${M.l}" y="${H - 8}">${x0.toFixed(1)}</text><text x="${W - M.r}" y="${H - 8}" text-anchor="end">${x1.toFixed(1)}</text></svg>`;
  }
  async function traces(r) {
    const j = +$("#pv-file").value, a = $("#pv-ch .a"), b = $("#pv-ch .b"), name = st.names[j], ppm = st.ctx.values().ppm;
    a.innerHTML = b.innerHTML = `<span class="mut">Reading ${esc(name)}...</span>`;
    try {
      const k = await sessionK(name);
      if (k == null) { a.innerHTML = b.innerHTML = ""; a.innerHTML = `<span class="mut">Open ${esc(name)} in Data to see its XIC and MS2.</span>`; return; }
      const x = await getJson(`api/xic?k=${k}&mz=${r.mz}&tol=${(r.mz * ppm * 1e-6).toFixed(5)}&level=1`), t = x.traces[0];
      st.ctx.chart(a, lineSvg(t.rt, t.y, r.rt), `XIC m/z ${r.mz.toFixed(4)} in ${name}`, `Ion trace of m/z ${r.mz.toFixed(4)} (±${ppm} ppm) in ${name}; the dashed line is the RT of the row, ${r.rt.toFixed(2)} min.`,
        { head: ["RT (min)", "Intensity"], rows: t.rt.map((v, i) => [v.toFixed(3), String(Math.round(t.y[i]))]).slice(0, 400) });
      const w = 0.15, s = await getJson(`api/spectrum?k=${k}&rt0=${(r.rt - w).toFixed(3)}&rt1=${(r.rt + w).toFixed(3)}&level=2&precursor=${r.mz}&bin=0.01`);
      if (!s.mz || !s.mz.length) { b.innerHTML = `<span class="mut">No MS2 of m/z ${r.mz.toFixed(4)} in ${esc(name)} around ${r.rt.toFixed(2)} min.</span>`; return; }
      st.ctx.chart(b, sticksSvg(s.mz, s.y), `MS2 of m/z ${r.mz.toFixed(4)} in ${name}`, `Fragmentation spectrum of the precursor m/z ${r.mz.toFixed(4)} in ${name}: ${s.mz.length} peaks.`,
        { head: ["m/z", "Intensity"], rows: s.mz.map((v, i) => [v.toFixed(4), String(Math.round(s.y[i]))]).slice(0, 400) });
    } catch (e) { b.innerHTML = `<span class="err">${esc(e.message || e)}</span>`; }
  }

  window.PIPER = { mount, state: st, lineSvg, sticksSvg };
})();
