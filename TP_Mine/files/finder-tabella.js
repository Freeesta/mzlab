// @ts-check
// TPMINE-PRIVATE  The numbers of the table of the products for the page: the .wasm «table» of wasm-feat/manifest.json (route tp_table) read over the .mzfeat of the last Dig.
// The roles of the files (treated, t0, blank) go to Rust as a campioni/1 built in ONE place, samplesOf, from the model campioni/2 of finder-ruoli.js. Imported by tpmine-worker.js on the first request.

import * as ruoli from "./finder-ruoli.js";

/** @type {any} */ let mod = null;

/**
 * campioni/1 of the files of a Dig, in the order of the table, for the Rust of the table. The roles and the minutes are those of campioni/2 (finder-ruoli.js, the Rust
 * `per_tp`): the type and the time the user set on the page are written into the model as explicit choices, and the model gives the indices per role.
 * @param {(op: string, json: string) => string | Promise<string>} call the operation `campioni`: the table .wasm (synchronous) or the native route of the desktop app (asynchronous)
 * @param {string[]} lc names of the files of the series, in the order of the table
 * @param {{name: string, type?: string, time?: number | null}[]} files the files of the page
 */
export async function samplesOf(call, lc, files) {
  const by = new Map(files.map(f => [f.name, f]));
  const cs = await ruoli.build(call, lc, null, null);
  cs.forEach((c, i) => {
    const f = by.get(lc[i]);
    if (!f) return;
    const time = typeof f.time === "number" && Number.isFinite(f.time) ? f.time : null;
    if (f.type === "blank") ruoli.setRole(cs, i, "blank");
    else if (f.type === "control") ruoli.setRole(cs, i, "dark");
    else { ruoli.setRole(cs, i, "sample"); ruoli.setTime(cs, i, time, "min"); }
  });
  const p = await ruoli.forTp(call, cs), kind = new Map();
  for (const k of p.treated) kind.set(k, "sample");
  for (const k of p.t0) kind.set(k, "t0");
  for (const k of p.blank) kind.set(k, "blank");
  for (const k of p.dark) kind.set(k, "dark");
  return JSON.stringify({
    schema: "campioni/1",
    campioni: lc.map((name, k) => ({ file: name, sha256: "", tipo: kind.get(k) || "std", tempo_min: p.minuti[k], replica: 1, gruppo: "", concentrazione: null, unita: null, ordine_iniezione: k + 1, istd: false, note: "" })),
  });
}

/**
 * tptab/1 of some rows of the table of the last Dig.
 * @param {{manifest: () => Promise<any>, url: (p: string) => string, sha: (b: ArrayBuffer) => Promise<string>}} deps
 * @param {{table: Uint8Array, lc: string[], files: any[]}} run
 * @param {string} rowsJson JSON list of the rows (the `id` of the rows of the ranking)
 */
export async function tpTable(deps, run, rowsJson) {
  const t = run.table;
  return (await engine(deps)).tp_table((/** @type {number} */ _k, /** @type {number} */ off, /** @type {number} */ len) => t.subarray(off, off + len), t.length, await samplesOf(campioni, run.lc, run.files), rowsJson);
}

/** the table .wasm, loaded once @param {{manifest: () => Promise<any>, url: (p: string) => string, sha: (b: ArrayBuffer) => Promise<string>}} deps */
async function engine(deps) {
  if (mod) return mod;
  const m = (await deps.manifest()).table;
  if (!m) throw new Error("the table engine is missing from this site");
  const bytes = await (await fetch(deps.url(m.wasm))).arrayBuffer();
  if (m.sha256 && (await deps.sha(bytes)) !== m.sha256) throw new Error("the table engine does not match its manifest: reload the page");
  const js = await import(deps.url(m.js));
  await js.default({ module_or_path: bytes });
  return (mod = js);
}
const campioni = (/** @type {string} */ op, /** @type {string} */ j) => /** @type {any} */ (mod).campioni(op, j);

/**
 * riga/1 of one row of the table of the last Dig, with its traces (riga_tracce/1: XIC of every file, MS1 at the best apex) read from the .mzidx of the series.
 * The traces are null when they cannot be read (the row is then drawn without them).
 * @param {{manifest: () => Promise<any>, url: (p: string) => string, sha: (b: ArrayBuffer) => Promise<string>}} deps
 * @param {{table: Uint8Array, lc: string[], files: any[]}} run
 * @param {number} id the `id` of the row of the ranking
 * @param {{rowTraces: (a: any) => Promise<string>}} pool the pool of feat-worker.js
 * @param {(name: string) => string} idxName the name of the .mzidx of a file in OPFS (without the extension)
 */
export async function tpRow(deps, run, id, pool, idxName) {
  const m = await engine(deps), t = run.table, samples = await samplesOf(campioni, run.lc, run.files);
  const row = JSON.parse(m.table_row((/** @type {number} */ _k, /** @type {number} */ off, /** @type {number} */ len) => t.subarray(off, off + len), t.length, samples, id));
  /** @type {any} */ let traces = null;
  try { traces = JSON.parse(await pool.rowTraces({ table: t, samples, sha: await deps.sha(/** @type {ArrayBuffer} */ (t.slice().buffer)), id, ppm: 10, paths: run.lc.map(n => "mzlab/idx/" + idxName(n) + ".mzidx") })); } catch (e) { traces = null; console.warn("tp_row: the traces cannot be read:", e); }
  return JSON.stringify({ row, traces });
}
