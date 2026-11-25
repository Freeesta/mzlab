// @ts-strict
// `api/rawinfo`, `api/rawxic`, `api/specsum` and `api/rawscan` in the worker of the engine (rust-worker.js, op "route"): the .wasm `raw` (crates/mzlab-raw-wasm; `rawxic`, crates/mzlab-rawxic-wasm, for `/api/rawxic` and `/api/specsum`; `rawscan`, crates/mzlab-rawscan-wasm, for `/api/rawscan`) of wasm-feat/manifest.json
// is fetched on the first request and reads the .mzidx of the file from OPFS itself; the members are those of `mzlab-cli raw info`, the rows those of `raw xic`.
// The engine's .wasm has no room for it (docs/agenti/raw-access.md).

import { load } from "./rust-msngrafo.js";
import * as ARCH from "./archivio.js";

/**
 * The answer of `Routes::raw_info_route` or, for `/api/rawxic` and `/api/rawscan`, `raw_xic_route` and `raw_scan_route` ("" = no index for the slot); a failure is thrown as the JSON string
 * `{error_key, params}` of the module.
 * @param {{slot: number, path?: string, query?: string}} m
 * @param {{nameOf: (slot: number) => string | undefined, polOf: (slot: number) => number, readerFor: (blob: Blob, held?: string[]) => any}} env
 */
export async function rawInfo(m, { nameOf, polOf, readerFor }) {
  const name = nameOf(m.slot), blob = name ? await ARCH.get("idx", name + ".mzidx") : null;
  if (!blob || !name) return "";
  const key = ({ "/api/rawxic": "rawxic", "/api/specsum": "rawxic", "/api/rawscan": "rawscan" })[m.path ?? ""] ?? "raw";
  const mod = await load(key);
  const ask = ({ raw: mod.raw_info, rawxic: m.path === "/api/specsum" ? mod.spec_sum : mod.raw_xic, rawscan: mod.raw_scan })[key];
  return ask(blob.size, readerFor(blob, ["idx", name + ".mzidx"]), polOf(m.slot), m.query || "");
}
