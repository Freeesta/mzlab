// @ts-strict
// The ion scale of the integrations table (`peak table --ions`) in the worker of the engine (rust-worker.js, ops "manint" with `ions` and "manionchoices"): the .wasm
// `manions` of wasm-feat/manifest.json (crates/mzlab-manions-wasm) is fetched on the first request that asks for it. It writes the table of `manual_table` (module
// `rotte`) with the ion columns after the others, and what the request told the engine per sample; "E" + JSON = the user's error, as in `rotte`.

import { load } from "./rust-msngrafo.js";

/**
 * @param {{files: string, windows?: string, sums?: boolean, ions: string}} m  `files`/`windows` as api/mantraces takes them, `ions` = `{"samples":[{"scan_type"?, "scan_from"?, "fill_ms"?, "dwell_ms"?, "scan_s"?, "step_da"?, "mass_lo"?, "mass_hi"?, "traces"?}]}`
 * @param {boolean} choices  true: the per-sample log (`manint::choices_text`) instead of the table
 */
export async function manIons(m, choices) {
  const mod = await load("manions");
  return choices ? mod.manual_ion_choices(m.files, m.ions) : mod.manual_table_ions(m.files, m.windows || "[]", !!m.sums, m.ions);
}
