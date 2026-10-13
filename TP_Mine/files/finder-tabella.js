// @ts-check
// TPMINE-PRIVATE  The numbers of the table of the products for the page: the .wasm «table» of wasm-feat/manifest.json (route tp_table) read over the .mzfeat of the last Dig.
// The roles of the files (treated, t0, blank) go to Rust as a campioni/1 built in ONE place, samplesOf. Imported by tpmine-worker.js on the first request.

/** @type {any} */ let mod = null;

/**
 * campioni/1 of the files of a Dig, in the order of the table: the only place where the type and the time of a file of the page become the role of a sample.
 * @param {string[]} lc names of the files of the series, in the order of the table
 * @param {{name: string, type?: string, time?: number | null}[]} files the files of the page
 */
export function samplesOf(lc, files) {
  const by = new Map(files.map(f => [f.name, f]));
  return JSON.stringify({
    schema: "campioni/1",
    campioni: lc.map((name, k) => {
      const f = by.get(name) || { name }, time = typeof f.time === "number" ? f.time : null;
      const tipo = f.type === "blank" ? "blank" : f.type === "control" ? "dark" : time === 0 ? "t0" : "sample";
      return { file: name, sha256: "", tipo, tempo_min: time, replica: 1, gruppo: "", concentrazione: null, unita: null, ordine_iniezione: k + 1, istd: false, note: "" };
    }),
  });
}

/**
 * tptab/1 of some rows of the table of the last Dig.
 * @param {{manifest: () => Promise<any>, url: (p: string) => string, sha: (b: ArrayBuffer) => Promise<string>}} deps
 * @param {{table: Uint8Array, lc: string[], files: any[]}} run
 * @param {string} rowsJson JSON list of the rows (the `id` of the rows of the ranking)
 */
export async function tpTable(deps, run, rowsJson) {
  if (!mod) {
    const m = (await deps.manifest()).table;
    if (!m) throw new Error("the table engine is missing from this site");
    const bytes = await (await fetch(deps.url(m.wasm))).arrayBuffer();
    if (m.sha256 && (await deps.sha(bytes)) !== m.sha256) throw new Error("the table engine does not match its manifest: reload the page");
    const js = await import(deps.url(m.js));
    await js.default({ module_or_path: bytes });
    mod = js;
  }
  const t = run.table;
  return mod.tp_table((/** @type {number} */ _k, /** @type {number} */ off, /** @type {number} */ len) => t.subarray(off, off + len), t.length, samplesOf(run.lc, run.files), rowsJson);
}
