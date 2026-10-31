// @ts-check
// Import of the user's own official regulatory file (ECHA Candidate List, Proposition 65; CSV) in the browser (docs/agenti/normative.md). The CSV never leaves the computer: a worker runs
// the .wasm «normative» on it with the CAS sidecar of the site (Cache Storage, normative-scarica.js), and the worker writes the private .mzsus and its lists to OPFS (`normative-imported/`). The app does
// the same through the routes `normative/import-official` and `normative/imported`, with the same bytes. Loaded on demand by libreria-normative.js.

import * as dl from "./normative-scarica.js";

export const DIR = "normative-imported";
export const READERS = ["svhc", "ca_prop65"];
/** The sidecar next to the package (`static/mzfinder/sospetti/cas/`) and its catalogue format. */
export const CAS = { base: new URL("mzfinder/sospetti/cas/", import.meta.url).href, format: "mzlab-cas/1" };
const WASM = new URL("wasm-feat/", import.meta.url).href;

const err = (/** @type {string} */ key, /** @type {Record<string, unknown>} */ params = {}) => Object.assign(new Error(key), { key, params });
const dir = async () => (await navigator.storage.getDirectory()).getDirectoryHandle(DIR, { create: true });

/** One call of the worker. @param {Record<string, any>} job */
function run(job) {
  return new Promise((resolve, reject) => {
    const w = new Worker(new URL("normative-importa-worker.js", import.meta.url), { type: "module" });
    w.onmessage = (/** @type {MessageEvent<any>} */ ev) => { w.terminate(); if (ev.data.ok) resolve(ev.data); else reject(err("normative.import.err.parse", { detail: ev.data.error })); };
    w.onerror = () => { w.terminate(); reject(err("normative.import.err.worker")); };
    w.postMessage({ id: 1, base: WASM, ...job }, [job.csv, job.sidecar.buffer]);
  });
}

/**
 * Imports `file` as the list of `reader`; returns the summary (`{id, citation, entries, screenable, unresolved, rows}`). The same list imported again replaces the first.
 * @param {{ reader: string, file: File, date?: string, onProgress?: (p: any) => void, caches?: CacheStorage }} o
 */
export async function importFile(o) {
  if (!READERS.includes(o.reader)) throw err("normative.import.err.parse", { detail: o.reader });
  const { raw } = await dl.ensure({ ...CAS, onProgress: o.onProgress, caches: o.caches });
  const label = JSON.stringify({ file: o.file.name, date: o.date || new Date().toISOString().slice(0, 10) });
  const r = await run({ reader: o.reader, csv: await o.file.arrayBuffer(), sidecar: raw, label });
  return JSON.parse(r.summary);
}

/** The lists imported so far, in the shape of the catalogue of the package. */
export async function imported() {
  /** @type {any[]} */ const lists = [];
  try {
    for await (const [name, h] of /** @type {any} */ (await dir()).entries()) {
      if (!name.endsWith(".json")) continue;
      try { lists.push(...JSON.parse(await (await h.getFile()).text())); } catch (_) { /* a damaged entry is skipped */ }
    }
  } catch (_) { /* no OPFS */ }
  return lists.sort((a, b) => String(a.id).localeCompare(String(b.id)));
}
