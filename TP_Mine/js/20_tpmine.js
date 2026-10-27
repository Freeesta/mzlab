// @ts-strict
// TPMINE-PRIVATE  TP Mine: the tab with the whole automatic search of transformation products (only for the course owner).
(() => {
  "use strict";
  const { ctx, esc, fmtA, fileKey, fileLabel, TYPES, KINDS, CSS, startWorker, send, call, callRaw, killWorker, setProgress, pageFiles, hue, chart } = /** @type {any} */ (window).TPBASE;

  // ---------------------------------------------------------------- the tab
  function open(sec) {
    if (!document.getElementById("tp-css")) { const st = document.createElement("style"); st.id = "tp-css"; st.textContent = CSS; document.head.appendChild(st); }
    const W = /** @type {any} */ (window), dt = k => (W.TPHELP ? ` data-tt="${k}"` : "");
    sec.innerHTML = `<div class="tp-wrap">
<div class="tp-tools"><button type="button" id="tp-sb" aria-controls="tp-left" aria-pressed="false" aria-label="Hide the loading panel">Hide panel</button></div>
<div class="tp">
<div id="tp-left">
 <div class="card"><h3>1 · Files</h3>
  <div class="row"><button id="tp-page" title="Reload the files already open in the Data tab (browser memory)">Reload from Data</button><button id="tp-pick" title=".mzML, .raw, .wiff (+ .wiff.scan)">Choose files...</button><button id="tp-rmall" type="button" aria-label="Remove all the files" hidden>Remove all</button></div>
  <input type="file" id="tp-in" accept=".mzML,.mzml,.raw,.wiff,.scan" multiple hidden>
  <div id="tp-load" role="status" aria-live="polite"></div><div id="tp-fsum" class="fsum"></div><div id="tp-files" class="mut"></div>
  <details id="tp-cond" hidden><summary title="Compare several conditions of the same degradation (gases, photocatalysts...): label the files with their condition">Conditions of the files</summary><div id="tp-cfiles"></div></details><div class="row"><button id="tp-more" type="button" aria-expanded="false" hidden></button></div></div>
 <div class="card"><h3>2 · Parent compound</h3><div id="tp-struct"></div><div id="tp-reatt"></div><div id="tp-reatt-ctl" hidden></div>
  <div class="row tp-ac"><input type="text" id="tp-name" class="grow-in" placeholder="Name" role="combobox" aria-autocomplete="list" aria-expanded="false" aria-controls="tp-sug" aria-label="Parent compound name" autocomplete="off" spellcheck="false"${dt("ft.f.name")}><ul id="tp-sug" role="listbox" aria-label="Suggestions" hidden></ul></div>
  <div class="row"><input type="text" id="tp-mol" class="grow-in" placeholder="Neutral formula" aria-label="Neutral molecular formula" spellcheck="false"${dt("ft.f.formula")}></div>
  <div class="row"><input type="text" id="tp-smi" class="grow-in" placeholder="SMILES" aria-label="SMILES" spellcheck="false"${dt("ft.f.smiles")}><button id="tp-draw" type="button" title="Draw the molecule in the Draw tab and bring the SMILES back here">Draw</button></div>
  <div id="tp-warn" role="status"></div>
  <div class="row"><label class="mut" for="tp-add"${dt("ft.f.adduct")}>Adduct</label><select id="tp-add"${dt("ft.f.adduct")}><option>[M+H]+</option><option>[M+Na]+</option><option>[M+NH4]+</option><option>[M-H]-</option><option>[M+HCOO]-</option></select><span class="mut" id="tp-prev"></span></div>
  <div class="row"><label class="mut grow" for="tp-prt"${dt("ft.f.prt")}>RT (min)</label><input type="number" id="tp-prt"${dt("ft.f.prt")} step="0.05" min="0" placeholder="auto" style="width:70px"></div></div>
 <div class="card"><h3>3 · Settings</h3>
  <div class="row"><button id="tp-est" type="button" title="Low resolution: propose the XIC window, the RT tolerance and the start of the analysis from the loaded files (yellow values, editable)">Estimate from data</button></div>
  <div class="row"><label class="mut grow" id="tp-tol-l" for="tp-tol"${dt("ft.f.tol")}>XIC ±Da</label><input type="number" id="tp-tol"${dt("ft.f.tol")} value="0.35" step="0.05" min="0.05" style="width:70px"></div>
  <div class="row"><label class="mut grow" for="tp-rtt"${dt("ft.f.rtt")}>RT ±min</label><input type="number" id="tp-rtt"${dt("ft.f.rtt")} value="0.25" step="0.05" min="0.05" style="width:70px"></div>
  <div class="row"><label class="mut grow" for="tp-steps"${dt("ft.f.steps")}>Steps</label><input type="number" id="tp-steps"${dt("ft.f.steps")} value="2" min="1" max="3" style="width:70px"></div>
  <div class="row"><label class="mut grow" for="tp-minf"${dt("ft.f.minf")}>Min. samples</label><input type="number" id="tp-minf"${dt("ft.f.minf")} value="3" min="0" max="20" style="width:70px"></div>
  <div class="row"><label class="mut grow" for="tp-rtmin"${dt("ft.f.rtmin")}>RT from (min)</label><input type="number" id="tp-rtmin"${dt("ft.f.rtmin")} value="0.5" step="0.1" min="0" style="width:70px"></div>
  <div class="row"><label id="tp-disc-h"${dt("ft.f.disc")}><input type="checkbox" id="tp-disc" checked> Unexpected ions</label></div>
  <details><summary>List of transformations (name;change)</summary><textarea id="tp-tr" spellcheck="false"></textarea></details></div>
 <div class="card" id="tp-minecard"><button id="tp-go" class="imp dig" type="button" aria-busy="false">${(typeof QICON !== "undefined" && QICON.mine) ? QICON.get("mine", 28) : ""}<span id="tp-go-t">Dig</span></button><div class="bar" id="tp-barbox" role="progressbar" aria-label="Progress" aria-valuemin="0" aria-valuemax="100"><i id="tp-bar"></i></div><div class="mut" id="tp-msg" role="status" style="margin-top:4px"></div></div>
 <div class="card" id="tp-untcard"><h3>Feature tables</h3><div class="mut">mzMine, MS-DIAL or GNPS2 tables: match every feature with an MS<sup>2</sup> against the spectral libraries and assign a level.</div><div class="row"><button id="tp-unt" type="button" aria-haspopup="dialog">Match a feature table...</button></div></div>
</div>
<div id="tp-right"><div class="card mut" id="tp-empty">Load the files, enter the parent and press <b>Dig</b>: Tabui calibrates the m/z on the parent, generates the candidates, extracts the XICs, looks for peaks that grow over time and are missing from the blank, checks the isotopes, compares the product ions with those of the parent and shows the prediction of each transformation (Prev. N). The identification level is yours to decide.</div></div></div></div>`;
    const $ = s => sec.querySelector(s);
    if (W.TPHELP) W.TPHELP.attach(sec);
    const FIELD = () => W.TPFIELD, parentFields = () => [$("#tp-mol"), $("#tp-smi")];
    if (W.TPFIELD) W.TPFIELD.watch(sec, () => { if ($("#tp-msg").dataset.field) msg(""); });       // the group returns to normal on a click or a key in it; so does its message
    if (W.TPUI) W.TPUI.sidePanel(sec);       // 45_tabui_ui.js: the button that hides the loading panel
    /** @type {any} */
    const st = { files: [], showAll: false, summary: null, sel: null, view: "tab", took: 0, pname: "", weakOn: false, noAuto: false, cond: null, gnames: /** @type {string[]} */ ([]), gctl: null, gv: null };
    const msg = (t, err) => { const m = $("#tp-msg"); m.textContent = t || ""; m.className = "mut" + (err ? " err" : ""); delete m.dataset.field; };
    const bar = f => { const w = Math.round(f * 100); $("#tp-bar").style.width = w + "%"; $("#tp-barbox").setAttribute("aria-valuenow", w); };
    setProgress((t, f, m) => { if (t) msg(t); if (f != null) bar(f); if (m && window.TPLIVE) TPLIVE.event(m); });

    $("#tp-more").onclick = () => { st.showAll = !st.showAll; renderFiles(); };
    // ---- files
    const renderFiles = () => {
      modeUI(); condUI(); if (!st.files.length) { $("#tp-files").innerHTML = ""; $("#tp-more").hidden = true; $("#tp-rmall").hidden = true; return; }
      $("#tp-files").innerHTML = `<table><tr><th>File</th><th>Data type</th><th>Sample</th><th>t (min)</th><th><span class="tp-sr">Remove</span></th></tr>` + st.files.map((f, i) => `<tr><td><span class="tp-fn" tabindex="0"${W.TPHELP ? ` data-tt="ft.riga.cell" data-tt-text="${esc(f.name)}"` : ""}>${esc(fileLabel(f.name))}</span></td><td>${KINDS[f.kind] || f.kind}${f.error ? ` <span class="err" title="${esc(f.error)}">!</span>` : ""}</td>
        <td>${f.kind === "full" || f.kind === "mrm" || f.kind === "hr" ? `<select data-i="${i}" data-k="type" aria-label="Data type of ${esc(fileLabel(f.name))}">${Object.entries(TYPES).map(([k, v]) => `<option value="${k}"${f.type === k ? " selected" : ""}>${v}</option>`).join("")}</select>` : (f.kind === "ms2" ? "fragments" : "-")}</td>
        <td>${f.kind === "full" || f.kind === "ms2" || f.kind === "mrm" || f.kind === "hr" ? `<input type="number" data-i="${i}" data-k="time" aria-label="t (min) of ${esc(fileLabel(f.name))}" value="${f.time ?? ""}" style="width:60px" step="any">` : ""}</td><td><button type="button" class="tp-rm" data-rm="${i}" aria-label="Remove ${esc(fileLabel(f.name))}">×</button></td></tr>`).join("") + "</table>";
      const MAXV = 6, many = st.files.length > MAXV;
      $("#tp-files").querySelectorAll("tr").forEach((tr, i) => { if (i > 0) tr.hidden = many && !st.showAll && i > MAXV; });
      const more = $("#tp-more"); more.hidden = !many; more.setAttribute("aria-expanded", st.showAll ? "true" : "false");
      more.textContent = st.showAll ? "Show fewer" : "Show all " + st.files.length + " files";
      const cn = {}; st.files.forEach(f => { const k = f.kind === "hr" ? "LC-HRMS" : f.kind === "msn" ? "MSn" : (KINDS[f.kind] || f.kind); cn[k] = (cn[k] || 0) + 1; });
      const bl = st.files.filter(f => f.type === "blank").length;
      $("#tp-fsum").textContent = Object.entries(cn).map(([k, v]) => v + " " + k).join(" · ") + (bl ? " · " + bl + (bl === 1 ? " blank" : " blanks") : "");
      $("#tp-rmall").hidden = false; $("#tp-files").querySelectorAll("[data-rm]").forEach(b => b.onclick = () => removeFiles([Number(/** @type {HTMLElement} */ (b).dataset.rm)]));
      $("#tp-files").querySelectorAll("[data-k]").forEach(el => el.onchange = () => { const f = st.files[+el.dataset.i]; f[el.dataset.k] = el.dataset.k === "time" ? (el.value === "" ? null : +el.value) : el.value; });
    };
    // ---- conditions of the files (several conditions of the same degradation): the table of roles, conditions, times and replicates is TP_Mine/lazy/gruppi-campioni.js (loaded when the section is opened);
    // with two conditions or more it rules the roles and times of the Dig, which then compares the conditions (gruppi/1). With one condition (or none) nothing changes.
    const TYPE_OF = { sample: "sample", t0: "sample", blank: "blank", std: "standard", dark: "control" };
    const useCond = () => !!(st.cond && st.cond.conditions.length >= 2);
    const condFiles = () => st.files.filter(f => f.kind === "full" || f.kind === "ms2");
    function condUI() {
      const det = /** @type {HTMLDetailsElement} */ ($("#tp-cond")), many = !st.files.some(f => f.kind === "hr") && st.files.filter(f => f.kind === "full").length >= 2;
      det.hidden = !many; if (!many) { st.cond = null; $("#tp-files").hidden = false; return; }
      if (det.open && st.gnames.join("\n") !== condFiles().map(f => f.name).join("\n")) void mountConds();
    }
    /** the table of conditions for the files now loaded; its answer (campioni/2 canonical, reference, conditions) is kept in st.cond */
    async function mountConds() {
      const W = /** @type {any} */ (window), names = condFiles().map(f => f.name), box = $("#tp-cfiles");
      try {
        await Promise.all([W.nbEnsure ? W.nbEnsure() : 0, W.TPLAZY.load("gruppi-campioni.js")]);
        const R = await import(URL.createObjectURL(new Blob([ctx().files["finder-ruoli.js"]], { type: "text/javascript" })));
        const call = async (/** @type {string} */ op, /** @type {string} */ j) => { const r = await fetch("api/campioni/" + op, { method: "POST", body: j }); const t = await r.text(); if (!r.ok) throw new Error(t); return t; };
        if (st.gctl) st.gctl.unmount();
        st.gnames = names;
        st.gctl = await W.TPGRUPPI_FILES.mount(box, names, { R, call, nb: typeof NB !== "undefined" ? NB : null, save: typeof nbSave === "function" ? nbSave : undefined, onChange: (/** @type {any} */ r) => { st.cond = r; syncFiles(); } });
      } catch (e) { box.textContent = "The table of conditions could not be loaded: " + String((e && e.message) || e); }
    }
    /** the roles and times of the table of conditions, shown also in the plain list of the files */
    function syncFiles() {
      if (!st.cond) return;
      const by = new Map(st.cond.campioni.map((/** @type {any} */ c, /** @type {number} */ i) => [st.gnames[i], c]));
      for (const f of st.files) { const c = by.get(f.name); if (!c) continue; if (c.tipo in TYPE_OF) f.type = /** @type {any} */ (TYPE_OF)[c.tipo]; f.time = c.tempo_min; }
      $("#tp-files").hidden = useCond(); renderFilesSummary();
    }
    const renderFilesSummary = () => { const bl = st.files.filter(f => f.type === "blank").length; const t = $("#tp-fsum").textContent || ""; $("#tp-fsum").textContent = t.replace(/ · \d+ blanks?$/, "") + (bl ? " · " + bl + (bl === 1 ? " blank" : " blanks") : ""); };
    $("#tp-cond").addEventListener("toggle", () => { if (/** @type {HTMLDetailsElement} */ ($("#tp-cond")).open) void mountConds(); });
    /** the files of the Dig: a list, or with two conditions or more the list with the campioni/2 of the table (the names are the ones of the files) */
    function digFiles(/** @type {any[]} */ list) {
      if (!useCond()) return JSON.stringify(list);
      const by = new Map(st.cond.campioni.map((/** @type {any} */ c, /** @type {number} */ i) => [st.gnames[i], { ...c, file: st.gnames[i] }]));
      return JSON.stringify({ files: list, campioni: { schema: "campioni/2", campioni: list.map(f => by.get(f.name)).filter(Boolean) } });
    }
    /** the options of the comparison of conditions (settings.gruppi): the reference and the fields of the notebook */
    function digGroups() {
      const o = (typeof NB !== "undefined" && /** @type {any} */ (NB).gruppi_opts) || {}, g = /** @type {any} */ ({ reference: st.cond.reference || null });
      for (const k of ["min_rep_frac", "min_area", "factors"]) if (o[k] !== undefined) g[k] = o[k];
      return g;
    }
    const BIG = 50 << 20;                   // as in the main program: above this a file is mounted from its Blob, not copied (13 HR files = 1 GB)
    // low resolution: the window, the tolerance of RT and the void front are proposed from the data (Rust, preview of the first sample); a proposed value is yellow until the user types
    let estSeq = 0, parentMz = 0;            // parentMz: the m/z of the parent ion (from the preview of the parent), for the drift of its RT
    for (const id of ["#tp-tol", "#tp-rtt", "#tp-rtmin"]) $(id).addEventListener("input", ev => { ev.target.dataset.user = "1"; ev.target.classList.remove("tp-est"); });
    $("#tp-est").addEventListener("click", () => { for (const id of ["#tp-tol", "#tp-rtt", "#tp-rtmin"]) delete $(id).dataset.user; estimateLr(); });
    async function estimateLr() {
      const lr = st.files.filter(f => f.kind === "full"), f = lr.find(x => x.type === "sample") || lr[0], seq = ++estSeq;
      if (!f || st.files.some(x => x.kind === "hr")) return;
      let e = null;
      const names = [f.name, ...lr.filter(x => x !== f).map(x => x.name)];       // the first one is previewed; all of them give the drift of the parent
      try { e = JSON.parse(await callRaw("estimate", JSON.stringify(names), parentMz ? String(parentMz) : "")); } catch (_) { /* no estimate: the defaults stay */ }
      if (!e || seq !== estSeq) return;
      const put = (id, v, dec, why) => { const el = $(id); if (v == null || !Number.isFinite(v) || el.dataset.user) return; el.value = v.toFixed(dec); el.classList.add("tp-est"); el.title = why; };
      put("#tp-tol", e.da && e.da.value, 2, `From the data: 2 × the tolerance measured (3 × the scatter of m/z) on the ${e.da && e.da.n} points of the strongest peaks of ${f.name}`);
      put("#tp-rtt", e.rt_tol, 2, e.drift && e.drift.value > 2 * (e.fwhm && e.fwhm.value || 0) ? `From the data: the RT of the parent drifts by ${e.drift.value.toFixed(2)} min over ${e.drift.n} files` : `From the data: 2 × the median width at half height of the strongest peaks of ${f.name} (${e.fwhm && e.fwhm.n} peaks)`);
      put("#tp-rtmin", e.dead, 1, `From the data: the unretained front of ${f.name} ends here`);
    }
    const sortFiles = () => st.files.sort((a, b) => (a.kind > b.kind ? 1 : a.kind < b.kind ? -1 : (a.time ?? -1) - (b.time ?? -1)));
    // 45_tabui_ui.js takes the rows out and puts them back (Undo); the files stay in the Data tab
    const removeFiles = (/** @type {number[]} */ idx) => W.TPUI.removeFiles({ st, idx, busy: () => running, say: msg, key: fileKey, label: fileLabel, sort: sortFiles, redraw: () => { renderFiles(); estimateLr(); }, pick: $("#tp-pick") });
    $("#tp-rmall").onclick = () => removeFiles(st.files.map((_, i) => i));
    async function addFiles(list, only) {         // list: [{name, buf}] or [{name, file}] (File/Blob: read only if small)
      msg("Reading files...");
      try {
        await startWorker();
        let k = 0;
        for (const f of list) {
          msg(`Reading files... ${k} of ${list.length}`); bar(k++ / list.length);
          if (f.nat) await send({ type: "put", name: f.name, blob: f.file, nat: true });          // the stub of a sample of a maker's file: the worker reads its .mzidx, no mzML is written
          else if (f.file && f.file.size > BIG) await send({ type: "put", name: f.name, blob: f.file });
          else { const buf = f.file ? await f.file.arrayBuffer() : f.buf; await send({ type: "put", name: f.name, buf }, [buf]); }
        }
        const info = await call("classify", list.map(f => f.name));
        const hint = (() => { try { return Object.fromEntries((window.E && E.files || []).map(x => [String(x.file || x.name || "").split(/[\\/]/).pop(), x])); } catch (_) { return {}; } })();
        const seen = new Set(), fresh = info.filter(i => !seen.has(fileKey(i.name)) && seen.add(fileKey(i.name)));      // one row per sample, whatever source it came from
        st.files = st.files.filter(f => !seen.has(fileKey(f.name)));
        if (!only) st.noAuto = false;
        for (const i of fresh) { if (i.hr_kind === "hr" || i.hr_kind === "msn") i.kind = i.hr_kind; const h = hint[i.name]; if (h && h.time != null) i.time = h.time; if (h && ["sample", "blank", "standard", "control"].includes(h.type)) i.type = h.type; st.files.push(i); }
        if (only) st.files = st.files.filter(f => f.kind === "full" || f.kind === "hr" || (f.kind === "ms2" && !st.files.some(x => x.kind === "hr")));   // automatic start: the Full Scan / HRMS series, and its MS2 files when it is low resolution
        sortFiles();
        renderFiles(); msg(""); bar(0);
        estimateLr();
      } catch (e) { msg(String(e.message || e), true); bar(0); }
    }
    $("#tp-page").onclick = async () => {
      await loadFromData(false); };
    const forAdd = x => x.nat ? { name: x.name, file: x.buf, nat: true } : x.buf instanceof Blob ? { name: x.name, file: x.buf } : { name: x.name, buf: /** @type {any} */ (x.buf).slice ? /** @type {any} */ (x.buf).slice(0) : x.buf };
    // files of the instrument (.raw, .wiff + .wiff.scan): the Data tab prepares them (one copy in OPFS, the .mzidx v2, the bar of nativo-fasi.js), they appear in its list too, and Tabui takes the stubs as "Reload from Data" does
    async function loadMaker(fs) {
      if (typeof upload !== "function" || typeof ST === "undefined") return msg("The Data tab is not ready: try again in a moment.", true);
      const before = new Set((ST.files || []).map(f => f.name)), box = $("#tp-load");
      const unmount = typeof PROGRESSO !== "undefined" ? PROGRESSO.mount(box) : () => {};      // the same bar as the Data tab (progresso.js): one model, forward only
      msg(`Preparing ${fs.length} file${fs.length === 1 ? "" : "s"} in the Data tab...`);
      try {
        window.MZLAB_CEN_IDX = true;      // the Data tab builds the centroid index of a profile sample in the same decoding as its own (rust-worker.js lr_index)
        try { await upload(fs, "data"); } finally { window.MZLAB_CEN_IDX = false; }
        const fresh = new Set((ST.files || []).map(f => f.name).filter(n => !before.has(n)));
        const got = (await pageFiles()).filter(x => fresh.has(x.name));
        if (!got.length) return msg("No new sample was added: the files are already in the Data tab, or they cannot be read (see the notice there).", true);
        await addFiles(got.map(forAdd));
      } catch (e) { msg(String(e.message || e), true); }
      finally { unmount(); box.textContent = ""; }
    }
    // default: the Full Scan / HRMS files already loaded in the Data tab (browser memory) are taken at once
    async function loadFromData(quiet) {
      msg("Looking for the files in browser memory...");
      const f = await pageFiles();
      if (!f.length) return msg(quiet ? "" : "No file in the Data tab of this browser: open them there or use “Choose files...”.", !quiet);
      await addFiles(f.map(forAdd), quiet);
    }
    $("#tp-pick").onclick = () => $("#tp-in").click();
    $("#tp-in").onchange = async ev => {
      const fs = [...ev.target.files]; ev.target.value = "";
      const mz = fs.filter(f => /\.mzml$/i.test(f.name)), other = fs.filter(f => !/\.mzml$/i.test(f.name));
      if (mz.length) await addFiles(mz.map(f => ({ name: f.name, file: f })));
      if (other.length) await loadMaker(other);
    };
    // mode: high resolution -> XIC window in ppm (default 5) and RT tolerance from the chromatographic method; unit resolution -> Da and 0.25 min
    let curMode = "";
    function modeUI() {
      const hr = st.files.some(f => f.kind === "hr"), m = !st.files.length ? "" : hr ? "hr" : "lr";
      const t = $("#tp-tol"), r = $("#tp-rtt");
      if (m !== curMode) {
        curMode = m;
        if (m === "hr") {
          $("#tp-tol-l").textContent = "XIC ±ppm"; t.value = 5; t.step = 1; t.min = 1;
          const span = Math.max(0, ...st.files.filter(f => f.kind === "hr").map(f => f.rt_max || 0));
          r.value = span ? Math.max(0.05, Math.round(span * 0.005 / 0.05) * 0.05).toFixed(2) : "0.10";      // 0.5 % of the gradient length (20 min: 0.10)
          r.title = "From the chromatographic method (0.5 % of the run length)";
        } else { $("#tp-tol-l").textContent = "XIC ±Da"; t.value = 0.35; t.step = 0.05; t.min = 0.05; r.value = 0.25; r.title = ""; }
      }
      const n = st.files.length;
      $("#tp-disc").closest(".row").hidden = m === "hr";                // the HR pipeline is already untargeted
    }
    $("#tp-mol").addEventListener("input", modeUI);
    setTimeout(() => loadFromData(true), 0);
    sec.addEventListener("tpshow", () => { if (!st.files.length && !st.noAuto) loadFromData(true); });

    // ---- parent: values, preview, structure
    const FORMULA = /^[A-Za-z0-9()]+$/;
    const parentVals = () => {          // formula field and SMILES field; a SMILES typed in the formula field still counts as a SMILES
      let mol = $("#tp-mol").value.trim(), smi = $("#tp-smi").value.trim();
      if (mol && !FORMULA.test(mol) && !smi) { smi = mol; mol = ""; }
      return { mol, smi };
    };
    let pvT = 0, OCLp = null;
    const ocl = () => (OCLp = OCLp || import(new URL("vendor/openchemlib.js", ctx().base).href));
    const smilesInfo = async t => { try { const m = (await ocl()).Molecule.fromSmiles(t); return { mol: m, formula: MOLUI.hillOf(m.getMolecularFormula().formula) }; } catch (_) { return null; } };
    /** @type {{ smiles: string, delta: string, level: number, note: string } | null} */ let drawnPartial = null;       // set by «Use this structure» for a drawing with a delta or an uncertain position; any edit of the fields drops it
    const preview = () => {
      clearTimeout(pvT);
      pvT = setTimeout(async () => {
        const { mol, smi } = parentVals(), ad = $("#tp-add").value;
        $("#tp-warn").textContent = "";
        FIELD().clear(parentFields());
        if (!mol && !smi) { $("#tp-prev").textContent = ""; structure(""); return; }
        structure(smi);
        let info = null;
        if (smi) info = await smilesInfo(smi);
        let q = mol || (info && info.formula) || smi;       // formula missing: the one computed from the SMILES (OpenChemLib)
        if (!mol && info && !$("#tp-mol").value.trim() && !(drawnPartial && drawnPartial.smiles === smi)) { $("#tp-mol").value = info.formula; }
        try {
          const r = await call("formula_info", q, ad);
          const mzNew = r.ok ? +r.mz : 0;
          if (mzNew !== parentMz) { parentMz = mzNew; if (parentMz) estimateLr(); }
          const pn = drawnPartial && drawnPartial.smiles === smi ? ` · ${esc(drawnPartial.note)}` : "";       // a drawing with a delta or an uncertain position: a candidate, not an identification
          $("#tp-prev").innerHTML = r.ok ? `${esc(r.formula)} · M ${r.neutral} · ${esc(ad)} <i>m/z</i> ${r.mz}${pn}` : `<span class="err">${esc(r.error)}</span>`;
          if (!r.ok) FIELD().set([$(mol ? "#tp-mol" : "#tp-smi")], $("#tp-prev"));
          if (r.ok && mol && info && !pn) $("#tp-warn").textContent = MOLUI.mismatchText(r.formula, info.formula);       // compositions, not strings: the order of the elements is irrelevant
        } catch (e) { $("#tp-prev").textContent = ""; }
      }, 350);
    };
    const reattAuto = (/** @type {string} */ fn) => { const a = /** @type {any} */ (window).TPREATTAUTO; if (a) a[fn](); };       // the automatic reactivity of the parent (lazy/reatt-auto.js), once loaded
    function reattPanel(/** @type {HTMLElement} */ rh, /** @type {string} */ t) {
      const W = /** @type {any} */ (window);
      if (!t) { rh.innerHTML = ""; if (W.TPREATTAUTO) W.TPREATTAUTO.attach(rh, ""); return; }
      if (typeof uipAdminOn !== "function" || !uipAdminOn()) return;      // nothing is loaded or calculated outside the administrator mode
      W.TPLAZY.load("viewer3d.js", "reattivita.js", "reatt-auto.js").then(() => { if (rh.dataset.smiles === t) W.TPREATTAUTO.attach(rh, t); }).catch(() => { rh.textContent = "The reactivity panel could not be loaded."; });
    }
    function structure(t) {             // parent structure drawn with OpenChemLib (already in the site for the Disegno tab)
      const el = $("#tp-struct"); if (!el) return;
      const rh = $("#tp-reatt"); if (rh && rh.dataset.smiles !== t) { rh.dataset.smiles = t; reattPanel(rh, t); }    // panel of reactivity (administrator mode only; its scripts load on the first structure; its calculation is started elsewhere)
      if (!t) { el.innerHTML = ""; return; }
      ocl().then(OCL => { el.innerHTML = `<div class="struct">${MOLUI.svg(OCL.Molecule.fromSmiles(t).toSVG(300, 170, "", MOLUI.SVG_OPTS))}</div>`; }).catch(() => { el.innerHTML = ""; });
    }
    $("#tp-mol").oninput = () => { drawnPartial = null; preview(); }; $("#tp-smi").oninput = () => { drawnPartial = null; preview(); }; $("#tp-add").onchange = preview;

    // ---- name -> formula / SMILES from the curated list data/composti.csv (already loaded with mzFinder; nothing is downloaded here)
    let LIST = null;
    const csvRows = txt => {            // minimal CSV reader (quotes, commas, newlines inside quotes)
      const out = []; let row = [], f = "", q = false;
      for (let i = 0; i < txt.length; i++) {
        const c = txt[i];
        if (q) { if (c === '"') { if (txt[i + 1] === '"') { f += '"'; i++; } else q = false; } else f += c; }
        else if (c === '"') q = true; else if (c === ",") { row.push(f); f = ""; }
        else if (c === "\n") { row.push(f); out.push(row); row = []; f = ""; } else if (c !== "\r") f += c;
      }
      if (f || row.length) { row.push(f); out.push(row); }
      return out;
    };
    const getList = () => LIST || (LIST = csvRows(ctx().files["composti.csv"] || "").slice(1).filter(r => r.length >= 5).map(r => {
      const names = r[0].split(";").map(x => x.trim()).filter(Boolean);
      return { names, low: names.map(x => x.toLowerCase()), formula: r[1], mass: parseFloat(r[2]), smiles: r[3] };
    }));
    const suggest = q => {
      const w = q.trim().toLowerCase(); if (w.length < 2) return [];
      const out = [];
      for (const c of getList()) {
        let best = 9;
        c.low.forEach(n => { const i = n.indexOf(w); if (i < 0) return; const sc = n === w ? 0 : i === 0 ? 1 : /[\s\-(,]/.test(n[i - 1]) ? 2 : 3; if (sc < best) best = sc; });
        if (best < 9) out.push([best, c.names[0].length, c]);
      }
      return out.sort((a, b) => a[0] - b[0] || a[1] - b[1]).slice(0, 8).map(x => x[2]);
    };
    /** @type {any[]} */ let sugItems = [], sugI = -1;
    const sugBox = $("#tp-sug"), nameIn = $("#tp-name");
    const sugClose = () => { sugBox.hidden = true; sugI = -1; nameIn.setAttribute("aria-expanded", "false"); nameIn.removeAttribute("aria-activedescendant"); };
    const pick = i => {
      const it = sugItems[i]; if (!it) return;
      if (it.link) { window.open(it.link, "_blank", "noopener"); sugClose(); return; }
      nameIn.value = MOLUI.cap(it.c.names[0]); nameIn.dataset.raw = it.c.names[0]; $("#tp-mol").value = MOLUI.hillOf(it.c.formula); $("#tp-smi").value = it.c.smiles; sugClose(); preview(); nameIn.focus();
    };
    const sugMark = () => { [...sugBox.children].forEach((li, k) => li.setAttribute("aria-selected", k === sugI ? "true" : "false")); if (sugI >= 0) nameIn.setAttribute("aria-activedescendant", "tp-sug-" + sugI); else nameIn.removeAttribute("aria-activedescendant"); };
    const sugShow = () => {
      const w = nameIn.value.trim(); if (w.length < 2) return sugClose();
      sugItems = suggest(w).map(c => ({ c }));
      sugItems.push({ link: "https://pubchem.ncbi.nlm.nih.gov/#query=" + encodeURIComponent(w), word: w });
      sugBox.innerHTML = sugItems.map((it, k) => it.link ? `<li role="option" id="tp-sug-${k}" data-k="${k}" class="mut">Search PubChem for "${esc(it.word)}"</li>` : `<li role="option" id="tp-sug-${k}" data-k="${k}">${esc(MOLUI.cap(it.c.names[0]))}<span>${esc(it.c.formula)} · ${it.c.mass.toFixed(4)}</span></li>`).join("");
      sugBox.hidden = false; nameIn.setAttribute("aria-expanded", "true"); sugI = -1; sugMark();
    };
    nameIn.addEventListener("input", () => { delete nameIn.dataset.raw; sugShow(); });
    nameIn.addEventListener("keydown", e => {
      if (sugBox.hidden) { if (e.key === "ArrowDown") sugShow(); return; }
      if (e.key === "ArrowDown") { sugI = (sugI + 1) % sugItems.length; sugMark(); e.preventDefault(); }
      else if (e.key === "ArrowUp") { sugI = (sugI - 1 + sugItems.length) % sugItems.length; sugMark(); e.preventDefault(); }
      else if (e.key === "Enter" && sugI >= 0) { pick(sugI); e.preventDefault(); }
      else if (e.key === "Escape") { sugClose(); e.preventDefault(); }
    });
    nameIn.addEventListener("blur", () => setTimeout(sugClose, 150));
    sugBox.addEventListener("mousedown", e => { const li = e.target.closest("li"); if (li) { e.preventDefault(); pick(+li.dataset.k); } });

    // ---- Draw: the Draw tab (Ketcher) with a small bar to bring the SMILES back
    const drawBtn = $("#tp-draw");
    if (window.MZLAB_PHONE) drawBtn.hidden = true;
    let dbar = document.getElementById("tp-drawbar");
    if (!dbar) {
      dbar = document.createElement("div"); dbar.id = "tp-drawbar"; dbar.hidden = true; dbar.setAttribute("role", "region"); dbar.setAttribute("aria-label", "Tabui: parent structure");
      dbar.innerHTML = `<span>Drawing the Tabui parent</span><span id="tp-draw-st" role="status"></span><button type="button" id="tp-draw-use" class="imp" disabled>Use this structure</button><button type="button" id="tp-draw-no">Cancel</button>`;
      document.body.appendChild(dbar);
    }
    let drawing = false;
    const back = () => { drawing = false; dbar.hidden = true; if (typeof setView === "function") setView("tpmine"); };
    const dst = /** @type {HTMLElement} */ (document.getElementById("tp-draw-st")), duse = /** @type {HTMLButtonElement} */ (document.getElementById("tp-draw-use"));
    const dsay = (/** @type {string} */ t) => { dst.textContent = t; };
    drawBtn.onclick = async () => {                                        // the editor loads on demand: «Use» waits for it, and the typed SMILES is shown in it
      drawing = true; dbar.hidden = false; duse.disabled = true; dsay("Loading the editor..."); setView("draw");
      const smi = $("#tp-smi").value.trim();
      try { if (smi) await window.TPDraw.load(smi); else await window.TPDraw.when(); duse.disabled = false; dsay(""); }
      catch (e) { dsay("The editor did not start: " + String((e && e.message) || e)); }
    };
    document.addEventListener("tpview", /** @type {(e: any) => void} */ e => { dbar.hidden = !(drawing && e.detail.view === "draw"); if (e.detail.view !== "draw" && e.detail.view !== "tpmine") drawing = false; });
    /** @type {any} */ (document.getElementById("tp-draw-no")).onclick = back;
    /** @type {any} */ (document.getElementById("tp-draw-use")).onclick = async () => {
      let t = ""; try { t = await window.TPDraw.smiles(); } catch (e) { dsay("Could not read the drawing: " + String((e && e.message) || e)); return; }
      if (!t) { toast("Nothing drawn yet."); return; }
      let p = null; try { p = await window.TPDraw.partial(); } catch (_) { /* plain drawing */ }
      drawnPartial = null; let f = "";
      if (p && (p.delta || p.positionUnknown)) {       // brackets with a delta and/or an attachment to one of several atoms: formula and m/z of the SMILES with the delta
        f = (p.info && p.info.formula) || "";
        drawnPartial = { smiles: t, delta: p.delta || "", level: p.level || 3, note: (p.delta ? `Δ ${p.delta}, ` : "") + `position uncertain, level ${p.level || 3}` };
      }
      $("#tp-smi").value = t; $("#tp-mol").value = f; preview(); back();
    };
    startWorker().then(async () => { $("#tp-tr").value = await callRaw("default_transformations"); if (typeof TPTR !== "undefined") TPTR.mount($("#tp-tr"), callRaw); }).catch(e => msg(String(e.message || e), true));

    // 19_tpmine_results.js must load before this file: the tab can open the moment it registers
    const { render, toast, dl } = /** @type {any} */ (window).TPRESULTS({ st, $, msg });

    // ---- run
    let running = false, cancelling = false;
    /** if the page was unresponsive for 200 ms or more during the Dig, say so with the one-click diagnostics (long tasks with the phase they fell in, diagnosi.js) */
    function lagNote(since) {
      setTimeout(() => {      // the observer reports a little after the task
        const D = typeof DIAG !== "undefined" ? DIAG : null, w = document.getElementById("tp-warn");
        if (!D || !w || D.LONG.da(since).every(x => x.dur < 200)) return;
        const l = D.LONG.da(since).sort((a, b) => b.dur - a.dur)[0];
        w.textContent = `The page was unresponsive for ${l.dur} ms${l.fase ? ` (${l.fase})` : ""}. `;
        const b = document.createElement("button"); b.type = "button"; b.className = "sec"; b.textContent = "Copy diagnostics";
        b.onclick = async () => { try { await navigator.clipboard.writeText(D.testoLento(window.MZLAB_VERSION, since)); b.textContent = "Copied"; } catch (_) { b.textContent = "Copy failed"; } };
        w.appendChild(b);
      }, 400);
    }
    /** «Cancel» of the Dig: the engine stops at its next check (the pool of workers at once); if it has not answered in 4 s (Pyodide, a long step of the .wasm) the worker is terminated and the files must be loaded again */
    function cancelRun() {
      if (!running || cancelling) return;
      cancelling = true; msg("Cancelling...");
      send({ type: "cancel" }).catch(() => {});
      setTimeout(() => {
        if (!running || !killWorker()) return;
        st.files = []; renderFiles();
        startWorker().catch(() => {});
      }, 4000);
    }
    const setBusy = b => { const g = $("#tp-go"); g.disabled = b; g.setAttribute("aria-busy", b ? "true" : "false"); $("#tp-go-t").textContent = b ? "Digging..." : "Dig"; };
    $("#tp-unt").onclick = () => { if (window.UNTUI) window.UNTUI.open(); };
    $("#tp-go").onclick = async () => {
      const pv = parentVals(), mol = pv.mol || pv.smi;      // the engine takes the formula, or the SMILES when the formula is missing
      const hrMode = st.files.some(f => f.kind === "hr");
      if (!mol) { msg("Enter a formula or SMILES.", true); $("#tp-msg").dataset.field = "1"; FIELD().set(parentFields(), $("#tp-msg")); $("#tp-mol").focus(); return; }
      if (!hrMode && !st.files.some(f => f.kind === "full")) return msg("Full-scan (MS1) sample files are needed.", true);
      const have = new Set(st.files.map(f => String(f.name || "").split(/[\\/]/).pop()));            // files loaded but not in this series: failed to read, or still being opened
      const lost = [...new Set([...(window.QQQ_LOADFAIL || []), ...(((typeof E !== "undefined" && E.pending) || []).filter(x => x.state === "err" || x.state === "wait").map(x => x.file))])].filter(n => !have.has(n));
      $("#tp-warn").textContent = lost.length ? `Warning: ${lost.length} file${lost.length > 1 ? "s" : ""} not read and left out of this run (${lost.slice(0, 4).join(", ")}${lost.length > 4 ? ", ..." : ""}). Load ${lost.length > 1 ? "them" : "it"} again before trusting the series.` : "";
      setBusy(true); reattAuto("tpsStart");      // the reactivity of the parent waits for the TPs
      $("#tp-bar").style.width = "2%"; msg(useCond() && !hrMode ? `Aligning ${st.files.filter(f => f.kind === "full").length} files of ${st.cond.conditions.length} conditions...` : "Digging..."); const t0 = performance.now();
      const tStart = performance.now(); if (typeof DIAG !== "undefined") DIAG.LONG.marca("start");
      const prevHtml = $("#tp-right").innerHTML; cancelling = false; running = true;      // what «Cancel» goes back to when no result is held
      if (window.TPLIVE) TPLIVE.start($("#tp-right"), hrMode, { esc, call, cancel: cancelRun });
      try {
        const files = st.files.filter(f => f.kind !== "error" && f.kind !== "empty").map(f => ({ name: f.name, type: f.type, time: f.time, kind: f.kind }));
        const settings = { tol_da: +$("#tp-tol").value, rt_tol_min: +$("#tp-rtt").value, max_steps: +$("#tp-steps").value, rt_min: +$("#tp-rtmin").value, min_files: +$("#tp-minf").value, discover: $("#tp-disc").checked, ms2_overrides: window.TPMS2 ? window.TPMS2.start(st.files) : [], ...(useCond() && !hrMode ? { gruppi: digGroups() } : {}) };
        const parent = { name: (nameIn.dataset.raw && nameIn.value === MOLUI.cap(nameIn.dataset.raw) ? nameIn.dataset.raw : $("#tp-name").value.trim()), neutral: mol, adduct: $("#tp-add").value };
        if (pv.smi) parent.smiles = pv.smi;
        if ($("#tp-prt").value !== "" && +$("#tp-prt").value >= 0) parent.rt = +$("#tp-prt").value;
        if (hrMode) {          // high resolution: its own engine and view (21_tpmine_hr.js)
          const hs = JSON.parse(await callRaw("run", JSON.stringify(files), JSON.stringify(parent), JSON.stringify({ ppm_prec: +$("#tp-tol").value, rt_tol_min: +$("#tp-rtt").value }), $("#tp-tr").value));
          st.took = performance.now() - t0; st.summary = null; st.det = {};
          if (window.TPLIVE) TPLIVE.stop(true);
          if (typeof DIAG !== "undefined") DIAG.LONG.marca("render");
          st.hs = hs; window.TPHR.render($("#tp-right"), hs, { esc, fmtA, chart, hue, call, callRaw, toast, dl });
          $("#tp-bar").style.width = "100%"; msg("Done."); $("#tp-right").scrollIntoView({ behavior: "smooth", block: "start" });
          running = false; setBusy(false); lagNote(tStart); reattAuto("tpsDone"); return;
        }
        st.hs = null; st.pname = parent.name || ""; st.summary = JSON.parse(await callRaw("run", digFiles(files), JSON.stringify(parent), JSON.stringify(settings), $("#tp-tr").value));
        if (window.TPMS2) window.TPMS2.setConditions(st.summary.gruppi ? new Map(st.files.filter(f => f.kind === "ms2").map(f => [f.name, (st.cond && st.cond.campioni[st.gnames.indexOf(f.name)] || {}).serie || ""])) : null);
        st.took = performance.now() - t0; st.sel = null; st.det = {}; if (window.TPLIVE) TPLIVE.stop(true); if (typeof DIAG !== "undefined") DIAG.LONG.marca("render"); render(); $("#tp-bar").style.width = "100%"; msg("Done."); $("#tp-right").scrollIntoView({ behavior: "smooth", block: "start" });
        running = false; setBusy(false); reattAuto("tpsDone");
      } catch (e) {
        reattAuto("tpsEnd");
        if (window.TPLIVE) TPLIVE.stop(false, cancelling ? null : String(e.message || e));      // a failed Dig keeps its panel with the error and «Copy diagnosis»
        if (cancelling) {        // cancelled by the user: the panel goes back to the previous results (or the invitation) and the files stay
          if (st.summary) render(); else if (st.hs) window.TPHR.render($("#tp-right"), st.hs, { esc, fmtA, chart, hue, call, callRaw, toast, dl }); else $("#tp-right").innerHTML = prevHtml;
          msg("Cancelled."); $("#tp-bar").style.width = "0"; $("#tp-go").focus();
        } else { msg(String(e.message || e), true); $("#tp-bar").style.width = "0"; }
      }
      running = false; cancelling = false; setBusy(false); lagNote(tStart);
    };

  }

  QTOOLS.register({ id: "tpmine", nav: true, name: "Tabui", desc: "Automatic search for transformation products", open,
    icon: (typeof QICON !== "undefined" && QICON.mine) ? QICON.get("mine", 16) : "⛏" });
})();