// @ts-check
// «Draw» of Draw for a SMILES: the molfile layout comes from the Rust engine (`draw` of the molkit .wasm, `draw/1`; the app asks `feat/grafo`, kind "draw";
// `mzlab-cli mol draw --format molfile --set layout=new` gives the same bytes), not from Ketcher's Indigo. Ketcher only ADDS the molfile it is given.
// An engine that is not there is not in the way (`{}`: the caller adds the SMILES as before); a refusal (an invalid SMILES) comes back as its text.
import { loadWasm, message } from "./disegno-verifica.js";

/** The `draw` answer of a request: the native engine in the app, the molkit .wasm on the site.
 * @param {string} request */
async function ask(request) {
  if (/** @type {any} */ (window).MZLAB_DESKTOP) {
    const base = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : "mzlab://localhost/";
    const r = await fetch(base + "rpc", { method: "POST", body: JSON.stringify({ op: "feat/grafo", kind: "draw", request }) });
    const x = await r.text();
    if (r.status !== 200) throw x;
    return x;
  }
  return (await loadWasm()).draw(request);
}

/** The molfile the engine lays out for a SMILES: `{ molfile }`; `{ refused }` (the translated text) when the engine will not draw it; `{}` when the engine is not there.
 * @param {string} smiles @returns {Promise<{ molfile?: string, refused?: string }>} */
export async function layoutSmiles(smiles) {
  let wasm;
  if (!(/** @type {any} */ (window).MZLAB_DESKTOP)) { try { wasm = await loadWasm(); } catch (e) { console.warn("draw/1 engine unavailable, the SMILES goes to Ketcher as it is:", e); return {}; } }
  try { return { molfile: await ask(JSON.stringify({ spec: "smiles:" + smiles, format: "molfile", layout: "new" })) }; }
  catch (e) {
    let refused = false;
    try { refused = !!JSON.parse(String(e)).error_key; } catch (_) { /* not an engine error */ }
    if (refused) return { refused: message(e) };
    console.warn("draw/1 engine unavailable, the SMILES goes to Ketcher as it is:", e);
    return {};
  }
}
