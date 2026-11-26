// @ts-strict
// `file method` in the worker of the engine (rust-worker.js, op "filemethod"): the .wasm `filemethod` of wasm-feat/manifest.json (crates/mzlab-filemethod-wasm) is
// fetched on the first request and gives `file_method/1` from the `method/1` and `items/1` texts the page already holds, the bytes `mzlab-cli file method` prints.

import { load } from "./rust-msngrafo.js";

/**
 * The `file_method/1` document of a sample; a failure is thrown as the JSON string `{error_key, params}` of the module.
 * @param {{sample?: number | null, method?: string | null, items: string}} m  sample id (null for an mzML), `method/1` text ("" or null: the file carries none), `items/1` text
 */
export async function fileMethod(m) {
  const mod = await load("filemethod");
  return mod.file_method(m.sample ?? -1, m.method || "", m.items);
}

/**
 * The ion-scale method values (`ion_facts` of the .wasm `ionfacts`, what `mzlab-cli raw xic --ions` reads from the file's method): `{query, read}`; `query` is appended to `/api/rawxic?...&ions=1`. A failure is thrown as the JSON string `{error_key, params}`.
 * @param {{method?: string | null, scan_type?: string | null}} m  `method/1` text ("" or null: the file carries none), scan type named ("" or null: every kind)
 */
export async function ionFacts(m) {
  const mod = await load("ionfacts");
  return mod.ion_facts(m.method || "", m.scan_type || "");
}
