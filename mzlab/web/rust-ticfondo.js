// @ts-strict
// `api/tic_background` in the worker of the engine (rust-worker.js, op "ticbg"): the .wasm `ticfondo` of wasm-feat/manifest.json
// (crates/mzlab-ticfondo-wasm) is fetched on the first request and opens the .mzidx of the file, and of its blank, from OPFS itself.
// The engine's .wasm has no room for it (docs/agenti/tic-fondo.md).

import { load } from "./rust-msngrafo.js";
import * as ARCH from "./archivio.js";

/**
 * The answer of `ticbg_route::route_pair` ("" = not served); a failure is thrown as the JSON string `{error_key, params}` of the module.
 * @param {{slot: number, blank?: number, query?: string}} m
 * @param {{nameOf: (slot: number) => string | undefined, polOf: (slot: number) => number, readerFor: (blob: Blob, held?: string[]) => any}} env
 */
export async function ticFondo(m, { nameOf, polOf, readerFor }) {
  const idx = async (/** @type {number} */ slot) => {
    const name = nameOf(slot), blob = name ? await ARCH.get("idx", name + ".mzidx") : null;
    return blob ? { size: blob.size, read: readerFor(blob, ["idx", name + ".mzidx"]), pol: polOf(slot) } : null;
  };
  const [mod, f, b] = await Promise.all([load("ticfondo"), idx(m.slot), (m.blank ?? -1) >= 0 ? idx(Number(m.blank)) : null]);
  if (!f) return "";
  return mod.tic_background(f.size, f.read, f.pol, b ? b.size : 0, b ? b.read : null, b ? b.pol : 0, m.query || "");
}
