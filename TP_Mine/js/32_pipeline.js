// @ts-strict
// TPMINE-PRIVATE  Panel «Pipeline» of mzFinder (window.PIPE): the six numbers of the feature pipeline with the proof of every estimated value, the preview on one file (number of
// features, histogram of the heights, three XIC with their edges) and the gate of quality (data/cancello.json). The engine is crates/mzlab-feat-wasm run by feat-worker.js (the .wasm are
// fetched only when the panel is opened); the .mzidx files are the ones in OPFS (mzlab/idx/). Texts follow the convention of mzFinder (English, no lang/ keys, except the messages of the worker).
(() => {
  "use strict";
  const esc = s => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const sci = x => (x >= 1e4 || (x > 0 && x < 0.01) ? Number(x).toExponential(1).replace("e+", "e") : String(+Number(x).toPrecision(3)));
  const DEF = { ppm: 5, floor: 2e4, min_height: 2e5, min_points: 5, rt_tol: 0.1, fill: true };
  /** @type {any} */ const st = { pool: null, crate: "", gate: null, files: [], prev: null, auto: { floor: true, ppm: false }, proofs: null, busy: false };
  let dlg = null;
  const hooks = [];                       // called after the list of files was read again (the section «Run» refreshes its treatment times)
  const $ = q => dlg.querySelector(q);
  const say = (t, err) => { const e = $("#pipe-msg"); if (e) { e.textContent = t || ""; e.className = err ? "err" : "mut"; } };
  const CSS = `#pipe{width:min(1100px,97vw);max-width:97vw;max-height:94vh;padding:12px 14px;border:1px solid var(--line);border-radius:8px;background:var(--panel);color:var(--ink)}
#pipe::backdrop{background:rgba(0,0,0,.35)}#pipe h3,#pipe h4{margin:0}#pipe .top{display:flex;align-items:center;gap:8px}#pipe .top h3{flex:1}#pipe .mut{color:var(--muted);font-size:12px}#pipe .err{color:var(--bad);font-size:12px}
#pipe .row{display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin:6px 0}#pipe .gate{padding:6px 10px;border:1px solid var(--line);border-left:3px solid var(--warn);border-radius:6px;margin:6px 0}#pipe .gate.ok{border-left-color:var(--accent)}
#pipe .par{display:grid;grid-template-columns:repeat(auto-fill,minmax(250px,1fr));gap:8px;margin:8px 0}#pipe .par label{display:flex;flex-direction:column;gap:2px;font-size:12px}#pipe .par input,#pipe .par select{min-height:28px;width:100%;box-sizing:border-box}
#pipe .par .why{color:var(--muted);font-size:11px}#pipe button{min-height:28px;min-width:28px}#pipe details{margin:6px 0}#pipe summary{cursor:pointer;min-height:28px;line-height:28px}
#pipe .charts{display:grid;grid-template-columns:repeat(auto-fill,minmax(300px,1fr));gap:12px;margin-top:8px}#pipe .ch{border:1px solid var(--line);border-radius:6px;padding:6px}#pipe svg{width:100%;height:auto;display:block}
#pipe svg .ax{stroke:var(--line);fill:none}#pipe svg .bar{fill:var(--accent)}#pipe svg .mk{stroke:var(--warn);stroke-width:2;stroke-dasharray:5 3}#pipe svg .ln{stroke:var(--accent);fill:none;stroke-width:1.5}
#pipe svg .eg{stroke:var(--ink);stroke-dasharray:2 3}#pipe svg text{fill:var(--muted);font-size:10px}#pipe .proof{margin:4px 0;font-size:12px}`;

  /** state of the gate of quality: candidates are shown only with `vista_candidati: true` measured on the crate version of the .wasm loaded (cancello/2, cancello/1 also read); `table` is the same test for the table view (`vista_tabella`; a cancello/1 follows `vista_candidati`) */
  function gateState(c, crate) {
    if (!c) return { ok: false, table: false, why: "absent" };
    if (c.format !== "cancello/1" && c.format !== "cancello/2") return { ok: false, table: false, why: "format" };
    const same = String(c.crate_version) === String(crate), cand = c.vista_candidati === true, tab = (c.format === "cancello/2" ? c.vista_tabella : c.vista_candidati) === true;
    const table = tab && same;
    if (!cand) return { ok: false, table, why: "closed", c };
    if (!same) return { ok: false, table, why: "stale", c };
    return { ok: true, table, why: "open", c };
  }
  const gateText = g => g.ok ? `Pipeline validated on ${esc(g.c.data || "?")} (version ${esc(g.c.crate_version)}): the candidate view is available.`
    : `Pipeline not validated${g.why === "stale" ? " for this version of the engine" : ""}: only traces and families are shown, no candidates.`;
  function readGate(files) {
    const b = files && files["cancello.json"];
    if (!b) return null;
    try { return JSON.parse(typeof b === "string" ? b : new TextDecoder().decode(b)); } catch (_) { return { format: "?" }; }
  }

  /** the .mzidx files in OPFS (mzlab/idx/), by name */
  async function listIdx() {
    try {
      let d = await navigator.storage.getDirectory();
      for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p);
      const out = [];
      for await (const [name, h] of /** @type {any} */ (d).entries()) if (h.kind === "file" && name.endsWith(".mzidx")) out.push(name);
      return out.sort();
    } catch (_) { return []; }
  }

  /** the pool of workers, created on the first use; the .wasm come from static/wasm-feat/manifest.json */
  async function getPool() {
    if (st.pool) return st.pool;
    const c = window.QTOOLS.ctx, u = p => new URL("wasm-feat/" + p, c.base).href;
    const man = await (await fetch(u("manifest.json"))).json();
    const url = URL.createObjectURL(new Blob([c.files["feat-worker.js"]], { type: "text/javascript" }));
    const mod = await import(url);
    const w = m => ({ jsUrl: u(m.js), wasmUrl: u(m.wasm), sha256: m.sha256 });
    st.pool = mod.createFeatPool({ workerUrl: url, size: 1, wasm: { feat: w(man.feat), proofs: w(man.proofs), ...(man.table ? { table: w(man.table) } : {}) }, notify: (k, p) => say(msg(k, p)) });
    st.crate = await st.pool.crateVersion();
    return st.pool;
  }
  const msg = (key, params) => { try { const t = window.I18N && I18N.t(key, params || {}); return t && t !== key ? t : key; } catch (_) { return key; } };

  /** puts the parameters of a recipe (the `parametri` of a record) into the form; a floor that is not in the recipe stays the estimated one */
  function apply(p) {
    const set = (id, v) => { if (v != null) $(id).value = v; };
    set("#pipe-ppm", p.ppm); set("#pipe-hmin", p.min_height); set("#pipe-npts", p.min_points); set("#pipe-rt", p.rt_tol); set("#pipe-pol", p.polarity); set("#pipe-mode", p.mode);
    if (typeof p.fill === "boolean") $("#pipe-fill").checked = p.fill;
    st.auto.floor = p.floor == null; if (p.floor != null) { $("#pipe-floor").value = p.floor; $("#pipe-floor-why").textContent = "chosen"; }
  }
  /** values of the six parameters (+ Advanced) from the form; `floor` is left out while it is the estimated one */
  function values() {
    const n = id => +$(id).value, v = { polarity: n("#pipe-pol"), mode: $("#pipe-mode").value, ppm: n("#pipe-ppm"), min_height: n("#pipe-hmin"), min_points: n("#pipe-npts"), rt_tol: n("#pipe-rt"), fill: $("#pipe-fill").checked };
    v.floor = n("#pipe-floor");
    return v;
  }
  const recipe = v => { const o = { polarity: v.polarity, mode: v.mode, ppm: v.ppm, min_height: v.min_height, min_points: v.min_points, rt_tol: v.rt_tol, fill: v.fill };
    if (!st.auto.floor && v.floor > 0) o.floor = v.floor; return JSON.stringify(o); };

  // ------------------------------------------------------------ charts (SVG, the data in a table through TDATI)
  const W = 320, H = 150, M = { l: 34, r: 8, t: 8, b: 22 };
  function chart(box, svg, title, desc, table) {
    box.innerHTML = `<h4>${esc(title)}</h4>${svg}`;
    const el = box.querySelector("svg"); el.setAttribute("role", "img"); el.setAttribute("aria-label", title);
    if (window.TDATI) { TDATI.describe(el, desc); TDATI.addButton(el, () => ({ title, head: table.head, rows: table.rows })); }
  }
  /** histogram of log10 values: bars of the counts (square root), a dashed mark at `mark` (an intensity) */
  function histSvg(h, mark) {
    const c = h.counts, first = Math.max(0, c.findIndex(x => x > 0)), last = Math.max(first, c.length - 1 - [...c].reverse().findIndex(x => x > 0));
    const n = last - first + 1, top = Math.max(1, ...c.slice(first, last + 1).map(Math.sqrt)), bw = (W - M.l - M.r) / n;
    let s = `<svg viewBox="0 0 ${W} ${H}" preserveAspectRatio="xMidYMid meet"><path class="ax" d="M${M.l},${M.t}V${H - M.b}H${W - M.r}"/>`;
    for (let i = first; i <= last; i++) { const bh = (H - M.t - M.b) * Math.sqrt(c[i]) / top; s += `<rect class="bar" x="${(M.l + (i - first) * bw).toFixed(1)}" y="${(H - M.b - bh).toFixed(1)}" width="${Math.max(1, bw - 0.5).toFixed(1)}" height="${bh.toFixed(1)}"/>`; }
    const x = lg => M.l + ((lg - h.lo) / h.step - first) * bw;
    if (mark > 0) s += `<line class="mk" x1="${x(Math.log10(mark)).toFixed(1)}" x2="${x(Math.log10(mark)).toFixed(1)}" y1="${M.t}" y2="${H - M.b}"/>`;
    for (let i = first; i <= last; i++) { const dec = h.lo + i * h.step; if (Math.abs(dec - Math.round(dec)) < 1e-9 && Math.round(dec) % 2 === 0) s += `<text x="${(M.l + (i - first) * bw).toFixed(1)}" y="${H - 8}" text-anchor="middle">1e${Math.round(dec)}</text>`; }
    return s + `<text x="4" y="${M.t + 8}">√n</text></svg>`;
  }
  const histRows = h => h.counts.map((n, i) => [sci(Math.pow(10, h.lo + i * h.step)), String(n)]).filter(r => r[1] !== "0");
  /** XIC of an example with the edges of its feature */
  function xicSvg(e) {
    const x0 = e.x[0], x1 = e.x[e.x.length - 1] || x0 + 1, ym = Math.max(1, ...e.y);
    const X = t => M.l + (t - x0) / (x1 - x0 || 1) * (W - M.l - M.r), Y = v => H - M.b - v / ym * (H - M.t - M.b);
    const d = e.x.map((t, i) => (i ? "L" : "M") + X(t).toFixed(1) + "," + Y(e.y[i]).toFixed(1)).join("");
    return `<svg viewBox="0 0 ${W} ${H}"><path class="ax" d="M${M.l},${M.t}V${H - M.b}H${W - M.r}"/><path class="ln" d="${d}"/>`
      + [e.left, e.right].map(t => `<line class="eg" x1="${X(t).toFixed(1)}" x2="${X(t).toFixed(1)}" y1="${M.t}" y2="${H - M.b}"/>`).join("")
      + `<text x="${M.l}" y="${H - 8}">${esc(x0.toFixed(2))}</text><text x="${W - M.r}" y="${H - 8}" text-anchor="end">${esc(x1.toFixed(2))} min</text><text x="4" y="${M.t + 8}">${esc(sci(ym))}</text></svg>`;
  }

  // ------------------------------------------------------------ preview
  function showProofs(p, f) {
    const ns = p.noise, pp = p.ppm, fw = p.fwhm;
    $("#pipe-proof").innerHTML = (ns ? `<p class="proof"><b>Noise floor ${esc(sci(ns.floor))}</b>: the noise is the lump of the histogram centred on ${esc(sci(ns.mode))} (width σ = ${esc(ns.sigma.toFixed(2))} decades); the floor is 3σ above it (dashed line).</p>` : `<p class="proof">No noise lump in the histogram: no floor can be estimated, the default is kept.</p>`)
      + (pp.value != null ? `<p class="proof"><b>Mass tolerance ${esc(pp.value.toFixed(1))} ppm</b>: three robust sigma (σ = ${esc(pp.sigma.toFixed(2))} ppm) of the m/z scatter of ${pp.n} points of the strongest traces.</p>` : `<p class="proof">Mass tolerance: too few strong traces (${pp.n} points) to estimate it.</p>`)
      + (fw.value != null ? `<p class="proof"><b>Median peak width ${esc(fw.value.toFixed(3))} min</b> (FWHM of ${fw.n} strong traces).</p>` : `<p class="proof">Peak width: too few strong traces (${fw.n}).</p>`);
    st.proofs = { noise: ns, ppm: pp, fwhm: fw, file: f, floor_chosen: values().floor };
  }
  function render(p) {
    const f = $("#pipe-file").value, hist = $("#pipe-hist"), ex = $("#pipe-ex");
    $("#pipe-n").textContent = `${p.n.toLocaleString("en")} features in ${f} (${(p.ms / 1000).toFixed(1)} s)`;
    showProofs(p, f);
    const mark = st.auto.floor ? (p.noise ? p.noise.floor : 0) : values().floor;
    chart(hist.querySelector(".a"), histSvg(p.intensities, mark), "Intensities of the peaks", `Histogram of the intensities of all the peaks (square root of the counts). The dashed line is the noise floor ${p.noise ? sci(p.noise.floor) : "(none)"}.`,
      { head: ["Intensity (from)", "Peaks"], rows: histRows(p.intensities) });
    chart(hist.querySelector(".b"), histSvg(p.heights, values().min_height), "Heights of the features", `Histogram of the heights of the ${p.n} features (square root of the counts). The dashed line is the minimum height.`,
      { head: ["Height (from)", "Features"], rows: histRows(p.heights) });
    ex.innerHTML = p.examples.map((e, i) => `<div class="ch" id="pipe-x${i}"></div>`).join("");
    p.examples.forEach((e, i) => chart(ex.querySelector("#pipe-x" + i), xicSvg(e), `XIC m/z ${e.mz.toFixed(4)}, RT ${e.rt.toFixed(2)} min`,
      `Ion trace of m/z ${e.mz.toFixed(4)}: height ${sci(e.height)}, apex at ${e.rt.toFixed(2)} min, edges at ${e.left.toFixed(2)} and ${e.right.toFixed(2)} min (dashed lines).`,
      { head: ["RT (min)", "Intensity"], rows: e.x.map((t, k) => [t.toFixed(3), String(Math.round(e.y[k]))]).slice(0, 400) }));
  }
  function suggest(p) {
    if (st.auto.floor && p.noise) $("#pipe-floor").value = sci(p.noise.floor);
    if (!st.auto.ppm && p.ppm.value != null) $("#pipe-ppm").value = p.ppm.value.toFixed(1);
    $("#pipe-floor-why").textContent = st.auto.floor ? (p.noise ? "estimated from the histogram (see below)" : "default") : "chosen";
  }
  async function preview() {
    if (st.busy) return;
    const name = $("#pipe-file").value;
    if (!name) return say("No .mzidx file: open an mzML in Data first.", true);
    st.busy = true; say("Reading the file...");
    try {
      const pool = await getPool(), p = await pool.preview({ name, path: "mzlab/idx/" + name }, recipe(values()));
      st.prev = p; suggest(p); render(p); say("");
    } catch (e) { say(msg(e.key || "error.featFailed", e.params) + (e.key ? "" : " " + (e.message || "")), true); }
    st.busy = false;
  }

  // ------------------------------------------------------------ panel
  const field = (id, label, val, extra = "", why = "") => `<label>${label}<input id="pipe-${id}" type="number" value="${esc(val)}" step="any" ${extra}><span class="why" id="pipe-${id}-why">${why}</span></label>`;
  function build() {
    dlg = document.createElement("dialog"); dlg.id = "pipe"; dlg.setAttribute("aria-label", "Pipeline"); document.body.appendChild(dlg);
    dlg.innerHTML = `<div class="top"><h3>Pipeline</h3><button type="button" id="pipe-x" aria-label="Close">&times;</button></div>
      <div id="pipe-gate" class="gate" role="status"></div>
      <div class="row"><label>File <select id="pipe-file" aria-label="Index file"></select></label><button type="button" id="pipe-ref" aria-label="Refresh the list of files">&#8635;</button>
        <button type="button" id="pipe-go" class="imp">Preview</button><span id="pipe-n" class="mut"></span></div>
      <div id="pipe-msg" class="mut" role="status"></div>
      <div class="par">${field("ppm", "Mass tolerance (ppm)", DEF.ppm, 'min="0.5"')}${field("floor", "Noise floor (intensity)", DEF.floor, 'min="1"', "default")}
        ${field("hmin", "Minimum height", DEF.min_height, 'min="1"')}${field("npts", "Minimum points", DEF.min_points, 'min="2" step="1"')}${field("rt", "RT tolerance of the alignment (min)", DEF.rt_tol, 'min="0.005"')}
        <label>Fill the gaps<input id="pipe-fill" type="checkbox" checked style="width:24px;height:24px"></label></div>
      <details><summary>Advanced</summary><div class="par"><label>Polarity<select id="pipe-pol"><option value="1">positive</option><option value="-1">negative</option></select></label>
        <label>Rows<select id="pipe-mode"><option value="family">one per family of ions</option><option value="ion">one per ion</option></select></label></div></details>
      <div id="pipe-proof"></div><div class="charts" id="pipe-hist"><div class="ch a"></div><div class="ch b"></div></div><div class="charts" id="pipe-ex"></div><div id="pipe-run"></div>`;
    $("#pipe-x").onclick = () => dlg.close();
    $("#pipe-go").onclick = preview; $("#pipe-ref").onclick = refresh;
    $("#pipe-floor").oninput = () => { st.auto.floor = false; $("#pipe-floor-why").textContent = "chosen"; };
    $("#pipe-ppm").oninput = () => { st.auto.ppm = true; };
    if (window.PIPER) window.PIPER.mount($("#pipe-run"), { q: s => dlg.querySelector(s), listIdx, onRefresh: fn => hooks.push(fn), gate: () => st.gate || { ok: false }, pool: getPool, recipe: () => recipe(values()), values, msg, chart, proofs: () => st.proofs, crate: () => st.crate, apply });
  }
  async function refresh() {
    st.files = await listIdx();
    hooks.forEach(fn => fn());
    $("#pipe-file").innerHTML = st.files.map(n => `<option>${esc(n)}</option>`).join("");
    if (!st.files.length) say("No .mzidx file: open an mzML in Data first, its index is made there.");
  }
  async function open() {
    if (!document.getElementById("pipe-css")) { const s = document.createElement("style"); s.id = "pipe-css"; s.textContent = CSS + (/** @type {any} */ (window).PIPEV ? /** @type {any} */ (window).PIPEV.CSS : "") + (/** @type {any} */ (window).PIPEQC ? /** @type {any} */ (window).PIPEQC.css : "") + (/** @type {any} */ (window).PIPET ? /** @type {any} */ (window).PIPET.css : "") + (/** @type {any} */ (window).PIPEX ? /** @type {any} */ (window).PIPEX.css : ""); document.head.appendChild(s); }
    if (!dlg) build();
    if (!dlg.open) dlg.showModal();
    st.gate = gateState(readGate(window.QTOOLS.ctx.files), "");
    $("#pipe-gate").innerHTML = gateText(st.gate); $("#pipe-gate").className = "gate";
    await refresh();
    try {
      await getPool();
      st.gate = gateState(readGate(window.QTOOLS.ctx.files), st.crate);
      $("#pipe-gate").innerHTML = gateText(st.gate); $("#pipe-gate").className = "gate" + (st.gate.ok ? " ok" : "");
    } catch (e) { say(msg(e.key || "error.featFailed", e.params), true); }
  }

  window.PIPE = { open, state: st, gateState, gateText, recipe, values, DEF };
})();
