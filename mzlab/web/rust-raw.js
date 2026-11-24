// @ts-strict
// `api/rawinfo` in the worker of the engine (rust-worker.js, op "route"): the .wasm `raw` of wasm-feat/manifest.json (crates/mzlab-raw-wasm)
// is fetched on the first request and reads the .mzidx of the file from OPFS itself; the members are those of `mzlab-cli raw info`.
// The engine's .wasm has no room for it (docs/agenti/raw-access.md).

import { load } from "./rust-msngrafo.js";
import * as ARCH from "./archivio.js";

/**
 * The answer of `Routes::raw_info_route` ("" = no index for the slot); a failure is thrown as the JSON string `{error_key, params}` of the module.
 * @param {{slot: number, query?: string}} m
 * @param {{nameOf: (slot: number) => string | undefined, polOf: (slot: number) => number, readerFor: (blob: Blob, held?: string[]) => any}} env
 */
export async function rawInfo(m, { nameOf, polOf, readerFor }) {
  const name = nameOf(m.slot), blob = name ? await ARCH.get("idx", name + ".mzidx") : null;
  if (!blob || !name) return "";
  const mod = await load("raw");
  return mod.raw_info(blob.size, readerFor(blob, ["idx", name + ".mzidx"]), polOf(m.slot), m.query || "");
}
