// @ts-check
// TPMINE-PRIVATE  «Attributio» (admin mode only): the table of levels of identification of a campaign folder, computed by the .wasm `attributio` (crates/mzlab-attributio-wasm) in a worker
// of mzFinder's pool, or by the native route `feat/attributio/annotate` in the desktop app. The folder is chosen with showDirectoryPicker (or an <input webkitdirectory>); the files stay in
// the browser and only the ones `campaign.toml` names are read. Texts in English, without keys of lang/ (the exception of mzFinder). The entry is made by ONE `if`: opening it to everybody is deleting that line.
(() => {
  "use strict";
  /** @type {{ pool: any, files: {path: string, file: File}[], root: string, data: any, summary: any, busy: boolean }} */
  const st = { pool: null, files: [], root: "", data: null, summary: null, busy: false };
  const MAX_FILES = 20000;

  /** the pool that runs the engine: the native routes in the app, else the worker of mzFinder with only the .wasm `attributio` */
  async function getPool() {
    if (st.pool) return st.pool;
    const c = /** @type {any} */ (window).QTOOLS.ctx, blob = (/** @type {string} */ n) => URL.createObjectURL(new Blob([c.files[n]], { type: "text/javascript" }));
    if (/** @type {any} */ (window).MZLAB_DESKTOP) st.pool = (await import(blob("feat-desktop-pool.js"))).createDesktopPool({});
    else {
      const u = (/** @type {string} */ p) => new URL("wasm-feat/" + p, c.base).href, man = await (await fetch(u("manifest.json"))).json();
      if (!man.attributio) throw new Error("This site has no engine for Attributio");
      const url = blob("feat-worker.js"), w = man.attributio;
      st.pool = (await import(url)).createFeatPool({ workerUrl: url, size: 1, wasm: { attributio: { jsUrl: u(w.js), wasmUrl: u(w.wasm), sha256: w.sha256 } } });
    }
    return st.pool;
  }

  /** every file under a directory handle, as {path relative to the folder, File} @param {any} dir @param {string} base @param {{path: string, file: File}[]} out */
  async function walk(dir, base, out) {
    for await (const [name, h] of dir.entries()) {
      if (out.length >= MAX_FILES || name === ".git") continue;
      if (h.kind === "directory") await walk(h, base + name + "/", out);
      else out.push({ path: base + name, file: await h.getFile() });
    }
  }

  /** @param {HTMLElement} box @param {string} text @param {boolean} [err] */
  function say(box, text, err) { const e = /** @type {HTMLElement} */ (box.querySelector("#at-msg")); e.textContent = text; e.className = err ? "err" : "mut"; }

  /** the engine on the chosen folder; kind json draws the table, csv and xlsx give the export @param {string} kind */
  async function run(kind) {
    const pool = await getPool(), r = await pool.attributioAnnotate(st.files, st.root, kind);
    return r;
  }

  /** @param {string} name @param {BlobPart} data @param {string} type */
  function download(name, data, type) {
    const a = document.createElement("a"); a.href = URL.createObjectURL(new Blob([data], { type })); a.download = name; a.click(); setTimeout(() => URL.revokeObjectURL(a.href), 2000);
  }

  /** the result and its fingerprint go to the notebook and the .mzflow, without data of the campaign @param {any} s */
  function log(s) {
    const W = /** @type {any} */ (window);
    if (W.WORKFLOW && W.WORKFLOW.logOp) W.WORKFLOW.logOp("attributio", { attributio: { fingerprint: s.fingerprint, settings: s.settings, counts: s.by_level } });
  }

  /** @param {HTMLElement} box @param {{path: string, file: File}[]} files @param {string} root */
  async function load(box, files, root) {
    if (st.busy) return;
    st.busy = true; st.files = files; st.root = root;
    const tab = /** @type {HTMLElement} */ (box.querySelector("#at-out"));
    tab.textContent = "";
    for (const b of box.querySelectorAll("button")) b.disabled = true;
    say(box, `Reading ${root} (${files.length} files)…`);
    try {
      const r = await run("json");
      st.data = JSON.parse(new TextDecoder().decode(r.bytes)); st.summary = r.summary;
      /** @type {any} */ (window).ATTRIBUTIO_TABLE.mount(tab, st.data);
      say(box, (r.summary.warnings || []).join(" · ") || "Done.");
      for (const b of box.querySelectorAll(".at-x")) b.disabled = false;
      log(r.summary);
    } catch (e) {
      const x = /** @type {any} */ (e);
      say(box, "Attributio: " + ((x && x.params && x.params.detail) || (x && x.message) || String(e)), true);
    } finally {
      st.busy = false;
      for (const b of box.querySelectorAll(".at-pick")) b.disabled = false;
    }
  }

  /** @param {HTMLElement} box @param {"csv" | "xlsx"} kind */
  async function exportTable(box, kind) {
    try {
      const r = await run(kind), stem = (st.root || "campaign") + "_attributio";
      if (kind === "csv") download(stem + ".csv", r.bytes, "text/csv;charset=utf-8");
      else {
        const sheet = JSON.parse(new TextDecoder().decode(r.bytes)).hojas[0];
        await /** @type {any} */ (window).dlx(stem + ".xlsx", [{ name: sheet.nombre, head: sheet.filas[0], rows: sheet.filas.slice(1) }], false);
      }
    } catch (e) { say(box, "Export failed: " + (/** @type {any} */ (e).message || e), true); }
  }

  /** @param {HTMLElement} sec */
  function open(sec) {
    if (!document.getElementById("at-css")) {
      const s = document.createElement("style"); s.id = "at-css";
      s.textContent = ".at-wrap{padding:14px;max-width:1400px}.at-scroll{overflow:auto;max-height:70vh}#at-table{border-collapse:collapse;font-size:12px}#at-table th,#at-table td{padding:3px 8px;text-align:left;border-bottom:1px solid var(--line)}"
        + "#at-table th button{background:none;border:0;font:inherit;font-weight:700;color:var(--ink);cursor:pointer;min-height:24px;min-width:24px}.at-why{max-width:60ch;white-space:pre-wrap}.at-wrap .err{color:var(--bad)}";
      document.head.appendChild(s);
    }
    sec.innerHTML = `<div class="at-wrap"><h2>Attributio</h2>
      <div class="row"><button type="button" class="at-pick" id="at-pick" aria-label="Choose campaign folder">Choose campaign folder</button>
      <input type="file" id="at-dir" webkitdirectory multiple hidden aria-label="Campaign folder">
      <button type="button" class="at-x" id="at-csv" aria-label="Export CSV" disabled>Export CSV</button><button type="button" class="at-x" id="at-xlsx" aria-label="Export XLSX" disabled>Export XLSX</button></div>
      <p id="at-msg" class="mut" role="status"></p><div id="at-out"></div></div>`;
    const q = (/** @type {string} */ s) => /** @type {any} */ (sec.querySelector(s));
    q("#at-dir").onchange = (/** @type {Event} */ ev) => {
      const list = /** @type {File[]} */ ([...(/** @type {HTMLInputElement} */ (ev.target).files || [])]).slice(0, MAX_FILES);
      if (!list.length) return;
      const root = (list[0].webkitRelativePath || "").split("/")[0] || "campaign";
      load(sec, list.map(f => ({ path: (f.webkitRelativePath || f.name).split("/").slice(1).join("/") || f.name, file: f })), root);
    };
    q("#at-pick").onclick = async () => {
      const W = /** @type {any} */ (window);
      if (typeof W.showDirectoryPicker !== "function") { q("#at-dir").click(); return; }
      try { const dir = await W.showDirectoryPicker(), files = /** @type {{path: string, file: File}[]} */ ([]); await walk(dir, "", files); load(sec, files, dir.name); }
      catch (e) { if (!(e && /** @type {any} */ (e).name === "AbortError")) say(sec, String(/** @type {any} */ (e).message || e), true); }
    };
    q("#at-csv").onclick = () => exportTable(sec, "csv");
    q("#at-xlsx").onclick = () => exportTable(sec, "xlsx");
  }

  if (typeof uipAdminOn === "function" && uipAdminOn()) /** @type {any} */ (window).QTOOLS.register({ id: "attributio", nav: true, admin: true, name: "Attributio", desc: "Levels of identification of a campaign folder", open });
})();
