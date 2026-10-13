// @ts-check
// TPMINE-PRIVATE  mzFinder in the desktop app: tpmine-worker.js sends every call here when the page says it runs in the app (init.desktop). Nothing is computed in this file: the
// steps (detection, alignment, hypotheses, proofs, the Dig) are the native routes `feat/*` and `finder/*` of the app (crates/mzlab-desktop/src/feat.rs, feat_finder.rs), the
// same functions the .wasm runs on the site, over the .mzidx of the app's index folder. No Pyodide and no wasm-feat/ here. The files are registered in the app by name,
// as the Data tab does (`lr_put`, `lr_index`); the bytes of a file that only exists in the page are sent once (`lr/upload`).
// Imported by tpmine-worker.js from <site>/mzfinder/finder-desktop.js.

const BASE = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : "mzlab://localhost/";
const CHUNK = 32 << 20;
const OF_DIG = new Set(["detail", "inclusion_csv", "msn_tree", "frag_tree", "frag_tree_compare"]);
const STATELESS = new Set(["identity_graph", "polarity_pairs", "origin_evidence"]);

/** an engine error is `{error_key, params}` (the detail of `error.finder` is the English text of the engine) @param {string} t */
function failure(t) {
  try { const j = JSON.parse(t); return new Error(j.params && j.params.detail || j.error_key || t); } catch (_) { return new Error(t); }
}
/** @param {string} route @param {BodyInit} body @param {Record<string, string | number>} [q] */
async function post(route, body, q) {
  const qs = q ? "?" + Object.entries(q).map(([k, v]) => k + "=" + encodeURIComponent(v)).join("&") : "";
  const r = await fetch(BASE + route + qs, { method: "POST", body });
  const t = await r.text();
  if (r.status !== 200) throw failure(t);
  return t;
}
/** @param {string} op @param {Record<string, any>} [p] */
const rpc = (op, p) => post("rpc", JSON.stringify({ op, ...p }));
/** the number a file has in the app: the tag of a file picked in the window, null for anything else @param {any} src */
const tagged = src => { const m = src instanceof Blob && /id=(\d+)/.exec(src.type || ""); return m ? Number(m[1]) : null; };

/** @param {string} name @param {Blob} blob the bytes go to the app in pieces, which gives the file a number */
async function upload(name, blob) {
  let id = null;
  for (let off = 0; off === 0 || off < blob.size; off += CHUNK) {
    const j = JSON.parse(await post("lr/upload", blob.slice(off, off + CHUNK), { name, off, total: blob.size }));
    if (j.id) id = j.id;
  }
  return id;
}

/** the rules of the hypotheses: regole_tp.json with the rows of the user's list of transformations @param {string} base @param {string | undefined} list */
async function rulesOf(base, list) {
  const rules = await (await fetch(base + "mzfinder/regole_tp.json")).text(), tr = (list || "").trim();
  return tr ? rpc("finder/transformations", { rules, text: tr }) : rules;      // the user's list: its rows become rules of the hypotheses
}

/** @param {string} base the static folder of the site @param {(text: string) => void} say @param {Map<string, any>} held */
export function createDesktop(base, say, held) {
  /** every named file is registered in the app and has its .mzidx (a file the page lost the bytes of, an empty stub of the Data tab, is already registered by name) @param {string[]} names */
  async function ensure(names) {
    const boot = /** @type {{name: string, size: number, indexed: boolean, id: number}[]} */ (JSON.parse(await rpc("lr_boot")));
    /** @type {string[]} */ const todo = [];
    for (const name of names) {
      const src = held.get(name), row = boot.find(x => x.name === name);
      const size = src instanceof Blob ? src.size : src ? src.byteLength : 0;
      let fid = tagged(src);
      if (fid == null && size > 0 && !(row && row.size === size)) { say(`Sending ${name}...`); fid = await upload(name, src instanceof Blob ? src : new Blob([src])); }
      if (fid != null && !(row && row.id === fid)) { await rpc("lr_put", { name, fid }); todo.push(name); }
      else if (!row || !row.indexed) todo.push(name);
    }
    for (const name of todo) { say(`Indexing ${name}...`); await rpc("lr_index", { name }); }
  }

  /** the Dig of a high-resolution series: the pipeline L1-L9 in the app, then the adapter; the MSn file of the standard (if any) is not in the series, the app builds its tree @param {string[]} a */
  async function run(a) {
    const all = JSON.parse(a[0]), msn = all.find((/** @type {any} */ f) => f.kind === "msn"), parentJson = a[1], settingsJson = a[2];
    const filesJson = JSON.stringify(all.filter((/** @type {any} */ f) => f.kind !== "msn")), t0 = performance.now(), timing = /** @type {Record<string, number>} */ ({});
    const lap = (/** @type {string} */ k, /** @type {number} */ t) => { timing[k] = Math.round((performance.now() - t) / 10) / 100; };
    const plan = JSON.parse(await rpc("finder/plan", { files: filesJson, parent: parentJson })), names = /** @type {string[]} */ (plan.lc);
    await ensure(msn ? [...names, msn.name] : names);
    let t = performance.now();
    for (const [k, name] of names.entries()) { say(`Detecting the features of ${name} (${k + 1} of ${names.length})...`); await rpc("feat/detect", { name, recipe: "" }); }
    lap("feature", t); t = performance.now();
    say("Aligning the series..."); await rpc("feat/align", { names, recipe: "" }); lap("align", t); t = performance.now();
    say("Searching the products of the parent...");
    const h = JSON.parse(await rpc("feat/hypotheses", { names, rules: await rulesOf(base, a[3]), structure: plan.structure, parent: plan.formula, recipe: "" }));
    await rpc("feat/evidence", { minutes: plan.minutes, recipe: "" }); lap("proofs", t); t = performance.now();
    say("Ranking the candidates...");
    const out = await rpc("finder/dig", { files: filesJson, parent: parentJson, registry: JSON.stringify(h.registry), settings: settingsJson || "{}", timing: "{}", msn: msn ? msn.name : "" });
    lap("dig", t); lap("total", t0);
    return JSON.stringify({ ...JSON.parse(out), timing });
  }

  /** the Dig of a low-resolution series (full scans only): detection, alignment and the hypotheses of the parent in the app, then the Dig of the low-resolution adapter @param {string[]} a @param {any} plan */
  async function runLr(a, plan) {
    const [filesJson, parentJson, settingsJson] = a, t0 = performance.now(), timing = /** @type {Record<string, number>} */ ({});
    const lap = (/** @type {string} */ k, /** @type {number} */ t) => { timing[k] = Math.round((performance.now() - t) / 10) / 100; };
    const names = /** @type {string[]} */ (plan.lc);
    await ensure(names);
    let t = performance.now();
    for (const [k, name] of names.entries()) { say(`Detecting the peaks of ${name} (${k + 1} of ${names.length})...`); await rpc("feat/detect", { name, recipe: "" }); }
    lap("feature", t); t = performance.now();
    say("Aligning the peaks of the series..."); await rpc("feat/align", { names, recipe: "" }); lap("align", t); t = performance.now();
    say("Generating candidates...");
    const h = JSON.parse(await rpc("feat/hypotheses", { names, rules: await rulesOf(base, a[3]), structure: "", parent: plan.formula, recipe: "" })); lap("candidates", t); t = performance.now();
    say("Computing the tables...");
    const out = await rpc("finder/lr_dig", { files: filesJson, parent: parentJson, registry: JSON.stringify(h.registry), settings: settingsJson || "{}", timing: "{}" });
    lap("dig", t); lap("total", t0);
    return JSON.stringify({ ...JSON.parse(out), timing });
  }

  return {
    /** one call of the page @param {string} fn @param {string[]} a */
    async call(fn, a) {
      if (fn === "tp_table") return "null";        // the numbers of the table of the products are not native yet: the page keeps a dash
      if (fn === "transf_rows") return rpc("finder/transf_rows", { text: a[0] });
      if (fn === "formula_info") return rpc("finder/formula_info", { text: a[0], adduct: a[1] });
      if (STATELESS.has(fn)) return rpc("finder/" + fn, { payload: a[0] });
      if (OF_DIG.has(fn)) return rpc("finder/" + fn, { id: +(a[0] || 0) });
      if (fn === "classify") { const names = JSON.parse(a[0]); await ensure(names); return rpc("finder/classify", { names }); }
      if (fn === "run") {
        const files = JSON.parse(a[0]);
        if (!files.some((/** @type {any} */ f) => f.kind === "hr")) {
          const plan = files.length && files.every((/** @type {any} */ f) => f.kind === "full") ? JSON.parse(await rpc("finder/lr_plan", { files: a[0], parent: a[1] })) : null;
          if (!plan || !plan.rust) throw new Error("this low-resolution series (MS2 or MRM files, or a parent without formula) is not in the app yet: use the website for this series");
          return runLr(a, plan);
        }
        return run(a);
      }
      throw new Error(`${fn} is not in the app`);
    },
  };
}
