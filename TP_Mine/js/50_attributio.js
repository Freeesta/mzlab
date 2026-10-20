// @ts-check
// TPMINE-PRIVATE  «Attributio» engine (admin mode only): the pool that runs the .wasm `attributio` (crates/mzlab-attributio-wasm) in a worker of mzFinder, or the native routes
// `feat/attributio/*` in the desktop app, plus the helpers to read a folder and save a file. There is no panel of its own: Flow (flow/lazy/flow-campaign.js) opens the campaign folder
// through ATTRIBUTIO_CORE. Texts in English, without keys of lang/ (the exception of mzFinder).
(() => {
  "use strict";
  /** @type {{ pool: any }} */
  const st = { pool: null };
  const MAX_FILES = 20000;

  /** the pool that runs the engine: the native routes in the app, else the worker of mzFinder with only the .wasm `attributio` */
  async function getPool() {
    if (st.pool) return st.pool;
    const c = /** @type {any} */ (window).QTOOLS.ctx, blob = (/** @type {string} */ n) => URL.createObjectURL(new Blob([c.files[n]], { type: "text/javascript" }));
    if (/** @type {any} */ (window).MZLAB_DESKTOP) st.pool = (await import(blob("feat-desktop-pool.js"))).createDesktopPool({});
    else {
      const u = (/** @type {string} */ p) => new URL("wasm-feat/" + p, c.base).href, man = await (await fetch(u("manifest.json"))).json();
      if (!man.attributio) throw new Error("This site has no engine for Attributio");
      const url = blob("feat-worker.js"), w = man.attributio, r = man.attributio_rev, x = man.attributio_export, y = man.attributio_ms2;
      // the .wasm `attributio_rev` (the Banco 2b) is fetched by the worker only when the Banco opens
      st.pool = (await import(url)).createFeatPool({ workerUrl: url, size: 1, wasm: { attributio: { jsUrl: u(w.js), wasmUrl: u(w.wasm), sha256: w.sha256 }, ...(r ? { attributio_rev: { jsUrl: u(r.js), wasmUrl: u(r.wasm), sha256: r.sha256, classesUrl: r.classes ? u(r.classes) : "" } } : {}), ...(x ? { attributio_export: { jsUrl: u(x.js), wasmUrl: u(x.wasm), sha256: x.sha256 } } : {}), ...(y ? { attributio_ms2: { jsUrl: u(y.js), wasmUrl: u(y.wasm), sha256: y.sha256 } } : {}) } });
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

  /** @param {string} name @param {BlobPart} data @param {string} type */
  function download(name, data, type) {
    const a = document.createElement("a"); a.href = URL.createObjectURL(new Blob([data], { type })); a.download = name; a.click(); setTimeout(() => URL.revokeObjectURL(a.href), 2000);
  }

  // the engine for the other tools of the package (Flow opens a campaign folder through it)
  /** @type {any} */ (window).ATTRIBUTIO_CORE = { pool: getPool, walk, download, MAX_FILES };

})();
