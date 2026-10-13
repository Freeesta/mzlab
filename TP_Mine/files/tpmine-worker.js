// @ts-check
// TPMINE-PRIVATE  Web Worker of mzFinder: it routes the calls of the page between Rust and its own Pyodide (Python + numpy, the private engine on the mzML bytes
// sent by the page), which starts only when a call needs it. Rust = the `finder` .wasm (crates/mzlab-feat-wasm, wasm-feat/manifest.json) and, for a Dig, the
// pool of feat-worker.js (detection, alignment, hypotheses, proofs). Rust serves: formula_info, identity_graph, polarity_pairs, origin_evidence and default_transformations always;
// classify (after finder-idx.js has built the .mzidx of OPFS mzlab/idx/ that a file sent by the page lacks); run on high-resolution files with their .mzidx and any list of transformations (finder_transformations merges the user text into regole_tp.json),
// then detail, inclusion_csv, msn_tree, frag_tree and frag_tree_compare of that run. With an MSn file of the standard the `finder_msn` .wasm, loaded only
// then, adds its tree, calibration and the localisation of the products (FinderMsn) to what the Dig gives; run on low-resolution full-scan files with their .mzidx (finder-lr.js, then detail, frag_tree
// and frag_tree_compare of that run). Everything else (MS2/MRM low-resolution files, isf_classify, transitions, ...) goes to Pyodide, and so does
// every call when the site has no wasm-feat/ (a copy built without it). In the desktop app (init.desktop) every call goes to finder-desktop.js, i.e. to the native engine of the app: no Pyodide, no wasm-feat/.
/** @type {any} */ let py = null;
/** @type {any} */ let api = null;
/** @type {any} */ let init = null;               // the init message: Pyodide is loaded from it only when needed
/** @type {Promise<any> | null} */ let pyReady = null;
/** @type {Map<string, any>} */ const puts = new Map();   // the files of the page, kept until Pyodide loads them (then written at once)
/** @type {Record<string, any>} */ const mods = {};   // the modules of the manifest already loaded (wasm-bindgen): finder, finder_msn
/** @type {any} */ let man = null;                // wasm-feat/manifest.json
/** @type {any} */ let featPool = null;
/** @type {Map<string, any>} */ const held = new Map();    // the Blob or ArrayBuffer of every file the page sent: the source of the .mzidx that finder-idx.js builds when OPFS has none
/** @type {any} */ let ingress = null;            // finder-idx.js
/** @type {any} */ let desk = null;               // finder-desktop.js: in the desktop app every call but the default list goes to the native engine
/** @type {any} */ let dig = null;                // FinderDig of the last run in Rust
/** @type {any} */ let msn = null;                // FinderMsn of that run (null without an MSn file)
let last = "", bigN = 0;                          // the engine of the last run: "rust" | "rust-lr" | "py"
/** @type {Map<string, Uint8Array>} */ const detected = new Map();   // .mzfeatf of a .mzidx (name, size, date): a second Dig skips the detection
const ALWAYS = new Set(["formula_info", "identity_graph", "polarity_pairs", "origin_evidence"]);
const OF_DIG = new Set(["detail", "inclusion_csv", "msn_tree", "frag_tree", "frag_tree_compare"]);
// chem.transformations_text() of the Python engine (tests/test_tpmine_worker.py keeps them equal)
const DEFAULT_TR = `hydroxylation;+O;one extra oxygen (aromatic or aliphatic OH)
dihydroxylation;+O2;two extra oxygens
dehydrogenation;-H2;loss of H2 (double bond, ring closure)
oxidation to carbonyl;+O-H2;alcohol -> ketone or aldehyde
demethylation;-CH2;N- or O-demethylation
methylation;+CH2
loss of propene (N-deisopropylation);-C3H6;loss of the isopropyl group
decarboxylation;-CO2
carboxylation;+CO2
hydration;+H2O;addition of water, hydrolysis without cleavage
dehydration;-H2O
loss of SO2;-SO2;SO2 extrusion (sulfonamides, sulfones)
loss of CO;-CO
oxidative deamination;-NH+O-H2
reductive dechlorination;-Cl+H
hydrolytic dechlorination;-Cl+OH
reductive debromination;-Br+H
hydrolytic defluorination;-F+OH
amide hydrolysis;+H2O;cleavage of the amide/ester bond (same mass as hydration)`;
/** @param {string} text */
const say = text => postMessage({ type: "step", text });
const base = () => new URL("../../", init.pyodideMjs).href;
/** @param {string} p */
const featUrl = p => base() + "wasm-feat/" + p;

// ---------------------------------------------------------------- Pyodide (started on the first call that needs it)
/** @param {string} name @param {any} m */
function putPy(name, m) {
  const dst = "/tp_data/" + name;
  try { py.FS.unlink(dst); } catch (_) { /* not there yet */ }
  if (m.blob) {      // a big file is not copied into the heap of WebAssembly: the Blob is mounted read-only (WORKERFS) and linked, like the main program does
    const dir = "/big/" + (++bigN);
    py.FS.mkdirTree(dir);
    py.FS.mount(py.FS.filesystems.WORKERFS, { blobs: [{ name: "data", data: m.blob }] }, dir);
    py.FS.symlink(dir + "/data", dst);
  } else py.FS.writeFile(dst, new Uint8Array(m.buf));
}
function pyodide() {
  return pyReady || (pyReady = (async () => {
    say("Carico Python...");
    const { loadPyodide } = await import(init.pyodideMjs);
    py = await loadPyodide({ indexURL: init.indexURL });
    say("Carico numpy...");
    await py.loadPackage("numpy");
    say("Preparo il motore...");
    py.FS.mkdirTree("/qqq"); py.unpackArchive(init.qqqZip, "zip", { extractDir: "/qqq" });
    py.FS.mkdirTree("/tpm"); py.unpackArchive(init.tpZip, "zip", { extractDir: "/tpm" });
    py.FS.mkdirTree("/tp_data");
    py.runPython("import sys\nsys.path[:0] = ['/qqq', '/tpm']\nimport tpmine.api");
    api = py.pyimport("tpmine.api");
    api._progress = (/** @type {string} */ text, /** @type {number} */ frac, /** @type {string} */ phase, /** @type {string} */ payload) =>
      postMessage({ type: "progress", text, frac, phase, payload: payload ? JSON.parse(payload) : null });
    for (const [name, m] of puts) putPy(name, m);
    if (puts.size) api.classify(JSON.stringify([...puts.keys()]));     // the Python engine learns the kinds of the files the page already sent
    puts.clear();
    return api;
  })());
}

// ---------------------------------------------------------------- Rust: the finder .wasm and the .mzidx of OPFS
async function idxDir() {
  try { return await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab")).getDirectoryHandle("idx"); } catch (_) { return null; }
}
/** the .mzidx (File) of every name, null when one is missing @param {string[]} names */
async function idxFiles(names) {
  const d = await idxDir();
  if (!d || !names.length) return null;
  try { return await Promise.all(names.map(async n => (await d.getFileHandle(n + ".mzidx")).getFile())); } catch (_) { return null; }
}
/** synchronous readers of the .mzidx of `names`: read(k, offset, length) as the .wasm wants it @param {string[]} names */
async function openIdx(names) {
  const d = await idxDir(), hs = /** @type {any[]} */ ([]);
  if (!d) throw new Error("no index of the file: open it again");
  try { for (const n of names) hs.push(await /** @type {any} */ (await d.getFileHandle(n + ".mzidx")).createSyncAccessHandle()); }
  catch (e) { for (const h of hs) h.close(); throw e; }
  let buf = new Uint8Array(1 << 16);
  const read = (/** @type {number} */ k, /** @type {number} */ off, /** @type {number} */ len) => {
    if (buf.length < len) buf = new Uint8Array(Math.max(len, buf.length * 2));
    const out = buf.subarray(0, len);
    if (hs[k].read(out, { at: off }) !== len) throw new Error("error.io");
    return out;
  };
  return { read, sizes: Float64Array.from(hs, h => h.getSize()), close: () => { for (const h of hs) { try { h.close(); } catch (_) { /* closed */ } } } };
}
async function manifest() { return man || (man = await (await fetch(featUrl("manifest.json"))).json()); }
/** @type {Promise<boolean> | null} */ let rustOk = null;
/** the site has the finder .wasm (a copy built without wasm-feat, like the ones of most e2e tests, keeps everything in Pyodide) */
function hasRust() {
  return rustOk || (rustOk = fetch(featUrl("manifest.json")).then(r => (r.ok ? r.json() : null)).then(m => { man = m; return !!(m && m.finder); }).catch(() => false));
}
/** @param {ArrayBuffer} b */
const sha = async b => [...new Uint8Array(await crypto.subtle.digest("SHA-256", b))].map(x => x.toString(16).padStart(2, "0")).join("");
/** the module `key` of the manifest, checked against its SHA-256 @param {string} key */
async function load(key) {
  if (mods[key]) return mods[key];
  const m = (await manifest())[key], bytes = await (await fetch(featUrl(m.wasm))).arrayBuffer();
  if (m.sha256 && await sha(bytes) !== m.sha256) throw new Error("the engine of mzFinder does not match its manifest: reload the page");
  const mod = await import(featUrl(m.js));
  await mod.default({ module_or_path: bytes });
  return (mods[key] = mod);
}
/** @type {any} */ let lastRun = null;           // the .mzfeat and the files of the last Dig in Rust (the numbers of the table of the products)
/** the numbers of the table of the products for some rows of the last Dig (finder-tabella.js); "null" when there is none @param {string} rows */
async function tpTable(rows) {
  if (last !== "rust" || !lastRun) return "null";
  const m = await import(base() + "mzfinder/finder-tabella.js");
  return m.tpTable({ manifest, url: featUrl, sha }, lastRun, rows);
}
const finder = () => load("finder");
/** every file the page sent has a fresh .mzidx in OPFS (finder-idx.js builds the missing ones) @param {string[]} names */
async function ensureIdx(names) {
  if (!ingress) ingress = (await import(base() + "mzfinder/finder-idx.js")).createIngress(base(), say, (key, params) => postMessage({ type: "notice", key, params }));
  await ingress.ensure(names, held);
}
async function pool() {
  if (featPool) return featPool;
  const m = await manifest(), url = base() + "mzfinder/feat-worker.js", mine = self.onmessage;
  const mod = await import(url);
  self.onmessage = mine;                         // feat-worker.js, imported inside a worker, takes onmessage for itself: it is given back
  const w = (/** @type {any} */ x) => ({ jsUrl: featUrl(x.js), wasmUrl: featUrl(x.wasm), sha256: x.sha256 });
  featPool = mod.createFeatPool({ workerUrl: url, size: mod.poolSize(false, navigator.hardwareConcurrency), wasm: { feat: w(m.feat), proofs: w(m.proofs) } });
  return featPool;
}

/** @param {string[]} names */
async function classifyRust(names) {
  const f = await finder(), ix = await openIdx(names);
  try { return f.finder_classify(names, ix.read, ix.sizes); } finally { ix.close(); }
}
/** L1-L3 of the files, reusing the .mzfeatf of a .mzidx already detected @param {any} p @param {{name: string, path: string}[]} files @param {File[]} idx @param {string} [recipe] */
async function detect(p, files, idx, recipe = "") {
  const key = (/** @type {number} */ k) => `${files[k].name}|${idx[k].size}|${idx[k].lastModified}|${recipe}`;
  const todo = files.map((_, k) => k).filter(k => !detected.has(key(k)));
  if (todo.length) {
    const got = await p.detectAll(todo.map(k => files[k]), recipe);
    todo.forEach((k, i) => detected.set(key(k), got[i]));
  }
  return files.map((_, k) => /** @type {Uint8Array} */ (detected.get(key(k))));
}
/** the Dig of a high-resolution series in Rust: pipeline L1-L9 in the pool, then the adapter (finder_dig) @param {string[]} a */
async function runRust(a) {
  const all = JSON.parse(a[0]), x = all.find((/** @type {any} */ f) => f.kind === "msn"), [, parentJson, settingsJson] = a;
  const filesJson = JSON.stringify(all.filter((/** @type {any} */ f) => f.kind !== "msn"));    // the series of the Dig: the LC files only
  const t0 = performance.now(), f = await finder(), p = await pool();
  const plan = JSON.parse(f.finder_plan(filesJson, parentJson)), idx = await idxFiles(plan.lc);
  if (!idx) throw new Error("no index of the file: open it again");
  const files = plan.lc.map((/** @type {string} */ n) => ({ name: n, path: "mzlab/idx/" + n + ".mzidx" })), paths = files.map((/** @type {any} */ x) => x.path);
  const timing = /** @type {Record<string, number>} */ ({}), lap = (/** @type {string} */ k, /** @type {number} */ t) => { timing[k] = Math.round((performance.now() - t) / 10) / 100; };
  let t = performance.now();
  say(`Detecting the features of ${files.length} files...`);
  const recipe = JSON.parse(parentJson).adduct?.endsWith("-") ? '{"polarity":-1}' : "";       // an anion parent: the negative scans
  const stored = await detect(p, files, idx, recipe); lap("feature", t); t = performance.now();
  say("Aligning the series...");
  const table = await p.align(stored, paths, recipe); lap("align", t); t = performance.now();
  say("Searching the products of the parent...");
  let rules = new Uint8Array(await (await fetch(base() + "mzfinder/regole_tp.json")).arrayBuffer());
  const tr = (a[3] || "").trim();
  if (tr) rules = f.finder_transformations(rules, tr);      // the user's list of transformations: its rows become rules of the hypotheses
  const h = await p.hypotheses({ stored, paths, rules, parent: plan.formula, parentRt: NaN, recipe, structure: plan.structure });
  const minutes = plan.minutes.map((/** @type {number | null} */ m) => (m == null ? NaN : m));
  const e = await p.evidence({ table, state: h.state, minutes, parentMz: NaN, parentRt: NaN, recipe }); lap("proofs", t); t = performance.now();
  say("Ranking the candidates...");
  const ix = await openIdx(plan.lc);
  try { dig = f.finder_dig(filesJson, parentJson, e.table, h.registry, ix.read, ix.sizes); } finally { ix.close(); }
  lap("dig", t); t = performance.now(); last = "rust"; msn = null; lastRun = { table: e.table, lc: plan.lc, files: JSON.parse(filesJson) };
  if (x) { await msnTree(x.name, parentJson); lap("albero", t); }
  lap("total", t0);
  const s = dig.summary(settingsJson || "{}", JSON.stringify(timing));
  return msn ? JSON.stringify(msnSummary(JSON.parse(s))) : s;
}
/** the MSn tree of the standard (`finder_msn` .wasm, loaded only now) from the .mzidx of `name` @param {string} name @param {string} parentJson */
async function msnTree(name, parentJson) {
  say("MSn tree of the parent...");
  const mod = await load("finder_msn"), ix = await openIdx([name]);
  try { msn = mod.finder_msn(ix.read, ix.sizes[0], parentJson); } finally { ix.close(); }
  msn.parentSet = false;
}
/** the summary of the Dig with the MSn tree: tree, calibration, region and margin of the rows @param {any} s */
function msnSummary(s) {
  s.tree = JSON.parse(msn.msn_tree());
  s.calibration = JSON.parse(msn.calibration());
  for (const r of s.rows) {
    if (!r.ms2_scans || !r.formula) continue;
    const loc = localize(JSON.parse(dig.detail(r.id)));
    r.region = loc && loc.ok ? loc.region : null;
    r.margin = loc && loc.ok ? Math.round(loc.margin * 100) / 100 : null;
  }
  return s;
}
/** the localisation of a candidate from its detail (kept in FinderMsn for frag_tree_compare); the first MS2 of the parent rebuilds the library @param {any} d */
function localize(d) {
  if (!msn || !d.ms2 || !d.row.formula) return null;
  if (!msn.parentSet) { msn.set_parent_ms2(Float64Array.from(d.ms2.parent.mz), Float64Array.from(d.ms2.parent.rel)); msn.parentSet = true; }
  const c = d.ms2.candidate;
  return JSON.parse(msn.localize(d.id, d.row.formula, Float64Array.from(c.mz), Float64Array.from(c.rel), d.row.mz));
}
/** `api.detail` of the Dig with the localisation of the MSn tree @param {number} id */
function detail(id) {
  const raw = dig.detail(id);
  if (!msn) return raw;
  const d = JSON.parse(raw), loc = localize(d);
  if (loc) { d.localization = loc; d.ms2.evidence = loc.evidence || []; }
  return JSON.stringify(d);
}
/** a run goes to Rust: high-resolution files (and at most the MSn file of the standard), all with their .mzidx @param {string[]} a */
async function rustRun(a) {
  const files = JSON.parse(a[0]);
  if (!files.some((/** @type {any} */ f) => f.kind === "hr")) return false;
  return !!(await idxFiles(files.map((/** @type {any} */ f) => f.name)));
}

/** the low-resolution Dig in Rust (finder-lr.js) and what it borrows from this worker */
const lrMod = () => import(base() + "mzfinder/finder-lr.js");
const lrCtx = { featUrl, base, manifest, sha, pool, idxFiles, openIdx, detect };
/** every file has its .mzidx: waits a little for the one that the main program is still writing (a file taken from the Data tab sends no bytes here) @param {string[]} names */
async function waitIdx(names) {
  for (let i = 0; i < 60; i++) {
    const r = await idxFiles(names);
    if (r || !(await idxDir())) return r;
    await new Promise(res => setTimeout(res, 500));
  }
  return null;
}

/** one call of the page @param {string} fn @param {any[]} a */
async function call(fn, a) {
  if (fn === "default_transformations") return DEFAULT_TR;
  if (fn === "tp_table") return tpTable(a[0]);
  if (fn === "transf_rows") return (await finder()).finder_transf_rows(a[0]);       // the table of the list of transformations (the text is parsed by the same .wasm in the desktop app)
  if (init.desktop) return (desk || (desk = (await import(base() + "mzfinder/finder-desktop.js")).createDesktop(base(), say, held))).call(fn, a);
  if (!(await hasRust())) return toPy(fn, a);
  if (ALWAYS.has(fn)) { const f = await finder(); return fn === "formula_info" ? f.finder_formula_info(a[0], a[1]) : f["finder_" + fn](a[0]); }
  if (fn === "classify") {
    const names = JSON.parse(a[0]);
    await ensureIdx(names).catch(e => console.warn("mzFinder: indexing failed, Pyodide takes over:", e));
    if (await waitIdx(names)) { const r = classifyRust(names); if (py) await toPy(fn, a); return r; }       // a Pyodide that is already running learns the kinds of the files too
    console.warn("mzFinder: no .mzidx yet for", names.filter(n => !held.has(n)).join(", ") || "(all files sent)", "- Pyodide takes over");
  }
  if (fn === "run") {
    if (await rustRun(a)) return runRust(a);
    last = "py"; dig = null; msn = null;
    const tr = (a[3] || "").trim(), lr = tr && tr !== DEFAULT_TR ? false : await (await lrMod()).run(lrCtx, a);
    if (lr !== false) { last = "rust-lr"; return lr; }
  }
  if (OF_DIG.has(fn) && last === "rust-lr") return fn === "msn_tree" ? "null" : (await lrMod()).ofDig(lrCtx, fn, a);     // a low-resolution run has no MSn tree
  if (OF_DIG.has(fn) && last === "rust") {
    if (fn === "detail") return detail(+a[0]);
    if (fn === "inclusion_csv") return dig.inclusion_csv(+(a[0] || 0));
    if (fn === "msn_tree") return msn ? msn.msn_tree() : "null";
    if (!msn) return fn === "frag_tree" ? dig.frag_tree() : dig.frag_tree_compare(+a[0]);
    if (fn === "frag_tree") return msn.frag_tree();
    detail(+a[0]);                                   // "candidate not found", and the localisation the comparison uses
    return msn.frag_tree_compare(+a[0]);
  }
  return toPy(fn, a);
}
/** @param {string} fn @param {any[]} a */
async function toPy(fn, a) {
  const r = (await pyodide())[fn](...a);
  return typeof r === "string" ? r : String(r);
}

onmessage = async ev => {
  const m = ev.data;
  try {
    if (m.type === "init") {
      init = m;
      postMessage({ type: "ready" });
    } else if (m.type === "put") {
      held.set(m.name, m.blob || m.buf);
      if (py) putPy(m.name, m);
      else { puts.delete(m.name); puts.set(m.name, m); }
      postMessage({ id: m.id, result: true });
    } else if (m.type === "mem") {          // size of the WebAssembly memory: it only grows, so at the end it is the peak of the run (used by the tests)
      postMessage({ id: m.id, result: { wasm: py ? py._module.HEAPU8.length : 0 } });
    } else if (m.type === "call") {
      postMessage({ id: m.id, result: await call(m.fn, m.args || []) });
    }
  } catch (e) {
    const err = /** @type {any} */ (e);
    postMessage({ id: m.id, error: String(err && err.message || err).split("\n").filter(Boolean).slice(-3).join(" | "), fatal: m.type === "init" });
  }
};
