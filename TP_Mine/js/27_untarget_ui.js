// TPMINE-PRIVATE  Untargeted window of mzFinder: load the files of a feature table (mzMine, MS-DIAL, GNPS2), match every feature with an MS2 against the libraries, table with level,
// best candidate, entropy, Δppm and evidence, filter by level, click → mirror and «What is it?» (the window of the Data tab), the chemist's decided level, export (28_untarget_export.js).
(() => {
  "use strict";
  const esc = s => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const PAGE = 300;
  const CSS = `#unt{width:min(1240px,97vw);max-width:97vw;max-height:94vh;padding:12px 14px;border:1px solid var(--line);border-radius:8px;background:var(--panel);color:var(--ink)}
#unt::backdrop{background:rgba(0,0,0,.35)}#unt h3{margin:0}#unt .top{display:flex;align-items:center;gap:8px}#unt .top h3{flex:1}
#unt .row{display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin:6px 0}#unt .mut{color:var(--muted);font-size:12px}#unt .drop{border:2px dashed var(--line);border-radius:8px;padding:10px;text-align:center}
#unt .drop.over{border-color:var(--accent);background:var(--sel)}#unt table{border-collapse:collapse;width:100%;font-size:12px}#unt th,#unt td{padding:3px 6px;border-bottom:1px solid var(--line);text-align:left;vertical-align:middle}
#unt th{position:sticky;top:0;background:var(--panel);color:var(--muted);font-weight:600}#unt td.n,#unt th.n{text-align:right;font-variant-numeric:tabular-nums}
#unt tr.row1{cursor:pointer}#unt tr.row1:hover,#unt tr.row1.sel{background:var(--sel)}#unt tr.row1.sel td:first-child{box-shadow:inset 3px 0 0 var(--accent)}
#unt .tbl{max-height:44vh;overflow:auto;border:1px solid var(--line);border-radius:6px}#unt .err{color:var(--bad)}#unt .warn{color:var(--warn);font-size:12px}
#unt progress{width:100%}#unt .det{display:grid;grid-template-columns:1fr 1fr;gap:12px;margin-top:8px}@media(max-width:900px){#unt .det{grid-template-columns:1fr}}
#unt canvas{width:100%;height:240px;display:block;border:1px solid var(--line);border-radius:6px}#unt ul{margin:2px 0;padding-left:18px}#unt .ext{max-width:230px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}`;

  const st = { table: null, res: null, files: [], level: "all", text: "", onlyMs2: false, nShown: PAGE, sel: null, busy: false, cancel: false, tol: 10, order: null };
  const U = () => window.UNT, M = () => window.UNTM, L = () => window.LIVELLI;
  let dlg = null;
  const $ = q => dlg.querySelector(q);
  const say = t => { const e = $("#unt-msg"); if (e) e.textContent = t || ""; };
  const fmtA = v => (v >= 1e6 ? (v / 1e6).toFixed(1) + "M" : v >= 1e3 ? (v / 1e3).toFixed(v >= 1e4 ? 0 : 1) + "k" : String(Math.round(v)));
  const maxArea = f => Math.max(0, ...Object.values(f.areas));
  const lvBadge = r => { const lv = M().shown(r); return `<span class="lvb lv-${esc(lv)}" title="${esc(L().sentence(lv) + " · " + (r.lv.decisive || ""))}">${esc(lv)}${r.decided ? " ✎" : ""}</span>`; };

  function open() {
    if (!document.getElementById("unt-css")) { const s = document.createElement("style"); s.id = "unt-css"; s.textContent = CSS; document.head.appendChild(s); }
    if (!dlg) {
      dlg = document.createElement("dialog"); dlg.id = "unt"; dlg.setAttribute("aria-label", "Feature tables"); document.body.appendChild(dlg);
      dlg.innerHTML = `<div class="top"><h3>Feature tables</h3><button type="button" id="unt-x" aria-label="Close">&times;</button></div>
        <p class="mut" style="margin:6px 0">Load the files of one table (an mzMine table or FBMN quant table, its MGF with the MS<sup>2</sup>, the annotations, the GNPS2 library file, or an MS-DIAL alignment). Everything stays in this browser.
        Each feature with an MS<sup>2</sup> is searched in the loaded libraries: the result is a candidate with a level and its evidence, never an identification.</p>
        <div class="drop" id="unt-drop">Drop the files here <button type="button" id="unt-pick">Choose files...</button><input type="file" id="unt-in" multiple hidden accept=".csv,.tsv,.txt,.mgf"></div>
        <div class="row"><label>Polarity <select id="unt-pol"><option value="auto">auto</option><option value="1">positive</option><option value="-1">negative</option></select></label>
          <label>Precursor tolerance <input type="number" id="unt-tol" value="10" min="1" step="1" style="width:64px"> ppm</label>
          <button type="button" id="unt-libs">Libraries...</button><button type="button" id="unt-go" class="imp" disabled>Match</button><button type="button" id="unt-stop" hidden>Stop</button></div>
        <div id="unt-sum" class="mut"></div><progress id="unt-prog" max="1" value="0" hidden aria-label="Progress"></progress><div id="unt-msg" role="status" class="mut"></div>
        <div id="unt-res" hidden><div class="row"><label>Level <select id="unt-lv"></select></label><label>Filter <input type="text" id="unt-q" placeholder="id, m/z, name..." style="width:170px"></label>
          <label><input type="checkbox" id="unt-ms2"> only with MS<sup>2</sup></label><span class="mut" id="unt-cnt"></span><span style="flex:1"></span><span id="unt-exp"></span></div>
          <div class="tbl"><table id="unt-t"></table></div><div class="row"><button type="button" id="unt-more" hidden>Show more</button></div><div id="unt-det" class="det" hidden></div></div>`;
      dlg.querySelector("#unt-x").onclick = () => dlg.close();
      const input = $("#unt-in"), drop = $("#unt-drop");
      $("#unt-pick").onclick = () => input.click();
      input.onchange = () => { load([...input.files]); input.value = ""; };
      drop.ondragover = e => { e.preventDefault(); drop.classList.add("over"); }; drop.ondragleave = () => drop.classList.remove("over");
      drop.ondrop = e => { e.preventDefault(); drop.classList.remove("over"); load([...e.dataTransfer.files]); };
      $("#unt-libs").onclick = () => { if (window.LIB) window.LIB.open(); };
      $("#unt-go").onclick = match; $("#unt-stop").onclick = () => { st.cancel = true; };
      $("#unt-pol").onchange = () => { if (st.files.length) build(); };
      $("#unt-lv").onchange = e => { st.level = e.target.value; st.nShown = PAGE; table(); };
      $("#unt-q").oninput = e => { st.text = e.target.value.trim().toLowerCase(); st.nShown = PAGE; table(); };
      $("#unt-ms2").onchange = e => { st.onlyMs2 = e.target.checked; st.nShown = PAGE; table(); };
      $("#unt-more").onclick = () => { st.nShown += PAGE; table(); };
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
      const t = U().build(st.files.map(f => ({ name: f.name, text: f.text })), { pol });
      const main = st.files.find(f => f.name === t.used[0]);
      if (main) { try { t.file.sha256 = await U().sha256Hex(await main.file.arrayBuffer()); } catch (e) { t.file.sha256 = ""; } }
      st.table = t; st.res = null; st.sel = null;
      const s = t.stats, nMs2 = t.features.filter(f => f.ms2).length, nAnn = t.features.filter(f => f.ann.length).length;
      $("#unt-sum").innerHTML = `<b>${esc(t.used[0])}</b>: ${t.features.length.toLocaleString("en")} features · ${t.samples.length} samples · ${nMs2.toLocaleString("en")} with MS<sup>2</sup> · ${nAnn.toLocaleString("en")} with an external annotation · polarity ${t.pol > 0 ? "positive" : t.pol < 0 ? "negative" : "?"}`
        + (t.used.length > 1 ? `<br>Also read: ${esc(t.used.slice(1).join(", "))}` : "") + (t.ignored.length ? `<br><span class="warn">Not recognised: ${esc(t.ignored.join(", "))}</span>` : "")
        + (s.lipidRowsDropped ? `<br>${s.lipidRowsDropped} lipid rows (matchedlipid) discarded` : "") + (s.unreliableStructures ? `<br>${s.unreliableStructures} library structures discarded (the formula does not explain the precursor)` : "")
        + (s.gnpsRows ? `<br>GNPS2: ${s.gnpsRows} rows, ${s.gnpsAnalogs} analogues, ${s.gnpsPropagated} propagated names discarded` : "") + t.warnings.map(w => `<br><span class="warn">${esc(w)}</span>`).join("");
      $("#unt-go").disabled = !nMs2 && !t.features.length; $("#unt-res").hidden = true; say("");
    } catch (e) { st.table = null; $("#unt-go").disabled = true; say(""); $("#unt-sum").innerHTML = `<span class="err">${esc(e.message || e)}</span>`; }
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
      if (st.cancel) { say("Stopped."); } else { st.res = res; st.level = "all"; st.nShown = PAGE; say(`${res.stats.queries.toLocaleString("en")} spectra searched in ${(res.stats.ms / 1000).toFixed(1)} s.`); fill(); }
    } catch (e) { say(""); $("#unt-sum").insertAdjacentHTML("beforeend", `<br><span class="err">${esc(e.message || e)}</span>`); }
    st.busy = false; go.disabled = false; stop.hidden = true; prog.hidden = true;
  }

  // ------------------------------------------------------------ table
  function rank(r) { return L().rank(M().shown(r)); }
  function fill() {
    const t = st.table, cnt = {}; for (const f of t.features) { const lv = M().shown(st.res.get(f.key)); cnt[lv] = (cnt[lv] || 0) + 1; }
    $("#unt-lv").innerHTML = `<option value="all">all (${t.features.length.toLocaleString("en")})</option>` + L().ORDER.filter(l => cnt[l]).map(l => `<option value="${l}">${l} · ${esc(L().name(l))} (${cnt[l].toLocaleString("en")})</option>`).join("");
    $("#unt-lv").value = st.level; $("#unt-res").hidden = false;
    st.order = t.features.map((f, i) => i).sort((a, b) => { const ra = st.res.get(t.features[a].key), rb = st.res.get(t.features[b].key); return rank(ra) - rank(rb) || (rb.ent ?? -1) - (ra.ent ?? -1) || a - b; });
    if (window.UNTX && !$("#unt-x-xlsx")) { $("#unt-exp").innerHTML = window.UNTX.buttons(); window.UNTX.bind(dlg, () => ({ table: st.table, res: st.res, tol: st.tol, all: () => st.order.map(i => st.table.features[i]), visible })); }
    table();
  }
  function visible() {
    const t = st.table, q = st.text, out = [];
    for (const i of st.order) {
      const f = t.features[i], r = st.res.get(f.key);
      if (st.level !== "all" && M().shown(r) !== st.level) continue;
      if (st.onlyMs2 && !r.hasMs2) continue;
      if (q && !(f.id.toLowerCase().includes(q) || f.mz.toFixed(4).includes(q) || (r.name || "").toLowerCase().includes(q) || r.external.some(x => x.toLowerCase().includes(q)))) continue;
      out.push(f);
    }
    return out;
  }
  function table() {
    const t = st.table, rows = visible(), shown = rows.slice(0, st.nShown);
    $("#unt-cnt").textContent = `${rows.length.toLocaleString("en")} features`;
    $("#unt-t").innerHTML = `<tr><th>Level</th><th>ID</th><th class="n"><i>m/z</i></th><th class="n">RT</th><th class="n">Max area</th><th class="n">MS<sup>2</sup></th><th>Best candidate</th><th class="n">Entropy</th><th class="n">Δppm</th><th>External evidence</th></tr>`
      + shown.map(f => { const r = st.res.get(f.key); return `<tr class="row1${st.sel === f.key ? " sel" : ""}" data-k="${esc(f.key)}" tabindex="0"><td>${lvBadge(r)}</td><td>${esc(f.id)}</td><td class="n">${f.mz.toFixed(4)}</td><td class="n">${f.rt.toFixed(2)}</td><td class="n">${fmtA(maxArea(f))}</td>
        <td class="n">${f.ms2 ? f.ms2.mz.length : ""}</td><td>${esc(r.name)}</td><td class="n">${r.ent == null ? "" : r.ent.toFixed(3)}</td><td class="n">${r.dppm == null ? "" : r.dppm.toFixed(1)}</td><td class="ext" title="${esc(r.external.join("\n"))}">${esc(r.external[0] || "")}${r.external.length > 1 ? ` (+${r.external.length - 1})` : ""}</td></tr>`; }).join("");
    const more = $("#unt-more"); more.hidden = rows.length <= shown.length; more.textContent = `Show ${Math.min(PAGE, rows.length - shown.length)} more of ${(rows.length - shown.length).toLocaleString("en")}`;
    $("#unt-t").querySelectorAll("tr.row1").forEach(tr => { tr.onclick = () => select(tr.dataset.k); tr.onkeydown = e => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); select(tr.dataset.k); } }; });
  }

  // ------------------------------------------------------------ detail: evidence, external annotations, decision, mirror, «What is it?»
  async function select(key) {
    st.sel = key; const t = st.table, f = t.features.find(x => x.key === key), r = st.res.get(key); if (!f || !r) return;
    $("#unt-t").querySelectorAll("tr.row1").forEach(tr => tr.classList.toggle("sel", tr.dataset.k === key));
    const D = $("#unt-det"); D.hidden = false;
    const lvOpts = ["", ...L().ORDER].map(l => `<option value="${l}"${(r.decided || "") === l ? " selected" : ""}>${l ? esc(l + " · " + L().name(l)) : "(no decision)"}</option>`).join("");
    D.innerHTML = `<div><h3>Feature ${esc(f.id)} · <i>m/z</i> ${f.mz.toFixed(4)} · RT ${f.rt.toFixed(2)} min ${lvBadge(r)}</h3>
      <div class="mut">${esc(L().sentence(M().shown(r)))}${r.name ? " · " + esc(r.name) : ""}${r.lib ? " (" + esc(r.lib) + ")" : ""}</div>
      <ul>${r.evidence.map(x => `<li>${esc(x)}</li>`).join("")}</ul>
      ${r.external.length ? `<div class="mut">External evidence (never a level):</div><ul>${r.external.map(x => `<li>${esc(x)}</li>`).join("")}</ul>` : ""}
      <div class="row"><label>Level decided by the chemist <select id="unt-dec">${lvOpts}</select></label></div><div class="mut" id="unt-decmsg" role="status"></div>
      <div class="row">${f.ms2 ? `<button type="button" id="unt-what">What is it?</button>` : ""}</div></div>
      <div>${f.ms2 ? `<canvas id="unt-cv" aria-label="Mirror plot of the MS2 spectrum of the feature against the best library candidate"></canvas><div class="mut" id="unt-mm"></div>` : `<div class="mut">No MS<sup>2</sup> for this feature: the level cannot be higher than 4.</div>`}</div>`;
    $("#unt-dec").onchange = e => {
      const o = M().decide(r, e.target.value); $("#unt-decmsg").textContent = o.ok ? "" : o.msg; if (!o.ok) e.target.value = r.decided || "";
      fill(); const tr = $("#unt-t").querySelector(`tr[data-k="${CSS_escape(key)}"]`); if (tr) tr.classList.add("sel");
    };
    const w = $("#unt-what"); if (w) w.onclick = () => { try { window.LIB.whatIs(M().panelOf(f, t)); } catch (e) { $("#unt-decmsg").textContent = String(e.message || e); } };
    if (f.ms2) mirror(f, t);
  }
  const CSS_escape = s => String(s).replace(/["\\]/g, "\\$&");
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

  window.UNTUI = { open, state: st };
})();
