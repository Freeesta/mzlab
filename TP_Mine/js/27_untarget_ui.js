// @ts-strict
// TPMINE-PRIVATE  Untargeted window of mzFinder: load the files of a feature table (mzMine, MS-DIAL, GNPS2), match every feature with an MS2 against the libraries, table with level,
// best candidate, entropy, Δppm and evidence as TPTAB tables (42_tp_table.js): chips of level, search, sort from the headers, "Columns", "Export view"; a click opens the row under itself (tabs Overview with the mirror
// plot, Samples, Identity with «What is it?», the external evidence and the chemist's decided level; «group by identity»: Identity with the network and the evidence of the links, Features); export (28_untarget_export.js).
(() => {
  "use strict";
  const esc = s => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const CSS = `#unt{width:min(1240px,97vw);max-width:97vw;max-height:94vh;padding:12px 14px;border:1px solid var(--line);border-radius:8px;background:var(--panel);color:var(--ink)}
#unt::backdrop{background:rgba(0,0,0,.35)}#unt h3{margin:0}#unt .top{display:flex;align-items:center;gap:8px}#unt .top h3{flex:1}
#unt .row{display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin:6px 0}#unt .mut{color:var(--muted);font-size:12px}#unt .drop{border:2px dashed var(--line);border-radius:8px;padding:10px;text-align:center}
#unt .drop.over{border-color:var(--accent);background:var(--sel)}#unt .err{color:var(--bad)}#unt .warn{color:var(--warn);font-size:12px}
#unt progress{width:100%}#unt .det{display:grid;grid-template-columns:1fr 1fr;gap:12px;margin-top:8px}@media(max-width:900px){#unt .det{grid-template-columns:1fr}}
#unt{--idn-adduct:#0072B2;--idn-isotope:#D55E00;--idn-loss:#009E73;--idn-dimer:#CC79A7;--idn-iin:#E69F00}
#unt .idn-e line{stroke:var(--idn-adduct);stroke-width:1.5}#unt .idn-isotope line{stroke:var(--idn-isotope);stroke-dasharray:5 3}#unt .idn-loss line{stroke:var(--idn-loss);stroke-dasharray:2 3}#unt .idn-dimer line{stroke:var(--idn-dimer);stroke-dasharray:8 2 2 2}
#unt .idn-iin line{stroke:var(--idn-iin);stroke-width:3}#unt .idn-e text,#unt .idn-n text{fill:var(--ink);font-size:10px}#unt .idn-n circle{fill:var(--panel);stroke:var(--accent);stroke-width:2}#unt .idn-main circle{fill:var(--accent)}
#unt canvas{width:100%;height:240px;display:block;border:1px solid var(--line);border-radius:6px}#unt ul{margin:2px 0;padding-left:18px}#unt .tpt-dec-mark{margin-left:3px}`;

  /** @type {any} */
  const st = { table: null, res: null, files: [], sel: null, busy: false, cancel: false, tol: 10, order: null, grp: false, graph: null, cyc: 1, gorder: null, TF: null, TG: null, mounted: null };
  const U = () => /** @type {any} */ (window).UNT, M = () => /** @type {any} */ (window).UNTM, L = () => window.LIVELLI, ID = () => /** @type {any} */ (window).UNTID;
  let dlg = null;
  const $ = q => dlg.querySelector(q);
  const say = t => { const e = $("#unt-msg"); if (e) e.textContent = t || ""; };
  const fmtA = v => (v >= 1e6 ? (v / 1e6).toFixed(1) + "M" : v >= 1e3 ? (v / 1e3).toFixed(v >= 1e4 ? 0 : 1) + "k" : String(Math.round(v)));
  const maxArea = f => Math.max(0, ...Object.values(f.areas));
  const TPTAB = () => /** @type {any} */ (window).TPTAB, TPRIGA = () => /** @type {any} */ (window).TPRIGA;
  /** the badge of a level: TPTAB.level, with the double border and «✎» when the chemist decided it */
  const lvBadge = (lv, decided) => `${TPTAB().level(lv, { decided: decided || null })}${decided ? `<span class="tpt-dec-mark" aria-hidden="true">✎</span>` : ""}`;
  const stateOf = (f, s) => !(f.areas[s] > 0) ? "absent" : Array.isArray(f.detected) && !f.detected.includes(s) ? "filled" : "detected";

  function open() {
    if (!document.getElementById("unt-css")) { const s = document.createElement("style"); s.id = "unt-css"; s.textContent = CSS; document.head.appendChild(s); }
    if (!dlg) {
      dlg = document.createElement("dialog"); dlg.id = "unt"; dlg.className = "tp"; dlg.setAttribute("aria-label", "Feature tables"); document.body.appendChild(dlg);
      dlg.innerHTML = `<div class="top"><h3>Feature tables</h3><button type="button" id="unt-x" aria-label="Close">&times;</button></div>
        <p class="mut" style="margin:6px 0">Load the files of one table (an mzMine table or FBMN quant table, its MGF with the MS<sup>2</sup>, the annotations, the GNPS2 library file, or an MS-DIAL alignment). Everything stays in this browser.
        Each feature with an MS<sup>2</sup> is searched in the loaded libraries: the result is a candidate with a level and its evidence, never an identification.</p>
        <div class="drop" id="unt-drop">Drop the files here <button type="button" id="unt-pick">Choose files...</button><input type="file" id="unt-in" multiple hidden accept=".csv,.tsv,.txt,.mgf"></div>
        <div class="row"><label>Polarity <select id="unt-pol"><option value="auto">auto</option><option value="1">positive</option><option value="-1">negative</option></select></label>
          <label>Precursor tolerance <input type="number" id="unt-tol" value="10" min="1" step="1" style="width:64px"> ppm</label>
          <button type="button" id="unt-libs">Libraries...</button><button type="button" id="unt-pipe">Pipeline...</button><button type="button" id="unt-go" class="imp" disabled>Match</button><button type="button" id="unt-stop" hidden>Stop</button></div>
        <div id="unt-sum" class="mut"></div><progress id="unt-prog" max="1" value="0" hidden aria-label="Progress"></progress><div id="unt-msg" role="status" class="mut"></div>
        <div id="unt-res" hidden><div class="row"><label><input type="checkbox" id="unt-grp"> group by identity</label><label id="unt-cyc-l" hidden>Cycle time <input type="number" id="unt-cyc" value="1" min="0.1" step="0.1" style="width:56px"> s</label><span style="flex:1"></span><span id="unt-exp"></span></div>
          <div class="tp"><div id="unt-tf"></div><div id="unt-tg" hidden></div></div></div>`;
      dlg.querySelector("#unt-x").onclick = () => dlg.close();
      const input = $("#unt-in"), drop = $("#unt-drop");
      $("#unt-pick").onclick = () => input.click();
      input.onchange = () => { load([...input.files]); input.value = ""; };
      drop.ondragover = e => { e.preventDefault(); drop.classList.add("over"); }; drop.ondragleave = () => drop.classList.remove("over");
      drop.ondrop = e => { e.preventDefault(); drop.classList.remove("over"); load([...e.dataTransfer.files]); };
      $("#unt-libs").onclick = () => { if (window.LIB) window.LIB.open(); };
      $("#unt-pipe").onclick = () => { if (window.PIPE) window.PIPE.open(); };
      $("#unt-go").onclick = match; $("#unt-stop").onclick = () => { st.cancel = true; };
      $("#unt-pol").onchange = () => { if (st.files.length) build(); };
      $("#unt-grp").onchange = e => { st.grp = e.target.checked; st.sel = null; group(); };
      $("#unt-cyc").onchange = e => { st.cyc = Math.max(0.1, +e.target.value || 1); st.graph = null; if (st.grp) group(); };
    }
    if (!dlg.open) dlg.showModal();
  }

  // ------------------------------------------------------------ files → table
  async function load(files) {
    if (!files.length) return;
    say("Reading the files...");
    try {
      for (const f of files) { const k = st.files.findIndex(x => x.name === f.name); const rec = { name: f.name, file: f, text: await f.text() }; if (k >= 0) st.files[k] = rec; else st.files.push(rec); }
      await build();
    } catch (e) { say(""); $("#unt-sum").innerHTML = `<span class="err">${esc(e.message || e)}</span>`; }
  }
  async function build() {
    const pv = $("#unt-pol").value, guess = st.files.map(f => f.name).join(" "), pol = pv !== "auto" ? +pv : /(^|[^a-z])neg/i.test(guess) && !/(^|[^a-z])pos/i.test(guess) ? -1 : /(^|[^a-z])pos/i.test(guess) && !/(^|[^a-z])neg/i.test(guess) ? 1 : 0;
    try {
      await U().ready();
      const t = await U().build(st.files.map(f => ({ name: f.name, text: f.text })), { pol });
      const main = st.files.find(f => f.name === t.used[0]);
      if (main) { try { t.file.sha256 = await U().sha256Hex(await main.file.arrayBuffer()); } catch (e) { t.file.sha256 = ""; } }
      st.table = t; st.res = null; st.sel = null; st.graph = null; st.mounted = null;
      const s = t.stats, nMs2 = t.features.filter(f => f.ms2).length, nAnn = t.features.filter(f => f.ann.length).length;
      $("#unt-sum").innerHTML = `<b>${esc(t.used[0])}</b>: ${t.features.length.toLocaleString("en")} features · ${t.samples.length} samples · ${nMs2.toLocaleString("en")} with MS<sup>2</sup> · ${nAnn.toLocaleString("en")} with an external annotation · polarity ${t.pol > 0 ? "positive" : t.pol < 0 ? "negative" : "?"}`
        + (t.used.length > 1 ? `<br>Also read: ${esc(t.used.slice(1).join(", "))}` : "") + (t.ignored.length ? `<br><span class="warn">Not recognised: ${esc(t.ignored.join(", "))}</span>` : "")
        + (s.lipidRowsDropped ? `<br>${s.lipidRowsDropped} lipid rows (matchedlipid) discarded` : "") + (s.unreliableStructures ? `<br>${s.unreliableStructures} library structures discarded (the formula does not explain the precursor)` : "")
        + (s.gnpsRows ? `<br>GNPS2: ${s.gnpsRows} rows, ${s.gnpsAnalogs} analogues, ${s.gnpsPropagated} propagated names discarded` : "") + t.warnings.map(w => `<br><span class="warn">${esc(w)}</span>`).join("");
      $("#unt-go").disabled = !nMs2 && !t.features.length; $("#unt-res").hidden = true; say("");
    } catch (e) { st.table = null; $("#unt-go").disabled = true; say(""); $("#unt-sum").innerHTML = `<span class="err">${esc(e.message || e)}</span>`; }
  }

  /** a table made elsewhere (the pipeline of mzFinder): it replaces the table read from files and is ready to match */
  function adopt(t) {
    open();
    st.table = t; st.res = null; st.sel = null; st.graph = null; st.mounted = null; st.files = [];
    $("#unt-sum").innerHTML = `<b>${esc(t.file.name)}</b>: ${t.features.length.toLocaleString("en")} features · ${t.samples.length} samples (pipeline of Tabui)`;
    $("#unt-go").disabled = !t.features.length; $("#unt-res").hidden = true;
  }

  // ------------------------------------------------------------ match
  async function match() {
    if (st.busy || !st.table) return;
    const t = st.table, nq = t.features.filter(f => f.ms2).length;
    if (!window.LIB) return say("The library module is not available.");
    if (!t.pol && nq) return say("Choose the polarity (positive or negative) before matching.");
    try { const l = await window.LIB.call("list"); if (nq && !(l.libs && l.libs.length)) { say("No spectral library loaded: open \"Libraries...\" and add one (the features without MS2 would all stay at level 5)."); return; } } catch (e) { /* the search reports it */ }
    st.busy = true; st.cancel = false; st.tol = Math.max(1, +$("#unt-tol").value || 10);
    const prog = $("#unt-prog"), go = $("#unt-go"), stop = $("#unt-stop"); go.disabled = true; stop.hidden = false; prog.hidden = false; prog.value = 0;
    try {
      const res = await M().run(t, { tolPpm: st.tol, frag: 0.01, isCancelled: () => st.cancel, onProgress: (i, n) => { prog.value = n ? i / n : 1; say(`Searching ${i.toLocaleString("en")} / ${n.toLocaleString("en")} spectra...`); } });
      if (st.cancel) { say("Stopped."); } else { st.res = res; st.graph = null; st.mounted = null; say(`${res.stats.queries.toLocaleString("en")} spectra searched in ${(res.stats.ms / 1000).toFixed(1)} s.`); fill(); }
    } catch (e) { say(""); $("#unt-sum").insertAdjacentHTML("beforeend", `<br><span class="err">${esc(e.message || e)}</span>`); }
    st.busy = false; go.disabled = false; stop.hidden = true; prog.hidden = true;
  }

  // ------------------------------------------------------------ tables (TPTAB): one row per feature, or per identity (group by identity); the open row is the panel under it
  function rank(r) { return L().rank(M().shown(r)); }
  const hasAreas = () => st.table.samples.length > 1;
  const sentence = r => L().sentence(M().shown(r)) + (r.name ? " · " + r.name : "") + (r.lib ? " (" + r.lib + ")" : "");
  /** the chips of level, MS2 and decided, the same in both tables; `get(row)` gives {lv, ms2, decided} */
  const chipsFor = get => [["l2", "L2 or better", "2b"], ["l3", "L3 or better", "3b"], ["l4", "L4 or better", "4"]].map(([key, label, to]) => ({ key, label, help: "ft.chip.level", test: x => L().rank(get(x).lv) <= L().rank(to) }))
    .concat([{ key: "l5", label: "L5 only", help: "ft.chip.l5", test: x => get(x).lv === "5" }, { key: "ms2", label: "With MS2", help: "ft.chip.ms2", test: x => !!get(x).ms2 }, { key: "dec", label: "Decided", help: "ft.chip.decided", test: x => !!get(x).decided }]);
  const nameOf = (x) => x.r.name || "m/z " + x.f.mz.toFixed(4);
  const extText = ext => (ext[0] || "") + (ext.length > 1 ? ` (+${ext.length - 1})` : "");
  /** the columns of the table of features; the areas per sample (cells) are shown by default only when the table has more than one sample */
  function colsF() {
    const mx = x => maxArea(x.f), states = x => st.table.samples.map(s => stateOf(x.f, s));
    return [
      { key: "level", label: "Level", help: "ft.col.level", sticky: true, sort: x => rank(x.r), cell: x => lvBadge(x.r.level, x.r.decided), text: x => L().tag(M().shown(x.r)) },
      { key: "identity", label: "Identity", help: "ft.col.cand", sticky: true, sort: x => nameOf(x).toLowerCase(), cell: x => esc(nameOf(x)), text: nameOf },
      { key: "mz", label: "m/z", help: "ft.col.mz", num: true, sort: x => x.f.mz, cell: x => x.f.mz.toFixed(4) },
      { key: "rt", label: "RT", help: "ft.col.rt", num: true, sort: x => x.f.rt, cell: x => x.f.rt.toFixed(2) },
      { key: "cells", label: "Cells", help: "ft.col.cells", off: !hasAreas(), cell: x => TPTAB().cells(states(x)), text: x => states(x).map(v => v[0]).join("") },
      { key: "max", label: "Max area", help: "ft.col.maxarea", num: true, sort: mx, cell: x => fmtA(mx(x)), text: x => String(Math.round(mx(x))) },
      { key: "ms2", off: true, label: "MS2", help: "ft.col.ms2", num: true, sort: x => x.f.ms2 ? x.f.ms2.mz.length : 0, cell: x => x.f.ms2 ? String(x.f.ms2.mz.length) : "" },
      { key: "ent", off: true, label: "Entropy", help: "ft.col.entropy", num: true, sort: x => x.r.ent, cell: x => x.r.ent == null ? "" : x.r.ent.toFixed(3) },
      { key: "ppm", off: true, label: "Δppm", help: "ft.col.ppm", num: true, sort: x => x.r.dppm, cell: x => x.r.dppm == null ? "" : x.r.dppm.toFixed(1) },
      { key: "id", off: true, label: "ID", help: "ft.col.id", sort: x => x.f.id, cell: x => esc(x.f.id) },
      { key: "ext", off: true, label: "External evidence", help: "ft.col.ext", cell: x => esc(extText(x.r.external)), text: x => x.r.external.join(" | ") },
    ];
  }
  /** the columns of the table of identities */
  function colsG() {
    const b = x => x.x.best, n = x => x.x.members.length;
    return [
      { key: "level", label: "Level", help: "ft.col.level", sticky: true, sort: x => L().rank(x.x.level), cell: x => lvBadge(b(x) ? b(x).level : x.x.level, x.x.decided ? x.x.level : null), text: x => L().tag(x.x.level) },
      { key: "identity", label: "Identity", help: "ft.col.members", sticky: true, sort: x => x.i, cell: x => `${esc(x.x.id)} · ${n(x)} feature${n(x) === 1 ? "" : "s"}`, text: x => `${x.x.id} · ${n(x)} features` },
      { key: "nmass", label: "Neutral mass", help: "ft.col.nmass", num: true, sort: x => x.x.neutral.mass, cell: x => x.x.neutral.mass.toFixed(4) },
      { key: "rt", label: "RT", help: "ft.col.rt", num: true, sort: x => x.x.rt, cell: x => x.x.rt.toFixed(2) },
      { key: "max", label: "Max area", help: "ft.col.maxarea", num: true, sort: x => x.x.maxArea, cell: x => fmtA(x.x.maxArea), text: x => String(Math.round(x.x.maxArea)) },
      { key: "cand", label: "Best candidate", help: "ft.col.cand", sort: x => (b(x) ? b(x).name : "").toLowerCase(), cell: x => esc(b(x) ? b(x).name : "") },
      { key: "ms2", off: true, label: "MS2", help: "ft.col.ms2", sort: x => x.x.hasMs2 ? 1 : 0, cell: x => x.x.hasMs2 ? "yes" : "" },
      { key: "ent", off: true, label: "Entropy", help: "ft.col.entropy", num: true, sort: x => b(x) ? b(x).ent : null, cell: x => b(x) && b(x).ent != null ? b(x).ent.toFixed(3) : "" },
      { key: "ppm", off: true, label: "Δppm", help: "ft.col.ppm", num: true, sort: x => b(x) ? b(x).dppm : null, cell: x => b(x) && b(x).dppm != null ? b(x).dppm.toFixed(1) : "" },
      { key: "ext", off: true, label: "External evidence", help: "ft.col.ext", cell: x => esc(extText(x.x.external)), text: x => x.x.external.join(" | ") },
    ];
  }
  const rowsF = () => st.order.map(i => ({ i, f: st.table.features[i], r: st.res.get(st.table.features[i].key) }));
  const rowsG = () => st.gorder.map(i => ({ i, x: st.graph.identities[i] }));

  /** both tables are mounted once per table read (and per match); the rows of each come with `fill` and `group` */
  function mountTables() {
    if (st.mounted === st.table && st.TF) return;
    st.mounted = st.table;
    const T = TPTAB(), TB = TPRIGA().tabs, byKey = new Map(st.table.features.map(f => [f.key, f]));
    st.TF = T.mount($("#unt-tf"), { id: "unt-t", panelId: "unt-det", label: "Features of the table", cols: colsF(), rowId: x => x.i, esc, name: x => `feature ${x.f.id}${x.r.name ? ", " + x.r.name : ""}`, columns: true, height: 460, exportName: "untargeted_view",
      chips: chipsFor(x => ({ lv: M().shown(x.r), ms2: x.r.hasMs2, decided: x.r.decided })), search: x => [x.f.id, x.f.mz.toFixed(4), x.r.name || "", ...x.r.external].join(" "),
      tabs: [TB[0], ...(hasAreas() ? [TB[1]] : []), TB[2]], head: headF, onOpen: fillF });
    st.TG = T.mount($("#unt-tg"), { id: "unt-g", panelId: "unt-gdet", label: "Identities of the table", cols: colsG(), rowId: x => x.i, esc, name: x => `${x.x.id}, ${x.x.members.length} features`, columns: true, height: 460, exportName: "untargeted_identities_view",
      chips: chipsFor(x => ({ lv: x.x.level, ms2: x.x.hasMs2, decided: x.x.decided })),
      search: x => [x.x.id, String(x.x.neutral.mass), ...x.x.members.map(k => { const f = byKey.get(k), r = st.res.get(k); return f ? `${f.id} ${f.mz.toFixed(4)} ${r ? r.name : ""}` : ""; })].join(" "),
      tabs: [TB[2], { key: "members", label: "Features", help: "ft.tab.members" }], head: headG, onOpen: fillG });
  }

  /** the head of the open row of a feature: level (the decided one with the double border), «Decided», the sentence of the level */
  function headF(x, box) { TPRIGA().head(box, { level: x.r.level, decided: x.r.decided, words: sentence(x.r) }); }
  function headG(x, box) { TPRIGA().head(box, { level: x.x.best ? x.x.best.level : x.x.level, decided: x.x.decided ? x.x.level : null, words: `RT ${x.x.rt.toFixed(2)} min · ${ID().hypothesisText(x.x.neutral, st.table)}` }); }
  /** the tab `tab` of the open row of a feature */
  function fillF(x, body, tab) {
    st.sel = x.f.key;
    if (tab === "samples") { TPRIGA().fill(body, "samples", { samples: st.table.samples.map(s => ({ file: s, area: x.f.areas[s] || 0, state: stateOf(x.f, s) })) }); return; }
    if (tab === "identity") { identity(x, body); return; }
    body.innerHTML = x.f.ms2 ? `<canvas id="unt-cv" aria-label="Mirror plot of the MS2 spectrum of the feature against the best library candidate"></canvas><div class="mut" id="unt-mm"></div>` : `<div class="mut">No MS<sup>2</sup> for this feature: the level cannot be higher than 4.</div>`;
    if (x.f.ms2) mirror(x.f, st.table);
  }
  /** the tab of an identity: the network and the evidence of the links, or the features of the group */
  function fillG(x, body, tab) {
    st.sel = x.x.id;
    if (tab === "members") { body.innerHTML = ID().membersHtml(x.x, st.table, st.res); body.querySelectorAll("button[data-open]").forEach(b => { b.onclick = () => showFeature(b.dataset.open); }); return; }
    body.innerHTML = ID().identityHtml(x.x, st.graph, st.table, st.res);
  }
  /** the tab Identity of a feature: evidence of the level, external evidence (never a level), the level decided by the chemist, «What is it?» */
  function identity(x, body) {
    const r = x.r, f = x.f;
    const lvOpts = ["", ...L().ORDER].map(l => `<option value="${l}"${(r.decided || "") === l ? " selected" : ""}>${l ? esc(L().tag(l) + " · " + L().name(l)) : "(no decision)"}</option>`).join("");
    body.innerHTML = `<div class="mut">${esc(sentence(r))}</div>
      <ul>${r.evidence.map(e => `<li>${esc(e)}</li>`).join("")}</ul>
      ${r.external.length ? `<div class="mut">External evidence (never a level):</div><ul>${r.external.map(e => `<li>${esc(e)}</li>`).join("")}</ul>` : ""}
      <div class="row"><label>Level decided by the chemist <select id="unt-dec">${lvOpts}</select></label>${f.ms2 ? `<button type="button" id="unt-what">What is it?</button>` : ""}</div><div class="mut" id="unt-decmsg" role="status"></div>`;
    $("#unt-dec").onchange = e => {
      const o = M().decide(r, e.target.value); $("#unt-decmsg").textContent = o.ok ? "" : o.msg; if (!o.ok) e.target.value = r.decided || "";
      levelsChanged(); const d = $("#unt-dec"); if (d) d.focus();
    };
    const w = $("#unt-what"); if (w) w.onclick = () => { try { window.LIB.whatIs(M().panelOf(f, st.table)); } catch (e) { $("#unt-decmsg").textContent = String(e.message || e); } };
  }

  function reorder() {
    const t = st.table;
    st.order = t.features.map((f, i) => i).sort((a, b) => { const ra = st.res.get(t.features[a].key), rb = st.res.get(t.features[b].key); return rank(ra) - rank(rb) || (rb.ent ?? -1) - (ra.ent ?? -1) || a - b; });
  }
  function fill() {
    $("#unt-res").hidden = false; reorder();
    if (/** @type {any} */ (window).UNTX && !$("#unt-x-xlsx")) { $("#unt-exp").innerHTML = /** @type {any} */ (window).UNTX.buttons(); /** @type {any} */ (window).UNTX.bind(dlg, () => ({ table: st.table, res: st.res, tol: st.tol, all: () => st.order.map(i => st.table.features[i]), visible })); }
    mountTables(); st.TF.setRows(rowsF());
    if (st.grp && !st.graph) group(); else refresh();
  }
  /** the identities take level and candidate from the (possibly changed) results; the tables show the right one */
  function refresh() {
    if (st.grp && st.graph) { ID().annotate(st.graph, st.table, st.res); order(); st.TG.setRows(rowsG()); }
    table();
  }
  function order() {
    const g = st.graph; st.gorder = g.identities.map((x, i) => i).sort((a, b) => { const xa = g.identities[a], xb = g.identities[b]; return L().rank(xa.level) - L().rank(xb.level) || ((xb.best && xb.best.ent) ?? -1) - ((xa.best && xa.best.ent) ?? -1) || a - b; });
  }
  /** a level was decided: the rows are ordered again and the head of the open row shows the new level */
  function levelsChanged() {
    reorder(); st.TF.setRows(rowsF()); refresh();
    const T = st.grp && st.graph ? st.TG : st.TF, box = T.panel.querySelector(".tpt-head"), row = T.view.find(x => x.i === T.openId);
    if (box && row) (T === st.TF ? headF : headG)(row, box);
  }
  /** the features that the view shows (the members of the identities when the rows are identities): for «only the filtered features» of the export */
  function visible() {
    if (st.grp && st.graph && st.TG) {
      const byKey = new Map(st.table.features.map(f => [f.key, f])), seen = new Set(), out = [];
      for (const x of st.TG.view) for (const k of x.x.members) if (!seen.has(k)) { seen.add(k); const f = byKey.get(k); if (f) out.push(f); }
      return out;
    }
    return st.TF.view.map(x => x.f);
  }
  function table() {
    const g = st.grp && !!st.graph;
    $("#unt-tf").hidden = g; $("#unt-tg").hidden = !g;
  }
  /** a member of an identity: the table goes back to the features and the row of the feature opens (the chips that hide it are cleared) */
  function showFeature(key) {
    st.grp = false; $("#unt-grp").checked = false; $("#unt-cyc-l").hidden = true; table();
    const i = st.table.features.findIndex(f => f.key === key);
    if (i >= 0 && !st.TF.open(i)) { st.TF.reset(); st.TF.open(i); }
  }

  // ------------------------------------------------------------ identities: one row per group of features of the same compound, with the evidence of every link in the open row
  async function group() {
    $("#unt-cyc-l").hidden = !st.grp;
    if (!st.grp) { table(); return; }
    if (!st.graph) {
      say("Grouping the features by identity..."); const tp = window.TPCALL;
      if (!tp) { say("The Tabui engine is not available."); st.grp = false; $("#unt-grp").checked = false; return; }
      try { const g = await ID().compute(st.table, tp, { opt: { cycle_s: st.cyc } }); ID().annotate(g, st.table, st.res); st.graph = g; g.warnings.forEach(w => say(w)); if (!g.warnings.length) say(`${g.stats.grouped.toLocaleString("en")} groups of 2 or more features, ${g.stats.identities.toLocaleString("en")} rows in all.`); }
      catch (e) { say(String(e.message || e)); st.grp = false; $("#unt-grp").checked = false; $("#unt-cyc-l").hidden = true; return; }
    }
    order(); st.TG.setRows(rowsG());
    table();
  }

  // ------------------------------------------------------------ detail: the mirror plot of the MS2 against the best library candidate
  async function mirror(f, t) {
    const cv = $("#unt-cv"), mm = $("#unt-mm"); if (!cv) return;
    try {
      const top = Math.max(...f.ms2.it) || 1, peaks = f.ms2.mz.map((m, j) => [m, f.ms2.it[j] / top]).filter(p => p[1] > 0), prec = f.ms2.prec > 0 ? f.ms2.prec : f.mz;
      const R = await window.LIB.call("search", { peaks, prec, pol: f.pol || t.pol || 0, tol: { v: st.tol, unit: "ppm" }, frag: 0.01 });
      if (st.sel !== f.key) return;
      if (!R.libs || !R.results.length) { mm.textContent = R.libs ? `No library entry within ${st.tol} ppm of the precursor.` : "No library loaded."; cv.hidden = true; return; }
      cv.hidden = false; window.LIB.mirror(cv, { mz: R.query.mz, it: R.query.it, hr: true, prec, frag: 0.01 }, R.results[0]);
      mm.textContent = `${R.results[0].name} · entropy ${R.results[0].ent.toFixed(3)} · ${R.results[0].shared} shared peaks`;
    } catch (e) { mm.textContent = String(e.message || e); }
  }

  window.UNTUI = { open, state: st, adopt };
})();
