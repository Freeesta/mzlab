// @ts-strict
// `api/rawinfo`, `api/rawxic`, `api/xicions`, `api/specsum`, `api/rawscan`, `api/rawiso` and `api/specpeak` in the worker of the engine (rust-worker.js, op "route"): the .wasm `raw` (crates/mzlab-raw-wasm; `rawxic`, crates/mzlab-rawxic-wasm, for `/api/rawxic`, `/api/xicions` and `/api/specsum`; `rawscan`, crates/mzlab-rawscan-wasm, for `/api/rawscan`; `rawiso`, crates/mzlab-rawiso-wasm, for `/api/rawiso`; `specpeak`, crates/mzlab-specpeak-wasm, for `/api/specpeak`) of wasm-feat/manifest.json
// is fetched on the first request and reads the .mzidx of the file from OPFS itself; the members are those of `mzlab-cli raw info`, the rows those of `raw xic`.
// The engine's .wasm has no room for it (docs/agenti/raw-access.md).

import { load } from "./rust-msngrafo.js";
import * as ARCH from "./archivio.js";
import { ionFacts } from "./rust-filemethod.js";

/**
 * The answer of `Routes::raw_info_route` or, for `/api/rawxic` and `/api/rawscan`, `raw_xic_route` and `raw_scan_route` ("" = no index for the slot); a failure is thrown as the JSON string
 * `{error_key, params}` of the module.
 * @param {{slot: number, path?: string, query?: string}} m
 * `/api/rawxic?ions=1` and `/api/xicions?ions=1` also take the ion-scale values of the file's own `method/1` (`methodOf`: the `lr_metodo` answer `{info: {source, method}}`) for each value the query leaves out, as `mzlab-cli raw xic --ions` does.
 * @param {{nameOf: (slot: number) => string | undefined, polOf: (slot: number) => number, readerFor: (blob: Blob, held?: string[]) => any, methodOf?: (name: string) => Promise<any>}} env
 */
export async function rawInfo(m, { nameOf, polOf, readerFor, methodOf }) {
  const name = nameOf(m.slot), blob = name ? await ARCH.get("idx", name + ".mzidx") : null;
  if (!blob || !name) return "";
  const key = ({ "/api/rawxic": "rawxic", "/api/xicions": "rawxic", "/api/specsum": "rawxic", "/api/rawscan": "rawscan", "/api/rawiso": "rawiso", "/api/specpeak": "specpeak" })[m.path ?? ""] ?? "raw";
  const mod = await load(key);
  const ask = ({ raw: mod.raw_info, rawxic: m.path === "/api/specsum" ? mod.spec_sum : m.path === "/api/xicions" ? mod.xic_ions : mod.raw_xic, rawscan: mod.raw_scan, rawiso: mod.raw_isotopes, specpeak: mod.spec_peak })[key];
  let query = m.query || "";
  if (key === "rawxic" && (m.path === "/api/rawxic" || m.path === "/api/xicions") && methodOf) query = await withMethod(query, () => methodOf(name));
  return ask(blob.size, readerFor(blob, ["idx", name + ".mzidx"]), polOf(m.slot), query);
}

/** The query with the method values from `methodOf` (the `lr_metodo` answer `{info: {source, method}}`, asked only with `ions=1`) for every ion-scale value it leaves out (a value in the query wins); unchanged without `ions=1`. */
async function withMethod(query, methodOf) {
  const q = new URLSearchParams(query);
  if (q.get("ions") !== "1") return query;
  const doc = JSON.parse((await methodOf()).info).method, method = doc ? JSON.stringify(doc) : "";
  const got = JSON.parse(await ionFacts({ method, scan_type: q.get("scan_type") })).query;
  const add = got.split("&").filter(kv => kv && !q.has(kv.split("=")[0]));
  return add.length ? query + (query && query !== "?" ? "&" : "") + add.join("&") : query;
}
