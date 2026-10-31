// @ts-strict
// The mass calibration check of one high-resolution file in the worker of the engine (rust-worker.js, op "masscal"): the `mass_accuracy` of the .wasm `proofs`
// (the function `flow run` calls for qc.mass_accuracy) over the .mzidx of OPFS and the background ions of the site (contaminanti.json). Nothing is written.

import { load } from "./rust-msngrafo.js";

/** @type {Promise<Uint8Array> | null} */ let listP = null;
const contaminants = () => listP || (listP = fetch(new URL("contaminanti.json", import.meta.url)).then(r => (r.ok ? r.arrayBuffer() : Promise.reject(new Error("contaminanti.json")))).then(b => new Uint8Array(b)).catch(e => { listP = null; throw e; }));

/**
 * The `massacc/1` text of the file `name`; a failure is thrown as the JSON string `{error_key, params}` of the module.
 * @param {string} name @param {string} params the parameters of the step as JSON
 * @param {(blob: Blob) => (offset: number, length: number) => Uint8Array} readerFor
 */
export async function masscal(name, params, readerFor) {
  const root = await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab");
  const [m, idx, list] = await Promise.all([load("proofs"), (await (await root.getDirectoryHandle("idx")).getFileHandle(name + ".mzidx")).getFile(), contaminants()]);
  const rd = readerFor(idx), none = new Uint8Array(0);
  const r = m.mass_accuracy((/** @type {number} */ _k, /** @type {number} */ off, /** @type {number} */ len) => rd(off, len), [idx.size], JSON.stringify([[name, ""]]), none, list, none, () => none, 0, params);
  try { return r.json(); } finally { r.free(); }
}
