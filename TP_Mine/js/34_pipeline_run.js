// @ts-strict
// TPMINE-PRIVATE  «Run» section of the panel Pipeline (window.PIPER): runs L1-L9 (feat-run.js, pool of PIPE) on every .mzidx in OPFS, shows the result with PIPEV (list of identities or, without the
// gate of quality, of rows), the detail of a row (proofs, cells per file, XIC and MS2 of the file chosen, from the routes api/xic and api/spectrum of the file open in Data) and sends the table to
// the window of the feature tables (UNT, model «mzlab»). Above the run: the roles and times of the files (campioni/2, TP_Mine/files/finder-ruoli.js; the Rust proposes and checks). Texts in English (mzFinder convention).
(() => {
  "use strict";
  const esc = s => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  /** @type {any} */
  const st = { ctx: null, data: null, list: [], view: null, files: [], gated: false, out: null, sel: null, res: {} };
  const $ = q => st.ctx.q(q);
  const PV = () => /** @type {any} */ (window).PIPEV;

  const html = () => `<details open><summary>Run on all the files</summary>
    <div class="par"><label>Parent neutral formula<input id="pipe-pf" type="text" placeholder="Neutral formula" autocomplete="off"><span class="why">hypotheses of transformation</span></label>
      <label>Parent SMILES<input id="pipe-psmi" type="text" placeholder="SMILES" autocomplete="off" spellcheck="false"><span class="why">the structure: fragments and groups of the rules</span></label>
      <label>Parent m/z<input id="pipe-pmz" type="number" step="any"></label><label>Parent RT (min)<input id="pipe-prt" type="number" step="any"></label>
</div>
    <div id="pipe-roles" class="roles"><style>#pipe-roles table{border-collapse:collapse;font-size:12px}#pipe-roles th,#pipe-roles td{padding:2px 6px;text-align:left}#pipe-roles tr.pr-prop input{background:color-mix(in srgb,var(--warn) 30%,transparent)}
      #pipe-roles input,#pipe-roles select{min-height:28px;min-width:28px}#pipe-roles input.pr-time{width:6em}#pr-warn{font-size:12px}#pr-warn .w{color:var(--warn)}#pr-warn .e{color:var(--bad)}</style>
      <div class="row"><strong>Roles and times</strong><label>Time unit <select id="pr-unit" aria-label="Unit of the times of the files">${["min", "s", "h", "d"].map(u => `<option>${u}</option>`).join("")}</select></label>
        <button type="button" id="pr-all" title="Accept the times proposed from the names (yellow)">Confirm times</button></div>
      <table id="pr-tab"><caption class="mut">Role and treatment time of each file; a time proposed from the name is yellow until you confirm it (Enter or «Confirm times»)</caption></table><div id="pr-warn" role="status"></div></div>
    <div class="row"><button type="button" id="pipe-run-go" class="imp">Run</button><button type="button" id="pipe-again" disabled title="Runs the last recipe of the session or of the opened .mzflow again and compares the table">Re-apply the recipe</button><span id="pipe-run-msg" class="mut" role="status"></span></div>
    <div id="pipe-res"></div></details>`;

  function mount(box, ctx) {
    st.ctx = ctx; box.innerHTML = html(); st.box = box;
    $("#pipe-run-go").onclick = () => run();
    $("#pipe-again").onclick = again; ctx.onRefresh(armAgain); armAgain();
    const sync = async () => { st.files = await ctx.listIdx(); await loadRoles(); };
    ctx.onRefresh(sync); sync();
    $("#pr-all").onclick = () => edit(() => st.R.confirmAll(st.cs));
    $("#pr-unit").onchange = () => edit(() => st.R.setUnit(st.cs, $("#pr-unit").value));
  }
  const msg = (t, err) => { const e = $("#pipe-run-msg"); if (e) { e.textContent = t || ""; e.className = err ? "err" : "mut"; } };

  /** grafo/1 JSON of a SMILES (the .wasm «grafo» of wasm-feat/manifest.json, verified against its SHA-256); "" without a SMILES */
  let grafo = null;
  async function structureOf(smiles) {
    if (!smiles) return "";
    if (window.MZLAB_DESKTOP) return (await st.ctx.pool()).analyse(smiles);      // the app: the native route feat/grafo
    if (!grafo) {
      const u = p => new URL("wasm-feat/" + p, window.QTOOLS.ctx.base).href, g = (await (await fetch(u("manifest.json"))).json()).grafo;
      if (!g) throw Object.assign(new Error("error.featFailed"), { key: "error.featFailed" });
      const bytes = await (await fetch(u(g.wasm))).arrayBuffer(), hex = b => [...new Uint8Array(b)].map(x => x.toString(16).padStart(2, "0")).join("");
      if (g.sha256 && hex(await crypto.subtle.digest("SHA-256", bytes)) !== g.sha256) throw Object.assign(new Error("error.featWasmHash"), { key: "error.featWasmHash" });
      const m = await import(u(g.js)); await m.default({ module_or_path: bytes }); grafo = m;
    }
    try { return grafo.analyse(smiles); }
    catch (e) { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } throw j && j.error_key ? Object.assign(new Error(j.error_key), { key: j.error_key, params: j.params }) : e; }
  }
  /** the parent of the proofs (null without the formula or the rules) */
  async function parentOf(files) {
    const f = $("#pipe-pf").value.trim(), rules = window.QTOOLS.ctx.files["regole_tp.json"];
    if (!st.gated || !f || !rules) return null;
    const num = id => { const v = parseFloat($(id).value); return Number.isFinite(v) ? v : NaN; };
    const smiles = $("#pipe-psmi").value.trim();
    return { rules: typeof rules === "string" ? new TextEncoder().encode(rules) : rules, parent: f, parentMz: num("#pipe-pmz"), parentRt: num("#pipe-prt"), minutes: files.map((_, i) => { const m = st.tp ? st.tp.minuti[i] : null; return m == null ? NaN : m; }),
      smiles, structure: await structureOf(smiles), campioni: st.cs ? (await st.R.canonical(st.call, st.cs)).json : "" };
  }
  async function run(op) {
    if (st.busy) return;
    const files = st.files.map(n => ({ name: n, path: "mzlab/idx/" + n }));
    if (!files.length) return msg("No .mzidx file: open an mzML in Data first.", true);
    if (!st.cs || st.block) return msg(st.block || "The roles and times are not ready.", true);
    st.busy = true; st.gated = st.ctx.gate().ok; $("#pipe-run-go").disabled = $("#pipe-again").disabled = true; $("#pipe-res").innerHTML = "";
    try {
      const pool = await st.ctx.pool(), url = URL.createObjectURL(new Blob([window.QTOOLS.ctx.files["feat-run.js"]], { type: "text/javascript" })), mod = await import(url);
      const recipe = op ? op.recipe : st.ctx.recipe(), parent = await parentOf(files);
      const out = await mod.runFeatPipeline(pool, { files, recipe, parent, onPhase: p => msg(p.state === "start" ? p.phase + "..." : "") });
      st.out = out; st.data = PV().readFeat(out.evid || out.table); st.names = files.map(f => f.name.replace(/\.mzidx$/, ""));
      msg(`${st.data.rows.toLocaleString("en")} rows in ${(out.totalMs / 1000).toFixed(1)} s`); show();
      await recordRun(recipe, parent, op);
    } catch (e) { msg(st.ctx.msg(e.key || "error.featFailed", e.params) + (e.key ? "" : " " + (e.message || "")), true); }
    st.busy = false; $("#pipe-run-go").disabled = !!st.block; armAgain();
  }

  // ------------------------------------------------------------ roles and times (campioni/2): the Rust proposes and checks, the page packs and shows
  const WARN = { noT0: "No t0: more false positives", noBlank: "No blank: background stays", onePoint: "One time point: no kinetics", timeUnconfirmed: "Times not confirmed", timeMissing: "Time missing",
    t0WithTime: "A t0 has a time other than 0", noControls: "No t0, blank or dark control", noTreated: "No treated sample" };
  const qaqcRoles = () => { try { return JSON.parse(localStorage.getItem(typeof NOMI !== "undefined" ? NOMI.KEY("qaqc") : "qqq.qaqc") || "{}").roles || null; } catch (_) { return null; } };
  const fail = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return Object.assign(new Error(j ? j.error_key : String(e && e.message || e)), j ? { key: j.error_key, params: j.params } : {}); };
  async function loadRoles() {
    try {
      if (!st.R) { const url = URL.createObjectURL(new Blob([window.QTOOLS.ctx.files["finder-ruoli.js"]], { type: "text/javascript" })); st.R = await import(url); }
      if (window.MZLAB_DESKTOP) { const pool = await st.ctx.pool(); st.call = (op, j) => pool.campioni(op, j); }     // the app: the native route finder/campioni
      else { const m = await /** @type {any} */ (window).PIPEQC.engine(window.QTOOLS.ctx.base); st.call = (op, j) => { try { return m.campioni(op, j); } catch (e) { throw fail(e); } }; }
      st.cs = await st.R.build(st.call, st.files, typeof NB !== "undefined" ? NB : null, qaqcRoles());
      await drawRoles();
    } catch (e) { st.cs = null; st.block = "The roles of the files could not be read."; await drawRoles(); msg(st.ctx.msg(e.key || "error.featFailed", e.params), true); }
  }
  /** one change of the model: apply it, keep it in the notebook, draw again with the focus where it was */
  function edit(change, focusId) {
    if (!st.cs) return;
    change(); if (typeof NB !== "undefined") { st.R.store(NB, st.cs); if (typeof nbSave === "function") nbSave(); }
    drawRoles(focusId);
  }
  async function drawRoles(focusId) {
    const tab = $("#pr-tab"), warn = $("#pr-warn"), go = $("#pipe-run-go"); if (!tab) return;
    const cs = st.cs || [], R = st.R;
    tab.innerHTML = tab.querySelector("caption").outerHTML + (cs.length ? `<tr><th scope="col">File</th><th scope="col">Role</th><th scope="col">Time</th></tr>` : "")
      + cs.map((c, i) => { const timed = ["sample", "t0", "dark"].includes(c.tipo), prop = timed && c.tempo != null && !c.tempo_confermato, n = esc(st.files[i]);
        return `<tr${prop ? ' class="pr-prop"' : ""}><td>${n}</td><td><select class="pr-role" data-i="${i}" id="pr-r${i}" aria-label="Role of ${n}">${R.ROLES.map(r => `<option${r === c.tipo ? " selected" : ""}>${r}</option>`).join("")}</select></td>`
          + `<td><input class="pr-time" data-i="${i}" id="pr-t${i}" type="text" inputmode="decimal" autocomplete="off" aria-label="Time of ${n}"${timed ? "" : " disabled"} value="${c.tempo == null ? "" : c.tempo}"${prop ? ' title="Proposed from the name: Enter confirms"' : ""}></td></tr>`; }).join("");
    const u = cs.find(c => c.tempo != null); if (u) $("#pr-unit").value = u.unita_tempo;
    tab.querySelectorAll(".pr-role").forEach(e => { e.onchange = () => edit(() => R.setRole(st.cs, +e.dataset.i, e.value), e.id); });
    tab.querySelectorAll(".pr-time").forEach(e => { e.onkeydown = ev => { if (ev.key === "Enter") { ev.preventDefault(); edit(() => R.setTime(st.cs, +e.dataset.i, e.value.trim() === "" ? null : parseFloat(e.value.replace(",", ".")), $("#pr-unit").value), e.id); } }; });
    let list = [], block = "";
    if (st.cs) {
      try {
        const v = await R.check(st.call, cs); st.tp = await R.forTp(st.call, cs);
        const by = new Map(); v.avvisi.filter(a => WARN[a.codice]).forEach(a => { const k = a.codice; by.set(k, { sev: a.gravita, n: (by.get(k) || { n: 0 }).n + 1 }); });
        list = [...by].map(([k, o]) => `<span class="${o.sev === "error" ? "e" : "w"}">${esc(WARN[k])}${o.n > 1 ? " (" + o.n + ")" : ""}</span>`);
        if (v.bloccante) block = [...by].filter(([, o]) => o.sev === "error").map(([k]) => WARN[k]).join("; ");
      } catch (e) { block = st.ctx.msg(e.key || "error.featFailed", e.params); }
    } else block = st.block || "The roles of the files are not ready.";
    st.block = block; warn.innerHTML = list.join(" · "); if (go) { go.disabled = !!st.busy || !!block; go.title = block ? "Run is off: " + block : ""; }
    if (focusId) { const f = $("#" + focusId); if (f) f.focus(); }
  }

  // ------------------------------------------------------------ recipe: the record goes to the log of operations (.mzflow), «Re-apply» runs the last one again
  const levelsText = async () => { const s = document.querySelector('script[src*="livelli-1.js"]'); try { return s ? await (await fetch(s.src)).text() : null; } catch (_) { return null; } };
  async function recordRun(recipe, parent, op) {
    const rec = await /** @type {any} */ (window).PIPEREC.record({ recipe, proofs: st.ctx.proofs(), crate: st.ctx.crate(), out: st.out, data: st.data, rules: window.QTOOLS.ctx.files["regole_tp.json"] || null, levels: await levelsText(),
      parent: parent ? { formula: parent.parent, mz: parent.parentMz, rt: parent.parentRt, minuti: parent.minutes, smiles: parent.smiles } : null });
    if (op) { const c = /** @type {any} */ (window).PIPEREC.compare(op, rec); msg(c.same ? `Re-applied: the .mzfeat is identical to the recorded one (SHA-256 ${rec.tabella_sha256.slice(0, 12)}...).` : "Re-applied, but " + c.lines.join("; ") + ".", !c.same); return; }
    if (window.WORKFLOW) { const { op: _o, t: _t, ...rest } = rec; window.WORKFLOW.logOp("feature_pipeline", rest); if (st.cs) window.WORKFLOW.logOp("campioni", /** @type {any} */ (window).PIPEREC.campioniOp(await st.R.canonical(st.call, st.cs))); }
    st.last = rec; armAgain();
  }
  const lastOp = () => (typeof NB !== "undefined" && window.WORKFLOW ? /** @type {any} */ (window).PIPEREC.lastOp(NB.ops, window.WORKFLOW.checkFeatureOp) : null);
  function armAgain() { const b = st.box && st.box.querySelector("#pipe-again"); if (b) b.disabled = !!st.busy || !lastOp(); }
  function again() {
    const op = lastOp(); if (!op) return;
    st.ctx.apply(op.parametri);
    const g = op.genitore; $("#pipe-pf").value = g ? g.formula : ""; $("#pipe-pmz").value = g && g.mz != null ? g.mz : ""; $("#pipe-prt").value = g && g.rt != null ? g.rt : ""; $("#pipe-psmi").value = g && g.smiles ? g.smiles : "";
    if (g && st.cs) edit(() => g.minuti.forEach((m, i) => { if (m != null && st.cs[i]) { st.R.setRole(st.cs, i, m === 0 ? "t0" : "sample"); st.R.setTime(st.cs, i, m, "min"); } })); run(op);
  }

  // ------------------------------------------------------------ result: toolbar, list, detail
  function show() {
    const d = st.data, cand = st.gated && d.evid, all = cand ? PV().identities(d) : PV().plainRows(d);
    st.all = all; st.cand = cand;
    $("#pipe-res").innerHTML = `<div class="pv-bar"><label>Filter<input type="search" id="pv-q" aria-label="Filter by ID, m/z or formula"></label>`
      + (cand ? `<label><input type="checkbox" id="pv-dis"> Discarded (<span id="pv-nd">${all.filter(r => r.discarded).length}</span>)</label>` : "")
      + `<button type="button" id="pv-unt">Send to the feature tables</button><span class="mut" id="pv-n"></span></div>`
      + `<div id="pv-list" class="pv-wrap"></div><div id="pv-det" class="pv-det" hidden></div><div id="pipe-qc"></div><div id="pipe-tab"></div><div id="pipe-exp"></div>`;
    st.view = PV().mount($("#pv-list"), { gated: cand, onPick: detail });
    $("#pv-q").oninput = refilter; if (cand) $("#pv-dis").onchange = refilter;
    $("#pv-unt").onclick = toUnt; refilter(); tableSteps();
  }
  /** steps 5 (quality control) and 6 (table): the numbers come from the table .wasm (PIPEQC, PIPET) */
  async function tableSteps() {
    const gate = st.ctx.gate(), chart = st.ctx.chart, say = e => st.ctx.msg(e.key || "error.featFailed", e.params) + (e.key ? "" : " " + (e.message || ""));
    try {
      const out = st.out, session = await /** @type {any} */ (window).PIPEQC.open({ base: window.QTOOLS.ctx.base, bytes: out.evid || out.table, names: st.names, inputs: st.data.meta.inputs || [], campioni: st.cs, pol: st.ctx.values().polarity, pool: st.ctx.pool });
      st.session = session; const g = { ...gate, table: gate.table === true };
      await /** @type {any} */ (window).PIPEQC.render($("#pipe-qc"), { session, gate: g, chart });
      const tab = /** @type {any} */ (window).PIPET.mount($("#pipe-tab"), { session, names: st.names, gate: g, say, cs: st.cs, ppm: () => st.ctx.values().ppm, pol: () => st.ctx.values().polarity, formula: ($("#pipe-pf").value || "").trim(), levels: levelMap(), identsOf: id => (st.cand ? st.all.filter(x => x.row === id - 1) : []) });
      /** @type {any} */ (window).PIPEX.mount($("#pipe-exp"), { session, spec: () => (tab ? tab.spec() : ""), names: st.names, pool: st.ctx.pool, pol: st.ctx.values().polarity, say, stem: "tabui_" + session.sha.slice(0, 8) });
    } catch (e) { $("#pipe-qc").innerHTML = `<p class="err">${esc(say(e))}</p>`; }
  }
  /** level of the best identity of each row of the table (row id = position + 1), none without the gate */
  function levelMap() {
    const m = new Map(); if (!st.cand) return m;
    for (const x of st.all) if (x.row >= 0 && x.level && !m.has(x.row + 1)) m.set(x.row + 1, x.level);
    return m;
  }
  function refilter() {
    const opt = { text: $("#pv-q").value, discarded: !!(st.cand && $("#pv-dis").checked) };
    st.list = st.cand ? PV().filter(st.all, opt) : st.all.filter(r => !opt.text || String(r.id).includes(opt.text) || r.mz.toFixed(4).includes(opt.text));
    st.view.set(st.list); $("#pv-n").textContent = `${st.list.length.toLocaleString("en")} shown`;
  }
  async function toUnt() {
    const t = PV().toUnt(st.data, st.names, st.ctx.values().polarity);
    try { t.file.sha256 = await /** @type {any} */ (window).UNT.sha256Hex(st.out.evid || st.out.table); } catch (_) { t.file.sha256 = ""; }
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
    const box = $("#pv-det"); st.sel = r; box.hidden = false; const cand = st.cand && !r.plain;
    const best = r.cells.reduce((b, c, j) => (c.state <= 1 && c.area > (r.cells[b] ? r.cells[b].area : -1) ? j : b), 0);
    box.innerHTML = `<h4>${cand ? "ID" + r.id + (r.formula ? " " + esc(r.formula) : "") : "Row " + r.id} · m/z ${r.mz.toFixed(4)} · RT ${r.rt.toFixed(2)} min</h4>`
      + (cand ? `<p class="mut">Hypothesis with its proofs, not an identification. ${r.reasons.map(c => esc(PV().REASON[c] || c)).join("; ")}.</p>${proofsTable(r)}` : "") + cellsTable(r)
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

  window.PIPER = { mount, state: st, lineSvg, sticksSvg, detail };
})();
