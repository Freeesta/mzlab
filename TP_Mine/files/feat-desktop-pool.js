// @ts-check
// TPMINE-PRIVATE  The pool of the Pipeline panel (32_pipeline.js) in the desktop app: the same calls as createFeatPool of feat-worker.js (crateVersion, preview, detectAll, align,
// hypotheses, evidence), but every step is a native route `feat/*` of the app (crates/mzlab-desktop/src/feat.rs): the .mzidx are the ones of the app's index folder, the
// .mzfeatf stay in the app (the page names the files, never sends them) and the table comes back as bytes. No wasm-feat/ and no OPFS here.
// Imported from a blob by 32_pipeline.js (`QTOOLS.ctx.files["feat-desktop-pool.js"]`) when the page runs in the app.

const BASE = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : "mzlab://localhost/";
const CHUNK = 32 << 20;

/** an engine error is `{error_key, params}`: the Error carries `key` and `params` as the pool's errors do (feat-run.js tests `key === "error.featMemory"`) @param {string} t */
function failure(t) {
  /** @type {any} */ let e;
  try { const j = JSON.parse(t); e = new Error(j.params && j.params.detail || j.error_key || t); e.key = j.error_key; e.params = j.params || {}; } catch (_) { e = new Error(t); }
  return e;
}
/** @param {string} op @param {Record<string, any>} [p] @returns {Promise<Response>} */
async function call(op, p) {
  const r = await fetch(BASE + "rpc", { method: "POST", body: JSON.stringify({ op, ...p }) });
  if (r.status !== 200) throw failure(await r.text());
  return r;
}
/** @param {string} op @param {Record<string, any>} [p] */
const json = async (op, p) => JSON.parse(await (await call(op, p)).text());
/** the table the last align or evidence step holds in the app (`total` = the `bytes` that step answered) @param {number} total @returns {Promise<Uint8Array>} */
async function tableBytes(total) {
  const out = new Uint8Array(total);
  for (let off = 0; off < total; off += CHUNK) out.set(new Uint8Array(await (await call("feat/table", { kind: "bytes", off, len: CHUNK })).arrayBuffer()), off);
  return out;
}
/** the name the app knows a file by, from the OPFS-like path or name of the panel (`mzlab/idx/<name>.mzidx`) @param {string} s */
const nameOf = s => s.replace(/^.*\//, "").replace(/\.mzidx$/, "");
/** the recipe is the JSON text of the six numbers (or empty) @param {any} r */
const recipeText = r => (typeof r === "string" ? r : r ? JSON.stringify(r) : "");

/** @param {{ notify?: (key: string, params?: any) => void }} [opts] */
export function createDesktopPool(opts = {}) {
  const pool = {
    onProgress: /** @type {((p: any) => void) | null} */ (null),
    notify: opts.notify || (() => {}),
    /** the names (with `.mzidx`, as the panel lists the OPFS files) of the files the app has an index of */
    async listIdx() { return (/** @type {string[]} */ (await json("feat/files"))).map(n => n + ".mzidx"); },
    async crateVersion() { return json("feat/version"); },
    /** @param {{ name: string, path?: string }} file @param {any} recipe */
    async preview(file, recipe) {
      const t0 = performance.now(), r = await json("feat/preview", { name: nameOf(file.name), recipe: recipeText(recipe) });
      const ms = Math.round(performance.now() - t0);
      if (pool.onProgress) pool.onProgress({ phase: "preview", name: file.name, done: 1, total: 1, ms });
      return { ...r, ms, crate: await json("feat/version") };
    },
    /** L1-L3 of every file: the .mzfeatf stay in the app, the list holds an empty placeholder per file @param {{ name: string }[]} files @param {any} recipe */
    async detectAll(files, recipe) {
      const out = [];
      for (const [k, f] of files.entries()) {
        const t0 = performance.now();
        await json("feat/detect", { name: nameOf(f.name), recipe: recipeText(recipe) });
        if (pool.onProgress) pool.onProgress({ phase: "detect", name: f.name, done: k + 1, total: files.length, ms: Math.round(performance.now() - t0) });
        out.push(new Uint8Array(0));
      }
      return out;
    },
    /** L4-L6 @param {Uint8Array[]} _stored @param {string[]} paths @param {any} recipe */
    async align(_stored, paths, recipe) {
      const t0 = performance.now();
      const r = await json("feat/align", { names: paths.map(nameOf), recipe: recipeText(recipe) });
      if (pool.onProgress) pool.onProgress({ phase: "align", ms: Math.round(performance.now() - t0) });
      return tableBytes(r.bytes);
    },
    /** L7: `{ state, tried, found, registry, ms }` (the state of the hypotheses stays in the app) @param {any} a */
    async hypotheses(a) {
      const t0 = performance.now(), rules = a.rules instanceof Uint8Array ? new TextDecoder().decode(a.rules) : a.rules;
      const r = await json("feat/hypotheses", { names: a.paths.map(nameOf), rules, parent: a.parent, parent_rt: a.parentRt, structure: a.structure || "", recipe: recipeText(a.recipe) });
      const ms = Math.round(performance.now() - t0);
      if (pool.onProgress) pool.onProgress({ phase: "hypotheses", ms });
      return { state: null, tried: r.tried, found: r.found, registry: r.registry, ms };
    },
    /** L8-L9: `{ table }` with the proofs; a NaN becomes null, which the app reads as no value @param {any} a */
    async evidence(a) {
      const t0 = performance.now();
      const r = await json("feat/evidence", { minutes: a.minutes, parent_mz: a.parentMz, parent_rt: a.parentRt, recipe: recipeText(a.recipe) });
      if (pool.onProgress) pool.onProgress({ phase: "proofs", ms: Math.round(performance.now() - t0) });
      return { table: await tableBytes(r.bytes) };
    },
    /** a view of the table the last step holds in the app (`feat/table`): `kind` = samples (no table needed) | qc | view | report | export | ms2, the answer is the engine's text @param {string} kind @param {Record<string, any>} [p] */
    async table(kind, p) { return (await call("feat/table", { kind, ...p })).text(); },
    /** the MS2 text (FBMN `.mgf`, SIRIUS `.ms`) of the table: the best MS2 of each row is read from the index of its file @param {{ samples: string, spec: string, sha: string, pol: number, kind: string, paths: string[] }} a */
    ms2File(a) { return pool.table("ms2", { samples: a.samples, spec: a.spec, sha256: a.sha, polarity: a.pol, export_kind: a.kind, names: a.paths.map(nameOf) }); },
    /** an operation of campioni/2 (proponi, valida, per_tp, da_legacy, canonico, digest) of the native engine: the text it answers @param {string} op @param {string} input */
    async campioni(op, input) { return (await call("finder/campioni", { cop: op, input })).text(); },
    cancel() {},
    resume() {},
  };
  return pool;
}
