// @ts-strict
// `api/rawinfo` and `api/rawxic` in the worker of the engine (rust-worker.js, op "route"): the .wasm `raw` (crates/mzlab-raw-wasm; `rawxic`, crates/mzlab-rawxic-wasm, for `/api/rawxic`) of wasm-feat/manifest.json
// is fetched on the first request and reads the .mzidx of the file from OPFS itself; the members are those of `mzlab-cli raw info`, the rows those of `raw xic`.
// The engine's .wasm has no room for it (docs/agenti/raw-access.md).

import { load } from "./rust-msngrafo.js";
import * as ARCH from "./archivio.js";

/**
 * The answer of `Routes::raw_info_route` or, for `/api/rawxic`, `raw_xic_route` ("" = no index for the slot); a failure is thrown as the JSON string
 * `{error_key, params}` of the module.
 * @param {{slot: number, path?: string, query?: string}} m
 * @param {{nameOf: (slot: number) => string | undefined, polOf: (slot: number) => number, readerFor: (blob: Blob, held?: string[]) => any}} env
 */
export async function rawInfo(m, { nameOf, polOf, readerFor }) {
  const name = nameOf(m.slot), blob = name ? await ARCH.get("idx", name + ".mzidx") : null;
  if (!blob || !name) return "";
  const xic = m.path === "/api/rawxic";
  const mod = await load(xic ? "rawxic" : "raw");
  const ask = xic ? mod.raw_xic : mod.raw_info;
  return ask(blob.size, readerFor(blob, ["idx", name + ".mzidx"]), polOf(m.slot), m.query || "");
}
